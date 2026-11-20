use std::path::{Path, PathBuf};

use librqbit::{
    api::{ApiTorrentListOpts, TorrentDetailsResponse, TorrentIdOrHash},
    Api, Session, TorrentStats, TorrentStatsState,
};
use serde::Serialize;

use crate::{downloads::relative_output_folder, error::AppError};

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mkv", "webm", "avi", "mov", "wmv", "flv", "mpg", "mpeg", "ts", "m2ts", "mts",
    "ogv", "3gp",
];
const AUDIO_EXTENSIONS: &[&str] = &["mp3", "m4a", "aac", "flac", "ogg", "opus", "wav", "wma"];
const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "vtt", "ass", "ssa", "sub"];

#[derive(Debug, Serialize)]
pub struct TorrentListResponse {
    pub torrents: Vec<TorrentSummary>,
    pub totals: Totals,
}

#[derive(Debug, Serialize)]
pub struct Totals {
    pub torrent_count: usize,
    pub active: usize,
    pub paused: usize,
    pub completed: usize,
    pub download_speed_bps: f64,
    pub upload_speed_bps: f64,
    pub downloaded_bytes: u64,
    pub uploaded_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct TorrentSummary {
    pub id: usize,
    pub info_hash: String,
    pub name: String,
    pub state: &'static str,
    pub label: Option<String>,
    pub folder: String,
    pub output_folder: String,
    pub progress: f64,
    pub progress_bytes: u64,
    pub total_bytes: u64,
    pub uploaded_bytes: u64,
    pub download_speed_bps: f64,
    pub upload_speed_bps: f64,
    pub eta_seconds: Option<u64>,
    pub ratio: f64,
    pub finished: bool,
    pub error: Option<String>,
    pub media_files: usize,
    pub total_files: usize,
    #[serde(skip)]
    pub file_progress: Vec<u64>,
}

#[derive(Debug, Serialize)]
pub struct TorrentDetailResponse {
    #[serde(flatten)]
    pub summary: TorrentSummary,
    pub files: Vec<TorrentFile>,
    pub peers: Vec<TorrentPeer>,
    pub trackers: Vec<TorrentTracker>,
}

#[derive(Debug, Serialize)]
pub struct TorrentPeer {
    pub address: String,
    pub state: String,
    pub downloaded_bytes: u64,
    pub pieces: u32,
    pub errors: u32,
    pub connections: u32,
    pub incoming: bool,
}

#[derive(Debug, Serialize)]
pub struct TorrentTracker {
    pub url: String,
    pub kind: String,
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct TorrentFile {
    pub id: usize,
    pub name: String,
    pub path: String,
    pub length: u64,
    pub included: bool,
    pub priority: &'static str,
    pub progress_bytes: u64,
    pub progress: f64,
    pub kind: &'static str,
    pub mime: String,
    pub playable: bool,
    pub stream_url: String,
}

#[derive(Debug, Serialize)]
pub struct LibraryResponse {
    pub items: Vec<LibraryItem>,
}

#[derive(Debug, Serialize)]
pub struct LibraryItem {
    pub torrent_id: usize,
    pub file_id: usize,
    pub info_hash: String,
    pub title: String,
    pub file_name: String,
    pub torrent_name: String,
    pub size: u64,
    pub downloaded_bytes: u64,
    pub progress: f64,
    pub kind: &'static str,
    pub mime: String,
    pub stream_url: String,
    pub preview_url: String,
    pub torrent_state: &'static str,
}

pub fn parse_torrent_id(raw: &str) -> Result<TorrentIdOrHash, AppError> {
    TorrentIdOrHash::parse(raw)
        .map_err(|_| AppError::bad_request("Ugyldigt torrent-id eller info-hash."))
}

pub fn list_torrents(api: &Api, base: &Path) -> TorrentListResponse {
    let list = api.api_torrent_list_ext(ApiTorrentListOpts { with_stats: true });
    let mut torrents = Vec::with_capacity(list.torrents.len());
    for mut details in list.torrents {
        let Some(id) = details.id else {
            continue;
        };
        let stats = details.stats.take();
        // The list endpoint does not include file metadata. Fetch it so the dashboard can
        // show correct media/file counts without a second API round-trip from the client.
        let full_details = api
            .api_torrent_details(TorrentIdOrHash::Id(id))
            .unwrap_or(details);
        let stats = match stats {
            Some(stats) => stats,
            None => match api.api_stats_v1(TorrentIdOrHash::Id(id)) {
                Ok(stats) => stats,
                Err(err) => {
                    tracing::warn!("skipping torrent {} without stats: {err}", id);
                    continue;
                }
            },
        };
        match summary_from_parts(&full_details, stats, base) {
            Ok(summary) => torrents.push(summary),
            Err(err) => {
                tracing::warn!("skipping torrent with inconsistent state: {}", err.message);
            }
        }
    }
    torrents.sort_by(|a, b| b.id.cmp(&a.id));

    let totals = Totals {
        torrent_count: torrents.len(),
        active: torrents
            .iter()
            .filter(|t| matches!(t.state, "downloading" | "seeding" | "initializing"))
            .count(),
        paused: torrents.iter().filter(|t| t.state == "paused").count(),
        completed: torrents.iter().filter(|t| t.finished).count(),
        download_speed_bps: torrents.iter().map(|t| t.download_speed_bps).sum(),
        upload_speed_bps: torrents.iter().map(|t| t.upload_speed_bps).sum(),
        downloaded_bytes: torrents.iter().map(|t| t.progress_bytes).sum(),
        uploaded_bytes: torrents.iter().map(|t| t.uploaded_bytes).sum(),
    };
    TorrentListResponse { torrents, totals }
}

pub fn get_torrent(
    api: &Api,
    session: &Session,
    id: TorrentIdOrHash,
    base: &Path,
) -> Result<TorrentDetailResponse, AppError> {
    let details = api
        .api_torrent_details(id)
        .map_err(|err| torrent_error("torrentet blev ikke fundet", err))?;
    let stats = api
        .api_stats_v1(id)
        .map_err(|err| torrent_error("kunne ikke læse torrent-status", err))?;
    let summary = summary_from_parts(&details, stats, base)?;

    let files = details
        .files
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(file_id, file)| {
            let mime = mime_guess::from_path(&file.name)
                .first_or_octet_stream()
                .essence_str()
                .to_owned();
            let kind = media_kind(&file.name).unwrap_or("other");
            let path = if file.components.is_empty() {
                file.name.clone()
            } else {
                file.components.join("/")
            };
            let progress_bytes = if file.included {
                file_progress_for(&summary, file_id, file.length)
            } else {
                0
            };
            TorrentFile {
                id: file_id,
                name: file.name.clone(),
                path,
                length: file.length,
                included: file.included,
                priority: if file.included { "normal" } else { "skip" },
                progress_bytes,
                progress: percent(progress_bytes, file.length),
                kind,
                mime,
                playable: matches!(kind, "video" | "audio"),
                stream_url: format!("/api/v1/torrents/{}/files/{file_id}/stream", summary.id),
            }
        })
        .collect();

