use std::{
    fs,
    io,
    path::{Component, Path, PathBuf},
};

use axum::{extract::State, Json};
use librqbit::{AddTorrent, AddTorrentOptions, TorrentStatsState};
use serde::Deserialize;
use tracing::warn;

use crate::{
    auth::AuthUser,
    error::AppError,
    state::AppState,
    torrents::{self, TorrentDetailResponse},
};

#[derive(Deserialize)]
pub struct MoveTorrentRequest {
    pub folder: String,
}

pub async fn move_torrent(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<MoveTorrentRequest>,
) -> Result<Json<TorrentDetailResponse>, AppError> {
    user.require_admin()?;
    let id = torrents::parse_torrent_id(&id)?;
    let mut details = state
        .api
        .api_torrent_details(id)
        .map_err(|err| AppError::bad_request(format!("Torrentet blev ikke fundet: {err}")))?;
    let stats = state
        .api
        .api_stats_v1(id)
        .map_err(|err| AppError::bad_request(format!("Torrentstatus kunne ikke læses: {err}")))?;
    let was_paused = matches!(stats.state, TorrentStatsState::Paused);
    let files = details.files.take().unwrap_or_default();
    if files.is_empty() {
        return Err(AppError::bad_request("Torrentet har ingen filer at flytte."));
    }

    let destination_parent = state
        .downloads
        .resolve(Some(request.folder.trim()))
        .map_err(AppError::bad_request)?;
    let source_root = PathBuf::from(&details.output_folder);
    if !source_root.is_dir() {
        return Err(AppError::bad_request("Den nuværende downloadmappe findes ikke."));
    }
    let torrent_name = details
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| AppError::bad_request("Torrentens navn mangler."))?;
    let name_path = PathBuf::from(torrent_name);
    if name_path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::bad_request("Torrentens navn er ugyldigt."));
    }
    let is_multi_file = files.len() > 1;
    let destination_root = if is_multi_file && !destination_parent.ends_with(&name_path) {
        destination_parent.join(&name_path)
    } else {
        destination_parent.clone()
    };
    if destination_root == source_root
        || destination_root.starts_with(&source_root)
        || source_root.starts_with(&destination_root)
    {
        return Err(AppError::bad_request(
            "Den nye mappe skal være en anden mappe end den nuværende og må ikke ligge inde i den.",
        ));
    }

    let torrent_path = state
        .config
        .session_dir()
        .join(format!("{}.torrent", details.info_hash));
    let torrent_bytes = tokio::fs::read(&torrent_path).await.map_err(|err| {
        AppError::bad_request(format!(
            "Torrentfilen kunne ikke læses ({}): {err}",
            torrent_path.display()
        ))
    })?;
    let only_files = files
        .iter()
        .enumerate()
        .filter(|(_, file)| file.included)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let only_files = if only_files.len() == files.len() {
        None
    } else {
        Some(only_files)
    };

    if !was_paused {
        state
            .api
            .api_torrent_action_pause(id)
            .await
            .map_err(|err| AppError::bad_request(format!("Torrentet kunne ikke pauses: {err}")))?;
    }
    state
        .api
        .api_torrent_action_forget(id)
        .await
        .map_err(|err| AppError::bad_request(format!("Torrentet kunne ikke frigives: {err}")))?;

    let mut move_plan = Vec::new();
    for file in &files {
        let mut relative = if file.components.is_empty() {
            PathBuf::from(&file.name)
        } else {
            file.components.iter().collect()
        };
        if relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(AppError::bad_request("Torrenten indeholder en ugyldig filsti."));
        }
        if is_multi_file && source_root.file_name() != name_path.file_name() {
            if let Ok(stripped) = relative.strip_prefix(&name_path) {
                relative = stripped.to_path_buf();
            }
        }
        let source = source_root.join(&relative);
        if !source.exists() {
            continue;
        }
        move_plan.push((source, destination_root.join(&relative)));
    }
    if move_plan.is_empty() {
        return Err(AppError::bad_request("Der blev ikke fundet nogen filer at flytte."));
    }

    let move_result = tokio::task::spawn_blocking({
        let plan = move_plan.clone();
        move || move_files(&plan)
    })
    .await
    .map_err(|err| AppError::internal("move worker failed", err))?;

    if let Err(err) = move_result {
        warn!("torrent move failed: {err:#}");
        let rollback_plan = move_plan
            .iter()
            .map(|(source, target)| (target.clone(), source.clone()))
            .collect::<Vec<_>>();
        let _ = tokio::task::spawn_blocking(move || move_files(&rollback_plan)).await;
        restore_torrent(
            &state,
            &torrent_bytes,
            &source_root,
            only_files.clone(),
            was_paused,
        )
        .await;
        return Err(AppError::internal("torrent files could not be moved", err));
    }

    let new_id = match add_torrent(
        &state,
        &torrent_bytes,
        &destination_parent,
        only_files.clone(),
        was_paused,
    )
    .await
    {
        Ok(id) => id,
        Err(err) => {
            let rollback_plan = move_plan
                .iter()
                .map(|(source, target)| (target.clone(), source.clone()))
                .collect::<Vec<_>>();
            let _ = tokio::task::spawn_blocking(move || move_files(&rollback_plan)).await;
            restore_torrent(
                &state,
                &torrent_bytes,
                &source_root,
                only_files.clone(),
                was_paused,
            )
            .await;
            return Err(err);
        }
    };

    let source_to_clean = source_root.clone();
    let download_root = state.downloads.base().to_path_buf();
    let _ = tokio::task::spawn_blocking(move || {
        if source_to_clean != download_root
            && fs::read_dir(&source_to_clean)
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false)
        {
            let _ = fs::remove_dir(&source_to_clean);
        }
    })
    .await;

    let mut detail = torrents::get_torrent(
        &state.api,
        &state.session,
        librqbit::api::TorrentIdOrHash::Id(new_id),
        state.downloads.base(),
    )?;
    state.stats.enrich_detail(&mut detail);
    state.labels.enrich_detail(&mut detail);
    Ok(Json(detail))
}

