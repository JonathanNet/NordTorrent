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
    /// Optional `#rrggbb` colour per label. Labels without a colour are absent,
    /// so the web panel keeps its neutral default styling for them.
    pub colors: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LabelEntry {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LabelValue {
    Entry(LabelEntry),
    Name(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct LabelsFile {
    version: u8,
    #[serde(default, deserialize_with = "deserialize_labels")]
    labels: Vec<LabelEntry>,
    #[serde(default)]
    assignments: HashMap<String, String>,
}

impl Default for LabelsFile {
    fn default() -> Self {
        Self {
            version: 2,
            labels: Vec::new(),
            assignments: HashMap::new(),
        }
    }
}

/// Accepts both the old format (`["Movies"]`) and the current one
/// (`[{"name": "Movies", "color": "#c6f45c"}]`) so existing data keeps working.
fn deserialize_labels<'de, D>(deserializer: D) -> std::result::Result<Vec<LabelEntry>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<LabelValue>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .map(|value| match value {
            LabelValue::Entry(entry) => LabelEntry {
                name: entry.name,
                color: entry.color.and_then(|color| normalize_color(&color).ok()),
            },
            LabelValue::Name(name) => LabelEntry { name, color: None },
        })
        .filter(|entry| !entry.name.trim().is_empty())
        .collect())
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
        view_of(&state)
    }

    pub fn create(&self, name: &str, color: Option<&str>) -> Result<LabelsView> {
        let name = normalize_label(name)?;
        let color = normalize_color_opt(color)?;
        let mut state = self.state.lock().expect("label mutex poisoned");
        if state
            .labels
            .iter()
            .any(|label| label.name.eq_ignore_ascii_case(&name))
        {
            bail!("Labelen findes allerede.");
        }
        state.labels.push(LabelEntry { name, color });
        state
            .labels
            .sort_by_key(|label| label.name.to_lowercase());
        save_file(&self.path, &state)?;
        Ok(view_of(&state))
    }

    pub fn set_color(&self, name: &str, color: Option<&str>) -> Result<LabelsView> {
        let name = normalize_label(name)?;
        let color = normalize_color_opt(color)?;
        let mut state = self.state.lock().expect("label mutex poisoned");
        let entry = state
            .labels
            .iter_mut()
            .find(|label| label.name.eq_ignore_ascii_case(&name))
            .with_context(|| "Labelen findes ikke.")?;
        entry.color = color;
        save_file(&self.path, &state)?;
        Ok(view_of(&state))
    }

    pub fn remove(&self, name: &str) -> Result<LabelsView> {
        let name = normalize_label(name)?;
        let mut state = self.state.lock().expect("label mutex poisoned");
        let before = state.labels.len();
        state
            .labels
            .retain(|label| !label.name.eq_ignore_ascii_case(&name));
        if state.labels.len() == before {
            bail!("Labelen findes ikke.");
        }
        state
            .assignments
            .retain(|_, label| !label.eq_ignore_ascii_case(&name));
        save_file(&self.path, &state)?;
        Ok(view_of(&state))
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
                .any(|existing| existing.name.eq_ignore_ascii_case(label))
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

fn view_of(state: &LabelsFile) -> LabelsView {
    LabelsView {
        labels: state.labels.iter().map(|label| label.name.clone()).collect(),
        assignments: state.assignments.clone(),
        colors: state
            .labels
            .iter()
            .filter_map(|label| {
                label
                    .color
                    .as_ref()
                    .map(|color| (label.name.clone(), color.clone()))
            })
            .collect(),
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

fn normalize_color_opt(color: Option<&str>) -> Result<Option<String>> {
    match color.map(str::trim).filter(|color| !color.is_empty()) {
        Some(color) => Ok(Some(normalize_color(color)?)),
        None => Ok(None),
    }
}

/// Accepts `#rgb`, `#rrggbb`, `rgb` or `rrggbb` and returns `#rrggbb` in lower
/// case. Anything else is rejected so the value can be used in CSS safely.
fn normalize_color(color: &str) -> Result<String> {
    let hex = color.trim().trim_start_matches('#');
    let expanded = match hex.len() {
        3 => hex
            .chars()
            .flat_map(|c| [c, c])
            .collect::<String>()
            .to_lowercase(),
        6 => hex.to_lowercase(),
        _ => bail!("Farven skal være en hex-farve som #c6f45c."),
    };
    if !expanded.chars().all(|c| c.is_ascii_hexdigit()) {
        bail!("Farven skal være en hex-farve som #c6f45c.");
    }
    Ok(format!("#{expanded}"))
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
        store.create("Film", None).unwrap();
        store.assign("abc", Some("Film")).unwrap();
        assert_eq!(store.view().assignments.get("abc").unwrap(), "Film");
        store.remove("film").unwrap();
        assert!(store.view().assignments.is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_duplicate_and_invalid_labels() {
        let (path, store) = store();
        store.create("Serier", None).unwrap();
        assert!(store.create("serier", None).is_err());
        assert!(store.create("", None).is_err());
        assert!(store.create(&"x".repeat(41), None).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn stores_and_clears_label_colors() {
        let (path, store) = store();
        store.create("Film", Some("#C6F45C")).unwrap();
        assert_eq!(store.view().colors.get("Film").unwrap(), "#c6f45c");
        store.set_color("film", Some("#7d3")).unwrap();
        assert_eq!(store.view().colors.get("Film").unwrap(), "#77dd33");
        store.set_color("Film", None).unwrap();
        assert!(store.view().colors.is_empty());
        assert!(store.create("Bad", Some("not-a-color")).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn reads_the_legacy_label_list() {
        let path = std::env::temp_dir().join(format!(
            "nordtorrent-labels-legacy-{}-{}.json",
            std::process::id(),
            rand::random::<u64>()
        ));
        fs::write(&path, r#"{"version":1,"labels":["Film","Serier"],"assignments":{}}"#).unwrap();
        let store = LabelStore::load(path.clone()).unwrap();
        let view = store.view();
        assert_eq!(view.labels, vec!["Film", "Serier"]);
        assert!(view.colors.is_empty());
        let _ = fs::remove_file(path);
    }
}
