mod move_torrent;
mod stream;
mod upload;

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use axum::{
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};

use crate::{
    auth::{origin_allowed, validate_password, AuthUser, PublicUser, Role, TotpSetup},
    discovery::DhtSettings,
    downloads::{DownloadBrowser, DownloadFolder},
    error::AppError,
    i2p::{self, I2pSettings},
    labels::LabelsView,
    limits::SpeedLimitSettings,
    network::NetworkStatus,
    rss::{RssFeedInput, RssFeedView},
    seeding::SeedingSettings,
    state::AppState,
    torrents::{self, TorrentListResponse},
};

const INDEX_HTML: &str = include_str!("../../../web/index.html");
const APP_JS: &str = include_str!("../../../web/assets/app.js");
const I18N_JS: &str = include_str!("../../../web/assets/i18n.js");
const THEME_JS: &str = include_str!("../../../web/assets/theme.js");
const STYLES_CSS: &str = include_str!("../../../web/assets/styles.css");
const FAVICON_SVG: &str = include_str!("../../../web/assets/favicon.svg");

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const FEATURES: &[&str] = &[
    "download-browser",
    "network-status",
    "theme-language",
    "url-routing",
    "torrent-details",
    "sessions",
    "private-trackers",
    "tracker-completed",
    "stats-persistence",
    "torrent-subfolder",
    "monthly-stats",
    "thumbnails",
    "move-torrent",
    "extension-api",
    "speed-limits",
    "seeding-limits",
    "dht-toggle",
    "rss",
    "2fa",
    "labels",
    "label-colors",
    "i2p",
    "totp-proxy-detect",
];

pub fn router(state: AppState) -> Router {
    let upload_route = Router::new()
        .route("/api/v1/torrents/upload", post(upload::upload_torrent))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024));

    Router::new()
        .route("/healthz", get(health))
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/all", get(index))
        .route("/downloading", get(index))
        .route("/completed", get(index))
        .route("/library", get(index))
        .route("/settings", get(index))
        .route("/stats", get(index))
        .route("/assets/app.js", get(app_js))
        .route("/assets/i18n.js", get(i18n_js))
        .route("/assets/theme.js", get(theme_js))
        .route("/assets/styles.css", get(styles_css))
        .route("/assets/favicon.svg", get(favicon))
        .route("/api/v1/status", get(status))
        .route("/api/v1/auth/setup", post(setup))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(me))
        .route("/api/v1/auth/password", post(change_password))
        .route("/api/v1/auth/totp/setup", post(setup_totp))
        .route("/api/v1/auth/totp/enable", post(enable_totp))
        .route("/api/v1/auth/totp/disable", post(disable_totp))
        .route("/api/v1/auth/sessions", get(list_sessions))
        .route("/api/v1/auth/sessions/{id}", delete(revoke_session))
        .route("/api/v1/users", get(list_users).post(create_user))
        .route("/api/v1/users/{username}", delete(delete_user))
        .route("/api/v1/system", get(system))
        .route("/api/v1/stats", get(stats_overview))
        .route("/api/v1/network", get(network_status).put(update_network))
        .route("/api/v1/network/check", post(check_network))
        .route("/api/v1/limits", get(speed_limits).put(update_speed_limits))
        .route("/api/v1/seeding", get(seeding_settings).put(update_seeding_settings))
        .route("/api/v1/dht", get(dht_settings).put(update_dht_settings))
        .route("/api/v1/i2p", get(i2p_settings).put(update_i2p_settings))
        .route("/api/v1/i2p/test", post(test_i2p))
        .route("/api/v1/rss", get(list_rss).post(create_rss))
        .route(
            "/api/v1/rss/{id}",
            put(update_rss).delete(delete_rss),
        )
        .route("/api/v1/rss/{id}/check", post(check_rss))
        .route("/api/v1/torrents", get(list_torrents).post(add_torrent))
        .route(
            "/api/v1/torrents/{id}",
            get(torrent_detail).delete(remove_torrent),
        )
        .route("/api/v1/torrents/{id}/pause", post(pause_torrent))
        .route("/api/v1/torrents/{id}/resume", post(resume_torrent))
        .route("/api/v1/torrents/{id}/move", post(move_torrent::move_torrent))
        .route("/api/v1/torrents/{id}/files", post(update_files))
        .route("/api/v1/torrents/{id}/label", put(set_torrent_label))
        .route("/api/v1/labels", get(list_labels).post(create_label))
        .route(
            "/api/v1/labels/{name}",
            delete(delete_label).put(update_label),
        )
        .route(
            "/api/v1/torrents/{id}/files/{file_id}/stream",
            get(stream::stream_file),
        )
        .route(
            "/api/v1/torrents/{id}/files/{file_id}/thumbnail",
            get(thumbnail),
        )
        .route("/api/v1/library", get(library))
        .route("/api/v1/download-folders", get(download_folders))
        .route("/api/v1/download-browser", get(download_browser))
        .merge(upload_route)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn_with_state(state.clone(), guard_origin))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(600),
        ))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn index() -> impl IntoResponse {
    no_cache(Html(INDEX_HTML))
}

