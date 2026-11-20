use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DhtSettings {
    pub enabled: bool,
}

impl Default for DhtSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub struct DhtStore {
    path: PathBuf,
    settings: Mutex<DhtSettings>,
}

impl DhtStore {
    pub fn load(path: PathBuf) -> Result<Self> {
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    path,
                    settings: Mutex::new(DhtSettings::default()),
                })
            }
            Err(err) => {
                return Err(err).with_context(|| format!("could not read {}", path.display()))
            }
        };
        let settings = serde_json::from_str::<DhtSettings>(raw.trim_start_matches('\u{feff}'))
            .unwrap_or_else(|err| {
                warn!("ignoring corrupt DHT settings: {err}");
                DhtSettings::default()
            });
        Ok(Self {
            path,
            settings: Mutex::new(settings),
        })
    }

    pub fn settings(&self) -> DhtSettings {
        *self.settings.lock().expect("DHT settings mutex poisoned")
    }

    pub fn update(&self, settings: DhtSettings) -> Result<()> {
        let data = serde_json::to_vec_pretty(&settings)
            .context("could not serialize DHT settings")?;
        fs::write(&self.path, data)
            .with_context(|| format!("could not write {}", self.path.display()))?;
        *self.settings.lock().expect("DHT settings mutex poisoned") = settings;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_enables_dht() {
        assert!(DhtSettings::default().enabled);
    }

    #[test]
    fn loads_bom_prefixed_disabled_setting() {
        let dir = std::env::temp_dir().join(format!(
            "nordtorrent-dht-test-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("dht.json");
        fs::write(&path, "\u{feff}{\"enabled\":false}").unwrap();
        let store = DhtStore::load(path).unwrap();
        assert!(!store.settings().enabled);
        let _ = fs::remove_dir_all(dir);
    }
}
