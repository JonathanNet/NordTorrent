use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use tracing::warn;

/// I2P support is experimental, like in qBittorrent. NordTorrent talks to the
/// I2P router's SAM bridge to verify connectivity and to obtain a destination,
/// and routes tracker/peer traffic through the router's SOCKS proxy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct I2pSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    /// When false, NordTorrent runs I2P-only: DHT is disabled so the client does
    /// not announce itself on the clearnet.
    #[serde(default)]
    pub mixed_mode: bool,
    /// Peer exchange over I2P. Stored per request from the UI; applied when the
    /// engine supports toggling PEX.
    #[serde(default = "default_true")]
    pub pex: bool,
    /// SOCKS5 endpoint of the I2P router. Peer and tracker traffic is proxied
    /// through this when I2P is enabled.
    #[serde(default = "default_socks")]
    pub socks_proxy: String,
}

fn default_host() -> String {
    "127.0.0.1".to_owned()
}

fn default_port() -> u16 {
    7656
}

fn default_socks() -> String {
    "127.0.0.1:4447".to_owned()
}

fn default_true() -> bool {
    true
}

impl Default for I2pSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            host: default_host(),
            port: default_port(),
            mixed_mode: false,
            pex: true,
            socks_proxy: default_socks(),
        }
    }
}

impl I2pSettings {
    pub fn validate(&self) -> Result<()> {
        if self.host.trim().is_empty() {
            bail!("Værtsnavnet til I2P-routeren mangler.");
        }
        if self.host.chars().any(|c| c.is_whitespace()) {
            bail!("Værtsnavnet til I2P-routeren er ugyldigt.");
        }
        if self.port == 0 {
            bail!("Porten til I2P-routeren er ugyldig.");
        }
        let socks = self.socks_proxy.trim();
        let Some((host, port)) = socks.rsplit_once(':') else {
            bail!("SOCKS-proxyen skal angives som host:port.");
        };
        if host.is_empty() {
            bail!("SOCKS-proxyen mangler et værtsnavn.");
        }
        let port: u16 = port
            .parse()
            .context("SOCKS-proxyens port er ugyldig.")
            .map_err(|_| anyhow::anyhow!("SOCKS-proxyens port er ugyldig."))?;
        if port == 0 {
            bail!("SOCKS-proxyens port er ugyldig.");
        }
        Ok(())
    }

    pub fn socks_url(&self) -> Option<String> {
        if !self.enabled {
            return None;
        }
        let socks = self.socks_proxy.trim();
        if socks.is_empty() {
            None
        } else {
            Some(format!("socks5://{socks}"))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct I2pProbe {
    pub reachable: bool,
    /// Machine-readable outcome so the web panel can localise the text:
    /// "ok", "connect_failed", "timeout", "hello_failed", "session_failed".
    pub reason: &'static str,
    /// Raw detail from the router or the socket, shown after the localised text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub sam_version: Option<String>,
    pub destination: Option<String>,
    pub socks_reachable: bool,
}

pub struct I2pStore {
    path: PathBuf,
    state: Mutex<I2pSettings>,
}

impl I2pStore {
    pub fn load(path: PathBuf) -> Result<Self> {
        let state = read_file(&path)?;
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    pub fn settings(&self) -> I2pSettings {
        self.state.lock().expect("i2p mutex poisoned").clone()
    }

    pub fn update(&self, settings: I2pSettings) -> Result<I2pSettings> {
        settings.validate()?;
        let mut state = self.state.lock().expect("i2p mutex poisoned");
        *state = settings;
        save_file(&self.path, &state)?;
        Ok(state.clone())
    }
}

fn read_file(path: &Path) -> Result<I2pSettings> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(I2pSettings::default())
        }
        Err(err) => return Err(err).with_context(|| format!("could not read {}", path.display())),
    };
    let raw = raw.trim_start_matches('\u{feff}');
    Ok(serde_json::from_str(raw).unwrap_or_else(|err| {
        warn!("ignoring corrupt I2P settings: {err}");
        I2pSettings::default()
    }))
}

fn save_file(path: &Path, settings: &I2pSettings) -> Result<()> {
    let data = serde_json::to_vec_pretty(settings).context("could not serialize I2P settings")?;
    fs::write(path, data).with_context(|| format!("could not write {}", path.display()))?;
    Ok(())
}

/// Performs a SAM HELLO VERSION handshake, creates a transient STREAM session
/// and returns the resulting destination as a `.b32.i2p` address.
pub async fn probe(settings: &I2pSettings) -> I2pProbe {
    let socks_reachable = socks_reachable(&settings.socks_proxy).await;
    let address = format!("{}:{}", settings.host.trim(), settings.port);
    let connect = tokio::time::timeout(Duration::from_secs(6), TcpStream::connect(&address)).await;
    let stream = match connect {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) => {
            return probe_error("connect_failed", Some(err.to_string()), socks_reachable)
        }
        Err(_) => return probe_error("timeout", Some(address), socks_reachable),
    };

    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    if let Err(err) = writer.write_all(b"HELLO VERSION MIN=3.0 MAX=3.3\n").await {
        return probe_error("hello_failed", Some(err.to_string()), socks_reachable);
    }
    let hello = match next_line(&mut lines).await {
        Ok(line) => line,
        Err(err) => return probe_error("timeout", Some(err), socks_reachable),
    };
    if !hello.starts_with("HELLO REPLY") || !hello.contains("RESULT=OK") {
        return probe_error("hello_failed", Some(hello), socks_reachable);
    }
    let version = attribute(&hello, "VERSION");

