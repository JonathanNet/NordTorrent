use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::torrents::{TorrentDetailResponse, TorrentListResponse};

#[derive(Debug, Clone, Serialize)]
pub struct LabelsView {
    pub labels: Vec<String>,
    pub assignments: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LabelsFile {
    version: u8,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    assignments: HashMap<String, String>,
}

impl Default for LabelsFile {
    fn default() -> Self {
        Self {
            version: 1,
            labels: Vec::new(),
            assignments: HashMap::new(),
        }
    }
}

pub struct LabelStore {
    path: PathBuf,
    state: Mutex<LabelsFile>,
}

impl LabelStore {
    pub fn load(path: PathBuf) -> Result<Self> {
        let state = read_file(&path)?;
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    pub fn view(&self) -> LabelsView {
        let state = self.state.lock().expect("label mutex poisoned");
        LabelsView {
            labels: state.labels.clone(),
            assignments: state.assignments.clone(),
        }
    }

    pub fn create(&self, name: &str) -> Result<LabelsView> {
        let name = normalize_label(name)?;
        let mut state = self.state.lock().expect("label mutex poisoned");
        if state
            .labels
            .iter()
            .any(|label| label.eq_ignore_ascii_case(&name))
        {
            bail!("Labelen findes allerede.");
        }
        state.labels.push(name);
        state.labels.sort_by_key(|label| label.to_lowercase());
        save_file(&self.path, &state)?;
        Ok(LabelsView {
            labels: state.labels.clone(),
            assignments: state.assignments.clone(),
        })
    }

    pub fn remove(&self, name: &str) -> Result<LabelsView> {
        let name = normalize_label(name)?;
        let mut state = self.state.lock().expect("label mutex poisoned");
        let before = state.labels.len();
        state.labels.retain(|label| !label.eq_ignore_ascii_case(&name));
        if state.labels.len() == before {
            bail!("Labelen findes ikke.");
        }
        state.assignments.retain(|_, label| !label.eq_ignore_ascii_case(&name));
        save_file(&self.path, &state)?;
        Ok(LabelsView {
            labels: state.labels.clone(),
            assignments: state.assignments.clone(),
        })
    }

    pub fn assign(&self, info_hash: &str, label: Option<&str>) -> Result<()> {
        let label = label
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(normalize_label)
            .transpose()?;
        let mut state = self.state.lock().expect("label mutex poisoned");
        if let Some(label) = &label {
            if !state
                .labels
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(label))
            {
                bail!("Labelen findes ikke.");
            }
        }
        if let Some(label) = label {
            state.assignments.insert(info_hash.to_owned(), label);
        } else {
            state.assignments.remove(info_hash);
        }
        save_file(&self.path, &state)?;
        Ok(())
    }

    pub fn forget(&self, info_hash: &str) {
        let mut state = self.state.lock().expect("label mutex poisoned");
        if state.assignments.remove(info_hash).is_some() {
            if let Err(err) = save_file(&self.path, &state) {
                warn!("could not persist labels: {err:#}");
            }
        }
    }

    pub fn enrich_list(&self, response: &mut TorrentListResponse) {
        let state = self.state.lock().expect("label mutex poisoned");
        for torrent in &mut response.torrents {
            torrent.label = state.assignments.get(&torrent.info_hash).cloned();
        }
    }

    pub fn enrich_detail(&self, detail: &mut TorrentDetailResponse) {
        detail.summary.label = self
            .state
            .lock()
            .expect("label mutex poisoned")
            .assignments
            .get(&detail.summary.info_hash)
            .cloned();
    }
}

fn normalize_label(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        bail!("Labelnavnet skal være mellem 1 og 40 tegn.");
    }
    if name.chars().any(char::is_control) {
        bail!("Labelnavnet indeholder ugyldige tegn.");
    }
    Ok(name.to_owned())
}

fn read_file(path: &Path) -> Result<LabelsFile> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LabelsFile::default())
        }
        Err(err) => return Err(err).with_context(|| format!("could not read {}", path.display())),
    };
    let raw = raw.trim_start_matches('\u{feff}');
    Ok(serde_json::from_str(raw).unwrap_or_else(|err| {
        warn!("ignoring corrupt label file: {err}");
        LabelsFile::default()
    }))
}

fn save_file(path: &Path, state: &LabelsFile) -> Result<()> {
    let data = serde_json::to_vec_pretty(state).context("could not serialize labels")?;
    fs::write(path, data).with_context(|| format!("could not write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (std::path::PathBuf, LabelStore) {
        let path = std::env::temp_dir().join(format!(
            "nordtorrent-labels-test-{}-{}.json",
            std::process::id(),
            rand::random::<u64>()
        ));
        (path.clone(), LabelStore::load(path).unwrap())
    }

    #[test]
    fn creates_assigns_and_removes_labels() {
        let (path, store) = store();
        store.create("Film").unwrap();
        store.assign("abc", Some("Film")).unwrap();
        assert_eq!(store.view().assignments.get("abc").unwrap(), "Film");
        store.remove("film").unwrap();
        assert!(store.view().assignments.is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_duplicate_and_invalid_labels() {
        let (path, store) = store();
        store.create("Serier").unwrap();
        assert!(store.create("serier").is_err());
        assert!(store.create("").is_err());
        assert!(store.create(&"x".repeat(41)).is_err());
        let _ = fs::remove_file(path);
    }
}
