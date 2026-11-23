use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub download_dir: PathBuf,
    pub public_url: Option<String>,
    pub cookie_secure: bool,
    pub trust_proxy: bool,
    pub session_ttl: Duration,
    pub torrent_port_start: u16,
    pub torrent_port_end: u16,
    pub external_ip: Option<String>,
    pub external_port: Option<u16>,
    pub ip_lookup_url: String,
    pub network_settings_path: PathBuf,
    pub default_download_folder: Option<String>,
    pub bootstrap_admin_user: Option<String>,
    pub bootstrap_admin_password: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let bind = env::var("NORDTORRENT_LISTEN")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
            .parse()
            .context("NORDTORRENT_LISTEN must be an address like 0.0.0.0:8080")?;

        let data_dir =
            PathBuf::from(env::var("NORDTORRENT_DATA_DIR").unwrap_or_else(|_| "./data".to_owned()));
        let download_dir = PathBuf::from(
            env::var("NORDTORRENT_DOWNLOAD_DIR").unwrap_or_else(|_| "./downloads".to_owned()),
        );

        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("could not create data dir {}", data_dir.display()))?;
        std::fs::create_dir_all(&download_dir)
            .with_context(|| format!("could not create download dir {}", download_dir.display()))?;

        let public_url = env::var("NORDTORRENT_PUBLIC_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_owned())
            .filter(|value| !value.is_empty());
        if let Some(url) = &public_url {
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                bail!("NORDTORRENT_PUBLIC_URL must start with http:// or https://");
            }
        }

        let cookie_secure = parse_bool("NORDTORRENT_COOKIE_SECURE", false)?;
        let trust_proxy = parse_bool("NORDTORRENT_TRUST_PROXY", false)?;
        let session_hours = parse_number("NORDTORRENT_SESSION_HOURS", 720_u64)?;
        if session_hours == 0 || session_hours > 24 * 365 {
            bail!("NORDTORRENT_SESSION_HOURS must be between 1 and 8760");
        }

        let torrent_port_start = parse_number("NORDTORRENT_TORRENT_PORT_START", 42000_u16)?;
        let torrent_port_end = parse_number("NORDTORRENT_TORRENT_PORT_END", 42009_u16)?;
        if torrent_port_start > torrent_port_end {
            bail!("NORDTORRENT_TORRENT_PORT_START must be <= NORDTORRENT_TORRENT_PORT_END");
        }

        let external_ip = env::var("NORDTORRENT_EXTERNAL_IP")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let external_port = match env::var("NORDTORRENT_EXTERNAL_PORT") {
            Ok(value) if !value.trim().is_empty() => {
                let port = value
                    .trim()
                    .parse::<u16>()
                    .context("NORDTORRENT_EXTERNAL_PORT must be between 1 and 65535")?;
                if port == 0 {
                    bail!("NORDTORRENT_EXTERNAL_PORT must be between 1 and 65535");
                }
                Some(port)
            }
            _ => None,
        };
        let ip_lookup_url = env::var("NORDTORRENT_IP_LOOKUP_URL")
            .unwrap_or_else(|_| "https://api.ipify.org".to_owned());
        if !(ip_lookup_url.starts_with("https://") || ip_lookup_url.starts_with("http://")) {
            bail!("NORDTORRENT_IP_LOOKUP_URL must start with http:// or https://");
        }
        let network_settings_path = data_dir.join("network.json");

        let default_download_folder = env::var("NORDTORRENT_DEFAULT_DOWNLOAD_FOLDER")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let bootstrap_admin_user = env::var("NORDTORRENT_ADMIN_USER")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let bootstrap_admin_password = env::var("NORDTORRENT_ADMIN_PASSWORD")
            .ok()
            .filter(|value| !value.is_empty());

        Ok(Self {
            bind,
            data_dir,
            download_dir,
            public_url,
            cookie_secure,
            trust_proxy,
            session_ttl: Duration::from_secs(session_hours * 3600),
            torrent_port_start,
            torrent_port_end,
            external_ip,
            external_port,
            ip_lookup_url,
            network_settings_path,
            default_download_folder,
            bootstrap_admin_user,
            bootstrap_admin_password,
        })
    }

    pub fn users_path(&self) -> PathBuf {
        self.data_dir.join("users.json")
    }

    pub fn secret_path(&self) -> PathBuf {
        self.data_dir.join("session-secret")
    }

    pub fn session_dir(&self) -> PathBuf {
        self.data_dir.join("session")
    }

    pub fn dht_path(&self) -> PathBuf {
        self.data_dir.join("dht.json")
    }

    pub fn stats_path(&self) -> PathBuf {
        self.data_dir.join("torrent-stats.json")
    }

    pub fn speed_limits_path(&self) -> PathBuf {
        self.data_dir.join("speed-limits.json")
    }

    pub fn seeding_path(&self) -> PathBuf {
        self.data_dir.join("seeding.json")
    }

    pub fn dht_settings_path(&self) -> PathBuf {
        self.data_dir.join("dht.json")
    }

    pub fn rss_path(&self) -> PathBuf {
        self.data_dir.join("rss.json")
    }

    pub fn labels_path(&self) -> PathBuf {
        self.data_dir.join("labels.json")
    }

    pub fn i2p_path(&self) -> PathBuf {
        self.data_dir.join("i2p.json")
    }

    pub fn is_local_url(&self) -> bool {
        self.public_url
            .as_deref()
            .map(|u| u.starts_with("http://"))
            .unwrap_or(false)
    }

    pub fn display_bind(&self) -> String {
        if self.bind.ip().is_unspecified() {
            match self.bind.ip() {
                IpAddr::V4(_) => format!("{}:{}", Ipv4Addr::UNSPECIFIED, self.bind.port()),
                IpAddr::V6(_) => format!("[::]:{}", self.bind.port()),
            }
        } else {
            self.bind.to_string()
        }
    }
}

pub fn ensure_private_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, permissions)
            .with_context(|| format!("could not secure {}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn parse_bool(name: &str, default: bool) -> Result<bool> {
    match env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => bail!("{name} must be true/false"),
        },
        Err(_) => Ok(default),
    }
}

fn parse_number<T>(name: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) => value
            .trim()
            .parse()
            .map_err(|err| anyhow::anyhow!("{name} is invalid: {err}")),
        Err(_) => Ok(default),
    }
}
