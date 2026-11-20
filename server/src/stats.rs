use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::Local;
use librqbit::Api;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::torrents::{
    list_torrents, TorrentDetailResponse, TorrentListResponse, TorrentSummary,
};

/// Lifetime counters are stored separately from librqbit's session state.  The
/// engine's uploaded counter is per process and resets on restart; the bitmap
/// only preserves which pieces are present.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct StoredStat {
    uploaded_bytes: u64,
    downloaded_bytes: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct MonthTorrentStat {
    uploaded_bytes: u64,
    downloaded_bytes: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct MonthStat {
    uploaded_bytes: u64,
    downloaded_bytes: u64,
    #[serde(default)]
    torrents: HashMap<String, MonthTorrentStat>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StatsFile {
    version: u8,
    torrents: HashMap<String, StoredStat>,
    #[serde(default)]
    months: HashMap<String, MonthStat>,
}

#[derive(Debug, Serialize)]
pub struct MonthlyStatView {
    pub month: String,
    pub downloaded_bytes: u64,
    pub uploaded_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct TopTorrentView {
    pub info_hash: String,
    pub name: String,
    pub uploaded_bytes: u64,
    pub downloaded_bytes: u64,
    pub ratio: f64,
}

#[derive(Debug, Serialize)]
pub struct StatsOverview {
    pub month: String,
    pub downloaded_bytes: u64,
    pub uploaded_bytes: u64,
    pub ratio: f64,
    pub torrent_count: usize,
    pub seeding_count: usize,
    pub all_time_downloaded_bytes: u64,
    pub all_time_uploaded_bytes: u64,
    pub months: Vec<MonthlyStatView>,
    pub top_torrents: Vec<TopTorrentView>,
}

pub struct StatsStore {
    path: PathBuf,
    /// Counters loaded at process start. Live engine counters are added to
    /// these without changing the base.
    base: RwLock<HashMap<String, StoredStat>>,
    /// Latest cumulative values, written to disk periodically and on shutdown.
    latest: Mutex<HashMap<String, StoredStat>>,
    /// Per-month deltas, keyed by `YYYY-MM`.
    months: Mutex<HashMap<String, MonthStat>>,
}

impl StatsStore {
    pub fn load(path: PathBuf) -> Result<Arc<Self>> {
        let (base, latest, months) = if path.exists() {
            let bytes = std::fs::read(&path)
                .with_context(|| format!("could not read stats {}", path.display()))?;
            let bytes = bytes
                .strip_prefix(&[0xEF, 0xBB, 0xBF])
                .unwrap_or(&bytes);
            match serde_json::from_slice::<StatsFile>(bytes) {
                Ok(file) if file.version == 1 || file.version == 2 => {
                    (file.torrents.clone(), file.torrents, file.months)
                }
                Ok(file) => {
                    warn!(
                        "ignoring unsupported stats version {} in {}",
                        file.version,
                        path.display()
                    );
                    (HashMap::new(), HashMap::new(), HashMap::new())
                }
                Err(err) => {
                    warn!(
                        "ignoring invalid stats file {}: {err}",
                        path.display()
                    );
                    (HashMap::new(), HashMap::new(), HashMap::new())
                }
            }
        } else {
            (HashMap::new(), HashMap::new(), HashMap::new())
        };
        Ok(Arc::new(Self {
            path,
            base: RwLock::new(base),
            latest: Mutex::new(latest),
            months: Mutex::new(months),
        }))
    }

    pub fn enrich_list(&self, response: &mut TorrentListResponse) {
        self.enrich(&mut response.torrents);
        response.totals.uploaded_bytes = response
            .torrents
            .iter()
            .map(|torrent| torrent.uploaded_bytes)
            .sum();
        response.totals.downloaded_bytes = response
            .torrents
            .iter()
            .map(|torrent| torrent.progress_bytes)
            .sum();
    }

    pub fn enrich_detail(&self, detail: &mut TorrentDetailResponse) {
        self.enrich(std::slice::from_mut(&mut detail.summary));
    }

    fn enrich(&self, torrents: &mut [TorrentSummary]) {
        let base = self
            .base
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for torrent in torrents {
            let Some(stored) = base.get(&torrent.info_hash.to_ascii_lowercase()) else {
                continue;
            };
            let downloaded = torrent.progress_bytes.max(stored.downloaded_bytes);
            torrent.uploaded_bytes = stored
                .uploaded_bytes
                .saturating_add(torrent.uploaded_bytes);
            torrent.ratio = if downloaded > 0 {
                torrent.uploaded_bytes as f64 / downloaded as f64
            } else {
                0.0
            };
        }
    }

    fn record(&self, torrents: &[TorrentSummary]) {
        let base = self
            .base
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut latest = self
            .latest
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut months = self
            .months
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let month_key = Local::now().format("%Y-%m").to_string();
        let month = months.entry(month_key).or_default();
        for torrent in torrents {
            let key = torrent.info_hash.to_ascii_lowercase();
            let previous_base = base.get(&key).cloned().unwrap_or_default();
            let previous_total = latest.get(&key).cloned().unwrap_or(previous_base.clone());
            let current = StoredStat {
                uploaded_bytes: previous_base
                    .uploaded_bytes
                    .saturating_add(torrent.uploaded_bytes),
                downloaded_bytes: previous_base
                    .downloaded_bytes
                    .max(torrent.progress_bytes),
            };
            let uploaded_delta = current
                .uploaded_bytes
                .saturating_sub(previous_total.uploaded_bytes);
            let downloaded_delta = current
                .downloaded_bytes
                .saturating_sub(previous_total.downloaded_bytes);
            if uploaded_delta > 0 || downloaded_delta > 0 {
                {
                    let torrent_month = month.torrents.entry(key.clone()).or_default();
                    torrent_month.uploaded_bytes = torrent_month
                        .uploaded_bytes
                        .saturating_add(uploaded_delta);
                    torrent_month.downloaded_bytes = torrent_month
                        .downloaded_bytes
                        .saturating_add(downloaded_delta);
                }
                month.uploaded_bytes = month.uploaded_bytes.saturating_add(uploaded_delta);
                month.downloaded_bytes = month
                    .downloaded_bytes
                    .saturating_add(downloaded_delta);
            }
            latest.insert(key, current);
        }
    }

    fn persist(&self) -> Result<()> {
        let latest = self
            .latest
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let months = self
            .months
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let file = StatsFile {
            version: 2,
            torrents: latest.clone(),
            months: months.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&file)?;
        crate::auth::atomic_write(&self.path, &bytes)
    }

    pub fn overview(&self, torrents: &[TorrentSummary]) -> StatsOverview {
        let month_key = Local::now().format("%Y-%m").to_string();
        let latest = self
            .latest
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let months = self
            .months
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let current = months.get(&month_key).cloned().unwrap_or_default();

        let mut month_points: Vec<MonthlyStatView> = months
            .iter()
            .map(|(month, stat)| MonthlyStatView {
                month: month.clone(),
                downloaded_bytes: stat.downloaded_bytes,
                uploaded_bytes: stat.uploaded_bytes,
            })
            .collect();
        month_points.sort_by(|left, right| left.month.cmp(&right.month));
        if month_points.len() > 6 {
            month_points.drain(0..month_points.len() - 6);
        }

        let name_for = |hash: &str| {
            torrents
                .iter()
                .find(|torrent| torrent.info_hash.eq_ignore_ascii_case(hash))
                .map(|torrent| torrent.name.clone())
                .unwrap_or_else(|| hash.chars().take(12).collect())
        };
        let mut top: Vec<TopTorrentView> = current
            .torrents
            .iter()
            .map(|(hash, stat)| {
                let ratio = if stat.downloaded_bytes > 0 {
                    stat.uploaded_bytes as f64 / stat.downloaded_bytes as f64
                } else {
                    0.0
                };
                TopTorrentView {
                    info_hash: hash.clone(),
                    name: name_for(hash),
                    uploaded_bytes: stat.uploaded_bytes,
                    downloaded_bytes: stat.downloaded_bytes,
                    ratio,
                }
            })
            .collect();
        top.sort_by(|left, right| right.uploaded_bytes.cmp(&left.uploaded_bytes));
        top.truncate(10);

        let all_time_uploaded_bytes = latest.values().map(|stat| stat.uploaded_bytes).sum();
        let all_time_downloaded_bytes = latest.values().map(|stat| stat.downloaded_bytes).sum();
        let ratio = if current.downloaded_bytes > 0 {
            current.uploaded_bytes as f64 / current.downloaded_bytes as f64
        } else {
            0.0
        };

        StatsOverview {
            month: month_key,
            downloaded_bytes: current.downloaded_bytes,
            uploaded_bytes: current.uploaded_bytes,
            ratio,
            torrent_count: torrents.len(),
            seeding_count: torrents
                .iter()
                .filter(|torrent| torrent.state == "seeding")
                .count(),
            all_time_downloaded_bytes,
            all_time_uploaded_bytes,
            months: month_points,
            top_torrents: top,
        }
    }

    pub fn flush(&self, api: &Api, base: &Path) -> Result<()> {
        let list = list_torrents(api, base);
        self.record(&list.torrents);
        self.persist()
    }

    pub fn spawn_recorder(self: &Arc<Self>, api: Api, base: PathBuf) {
        let store = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(20));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let store = store.clone();
                let api = api.clone();
                let base = base.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let list = list_torrents(&api, &base);
                    store.record(&list.torrents);
                    store.persist()
                })
                .await;
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => warn!("could not persist torrent stats: {err:#}"),
                    Err(err) => warn!("torrent stats worker failed: {err:#}"),
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_runtime_counter_to_stored_baseline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("torrent-stats.json");
        let file = StatsFile {
            version: 2,
            months: HashMap::new(),
            torrents: HashMap::from([(
                "abc".to_owned(),
                StoredStat {
                    uploaded_bytes: 100,
                    downloaded_bytes: 80,
                },
            )]),
        };
        std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();
        let store = StatsStore::load(path).unwrap();

        let mut torrent = TorrentSummary {
            id: 1,
            info_hash: "abc".to_owned(),
            name: "Test".to_owned(),
            state: "seeding",
            label: None,
            folder: String::new(),
            output_folder: String::new(),
            progress: 100.0,
            progress_bytes: 120,
            total_bytes: 120,
            uploaded_bytes: 10,
            download_speed_bps: 0.0,
            upload_speed_bps: 0.0,
            eta_seconds: None,
            ratio: 0.0,
            finished: true,
            error: None,
            media_files: 0,
            total_files: 0,
            file_progress: Vec::new(),
        };
        store.enrich(std::slice::from_mut(&mut torrent));
        assert_eq!(torrent.uploaded_bytes, 110);
        assert!((torrent.ratio - (110.0 / 120.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn records_monthly_deltas_once() {
        let dir = tempfile::tempdir().unwrap();
        let store = StatsStore::load(dir.path().join("torrent-stats.json")).unwrap();
        let summary = |uploaded| TorrentSummary {
            id: 1,
            info_hash: "abc".to_owned(),
            name: "Test".to_owned(),
            state: "seeding",
            label: None,
            folder: String::new(),
            output_folder: String::new(),
            progress: 100.0,
            progress_bytes: 100,
            total_bytes: 100,
            uploaded_bytes: uploaded,
            download_speed_bps: 0.0,
            upload_speed_bps: 0.0,
            eta_seconds: None,
            ratio: 0.0,
            finished: true,
            error: None,
            media_files: 0,
            total_files: 0,
            file_progress: Vec::new(),
        };
        store.record(&[summary(10)]);
        store.record(&[summary(15)]);
        let overview = store.overview(&[summary(15)]);
        assert_eq!(overview.uploaded_bytes, 15);
        assert_eq!(overview.downloaded_bytes, 100);
        assert_eq!(overview.top_torrents.len(), 1);
        assert_eq!(overview.top_torrents[0].uploaded_bytes, 15);
    }
}