async fn app_js() -> impl IntoResponse {
    asset("text/javascript; charset=utf-8", APP_JS)
}

async fn i18n_js() -> impl IntoResponse {
    asset("text/javascript; charset=utf-8", I18N_JS)
}

async fn theme_js() -> impl IntoResponse {
    asset("text/javascript; charset=utf-8", THEME_JS)
}

async fn styles_css() -> impl IntoResponse {
    asset("text/css; charset=utf-8", STYLES_CSS)
}

async fn favicon() -> impl IntoResponse {
    asset("image/svg+xml", FAVICON_SVG)
}

#[derive(Serialize)]
struct StatusResponse {
    setup_required: bool,
    version: &'static str,
    server_time: String,
    api_version: u8,
    features: &'static [&'static str],
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    Json(StatusResponse {
        setup_required: state.auth.setup_required(),
        version: VERSION,
        server_time: Utc::now().to_rfc3339(),
        api_version: 1,
        features: FEATURES,
    })
}

#[derive(Deserialize)]
struct SetupRequest {
    username: String,
    #[serde(default)]
    display_name: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
    #[serde(default)]
    totp_code: Option<String>,
}

#[derive(Serialize)]
struct AuthResponse {
    user: PublicUser,
    access_token: String,
    token_type: &'static str,
    expires_in: i64,
}

async fn setup(
    State(state): State<AppState>,
    Json(request): Json<SetupRequest>,
) -> Result<Response, AppError> {
    if !state.auth.setup_required() {
        return Err(AppError::conflict("Opsætningen er allerede gennemført."));
    }
    validate_password(&request.password, &request.username)?;
    let (user, token) = state
        .auth
        .setup(&request.username, &request.display_name, &request.password)
        .await?;
    Ok(auth_response(&state, user, token))
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let client = client_key(&state, addr, Some(&headers));
    let extension_client = headers
        .get("x-nordtorrent-client")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("chrome-extension"))
        || headers
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("chrome-extension://"));
    let user_agent = if extension_client {
        let version = headers
            .get("x-nordtorrent-client-version")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
            .collect::<String>();
        if version.is_empty() {
            "NordTorrent Chrome Extension".to_owned()
        } else {
            format!("NordTorrent Chrome Extension {version}")
        }
    } else {
        headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned()
    };
    let (user, token) = state
        .auth
        .authenticate_with_totp_context(
            &request.username,
            &request.password,
            &client,
            &client,
            &user_agent,
            request.totp_code.as_deref(),
            is_local_client(addr, Some(&headers)),
        )
        .await?;
    Ok(auth_response(&state, user, token))
}

async fn logout(State(state): State<AppState>, user: AuthUser) -> Response {
    let _ = state.auth.revoke_session(&user.session_id);
    let mut response = Json(serde_json::json!({ "ok": true })).into_response();
    if let Ok(value) = HeaderValue::from_str(&state.auth.clear_cookie()) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    no_store(response)
}

