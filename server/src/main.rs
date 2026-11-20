mod api;
mod auth;
mod config;
mod discovery;
mod downloads;
mod error;
mod labels;
mod limits;
mod network;
mod rss;
mod seeding;
mod state;
mod stats;
mod thumbnails;
mod torrents;

use std::{net::SocketAddr, sync::Arc};

use anyhow::{bail, Context, Result};
use librqbit::{dht::PersistentDhtConfig, Session, SessionOptions, SessionPersistenceConfig};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::{
    auth::AuthService, config::Config, discovery::DhtStore, downloads::DownloadFolders,
    labels::LabelStore, limits::RateLimitStore, network::NetworkService, rss::RssStore,
    seeding::SeedingStore, state::AppState, stats::StatsStore,
};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let config = Arc::new(Config::from_env().context("invalid configuration")?);

    info!(version = api::VERSION, bind = %config.display_bind(), "starting NordTorrent");
    info!(download_dir = %config.download_dir.display(), "download directory");
    info!(data_dir = %config.data_dir.display(), "data directory");

    let auth = AuthService::load(
        config.users_path(),
        &config.secret_path(),
        config.session_ttl,
        config.cookie_secure,
    )
    .context("could not initialize authentication")?;

    bootstrap_admin(&config, &auth).await?;

    let speed_limits = Arc::new(
        RateLimitStore::load(config.speed_limits_path())
            .context("could not load speed limit settings")?,
    );
    let dht = Arc::new(
        DhtStore::load(config.dht_settings_path()).context("could not load DHT settings")?,
    );
    let seeding = Arc::new(
        SeedingStore::load(config.seeding_path()).context("could not load seeding settings")?,
    );
    let rss = Arc::new(RssStore::load(config.rss_path()).context("could not load RSS settings")?);
    let labels = Arc::new(
        LabelStore::load(config.labels_path()).context("could not load labels")?,
    );

    let mut options = SessionOptions {
        fastresume: true,
        persistence: Some(SessionPersistenceConfig::Json {
            folder: Some(config.session_dir()),
        }),
        dht_config: Some(PersistentDhtConfig {
            dump_interval: None,
            config_filename: Some(config.dht_path()),
        }),
        listen_port_range: Some(
            config.torrent_port_start..config.torrent_port_end.saturating_add(1),
        ),
        enable_upnp_port_forwarding: false,
        defer_writes_up_to: Some(64),
        ratelimits: speed_limits.limits_config(),
        disable_dht: !dht.settings().enabled,
        ..Default::default()
    };
    options
        .cancellation_token
        .get_or_insert_with(Default::default);

    let session = Session::new_with_opts(config.download_dir.clone(), options)
        .await
        .context("could not start the torrent engine (are the torrent ports free?)")?;

    if let Some(port) = session.tcp_listen_port() {
        info!(port, "torrent peer TCP port");
    }

    let downloads = Arc::new(DownloadFolders::new(
        config.download_dir.clone(),
        config.default_download_folder.clone(),
    ));
    let network = Arc::new(
        NetworkService::load(
            config.network_settings_path.clone(),
            config.external_ip.clone(),
            config.external_port,
            config.ip_lookup_url.clone(),
            config.torrent_port_start,
            config.torrent_port_end,
            config.public_url.clone(),
        )
        .context("could not load network settings")?,
    );
    let stats = StatsStore::load(config.stats_path())
        .context("could not load torrent statistics")?;
    let state = AppState::new(
        config.clone(),
        auth,
        session.clone(),
        downloads,
        network,
        stats,
        speed_limits,
        dht,
        seeding,
        rss,
        labels,
    );
    state
        .stats
        .spawn_recorder(state.api.clone(), config.download_dir.clone());
    state
        .seeding
        .clone()
        .spawn_monitor(state.api.clone(), config.download_dir.clone());
    state
        .rss
        .clone()
        .spawn_poller(state.api.clone(), state.downloads.clone());
    let shutdown_state = state.clone();
    let app = api::router(state);
    let listener = TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("could not bind to {}", config.bind))?;
    let local_addr = listener.local_addr()?;
    info!(address = %local_addr, "web UI and API listening");

    if config.public_url.is_none() {
        tracing::warn!(
            "NORDTORRENT_PUBLIC_URL is not set; set it to your external URL for the strongest origin checks"
        );
    }
    if config.is_local_url() {
        tracing::warn!("the configured public URL uses plain HTTP; terminate TLS in a reverse proxy for remote access");
    }

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("web server failed")?;

    if let Err(err) = shutdown_state
        .stats
        .flush(&shutdown_state.api, &config.download_dir)
    {
        tracing::warn!("could not flush torrent statistics: {err:#}");
    }
    info!("shutting down torrent session");
    session.stop().await;
    Ok(())
}

async fn bootstrap_admin(config: &Config, auth: &AuthService) -> Result<()> {
    match (
        config.bootstrap_admin_user.as_deref(),
        config.bootstrap_admin_password.as_deref(),
    ) {
        (Some(_), None) | (None, Some(_)) => {
            bail!("set both NORDTORRENT_ADMIN_USER and NORDTORRENT_ADMIN_PASSWORD, or neither")
        }
        (Some(username), Some(password)) => match auth.bootstrap_admin(username, password).await {
            Ok(Some(user)) => info!(username = %user.username, "created bootstrap administrator"),
            Ok(None) => {}
            Err(err) => return Err(err).context("could not create bootstrap administrator"),
        },
        (None, None) => {}
    }
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!("could not install Ctrl-C handler: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                tracing::error!("could not install SIGTERM handler: {err}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("nordtorrent=info,tower_http=info,librqbit=warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
