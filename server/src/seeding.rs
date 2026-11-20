use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use librqbit::{api::TorrentIdOrHash, Api};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::torrents::{list_torrents, TorrentSummary};

const MAX_STOP_RATIO: f64 = 1000.0;
const MAX_STOP_MINUTES: u64 = 525_600;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SeedingSettings {
    pub keep_seeding: bool,
    pub stop_ratio: f64,
    pub stop_minutes: u64,
}

impl Default for SeedingSettings {
    fn default() -> Self {
        Self {
            // Preserve the existing behaviour for upgrades: seed until the
            // user explicitly asks for a limit.
            keep_seeding: true,
            stop_ratio: 0.0,
            stop_minutes: 0,
        }
    }
}

impl SeedingSettings {
    pub fn validate(&self) -> Result<()> {
        if !self.stop_ratio.is_finite()
            || self.stop_ratio < 0.0
            || self.stop_ratio > MAX_STOP_RATIO
        {
            bail!("Stop-ratio skal være mellem 0 og {MAX_STOP_RATIO}.");
        }
        if self.stop_minutes > MAX_STOP_MINUTES {
            bail!("Stop-tid skal være mellem 0 og {MAX_STOP_MINUTES} minutter.");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedingRecord {
    completed_at: i64,
    #[serde(default)]
    stopped: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SeedingFile {
    version: u8,
    settings: SeedingSettings,
    #[serde(default)]
    torrents: HashMap<String, SeedingRecord>,
}

impl Default for SeedingFile {
    fn default() -> Self {
        Self {
            version: 1,
            settings: SeedingSettings::default(),
            torrents: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PauseTarget {
    pub id: usize,
    pub info_hash: String,
    pub reason: &'static str,
}

pub struct SeedingStore {
    path: PathBuf,
    state: Mutex<SeedingFile>,
}

impl SeedingStore {
    pub fn load(path: PathBuf) -> Result<Self> {
        let state = read_file(&path)?;
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    pub fn settings(&self) -> SeedingSettings {
        self.state
            .lock()
            .expect("seeding mutex poisoned")
            .settings
    }

    pub fn update_settings(&self, settings: SeedingSettings) -> Result<()> {
        settings.validate()?;
        let mut state = self.state.lock().expect("seeding mutex poisoned");
        state.settings = settings;
        save_file(&self.path, &state)?;
        Ok(())
    }

    pub fn plan(&self, torrents: &[TorrentSummary], now: i64) -> Vec<PauseTarget> {
        let mut state = self.state.lock().expect("seeding mutex poisoned");
        let settings = state.settings;
        let mut dirty = false;
        let mut live = HashSet::new();
        let mut targets = Vec::new();

        for torrent in torrents {
            if !torrent.finished {
                dirty |= state.torrents.remove(&torrent.info_hash).is_some();
                continue;
            }
            live.insert(torrent.info_hash.clone());
            if torrent.state == "paused" {
                continue;
            }
            let is_new = !state.torrents.contains_key(&torrent.info_hash);
            let record = state
                .torrents
                .entry(torrent.info_hash.clone())
                .or_insert(SeedingRecord {
                    completed_at: now,
                    stopped: false,
                });
            dirty |= is_new;
            if record.stopped {
                continue;
            }
            if let Some(reason) = stop_reason(&settings, record, torrent, now) {
                targets.push(PauseTarget {
                    id: torrent.id,
                    info_hash: torrent.info_hash.clone(),
                    reason,
                });
            }
        }
        let before_retain = state.torrents.len();
        state.torrents.retain(|hash, _| live.contains(hash));
        dirty |= state.torrents.len() != before_retain;

        if dirty {
            if let Err(err) = save_file(&self.path, &state) {
                warn!("could not persist seeding state: {err:#}");
            }
        }
        targets
    }

    pub fn mark_stopped(&self, info_hash: &str) {
        let mut state = self.state.lock().expect("seeding mutex poisoned");
        let Some(record) = state.torrents.get_mut(info_hash) else {
            return;
        };
        if record.stopped {
            return;
        }
        record.stopped = true;
        if let Err(err) = save_file(&self.path, &state) {
            warn!("could not persist stopped seeding state: {err:#}");
        }
    }

    pub fn spawn_monitor(self: Arc<Self>, api: Api, download_dir: PathBuf) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(20));
            loop {
                interval.tick().await;
                let response = list_torrents(&api, &download_dir);
                let targets = self.plan(&response.torrents, Utc::now().timestamp());
                for target in targets {
                    match api
                        .api_torrent_action_pause(TorrentIdOrHash::Id(target.id))
                        .await
                    {
                        Ok(_) => {
                            info!(
                                id = target.id,
                                info_hash = target.info_hash,
                                reason = target.reason,
                                "seeding limit reached; torrent paused"
                            );
                            self.mark_stopped(&target.info_hash);
                        }
                        Err(err) => {
                            warn!(id = target.id, "could not pause seeded torrent: {err:#}")
                        }
                    }
                }
            }
        });
    }
}

fn stop_reason(
    settings: &SeedingSettings,
    record: &SeedingRecord,
    torrent: &TorrentSummary,
    now: i64,
) -> Option<&'static str> {
    if !settings.keep_seeding {
        return Some("keep seeding disabled");
    }
    if settings.stop_ratio > 0.0 && torrent.ratio >= settings.stop_ratio {
        return Some("ratio reached");
    }
    if settings.stop_minutes > 0 {
        let stop_after = (settings.stop_minutes as i64).saturating_mul(60);
        if now.saturating_sub(record.completed_at) >= stop_after {
            return Some("time reached");
        }
    }
    None
}

fn read_file(path: &Path) -> Result<SeedingFile> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SeedingFile::default())
        }
        Err(err) => return Err(err).with_context(|| format!("could not read {}", path.display())),
    };
    let raw = raw.trim_start_matches('\u{feff}');
    let mut file = match serde_json::from_str::<SeedingFile>(raw) {
        Ok(file) => file,
        Err(err) => {
            warn!("ignoring corrupt seeding settings: {err}");
            return Ok(SeedingFile::default());
        }
    };
    if file.settings.validate().is_err() {
        warn!("ignoring invalid seeding settings");
        file.settings = SeedingSettings::default();
    }
    Ok(file)
}