async fn list_sessions(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<crate::auth::SessionInfo>>, AppError> {
    user.require_admin()?;
    Ok(Json(state.auth.list_sessions(&user.session_id)))
}

async fn revoke_session(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Response, AppError> {
    user.require_admin()?;
    state.auth.revoke_session(&id)?;
    let revoked_current = id == user.session_id;
    let mut response = Json(serde_json::json!({
        "ok": true,
        "revoked_current": revoked_current,
    }))
    .into_response();
    if revoked_current {
        if let Ok(value) = HeaderValue::from_str(&state.auth.clear_cookie()) {
            response.headers_mut().insert(header::SET_COOKIE, value);
        }
    }
    Ok(no_store(response))
}

async fn me(State(state): State<AppState>, user: AuthUser) -> Result<Json<PublicUser>, AppError> {
    state
        .auth
        .user(&user.username)
        .map(Json)
        .ok_or_else(|| AppError::unauthorized("Brugeren findes ikke længere."))
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Response, AppError> {
    let (public, token) = state
        .auth
        .change_password(
            &user.username,
            &request.current_password,
            &request.new_password,
        )
        .await?;
    Ok(auth_response(&state, public, token))
}

#[derive(Deserialize)]
struct TotpCodeRequest {
    code: String,
}

#[derive(Deserialize)]
struct TotpDisableRequest {
    password: String,
}

async fn setup_totp(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<TotpSetup>, AppError> {
    state.auth.setup_totp(&user.username).map(Json)
}

async fn enable_totp(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<TotpCodeRequest>,
) -> Result<Json<PublicUser>, AppError> {
    state
        .auth
        .enable_totp(&user.username, &request.code)
        .map(Json)
}

async fn disable_totp(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<TotpDisableRequest>,
) -> Result<Json<PublicUser>, AppError> {
    state
        .auth
        .disable_totp(&user.username, &request.password)
        .await
        .map(Json)
}

async fn list_users(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<PublicUser>>, AppError> {
    user.require_admin()?;
    Ok(Json(state.auth.list_users()))
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    #[serde(default)]
    display_name: String,
    password: String,
    role: Role,
}

async fn create_user(
    State(state): State<AppState>,
    actor: AuthUser,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<PublicUser>, AppError> {
    actor.require_admin()?;
    let user = state
        .auth
        .add_user(
            &request.username,
            &request.display_name,
            &request.password,
            request.role,
        )
        .await?;
    Ok(Json(user))
}

async fn delete_user(
    State(state): State<AppState>,
    actor: AuthUser,
    axum::extract::Path(username): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    actor.require_admin()?;
    state.auth.delete_user(&username, &actor.username)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Serialize)]
struct SystemPaths {
    data_dir: String,
    download_dir: String,
}

#[derive(Serialize)]
struct SystemResponse {
    version: &'static str,
    uptime_seconds: u64,
    server_time: String,
    torrents: TorrentListResponse,
    torrent_port_start: u16,
    torrent_port_end: u16,
    user_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    paths: Option<SystemPaths>,
}

async fn system(State(state): State<AppState>, user: AuthUser) -> Json<SystemResponse> {
    let mut torrents = torrents::list_torrents(&state.api, state.downloads.base());
    if !user.is_admin() {
        redact_torrent_paths(&mut torrents);
    }
    let paths = if user.is_admin() {
        Some(SystemPaths {
            data_dir: state.config.data_dir.display().to_string(),
            download_dir: state.config.download_dir.display().to_string(),
        })
    } else {
        None
    };
    Json(SystemResponse {
        version: VERSION,
        uptime_seconds: state.started_at.elapsed().as_secs(),
        server_time: Utc::now().to_rfc3339(),
        torrents,
        torrent_port_start: state.config.torrent_port_start,
        torrent_port_end: state.config.torrent_port_end,
        user_count: state.auth.list_users().len(),
        paths,
    })
}

async fn list_torrents(State(state): State<AppState>, user: AuthUser) -> Json<TorrentListResponse> {
    let mut response = torrents::list_torrents(&state.api, state.downloads.base());
    state.stats.enrich_list(&mut response);
    state.labels.enrich_list(&mut response);
    if !user.is_admin() {
        redact_torrent_paths(&mut response);
    }
    Json(response)
}

async fn list_labels(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Json<LabelsView> {
    Json(state.labels.view())
}

#[derive(Deserialize)]
struct CreateLabelRequest {
    name: String,
    #[serde(default)]
    color: Option<String>,
}

async fn create_label(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateLabelRequest>,
) -> Result<Json<LabelsView>, AppError> {
    user.require_admin()?;
    state
        .labels
        .create(&request.name, request.color.as_deref())
        .map(Json)
        .map_err(|err| AppError::bad_request(err.to_string()))
}

#[derive(Deserialize)]
struct SetLabelColorRequest {
    #[serde(default)]
    color: Option<String>,
}

async fn update_label(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(request): Json<SetLabelColorRequest>,
) -> Result<Json<LabelsView>, AppError> {
    user.require_admin()?;
    state
        .labels
        .set_color(&name, request.color.as_deref())
        .map(Json)
        .map_err(|err| AppError::bad_request(err.to_string()))
}

async fn delete_label(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<LabelsView>, AppError> {
    user.require_admin()?;
    state
        .labels
        .remove(&name)
        .map(Json)
        .map_err(|err| AppError::bad_request(err.to_string()))
}

#[derive(Deserialize)]
struct SetLabelRequest {
    #[serde(default)]
    label: Option<String>,
}

async fn set_torrent_label(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<SetLabelRequest>,
) -> Result<Json<torrents::TorrentDetailResponse>, AppError> {
    user.require_admin()?;
    let id = torrents::parse_torrent_id(&id)?;
    let mut detail = torrents::get_torrent(
        &state.api,
        &state.session,
        id,
        state.downloads.base(),
    )?;
    state
        .labels
        .assign(&detail.summary.info_hash, request.label.as_deref())
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    state.stats.enrich_detail(&mut detail);
    state.labels.enrich_detail(&mut detail);
    Ok(Json(detail))
}

async fn stats_overview(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Json<crate::stats::StatsOverview> {
    let list = torrents::list_torrents(&state.api, state.downloads.base());
    Json(state.stats.overview(&list.torrents))
}

async fn torrent_detail(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<torrents::TorrentDetailResponse>, AppError> {
    let id = torrents::parse_torrent_id(&id)?;
    let mut detail = torrents::get_torrent(
        &state.api,
        &state.session,
        id,
        state.downloads.base(),
    )?;
    state.stats.enrich_detail(&mut detail);
    state.labels.enrich_detail(&mut detail);
    if !user.is_admin() {
        detail.summary.output_folder.clear();
        // Tracker announce URLs often contain private passkeys.
        detail.trackers.clear();
    }
    Ok(Json(detail))
}

fn redact_torrent_paths(response: &mut TorrentListResponse) {
    for torrent in &mut response.torrents {
        torrent.output_folder.clear();
    }
}

#[derive(Deserialize)]
struct AddTorrentRequest {
    magnet: String,
    #[serde(default)]
    paused: bool,
    #[serde(default)]
    folder: Option<String>,
    #[serde(default)]
    label: Option<String>,
}

async fn add_torrent(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<AddTorrentRequest>,
) -> Result<Json<torrents::TorrentDetailResponse>, AppError> {
    user.require_admin()?;
    let magnet = request.magnet.trim();
    if magnet.len() < 20 || magnet.len() > 8192 {
        return Err(AppError::bad_request("Magnet-linket er ugyldigt."));
    }
    let is_hash = magnet.len() == 40 && magnet.chars().all(|c| c.is_ascii_hexdigit());
    if !magnet.starts_with("magnet:?") && !is_hash {
        return Err(AppError::bad_request(
            "Indsæt et magnet-link eller en 40-tegns info-hash. Upload en .torrent-fil for filbaseret tilføjelse.",
        ));
    }
    let output_folder = state
        .downloads
        .resolve(request.folder.as_deref())
        .map_err(|err| AppError::bad_request(err))?;
    let response = state
        .api
        .api_add_torrent(
            librqbit::AddTorrent::from_url(magnet.to_owned()),
            Some(librqbit::AddTorrentOptions {
                paused: request.paused,
                output_folder: Some(output_folder.to_string_lossy().into_owned()),
                ..Default::default()
            }),
        )
        .await
        .map_err(|err| {
            tracing::debug!("could not add torrent: {err:#}");
            AppError::bad_request("Torrentet kunne ikke tilføjes. Kontrollér magnet-linket.")
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
        .assign(&detail.summary.info_hash, request.label.as_deref())
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    state.labels.enrich_detail(&mut detail);
    Ok(Json(detail))
}

async fn pause_torrent(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    user.require_admin()?;
    let id = torrents::parse_torrent_id(&id)?;
    state
        .api
        .api_torrent_action_pause(id)
        .await
        .map_err(|err| {
            tracing::debug!("pause failed: {err:#}");
            AppError::bad_request("Torrentet kunne ikke pauses.")
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn resume_torrent(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    user.require_admin()?;
    let id = torrents::parse_torrent_id(&id)?;
    state
        .api
        .api_torrent_action_start(id)
        .await
        .map_err(|err| {
            tracing::debug!("resume failed: {err:#}");
            AppError::bad_request("Torrentet kunne ikke startes.")
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
struct DeleteQuery {
    #[serde(default)]
    delete_files: bool,
}

async fn remove_torrent(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<DeleteQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    user.require_admin()?;
    let id = torrents::parse_torrent_id(&id)?;
    if let Ok(details) = state.api.api_torrent_details(id) {
        state.labels.forget(&details.info_hash);
    }
    if query.delete_files {
        state
            .api
            .api_torrent_action_delete(id)
            .await
            .map_err(|err| {
                tracing::debug!("delete with files failed: {err:#}");
                AppError::bad_request("Torrentet kunne ikke slettes.")
            })?;
    } else {
        state
            .api
            .api_torrent_action_forget(id)
            .await
            .map_err(|err| {
                tracing::debug!("forget failed: {err:#}");
                AppError::bad_request("Torrentet kunne ikke fjernes.")
            })?;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
struct UpdateFilesRequest {
    included: Vec<usize>,
}

async fn update_files(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<UpdateFilesRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    user.require_admin()?;
    let id = torrents::parse_torrent_id(&id)?;
    if request.included.is_empty() {
        return Err(AppError::bad_request(
            "Vælg mindst én fil, der skal downloades.",
        ));
    }
    let included: std::collections::HashSet<usize> = request.included.into_iter().collect();
    state
        .api
        .api_torrent_action_update_only_files(id, &included)
        .await
        .map_err(|err| {
            tracing::debug!("file selection failed: {err:#}");
            AppError::bad_request("Filvalget kunne ikke gemmes.")
        })?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn library(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Json<torrents::LibraryResponse> {
    Json(torrents::get_library(&state.api, state.downloads.base()))
}

async fn thumbnail(
    State(state): State<AppState>,
    _user: AuthUser,
    axum::extract::Path((id, file_id)): axum::extract::Path<(String, usize)>,
) -> Result<Response, AppError> {
    let id = torrents::parse_torrent_id(&id)?;
    let (source, info_hash) =
        torrents::file_disk_path(&state.api, id, file_id, state.downloads.base())?;
    let cache = crate::thumbnails::ensure_thumbnail(
        &state.config.data_dir,
        &info_hash,
        file_id,
        &source,
    )
    .await
    .map_err(|err| {
        tracing::debug!("thumbnail generation failed for {}: {err:#}", source.display());
        AppError::not_found("Der kunne ikke laves et preview af denne fil.")
    })?;
    let bytes = tokio::fs::read(&cache)
        .await
        .map_err(|err| AppError::internal("could not read thumbnail", err))?;
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/jpeg"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    Ok(response)
}

async fn download_folders(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<DownloadFolder>>, AppError> {
    user.require_admin()?;
    Ok(Json(state.downloads.list()))
}

#[derive(Deserialize)]
struct DownloadBrowserQuery {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    q: Option<String>,
}

async fn download_browser(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Query(query): axum::extract::Query<DownloadBrowserQuery>,
) -> Result<Json<DownloadBrowser>, AppError> {
    user.require_admin()?;
    state
        .downloads
        .browse(query.path.as_deref(), query.q.as_deref())
        .map(Json)
        .map_err(|err| AppError::bad_request(err))
}

async fn network_status(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Json<NetworkStatus> {
    Json(
        state
            .network
            .status(state.session.tcp_listen_port(), false)
            .await,
    )
}

async fn check_network(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Json<NetworkStatus> {
    Json(
        state
            .network
            .status(state.session.tcp_listen_port(), true)
            .await,
    )
}

#[derive(Deserialize)]
struct UpdateNetworkRequest {
    #[serde(default)]
    external_ip: Option<String>,
    #[serde(default)]
    external_port: Option<u16>,
}

async fn speed_limits(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<SpeedLimitSettings>, AppError> {
    user.require_admin()?;
    Ok(Json(state.limits.settings()))
}

async fn update_speed_limits(
    State(state): State<AppState>,
    user: AuthUser,
    Json(settings): Json<SpeedLimitSettings>,
) -> Result<Json<SpeedLimitSettings>, AppError> {
    user.require_admin()?;
    state
        .limits
        .update(settings)
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    let current = state.limits.settings();
    let config = current.to_limits_config();
    state.session.ratelimits.set_download_bps(config.download_bps);
    state.session.ratelimits.set_upload_bps(config.upload_bps);
    Ok(Json(current))
}

async fn seeding_settings(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<SeedingSettings>, AppError> {
    user.require_admin()?;
    Ok(Json(state.seeding.settings()))
}

async fn update_seeding_settings(
    State(state): State<AppState>,
    user: AuthUser,
    Json(settings): Json<SeedingSettings>,
) -> Result<Json<SeedingSettings>, AppError> {
    user.require_admin()?;
    state
        .seeding
        .update_settings(settings)
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    Ok(Json(state.seeding.settings()))
}

async fn dht_settings(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<DhtSettings>, AppError> {
    user.require_admin()?;
    Ok(Json(state.dht.settings()))
}

async fn update_dht_settings(
    State(state): State<AppState>,
    user: AuthUser,
    Json(settings): Json<DhtSettings>,
) -> Result<Json<DhtSettings>, AppError> {
    user.require_admin()?;
    state
        .dht
        .update(settings)
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    Ok(Json(state.dht.settings()))
}

async fn i2p_settings(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<I2pSettings>, AppError> {
    user.require_admin()?;
    Ok(Json(state.i2p.settings()))
}

async fn update_i2p_settings(
    State(state): State<AppState>,
    user: AuthUser,
    Json(settings): Json<I2pSettings>,
) -> Result<Json<I2pSettings>, AppError> {
    user.require_admin()?;
    let settings = state
        .i2p
        .update(settings)
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    // SOCKS routing and I2P-only mode are read when the torrent session starts.
    Ok(Json(settings))
}

async fn test_i2p(
    user: AuthUser,
    Json(settings): Json<I2pSettings>,
) -> Result<Json<i2p::I2pProbe>, AppError> {
    user.require_admin()?;
    settings
        .validate()
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    Ok(Json(i2p::probe(&settings).await))
}

async fn list_rss(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<RssFeedView>>, AppError> {
    user.require_admin()?;
    Ok(Json(state.rss.feeds()))
}

async fn create_rss(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<RssFeedInput>,
) -> Result<Json<RssFeedView>, AppError> {
    user.require_admin()?;
    state
        .rss
        .add(input, &state.downloads)
        .map(Json)
        .map_err(|err| AppError::bad_request(err.to_string()))
}

async fn update_rss(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(input): Json<RssFeedInput>,
) -> Result<Json<RssFeedView>, AppError> {
    user.require_admin()?;
    state
        .rss
        .update(&id, input, &state.downloads)
        .map(Json)
        .map_err(|err| AppError::bad_request(err.to_string()))
}

async fn delete_rss(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    user.require_admin()?;
    state
        .rss
        .delete(&id)
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn check_rss(
    State(state): State<AppState>,
    user: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<RssFeedView>, AppError> {
    user.require_admin()?;
    state
        .rss
        .check(&id, &state.api, &state.downloads)
        .await
        .map(Json)
        .map_err(|err| AppError::bad_request(err.to_string()))
}

async fn update_network(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<UpdateNetworkRequest>,
) -> Result<Json<NetworkStatus>, AppError> {
    user.require_admin()?;
    state
        .network
        .update(request.external_ip, request.external_port)
        .await
        .map_err(|err| AppError::bad_request(err.to_string()))?;
    Ok(Json(
        state
            .network
            .status(state.session.tcp_listen_port(), true)
            .await,
    ))
}

fn auth_response(state: &AppState, user: PublicUser, token: String) -> Response {
    let mut response = Json(AuthResponse {
        user,
        access_token: token.clone(),
        token_type: "Bearer",
        expires_in: state.auth.ttl_seconds(),
    })
    .into_response();
    if let Ok(value) = HeaderValue::from_str(&state.auth.session_cookie(&token)) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    no_store(response)
}

fn client_key(
    state: &AppState,
    addr: SocketAddr,
    headers: Option<&axum::http::HeaderMap>,
) -> String {
    if state.config.trust_proxy {
        if let Some(headers) = headers {
            if let Some(value) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
                if let Some(first) = value.split(',').next() {
                    let first = first.trim();
                    if !first.is_empty() {
                        return first.to_owned();
                    }
                }
            }
        }
    }
    addr.ip().to_string()
}

fn is_private_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || (octets[0] == 100 && (octets[1] & 0xc0) == 64)
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unicast_link_local()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

/// Collects every address that can identify the caller: the socket peer plus
/// the forwarding headers Cloudflare and reverse proxies add.
fn client_addresses(addr: SocketAddr, headers: Option<&HeaderMap>) -> Vec<IpAddr> {
    let mut addresses = vec![addr.ip()];
    let Some(headers) = headers else {
        return addresses;
    };
    for name in ["cf-connecting-ip", "x-real-ip", "true-client-ip"] {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            if let Ok(ip) = value.trim().parse::<IpAddr>() {
                addresses.push(ip);
            }
        }
    }
    if let Some(value) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        for part in value.split(',') {
            if let Ok(ip) = part.trim().parse::<IpAddr>() {
                addresses.push(ip);
            }
        }
    }
    addresses
}

/// Local logins may skip 2FA. A request counts as local only when *every*
/// address we can see is private.
///
/// This matters behind a tunnel or reverse proxy: the socket peer is then
/// 127.0.0.1 or a LAN address, so looking at the peer alone would treat every
/// internet login as local and silently skip the 2FA challenge. Cloudflare adds
/// `CF-Connecting-IP`, proxies add `X-Forwarded-For`/`X-Real-IP`, and any public
/// address in those headers marks the request as remote.
fn is_local_client(addr: SocketAddr, headers: Option<&HeaderMap>) -> bool {
    client_addresses(addr, headers)
        .iter()
        .all(|ip| is_private_address(*ip))
}

async fn guard_origin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let method = request.method();
    let safe = matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS);
    if !safe && request.uri().path().starts_with("/api/") {
        let origin = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        let extension_origin = origin.starts_with("chrome-extension://");
        let bearer = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("Bearer "));
        let extension_login = extension_origin
            && request.uri().path() == "/api/v1/auth/login"
            && !bearer;
        if let Some(fetch_site) = request
            .headers()
            .get("sec-fetch-site")
            .and_then(|value| value.to_str().ok())
        {
            if fetch_site == "cross-site" && !(extension_origin && (bearer || extension_login)) {
                return Err(AppError::forbidden("Cross-site anmodning afvist."));
            }
        }
        if !origin_allowed(&state, request.headers())
            && !(extension_origin && (bearer || extension_login))
        {
            return Err(AppError::forbidden("Origin er ikke tilladt."));
        }
    }
    Ok(next.run(request).await)
}

async fn security_headers(request: Request, next: Next) -> Response {
    let extension_origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .filter(|origin| origin.starts_with("chrome-extension://"))
        .map(str::to_owned);
    let mut response = next.run(request).await;
    if let Some(origin) = extension_origin {
        if let Ok(value) = HeaderValue::from_str(&origin) {
            response
                .headers_mut()
                .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
        }
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(
                "authorization, content-type, x-nordtorrent-client, x-nordtorrent-client-version",
            ),
        );
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, DELETE, OPTIONS"),
        );
        response
            .headers_mut()
            .insert(header::VARY, HeaderValue::from_static("Origin"));
    }
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; img-src 'self' data:; media-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'; connect-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        ),
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        "cross-origin-opener-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        "cross-origin-resource-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    response
}

fn no_cache(response: impl IntoResponse) -> Response {
    let mut response = response.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    response
}

fn no_store(response: impl IntoResponse) -> Response {
    let mut response = response.into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn asset(content_type: &'static str, body: &'static str) -> Response {
    let mut response = body.into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, max-age=0"),
    );
    response
}



#[cfg(test)]
mod locality_tests {
    use super::{client_addresses, is_local_client, is_private_address};
    use axum::http::{HeaderMap, HeaderValue};
    use std::net::{IpAddr, SocketAddr};

    fn addr(value: &str) -> SocketAddr {
        format!("{value}:1234").parse().unwrap()
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    #[test]
    fn classifies_ranges() {
        assert!(is_private_address("127.0.0.1".parse().unwrap()));
        assert!(is_private_address("192.168.1.10".parse().unwrap()));
        assert!(is_private_address("10.0.0.5".parse().unwrap()));
        assert!(is_private_address("100.64.1.2".parse().unwrap()));
        assert!(is_private_address("fd00::1".parse().unwrap()));
        assert!(is_private_address("fe80::1".parse().unwrap()));
        assert!(!is_private_address("203.0.113.10".parse().unwrap()));
        assert!(!is_private_address("2a00:1450:400f::1".parse().unwrap()));
    }

    #[test]
    fn direct_lan_login_is_local() {
        let peer = addr("192.168.1.20");
        assert!(is_local_client(peer, None));
        assert!(is_local_client(peer, Some(&HeaderMap::new())));
    }

    #[test]
    fn loopback_peer_without_headers_is_local() {
        assert!(is_local_client(addr("127.0.0.1"), None));
    }

    #[test]
    fn cloudflare_header_makes_a_loopback_proxy_remote() {
        let map = headers(&[("cf-connecting-ip", "203.0.113.10")]);
        assert!(!is_local_client(addr("127.0.0.1"), Some(&map)));
    }

    #[test]
    fn x_forwarded_for_public_hop_makes_it_remote() {
        let map = headers(&[("x-forwarded-for", "203.0.113.10, 192.168.1.2")]);
        assert!(!is_local_client(addr("127.0.0.1"), Some(&map)));
        assert!(!is_local_client(addr("192.168.1.2"), Some(&map)));
    }

    #[test]
    fn x_real_ip_is_honoured() {
        let map = headers(&[("x-real-ip", "198.51.100.7")]);
        assert!(!is_local_client(addr("10.0.0.2"), Some(&map)));
    }

    #[test]
    fn forwarding_private_only_stays_local() {
        let map = headers(&[("x-forwarded-for", "192.168.1.30")]);
        assert!(is_local_client(addr("127.0.0.1"), Some(&map)));
    }

    #[test]
    fn public_socket_peer_wins_over_spoofed_private_header() {
        let map = headers(&[("x-forwarded-for", "192.168.1.30")]);
        assert!(!is_local_client(addr("203.0.113.10"), Some(&map)));
    }

    #[test]
    fn collects_peer_and_forwarding_addresses() {
        let map = headers(&[
            ("cf-connecting-ip", "203.0.113.10"),
            ("x-forwarded-for", "198.51.100.6, 10.0.0.1"),
        ]);
        let addresses: Vec<IpAddr> = client_addresses(addr("127.0.0.1"), Some(&map));
        assert_eq!(addresses.len(), 4);
        assert_eq!(addresses[0], "127.0.0.1".parse::<IpAddr>().unwrap());
    }
}
