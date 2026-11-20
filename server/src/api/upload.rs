use axum::{
    extract::{Multipart, State},
    Json,
};

use crate::{
    auth::AuthUser,
    error::AppError,
    state::AppState,
    torrents::{self, TorrentDetailResponse},
};

pub async fn upload_torrent(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<TorrentDetailResponse>, AppError> {
    user.require_admin()?;
    let mut torrent_bytes: Option<bytes::Bytes> = None;
    let mut paused = false;
    let mut folder: Option<String> = None;
    let mut label: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad_request("Upload af .torrent-filen mislykkedes."))?
    {
        match field.name() {
            Some("paused") => {
                let value = field.text().await.unwrap_or_default();
                paused = matches!(value.as_str(), "1" | "true" | "yes");
            }
            Some("folder") => {
                let value = field.text().await.unwrap_or_default();
                folder = Some(value).filter(|value| !value.trim().is_empty());
            }
            Some("label") => {
                let value = field.text().await.unwrap_or_default();
                label = Some(value).filter(|value| !value.trim().is_empty());
            }
            Some("file") => {
                // The torrent body is validated by librqbit below.  The
                // filename can legitimately be missing an extension when it
                // comes from a browser download or a share handler, so do not
                // reject an upload based on its display name alone.
                let data = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::bad_request("Filen kunne ikke læses."))?;
                if data.is_empty() {
                    return Err(AppError::bad_request("Filen er tom."));
                }
                torrent_bytes = Some(data);
            }
            _ => {}
        }
    }

    let torrent_bytes =
        torrent_bytes.ok_or_else(|| AppError::bad_request("Vælg en .torrent-fil først."))?;

    let output_folder = state
        .downloads
        .resolve(folder.as_deref())
        .map_err(|err| AppError::bad_request(err))?;
    let response = state
        .api
        .api_add_torrent(
            librqbit::AddTorrent::from_bytes(torrent_bytes),
            Some(librqbit::AddTorrentOptions {
                paused,
                output_folder: Some(output_folder.to_string_lossy().into_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|err| {
            tracing::debug!("could not add uploaded torrent: {err:#}");
            AppError::bad_request("Torrentfilen er ugyldig eller kunne ikke indlæses.")
        })?;
    let id = response
        .id
        .ok_or_else(|| AppError::bad_request("Torrentet gav ingen metadata."))?;
    let mut detail = torrents::get_torrent(
        &state.api,
        &state.session,
        librqbit::api::TorrentIdOrHash::Id(id),
        state.downloads.base(),
    )?;
    state.stats.enrich_detail(&mut detail);
    state
        .labels
        .assign(&detail.summary.info_hash, label.as_deref())
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    state.labels.enrich_detail(&mut detail);
    Ok(Json(detail))
}