    let session = "SESSION CREATE STYLE=STREAM ID=nordtorrent-probe DESTINATION=TRANSIENT\n";
    if let Err(err) = writer.write_all(session.as_bytes()).await {
        return I2pProbe {
            reachable: true,
            reason: "session_failed",
            detail: Some(err.to_string()),
            sam_version: version,
            destination: None,
            socks_reachable,
        };
    }
    let reply = match next_line(&mut lines).await {
        Ok(line) => line,
        Err(err) => {
            return I2pProbe {
                reachable: true,
                reason: "session_failed",
                detail: Some(err),
                sam_version: version,
                destination: None,
                socks_reachable,
            }
        }
    };
    let _ = writer.write_all(b"SESSION REMOVE ID=nordtorrent-probe\n").await;

    if !reply.starts_with("SESSION STATUS") || !reply.contains("RESULT=OK") {
        return I2pProbe {
            reachable: true,
            reason: "session_failed",
            detail: Some(reply),
            sam_version: version,
            destination: None,
            socks_reachable,
        };
    }
    // A transient destination is only returned once the session is destroyed, so
    // report it when the router includes it in the reply.
    let destination = attribute(&reply, "DESTINATION").and_then(|value| b32_address(&value));
    I2pProbe {
        reachable: true,
        reason: "ok",
        detail: None,
        sam_version: version,
        destination,
        socks_reachable,
    }
}

fn probe_error(reason: &'static str, detail: Option<String>, socks_reachable: bool) -> I2pProbe {
    I2pProbe {
        reachable: false,
        reason,
        detail,
        sam_version: None,
        destination: None,
        socks_reachable,
    }
}

async fn next_line(
    lines: &mut tokio::io::Lines<BufReader<tokio::net::tcp::OwnedReadHalf>>,
) -> Result<String, String> {
    match tokio::time::timeout(Duration::from_secs(10), lines.next_line()).await {
        Ok(Ok(Some(line))) => Ok(line),
        Ok(Ok(None)) => Err("SAM-broen lukkede forbindelsen.".to_owned()),
        Ok(Err(err)) => Err(format!("Kunne ikke læse fra SAM-broen: {err}")),
        Err(_) => Err("SAM-broen svarede ikke i tide.".to_owned()),
    }
}

fn attribute(line: &str, key: &str) -> Option<String> {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix(&format!("{key}=")))
        .map(str::to_owned)
}

/// The b32 address of an I2P destination is base32(SHA-256(destination)).
fn b32_address(encoded: &str) -> Option<String> {
    let bytes = BASE64.decode(encoded.trim().as_bytes()).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let digest = Sha256::digest(&bytes);
    let encoded = data_encoding::BASE32_NOPAD.encode(&digest);
    Some(format!("{}.b32.i2p", encoded.to_lowercase()))
}

async fn socks_reachable(socks_proxy: &str) -> bool {
    let address = socks_proxy.trim();
    if address.is_empty() {
        return false;
    }
    matches!(
        tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(address)).await,
        Ok(Ok(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_settings() {
        let mut settings = I2pSettings::default();
        assert!(settings.validate().is_ok());
        assert_eq!(settings.socks_url(), None, "disabled means no proxy");

        settings.enabled = true;
        assert_eq!(settings.socks_url().as_deref(), Some("socks5://127.0.0.1:4447"));

        settings.host = " ".to_owned();
        assert!(settings.validate().is_err());
        settings.host = "127.0.0.1".to_owned();

        settings.socks_proxy = "127.0.0.1".to_owned();
        assert!(settings.validate().is_err());
        settings.socks_proxy = "127.0.0.1:0".to_owned();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn round_trips_settings() {
        let path = std::env::temp_dir().join(format!(
            "nordtorrent-i2p-test-{}-{}.json",
            std::process::id(),
            rand::random::<u64>()
        ));
        let store = I2pStore::load(path.clone()).unwrap();
        assert!(!store.settings().enabled);
        let mut settings = store.settings();
        settings.enabled = true;
        settings.mixed_mode = true;
        store.update(settings.clone()).unwrap();
        let reloaded = I2pStore::load(path.clone()).unwrap();
        assert!(reloaded.settings().enabled);
        assert!(reloaded.settings().mixed_mode);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn derives_b32_addresses() {
        // 32 zero bytes encoded as base64 is a valid (if useless) destination.
        let encoded = BASE64.encode(&[0u8; 32]);
        let address = b32_address(&encoded).unwrap();
        assert!(address.ends_with(".b32.i2p"));
        assert_eq!(address.len(), 52 + ".b32.i2p".len());
        assert!(b32_address("not base64!").is_none());
    }

    #[test]
    fn parses_sam_attributes() {
        let line = "HELLO REPLY RESULT=OK VERSION=3.3";
        assert_eq!(attribute(line, "VERSION").as_deref(), Some("3.3"));
        assert_eq!(attribute(line, "MISSING"), None);
    }
}