fn save_file(path: &Path, state: &SeedingFile) -> Result<()> {
    let data = serde_json::to_vec_pretty(state).context("could not serialize seeding state")?;
    fs::write(path, data).with_context(|| format!("could not write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_seeding() {
        let settings = SeedingSettings::default();
        assert!(settings.keep_seeding);
        assert_eq!(settings.stop_ratio, 0.0);
        assert_eq!(settings.stop_minutes, 0);
    }

    #[test]
    fn validates_ratio_and_time() {
        let mut settings = SeedingSettings::default();
        settings.stop_ratio = -1.0;
        assert!(settings.validate().is_err());
        settings.stop_ratio = MAX_STOP_RATIO + 1.0;
        assert!(settings.validate().is_err());
        settings.stop_ratio = 0.0;
        settings.stop_minutes = MAX_STOP_MINUTES + 1;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn stops_when_keep_seeding_is_disabled() {
        let settings = SeedingSettings {
            keep_seeding: false,
            ..Default::default()
        };
        let record = SeedingRecord {
            completed_at: 0,
            stopped: false,
        };
        let torrent = fake_torrent(0.0);
        assert_eq!(
            stop_reason(&settings, &record, &torrent, 10),
            Some("keep seeding disabled")
        );
    }

    #[test]
    fn stops_at_ratio_or_time() {
        let record = SeedingRecord {
            completed_at: 100,
            stopped: false,
        };
        let ratio_settings = SeedingSettings {
            keep_seeding: true,
            stop_ratio: 1.5,
            stop_minutes: 0,
        };
        assert_eq!(
            stop_reason(&ratio_settings, &record, &fake_torrent(1.5), 200),
            Some("ratio reached")
        );
        let time_settings = SeedingSettings {
            keep_seeding: true,
            stop_ratio: 0.0,
            stop_minutes: 2,
        };
        assert_eq!(
            stop_reason(&time_settings, &record, &fake_torrent(0.0), 220),
            Some("time reached")
        );
        assert_eq!(
            stop_reason(&time_settings, &record, &fake_torrent(0.0), 219),
            None
        );
    }

    fn fake_torrent(ratio: f64) -> TorrentSummary {
        TorrentSummary {
            id: 1,
            info_hash: "hash".to_owned(),
            name: "Test".to_owned(),
            state: "seeding",
            label: None,
            folder: String::new(),
            output_folder: String::new(),
            progress: 100.0,
            progress_bytes: 1,
            total_bytes: 1,
            uploaded_bytes: 1,
            download_speed_bps: 0.0,
            upload_speed_bps: 0.0,
            eta_seconds: None,
            ratio,
            finished: true,
            error: None,
            media_files: 0,
            total_files: 1,
            file_progress: Vec::new(),
        }
    }
}