async fn add_torrent(
    state: &AppState,
    bytes: &[u8],
    output_folder: &Path,
    only_files: Option<Vec<usize>>,
    paused: bool,
) -> Result<usize, AppError> {
    let response = state
        .api
        .api_add_torrent(
            AddTorrent::from_bytes(bytes.to_vec()),
            Some(AddTorrentOptions {
                paused,
                output_folder: Some(output_folder.to_string_lossy().into_owned()),
                only_files,
                overwrite: true,
                ..Default::default()
            }),
        )
        .await
        .map_err(|err| AppError::bad_request(format!("Torrentet kunne ikke genindlæses: {err}")))?;
    response
        .id
        .ok_or_else(|| AppError::bad_request("Torrentet gav ingen metadata."))
}

async fn restore_torrent(
    state: &AppState,
    bytes: &[u8],
    output_folder: &Path,
    only_files: Option<Vec<usize>>,
    was_paused: bool,
) {
    match add_torrent(state, bytes, output_folder, only_files, was_paused).await {
        Ok(_) => {}
        Err(err) => warn!("could not restore torrent after failed move: {}", err.message),
    }
}

fn move_files(plan: &[(PathBuf, PathBuf)]) -> anyhow::Result<()> {
    for (source, destination) in plan {
        if !source.exists() {
            continue;
        }
        if destination.exists() {
            anyhow::bail!("destination already exists: {}", destination.display());
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        move_path(source, destination)?;
    }
    Ok(())
}

fn move_path(source: &Path, destination: &Path) -> io::Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            // Moving between two Docker mounts is a cross-device move; fall
            // back to copy + delete.  The API timeout is raised for this route.
            if source.is_dir() {
                copy_directory(source, destination)?;
            } else {
                fs::copy(source, destination)?;
            }
            if source.is_dir() {
                fs::remove_dir_all(source)?;
            } else {
                fs::remove_file(source)?;
            }
            let _ = rename_error;
            Ok(())
        }
    }
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
