use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use tokio::process::Command;

pub fn cache_path(data_dir: &Path, info_hash: &str, file_id: usize) -> PathBuf {
    let safe_hash: String = info_hash
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect();
    data_dir
        .join("thumbnails")
        .join(safe_hash)
        .join(format!("{file_id}.jpg"))
}

pub async fn ensure_thumbnail(
    data_dir: &Path,
    info_hash: &str,
    file_id: usize,
    source: &Path,
) -> Result<PathBuf> {
    let destination = cache_path(data_dir, info_hash, file_id);
    if let Ok(metadata) = tokio::fs::metadata(&destination).await {
        if metadata.len() > 0 {
            return Ok(destination);
        }
    }

    let parent = destination
        .parent()
        .context("thumbnail destination has no parent")?;
    tokio::fs::create_dir_all(parent)
        .await
        .with_context(|| format!("could not create {}", parent.display()))?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temporary = parent.join(format!(".{file_id}-{nonce}.tmp.jpg"));
    let ffmpeg = std::env::var("NORDTORRENT_FFMPEG").unwrap_or_else(|_| "ffmpeg".to_owned());

    // Prefer a frame a few seconds in; if the clip is shorter, retry at the start.
    let mut generated = false;
    for seek in ["5", "0"] {
        let status = tokio::time::timeout(
            Duration::from_secs(30),
            Command::new(&ffmpeg)
                .arg("-hide_banner")
                .arg("-loglevel")
                .arg("error")
                .arg("-y")
                .arg("-ss")
                .arg(seek)
                .arg("-i")
                .arg(source)
                .arg("-frames:v")
                .arg("1")
                .arg("-vf")
                .arg("scale=640:-2")
                .arg("-q:v")
                .arg("4")
                .arg(&temporary)
                .kill_on_drop(true)
                .status(),
        )
        .await;

        match status {
            Ok(Ok(status)) if status.success() => {
                if let Ok(metadata) = tokio::fs::metadata(&temporary).await {
                    if metadata.len() > 0 {
                        generated = true;
                        break;
                    }
                }
            }
            Ok(Ok(status)) => {
                tracing::debug!("ffmpeg thumbnail failed with {status} for {}", source.display());
            }
            Ok(Err(err)) => {
                let _ = tokio::fs::remove_file(&temporary).await;
                bail!("could not start ffmpeg ({ffmpeg}): {err}");
            }
            Err(_) => {
                tracing::debug!("ffmpeg thumbnail timed out for {}", source.display());
            }
        }
        let _ = tokio::fs::remove_file(&temporary).await;
    }

    if !generated {
        let _ = tokio::fs::remove_file(&temporary).await;
        bail!("ffmpeg could not create a preview image");
    }

    if destination.exists() {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Ok(destination);
    }
    tokio::fs::rename(&temporary, &destination)
        .await
        .with_context(|| format!("could not store {}", destination.display()))?;
    Ok(destination)
}