    let peers = peers_for(api, id);
    let trackers = trackers_for(session, id);

    Ok(TorrentDetailResponse {
        summary,
        files,
        peers,
        trackers,
    })
}

fn peers_for(api: &Api, id: TorrentIdOrHash) -> Vec<TorrentPeer> {
    let Ok(snapshot) = api.api_peer_stats(id, Default::default()) else {
        return Vec::new();
    };
    let mut peers: Vec<TorrentPeer> = snapshot
        .peers
        .into_iter()
        .map(|(address, peer)| TorrentPeer {
            address,
            state: peer.state.to_owned(),
            downloaded_bytes: peer.counters.fetched_bytes,
            pieces: peer.counters.downloaded_and_checked_pieces,
            errors: peer.counters.errors,
            connections: peer.counters.connections,
            incoming: peer.counters.incoming_connections > 0,
        })
        .collect();
    peers.sort_by(|left, right| left.address.cmp(&right.address));
    peers
}

fn trackers_for(session: &Session, id: TorrentIdOrHash) -> Vec<TorrentTracker> {
    let Some(handle) = session.get(id) else {
        return Vec::new();
    };
    let mut trackers: Vec<TorrentTracker> = handle
        .shared()
        .trackers
        .iter()
        .map(|url| TorrentTracker {
            url: url.to_string(),
            kind: url.scheme().to_owned(),
            status: "unknown",
        })
        .collect();
    trackers.sort_by(|left, right| left.url.cmp(&right.url));
    trackers
}

pub fn get_library(api: &Api, base: &Path) -> LibraryResponse {
    let list = api.api_torrent_list_ext(ApiTorrentListOpts { with_stats: true });
    let mut items = Vec::new();
    for item in list.torrents {
        let Some(id) = item.id else {
            continue;
        };
        let Ok(details) = api.api_torrent_details(TorrentIdOrHash::Id(id)) else {
            continue;
        };
        let Ok(stats) = api.api_stats_v1(TorrentIdOrHash::Id(id)) else {
            continue;
        };
        let Ok(summary) = summary_from_parts(&details, stats, base) else {
            continue;
        };
        let files = details.files.unwrap_or_default();
        let video_count = files
            .iter()
            .filter(|file| file.included && media_kind(&file.name) == Some("video"))
            .count();
        for (file_id, file) in files.iter().enumerate() {
            if !file.included {
                continue;
            }
            let Some(kind) = media_kind(&file.name) else {
                continue;
            };
            if kind != "video" && kind != "audio" {
                continue;
            }
            let mime = mime_guess::from_path(&file.name)
                .first_or_octet_stream()
                .essence_str()
                .to_owned();
            let file_name = Path::new(&file.name)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&file.name)
                .to_owned();
            let title = if video_count == 1 && kind == "video" && !summary.name.is_empty() {
                summary.name.clone()
            } else {
                strip_extension(&file_name).to_owned()
            };
            let downloaded_bytes = file_progress_for(&summary, file_id, file.length);
            items.push(LibraryItem {
                torrent_id: id,
                file_id,
                info_hash: summary.info_hash.clone(),
                title,
                file_name,
                torrent_name: summary.name.clone(),
                size: file.length,
                downloaded_bytes,
                progress: percent(downloaded_bytes, file.length),
                kind,
                mime,
                stream_url: format!("/api/v1/torrents/{id}/files/{file_id}/stream"),
                preview_url: if kind == "video" {
                    format!("/api/v1/torrents/{id}/files/{file_id}/thumbnail")
                } else {
                    String::new()
                },
                torrent_state: summary.state,
            });
        }
    }
    items.sort_by(|a, b| {
        b.torrent_id
            .cmp(&a.torrent_id)
            .then_with(|| a.file_id.cmp(&b.file_id))
    });
    LibraryResponse { items }
}

