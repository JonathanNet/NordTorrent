use std::{sync::Arc, time::Instant};

use librqbit::{Api, Session};

use crate::{
    i2p::I2pStore,
    auth::AuthService, config::Config, discovery::DhtStore, downloads::DownloadFolders,
    labels::LabelStore, limits::RateLimitStore, network::NetworkService, rss::RssStore,
    seeding::SeedingStore, stats::StatsStore,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub auth: Arc<AuthService>,
    pub api: Api,
    pub downloads: Arc<DownloadFolders>,
    pub network: Arc<NetworkService>,
    pub stats: Arc<StatsStore>,
    pub limits: Arc<RateLimitStore>,
    pub dht: Arc<DhtStore>,
    pub seeding: Arc<SeedingStore>,
    pub rss: Arc<RssStore>,
    pub labels: Arc<LabelStore>,
    pub i2p: Arc<I2pStore>,
    #[allow(dead_code)]
    pub session: Arc<Session>,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(
        config: Arc<Config>,
        auth: Arc<AuthService>,
        session: Arc<Session>,
        downloads: Arc<DownloadFolders>,
        network: Arc<NetworkService>,
        stats: Arc<StatsStore>,
        limits: Arc<RateLimitStore>,
        dht: Arc<DhtStore>,
        seeding: Arc<SeedingStore>,
        rss: Arc<RssStore>,
        labels: Arc<LabelStore>,
        i2p: Arc<I2pStore>,
    ) -> Self {
        let api = Api::new(
            session.clone(),
            None::<tokio::sync::mpsc::UnboundedSender<String>>,
        );
        Self {
            config,
            auth,
            api,
            downloads,
            network,
            stats,
            limits,
            dht,
            seeding,
            rss,
            labels,
            i2p,
            session,
            started_at: Instant::now(),
        }
    }
}
