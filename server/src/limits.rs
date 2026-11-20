use std::{
    fs,
    num::NonZeroU32,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{bail, Context, Result};
use librqbit::limits::LimitsConfig;
use serde::{Deserialize, Serialize};
use tracing::warn;

const DEFAULT_DOWNLOAD_KBPS: u32 = 1_000;
const DEFAULT_UPLOAD_KBPS: u32 = 200;
const MAX_KBPS: u32 = u32::MAX / 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SpeedLimitSettings {
    pub download_enabled: bool,
    pub download_kbps: u32,
    pub upload_enabled: bool,
    pub upload_kbps: u32,
}

impl Default for SpeedLimitSettings {
    fn default() -> Self {
        Self {
            download_enabled: false,
            download_kbps: DEFAULT_DOWNLOAD_KBPS,
            upload_enabled: false,
            upload_kbps: DEFAULT_UPLOAD_KBPS,
        }
    }
}

impl SpeedLimitSettings {
    pub fn validate(&self) -> Result<()> {
        for (enabled, kbps, label) in [
            (self.download_enabled, self.download_kbps, "Download"),
            (self.upload_enabled, self.upload_kbps, "Upload"),
        ] {
            if kbps == 0 || kbps > MAX_KBPS {
                bail!("{label}-hastigheden skal være mellem 1 og {MAX_KBPS} KB/s.");
            }
            let _ = enabled;
        }
        Ok(())
    }

    pub fn to_limits_config(&self) -> LimitsConfig {
        LimitsConfig {
            download_bps: self
                .download_enabled
                .then(|| NonZeroU32::new(self.download_kbps.saturating_mul(1024)))
                .flatten(),
            upload_bps: self
                .upload_enabled
                .then(|| NonZeroU32::new(self.upload_kbps.saturating_mul(1024)))
                .flatten(),
        }
    }
}

pub struct RateLimitStore {
    path: PathBuf,
    settings: Mutex<SpeedLimitSettings>,
}

impl RateLimitStore {
    pub fn load(path: PathBuf) -> Result<Self> {
        let settings = read_settings(&path)?;
        Ok(Self {
            path,
            settings: Mutex::new(settings),
        })
    }

    pub fn settings(&self) -> SpeedLimitSettings {
        *self.settings.lock().expect("speed limit mutex poisoned")
    }

    pub fn limits_config(&self) -> LimitsConfig {
        self.settings().to_limits_config()
    }

    pub fn update(&self, settings: SpeedLimitSettings) -> Result<()> {
        settings.validate()?;
        let serialized = serde_json::to_vec_pretty(&settings)
            .context("could not serialize speed limit settings")?;
        fs::write(&self.path, serialized)
            .with_context(|| format!("could not write {}", self.path.display()))?;
        *self.settings.lock().expect("speed limit mutex poisoned") = settings;
        Ok(())
    }
}

fn read_settings(path: &Path) -> Result<SpeedLimitSettings> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SpeedLimitSettings::default());
        }
        Err(err) => return Err(err).with_context(|| format!("could not read {}", path.display())),
    };
    let raw = raw.trim_start_matches('\u{feff}');
    match serde_json::from_str::<SpeedLimitSettings>(raw) {
        Ok(settings) if settings.validate().is_ok() => Ok(settings),
        Ok(settings) => {
            warn!(?settings, "ignoring invalid speed limit settings");
            Ok(SpeedLimitSettings::default())
        }
        Err(err) => {
            warn!("ignoring corrupt speed limit settings: {err}");
            Ok(SpeedLimitSettings::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_unlimited() {
        let settings = SpeedLimitSettings::default();
        let limits = settings.to_limits_config();
        assert!(limits.download_bps.is_none());
        assert!(limits.upload_bps.is_none());
    }

    #[test]
    fn converts_kbps_to_bps_when_enabled() {
        let settings = SpeedLimitSettings {
            download_enabled: true,
            download_kbps: 1_000,
            upload_enabled: true,
            upload_kbps: 200,
        };
        let limits = settings.to_limits_config();
        assert_eq!(limits.download_bps.unwrap().get(), 1_024_000);
        assert_eq!(limits.upload_bps.unwrap().get(), 204_800);
    }

    #[test]
    fn rejects_zero_and_too_large_values() {
        let mut settings = SpeedLimitSettings::default();
        settings.download_kbps = 0;
        assert!(settings.validate().is_err());
        settings.download_kbps = MAX_KBPS + 1;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn loads_bom_prefixed_file() {
        let dir = std::env::temp_dir().join(format!(
            "nordtorrent-limits-test-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("limits.json");
        fs::write(
            &path,
            "\u{feff}{\"download_enabled\":true,\"download_kbps\":42,\"upload_enabled\":false,\"upload_kbps\":7}",
        )
        .unwrap();
        let store = RateLimitStore::load(path.clone()).unwrap();
        let settings = store.settings();
        assert!(settings.download_enabled);
        assert_eq!(settings.download_kbps, 42);
        assert!(!settings.upload_enabled);
        let _ = fs::remove_dir_all(dir);
    }
}