pub fn file_for_stream(
    api: &Api,
    id: TorrentIdOrHash,
    file_id: usize,
) -> Result<(String, String), AppError> {
    let details = api
        .api_torrent_details(id)
        .map_err(|err| torrent_error("torrentet blev ikke fundet", err))?;
    let files = details.files.unwrap_or_default();
    let file = files
        .get(file_id)
        .ok_or_else(|| AppError::not_found("Filen findes ikke i dette torrent."))?;
    if !file.included {
        return Err(AppError::forbidden("Filen er fravalgt og downloades ikke."));
    }
    let mime = mime_guess::from_path(&file.name)
        .first_or_octet_stream()
        .essence_str()
        .to_owned();
    let name = Path::new(&file.name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download")
        .to_owned();
    Ok((mime, name))
}

pub fn file_disk_path(
    api: &Api,
    id: TorrentIdOrHash,
    file_id: usize,
    download_root: &Path,
) -> Result<(PathBuf, String), AppError> {
    let details = api
        .api_torrent_details(id)
        .map_err(|err| torrent_error("torrentet blev ikke fundet", err))?;
    let file = details
        .files
        .as_ref()
        .and_then(|files| files.get(file_id))
        .ok_or_else(|| AppError::not_found("Filen findes ikke i dette torrent."))?;
    if !file.included {
        return Err(AppError::forbidden("Filen er fravalgt og downloades ikke."));
    }
    let mut path = PathBuf::from(&details.output_folder);
    if file.components.is_empty() {
        path.push(&file.name);
    } else {
        for component in &file.components {
            path.push(component);
        }
    }
    let path = path
        .canonicalize()
        .map_err(|_| AppError::not_found("Filen er ikke færdigdownloadet endnu."))?;
    let root = download_root
        .canonicalize()
        .map_err(|err| AppError::internal("download folder unavailable", err))?;
    if !path.starts_with(&root) {
        return Err(AppError::forbidden("Filen ligger uden for downloadmappen."));
    }
    if !path.is_file() {
        return Err(AppError::not_found("Filen findes ikke på disken."));
    }
    Ok((path, details.info_hash))
}

fn summary_from_parts(
    details: &TorrentDetailsResponse,
    stats: TorrentStats,
    base: &Path,
) -> Result<TorrentSummary, AppError> {
    let id = details
        .id
        .ok_or_else(|| AppError::internal("torrent id missing", "missing id"))?;
    let download_speed_bps = stats
        .live
        .as_ref()
        .map(|live| (live.download_speed.mbps * 1024.0 * 1024.0).max(0.0))
        .unwrap_or(0.0);
    let upload_speed_bps = stats
        .live
        .as_ref()
        .map(|live| (live.upload_speed.mbps * 1024.0 * 1024.0).max(0.0))
        .unwrap_or(0.0);
    let remaining = stats.total_bytes.saturating_sub(stats.progress_bytes);
    let eta_seconds = if !stats.finished && download_speed_bps > 1.0 {
        Some((remaining as f64 / download_speed_bps).ceil() as u64)
    } else {
        None
    };
    let state = match stats.state {
        TorrentStatsState::Initializing => "initializing",
        TorrentStatsState::Paused => "paused",
        TorrentStatsState::Error => "error",
        TorrentStatsState::Live if stats.finished => "seeding",
        TorrentStatsState::Live => "downloading",
    };
    let total_files = details.files.as_ref().map(|files| files.len()).unwrap_or(0);
    let media_files = details
        .files
        .as_ref()
        .map(|files| {
            files
                .iter()
                .filter(|file| matches!(media_kind(&file.name), Some("video" | "audio")))
                .count()
        })
        .unwrap_or(0);

    Ok(TorrentSummary {
        id,
        info_hash: details.info_hash.clone(),
        name: clean_name(details.name.as_deref().unwrap_or("Ukendt torrent")),
        state,
        label: None,
        folder: relative_output_folder(base, &details.output_folder),
        output_folder: details.output_folder.clone(),
        progress: percent(stats.progress_bytes, stats.total_bytes),
        progress_bytes: stats.progress_bytes,
        total_bytes: stats.total_bytes,
        uploaded_bytes: stats.uploaded_bytes,
        download_speed_bps,
        upload_speed_bps,
        eta_seconds,
        ratio: if stats.progress_bytes > 0 {
            stats.uploaded_bytes as f64 / stats.progress_bytes as f64
        } else {
            0.0
        },
        finished: stats.finished,
        error: stats.error.map(|err| truncate(&err, 500)),
        media_files,
        total_files,
        file_progress: stats.file_progress,
    })
}

fn file_progress_for(summary: &TorrentSummary, file_id: usize, length: u64) -> u64 {
    if let Some(progress) = summary.file_progress.get(file_id) {
        (*progress).min(length)
    } else if summary.finished {
        length
    } else {
        0
    }
}

pub fn media_kind(name: &str) -> Option<&'static str> {
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();
    if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        Some("video")
    } else if AUDIO_EXTENSIONS.contains(&extension.as_str()) {
        Some("audio")
    } else if SUBTITLE_EXTENSIONS.contains(&extension.as_str()) {
        Some("subtitle")
    } else {
        None
    }
}

fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
    }
}

fn strip_extension(name: &str) -> &str {
    Path::new(name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(name)
}

fn clean_name(name: &str) -> String {
    name.chars()
        .filter(|character| !character.is_control())
        .take(300)
        .collect::<String>()
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn torrent_error(context: &str, err: impl std::fmt::Display) -> AppError {
    tracing::debug!("{context}: {err}");
    AppError::not_found("Torrentet blev ikke fundet.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_media() {
        assert_eq!(media_kind("Movie.2024.1080p.mkv"), Some("video"));
        assert_eq!(media_kind("sound.FLAC"), Some("audio"));
        assert_eq!(media_kind("subs.srt"), Some("subtitle"));
        assert_eq!(media_kind("readme.txt"), None);
    }

    #[test]
    fn strips_extensions() {
        assert_eq!(strip_extension("The.Movie.mkv"), "The.Movie");
    }
}

