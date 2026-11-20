use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpStream,
    sync::RwLock,
    time::timeout,
};

use crate::auth::atomic_write;

const IP_CACHE_SECONDS: u64 = 10 * 60;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct NetworkSettings {
    #[serde(default)]
    external_ip: Option<String>,
    #[serde(default)]
    external_port: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PortStatus {
    Open,
    Closed,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkStatus {
    pub listen_port: Option<u16>,
    pub port_start: u16,
    pub port_end: u16,
    pub configured_external_ip: Option<String>,
    pub configured_external_port: Option<u16>,
    pub external_ip: Option<String>,
    pub external_port: Option<u16>,
    pub external_endpoint: Option<String>,
    pub ip_source: &'static str,
    pub status: PortStatus,
    pub message: String,
    pub checked_at: String,
    pub public_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct NetworkSettingsFile {
    external_ip: Option<String>,
    external_port: Option<u16>,
}

pub struct NetworkService {
    path: PathBuf,
    lookup_url: String,
    port_start: u16,
    port_end: u16,
    public_url: Option<String>,
    settings: RwLock<NetworkSettings>,
    cached_ip: RwLock<Option<(IpAddr, Instant)>>,
}

impl NetworkService {
    pub fn load(
        path: PathBuf,
        external_ip: Option<String>,
        external_port: Option<u16>,
        lookup_url: String,
        port_start: u16,
        port_end: u16,
        public_url: Option<String>,
    ) -> Result<Self> {
        let mut settings = NetworkSettings {
            external_ip,
            external_port,
        };
        if path.exists() {
            match std::fs::read(&path) {
                Ok(bytes) => match serde_json::from_slice::<NetworkSettingsFile>(&bytes) {
                    Ok(file) => {
                        settings.external_ip = file.external_ip;
                        settings.external_port = file.external_port;
                    }
                    Err(err) => {
                        tracing::warn!("ignoring invalid network settings {}: {err}", path.display());
                    }
                },
                Err(err) => {
                    tracing::warn!("could not read network settings {}: {err}", path.display());
                }
            }
        }
        Ok(Self {
            path,
            lookup_url,
            port_start,
            port_end,
            public_url,
            settings: RwLock::new(settings),
            cached_ip: RwLock::new(None),
        })
    }

    pub async fn update(
        &self,
        external_ip: Option<String>,
        external_port: Option<u16>,
    ) -> Result<()> {
        let external_ip = normalize_external_ip(external_ip)?;
        if let Some(port) = external_port {
            if port == 0 {
                anyhow::bail!("Ekstern port skal være mellem 1 og 65535.");
            }
        }
        let file = NetworkSettingsFile {
            external_ip: external_ip.clone(),
            external_port,
        };
        let bytes = serde_json::to_vec_pretty(&file)?;
        atomic_write(&self.path, &bytes).context("kunne ikke gemme netværksindstillinger")?;
        let mut settings = self.settings.write().await;
        settings.external_ip = external_ip;
        settings.external_port = external_port;
        Ok(())
    }

    pub async fn status(&self, listen_port: Option<u16>, force_ip_refresh: bool) -> NetworkStatus {
        let settings = self.settings.read().await.clone();
        let configured_ip = settings.external_ip.clone();
        let configured_port = settings.external_port;
        let mut message = String::new();

        let (external_ip, ip_source) = match configured_ip.as_deref() {
            Some(value) => match IpAddr::from_str(value) {
                Ok(ip) => (Some(ip), "manual"),
                Err(_) => {
                    message = "Den angivne eksterne IP er ugyldig.".to_owned();
                    (None, "manual")
                }
            },
            None => match self.discover_public_ip(force_ip_refresh).await {
                Ok(ip) => (Some(ip), "auto"),
                Err(err) => {
                    message = format!(
                        "Kunne ikke finde den offentlige IP automatisk: {err}. Udfyld den manuelt."
                    );
                    (None, "unknown")
                }
            },
        };

        let external_port = configured_port.or(listen_port);
        let endpoint = match (external_ip, external_port) {
            (Some(ip), Some(port)) if port > 0 => Some(format_endpoint(ip, port)),
            _ => None,
        };
        let (status, check_message) = match (external_ip, external_port) {
            (Some(ip), Some(port)) if port > 0 => {
                let (status, check_message) = check_port(ip, port).await;
                (status, check_message)
            }
            _ => (
                PortStatus::Unknown,
                "Angiv en ekstern IP og port, eller start torrentmotoren.".to_owned(),
            ),
        };
        if !message.is_empty() && status != PortStatus::Open {
            message = format!("{message} {check_message}");
        } else {
            message = check_message;
        }

        NetworkStatus {
            listen_port,
            port_start: self.port_start,
            port_end: self.port_end,
            configured_external_ip: configured_ip,
            configured_external_port: configured_port,
            external_ip: external_ip.map(|ip| ip.to_string()),
            external_port,
            external_endpoint: endpoint,
            ip_source,
            status,
            message,
            checked_at: Utc::now().to_rfc3339(),
            public_url: self.public_url.clone(),
        }
    }

    async fn discover_public_ip(&self, force: bool) -> Result<IpAddr> {
        if !force {
            if let Some((ip, at)) = *self.cached_ip.read().await {
                if at.elapsed() < Duration::from_secs(IP_CACHE_SECONDS) {
                    return Ok(ip);
                }
            }
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .user_agent("NordTorrent/0.1 public-ip-check")
            .build()
            .context("kunne ikke oprette HTTP-klient")?;
        let response = client
            .get(&self.lookup_url)
            .send()
            .await
            .with_context(|| format!("kunne ikke kontakte {}", self.lookup_url))?;
        let text = response
            .error_for_status()
            .context("IP-tjenesten svarede med en fejl")?
            .text()
            .await
            .context("kunne ikke læse IP-svaret")?;
        let ip = IpAddr::from_str(text.trim())
            .context("IP-tjenesten returnerede ikke en gyldig IP-adresse")?;
        *self.cached_ip.write().await = Some((ip, Instant::now()));
        Ok(ip)
    }
}

fn normalize_external_ip(value: Option<String>) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let ip = IpAddr::from_str(value).context("Ekstern IP skal være en gyldig IPv4- eller IPv6-adresse.")?;
    Ok(Some(ip.to_string()))
}

fn format_endpoint(ip: IpAddr, port: u16) -> String {
    if ip.is_ipv6() {
        format!("[{ip}]:{port}")
    } else {
        format!("{ip}:{port}")
    }
}

async fn check_port(ip: IpAddr, port: u16) -> (PortStatus, String) {
    let address = SocketAddr::new(ip, port);
    match timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await {
        Ok(Ok(_)) => (
            PortStatus::Open,
            "Porten svarede fra den offentlige IP. Andre bør kunne forbinde.".to_owned(),
        ),
        Ok(Err(err)) if err.kind() == std::io::ErrorKind::ConnectionRefused => (
            PortStatus::Closed,
            "Forbindelsen blev afvist. Tjek portforwarding og firewall. Nogle routere afviser også NAT-loopback, selv om porten er åben udefra."
                .to_owned(),
        ),
        Ok(Err(err)) => (
            PortStatus::Unknown,
            format!("Kunne ikke afgøre portstatus: {err}. Tjek portforwarding/NAT-loopback."),
        ),
        Err(_) => (
            PortStatus::Unknown,
            "Porttjek fik timeout. Tjek portforwarding, firewall og at routeren tillader loopback."
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_ip() {
        assert_eq!(
            normalize_external_ip(Some(" 203.0.113.10 ".to_owned())).unwrap(),
            Some("203.0.113.10".to_owned())
        );
        assert_eq!(normalize_external_ip(Some(" ".to_owned())).unwrap(), None);
        assert!(normalize_external_ip(Some("example.com".to_owned())).is_err());
    }

    #[test]
    fn formats_ipv6_endpoint() {
        let ip = IpAddr::from_str("2001:db8::1").unwrap();
        assert_eq!(format_endpoint(ip, 42000), "[2001:db8::1]:42000");
    }
}
