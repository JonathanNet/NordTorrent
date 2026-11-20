use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use librqbit::{AddTorrent, AddTorrentOptions, Api};
use quick_xml::{events::Event, Reader};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::downloads::DownloadFolders;

const MAX_FEED_BYTES: usize = 8 * 1024 * 1024;
const MAX_SEEN_PER_FEED: usize = 1000;
const MAX_DOWNLOADS_PER_CHECK: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssFeed {
    pub id: String,
    pub name: String,
    pub url: String,
    pub folder: Option<String>,
    pub enabled: bool,
    pub auto_download: bool,
    pub paused: bool,
    pub include: String,
    pub exclude: String,
    pub created_at: i64,
    pub last_checked: Option<i64>,
    pub last_error: Option<String>,
    pub last_items: usize,
    #[serde(default)]
    pub seen: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RssFeedView {
    pub id: String,
    pub name: String,
    pub url: String,
    pub folder: Option<String>,
    pub enabled: bool,
    pub auto_download: bool,
    pub paused: bool,
    pub include: String,
    pub exclude: String,
    pub created_at: i64,
    pub last_checked: Option<i64>,
    pub last_error: Option<String>,
    pub last_items: usize,
    pub seen_count: usize,
}

impl RssFeed {
    fn view(&self) -> RssFeedView {
        RssFeedView {
            id: self.id.clone(),
            name: self.name.clone(),
            url: self.url.clone(),
            folder: self.folder.clone(),
            enabled: self.enabled,
            auto_download: self.auto_download,
            paused: self.paused,
            include: self.include.clone(),
            exclude: self.exclude.clone(),
            created_at: self.created_at,
            last_checked: self.last_checked,
            last_error: self.last_error.clone(),
            last_items: self.last_items,
            seen_count: self.seen.len(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RssFeedInput {
    #[serde(default)]
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub auto_download: bool,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub include: String,
    #[serde(default)]
    pub exclude: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
struct RssFile {
    version: u8,
    #[serde(default)]
    feeds: Vec<RssFeed>,
}

impl Default for RssFile {
    fn default() -> Self {
        Self {
            version: 1,
            feeds: Vec::new(),
        }
    }
}

pub struct RssStore {
    path: PathBuf,
    client: reqwest::Client,
    state: Mutex<RssFile>,
}

impl RssStore {
    pub fn load(path: PathBuf) -> Result<Self> {
        let state = read_file(&path)?;
        let client = reqwest::Client::builder()
            .user_agent("NordTorrent/0.1.0 (RSS)")
            .timeout(Duration::from_secs(30))
            .build()
            .context("could not create RSS HTTP client")?;
        Ok(Self {
            path,
            client,
            state: Mutex::new(state),
        })
    }

    pub fn feeds(&self) -> Vec<RssFeedView> {
        self.state
            .lock()
            .expect("RSS mutex poisoned")
            .feeds
            .iter()
            .map(RssFeed::view)
            .collect()
    }

    pub fn add(&self, input: RssFeedInput, downloads: &DownloadFolders) -> Result<RssFeedView> {
        let input = validate_input(input, downloads)?;
        let mut state = self.state.lock().expect("RSS mutex poisoned");
        let mut id = format!("{:016x}", random::<u64>());
        while state.feeds.iter().any(|feed| feed.id == id) {
            id = format!("{:016x}", random::<u64>());
        }
        let feed = RssFeed {
            id,
            name: input.name,
            url: input.url,
            folder: input.folder,
            enabled: input.enabled,
            auto_download: input.auto_download,
            paused: input.paused,
            include: input.include,
            exclude: input.exclude,
            created_at: Utc::now().timestamp(),
            last_checked: None,
            last_error: None,
            last_items: 0,
            seen: Vec::new(),
        };
        let view = feed.view();
        state.feeds.push(feed);
        save_file(&self.path, &state)?;
        Ok(view)
    }

    pub fn update(
        &self,
        id: &str,
        input: RssFeedInput,
        downloads: &DownloadFolders,
    ) -> Result<RssFeedView> {
        let input = validate_input(input, downloads)?;
        let mut state = self.state.lock().expect("RSS mutex poisoned");
        let feed = state
            .feeds
            .iter_mut()
            .find(|feed| feed.id == id)
            .ok_or_else(|| anyhow::anyhow!("RSS-feedet blev ikke fundet."))?;
        feed.name = input.name;
        feed.url = input.url;
        feed.folder = input.folder;
        feed.enabled = input.enabled;
        feed.auto_download = input.auto_download;
        feed.paused = input.paused;
        feed.include = input.include;
        feed.exclude = input.exclude;
        feed.last_error = None;
        let view = feed.view();
        save_file(&self.path, &state)?;
        Ok(view)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let mut state = self.state.lock().expect("RSS mutex poisoned");
        let before = state.feeds.len();
        state.feeds.retain(|feed| feed.id != id);
        if state.feeds.len() == before {
            bail!("RSS-feedet blev ikke fundet.");
        }
        save_file(&self.path, &state)?;
        Ok(())
    }

    pub async fn check(
        &self,
        id: &str,
        api: &Api,
        downloads: &DownloadFolders,
    ) -> Result<RssFeedView> {
        let feed = {
            let state = self.state.lock().expect("RSS mutex poisoned");
            state
                .feeds
                .iter()
                .find(|feed| feed.id == id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("RSS-feedet blev ikke fundet."))?
        };

        let output_folder = downloads.resolve(feed.folder.as_deref()).map_err(anyhow::Error::msg)?;
        let fetch_result = self.fetch_items(&feed.url).await;
        let (items, fetch_error) = match fetch_result {
            Ok(items) => (items, None),
            Err(err) => {
                self.update_check_result(id, 0, Some(err.to_string()), None);
                return Err(err);
            }
        };

        let mut seen = feed.seen.clone();
        let mut last_error = fetch_error;
        let mut added = 0usize;
        for item in &items {
            let key = item.key();
            if key.is_empty() || seen.iter().any(|seen_key| seen_key == &key) {
                continue;
            }
            let should_download = feed.auto_download
                && matches_filters(&feed, &item.title)
                && added < MAX_DOWNLOADS_PER_CHECK;
            let Some(url) = item.torrent_url() else {
                seen.push(key);
                continue;
            };
            if !should_download {
                seen.push(key);
                continue;
            }
            match api
                .api_add_torrent(
                    AddTorrent::from_url(url.to_owned()),
                    Some(AddTorrentOptions {
                        paused: feed.paused,
                        output_folder: Some(output_folder.to_string_lossy().into_owned()),
                        ..Default::default()
                    }),
                )
                .await
            {
                Ok(_) => {
                    added += 1;
                    seen.push(key);
                }
                Err(err) => {
                    last_error = Some(format!("Kunne ikke tilføje {}: {err}", item.title));
                }
            }
        }
        if seen.len() > MAX_SEEN_PER_FEED {
            let excess = seen.len() - MAX_SEEN_PER_FEED;
            seen.drain(0..excess);
        }
        self.update_check_result(id, items.len(), last_error, Some(seen));
        Ok(self
            .state
            .lock()
            .expect("RSS mutex poisoned")
            .feeds
            .iter()
            .find(|feed| feed.id == id)
            .map(RssFeed::view)
            .unwrap_or_else(|| feed.view()))
    }

    pub fn spawn_poller(
        self: Arc<Self>,
        api: Api,
        downloads: Arc<DownloadFolders>,
    ) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(15 * 60));
            loop {
                interval.tick().await;
                let feeds = self
                    .state
                    .lock()
                    .expect("RSS mutex poisoned")
                    .feeds
                    .iter()
                    .filter(|feed| feed.enabled)
                    .map(|feed| feed.id.clone())
                    .collect::<Vec<_>>();
                for id in feeds {
                    if let Err(err) = self.check(&id, &api, &downloads).await {
                        warn!(feed_id = id, "RSS check failed: {err:#}");
                    }
                }
            }
        });
    }

    async fn fetch_items(&self, url: &str) -> Result<Vec<ParsedItem>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("could not fetch RSS feed {url}"))?;
        if !response.status().is_success() {
            bail!("RSS-serveren svarede {}", response.status());
        }
        let bytes = response
            .bytes()
            .await
            .context("could not read RSS response")?;
        if bytes.len() > MAX_FEED_BYTES {
            bail!("RSS-feedet er større end 8 MB.");
        }
        let text = String::from_utf8_lossy(&bytes);
        parse_items(&text)
    }

    fn update_check_result(
        &self,
        id: &str,
        item_count: usize,
        error: Option<String>,
        seen: Option<Vec<String>>,
    ) {
        let mut state = self.state.lock().expect("RSS mutex poisoned");
        let Some(feed) = state.feeds.iter_mut().find(|feed| feed.id == id) else {
            return;
        };
        feed.last_checked = Some(Utc::now().timestamp());
        feed.last_items = item_count;
        feed.last_error = error;
        if let Some(seen) = seen {
            feed.seen = seen;
        }
        if let Err(err) = save_file(&self.path, &state) {
            warn!("could not persist RSS feed: {err:#}");
        }
    }
}

fn validate_input(input: RssFeedInput, downloads: &DownloadFolders) -> Result<RssFeedInput> {
    let url = input.url.trim().to_owned();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        bail!("RSS-adressen skal starte med http:// eller https://.");
    }
    if url.len() > 4096 {
        bail!("RSS-adressen er for lang.");
    }
    let folder = input
        .folder
        .as_deref()
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(str::to_owned);
    downloads
        .resolve(folder.as_deref())
        .map_err(anyhow::Error::msg)?;
    let name = if input.name.trim().is_empty() {
        url.clone()
    } else {
        input.name.trim().to_owned()
    };
    Ok(RssFeedInput {
        name,
        url,
        folder,
        enabled: input.enabled,
        auto_download: input.auto_download,
        paused: input.paused,
        include: input.include.trim().to_owned(),
        exclude: input.exclude.trim().to_owned(),
    })
}

fn matches_filters(feed: &RssFeed, title: &str) -> bool {
    let title = title.to_lowercase();
    let includes = split_terms(&feed.include);
    if !includes.is_empty() && !includes.iter().any(|term| title.contains(term)) {
        return false;
    }
    let excludes = split_terms(&feed.exclude);
    !excludes.iter().any(|term| title.contains(term))
}

fn split_terms(raw: &str) -> Vec<String> {
    raw.split([',', ';'])
        .map(|term| term.trim().to_lowercase())
        .filter(|term| !term.is_empty())
        .collect()
}

#[derive(Debug, Default)]
struct ParsedItem {
    title: String,
    link: String,
    enclosure: String,
    enclosure_type: String,
    guid: String,
}

impl ParsedItem {
    fn key(&self) -> String {
        if !self.guid.trim().is_empty() {
            return self.guid.trim().to_owned();
        }
        if !self.link.trim().is_empty() {
            return self.link.trim().to_owned();
        }
        self.enclosure.trim().to_owned()
    }

    fn torrent_url(&self) -> Option<&str> {
        for value in [&self.enclosure, &self.link] {
            let value = value.trim();
            if value.starts_with("magnet:?") {
                return Some(value);
            }
            if value.to_lowercase().contains(".torrent") {
                return Some(value);
            }
            if !self.enclosure_type.is_empty()
                && value == self.enclosure.trim()
                && self.enclosure_type.to_lowercase().contains("bittorrent")
            {
                return Some(value);
            }
        }
        None
    }
}

#[derive(Clone, Copy)]
enum Capture {
    Title,
    Link,
    Guid,
}

fn parse_items(xml: &str) -> Result<Vec<ParsedItem>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut items = Vec::new();
    let mut current: Option<ParsedItem> = None;
    let mut capture: Option<Capture> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) => {
                let name = event.local_name();
                let name = String::from_utf8_lossy(name.as_ref());
                if name == "item" || name == "entry" {
                    current = Some(ParsedItem::default());
                    capture = None;
                } else if current.is_some() {
                    match name.as_ref() {
                        "title" => capture = Some(Capture::Title),
                        "guid" | "id" => capture = Some(Capture::Guid),
                        "link" => {
                            let mut href = String::new();
                            let mut rel = String::new();
                            for attr in event.attributes().flatten() {
                                let key = attr.key.local_name();
                                let key = String::from_utf8_lossy(key.as_ref());
                                if key == "href" {
                                    href = attr.unescape_value().unwrap_or_default().into_owned();
                                } else if key == "rel" {
                                    rel = attr.unescape_value().unwrap_or_default().into_owned();
                                }
                            }
                            if let Some(item) = current.as_mut() {
                                if !href.is_empty() {
                                    if rel == "enclosure" {
                                        item.enclosure = href;
                                    } else {
                                        item.link = href;
                                    }
                                } else {
                                    capture = Some(Capture::Link);
                                }
                            }
                        }
                        "enclosure" => {
                            if let Some(item) = current.as_mut() {
                                for attr in event.attributes().flatten() {
                                    let key = attr.key.local_name();
                                    let key = String::from_utf8_lossy(key.as_ref());
                                    let value = attr.unescape_value().unwrap_or_default().into_owned();
                                    if key == "url" {
                                        item.enclosure = value;
                                    } else if key == "type" {
                                        item.enclosure_type = value;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(event)) => {
                let name = event.local_name();
                let name = String::from_utf8_lossy(name.as_ref());
                if let Some(item) = current.as_mut() {
                    if name.as_ref() == "enclosure" {
                        for attr in event.attributes().flatten() {
                            let key = attr.key.local_name();
                            let key = String::from_utf8_lossy(key.as_ref());
                            let value = attr.unescape_value().unwrap_or_default().into_owned();
                            if key == "url" {
                                item.enclosure = value;
                            } else if key == "type" {
                                item.enclosure_type = value;
                            }
                        }
                    } else if name.as_ref() == "link" {
                        let mut href = String::new();
                        let mut rel = String::new();
                        for attr in event.attributes().flatten() {
                            let key = attr.key.local_name();
                            let key = String::from_utf8_lossy(key.as_ref());
                            if key == "href" {
                                href = attr.unescape_value().unwrap_or_default().into_owned();
                            } else if key == "rel" {
                                rel = attr.unescape_value().unwrap_or_default().into_owned();
                            }
                        }
                        if !href.is_empty() {
                            if rel == "enclosure" {
                                item.enclosure = href;
                            } else {
                                item.link = href;
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(capture) = capture {
                    let value = text.unescape().unwrap_or_default();
                    append_capture(&mut current, capture, &value);
                }
            }
            Ok(Event::CData(text)) => {
                if let Some(capture) = capture {
                    let value = String::from_utf8_lossy(text.as_ref());
                    append_capture(&mut current, capture, &value);
                }
            }
            Ok(Event::End(event)) => {
                let name = event.local_name();
                let name = String::from_utf8_lossy(name.as_ref());
                if name == "item" || name == "entry" {
                    if let Some(item) = current.take() {
                        items.push(item);
                    }
                    capture = None;
                } else if matches!(name.as_ref(), "title" | "link" | "guid" | "id") {
                    capture = None;
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(anyhow::anyhow!("ugyldig RSS/XML: {err}")),
            _ => {}
        }
        buf.clear();
    }
    Ok(items)
}

fn append_capture(current: &mut Option<ParsedItem>, capture: Capture, value: &str) {
    let Some(item) = current.as_mut() else {
        return;
    };
    match capture {
        Capture::Title => item.title.push_str(value),
        Capture::Link => item.link.push_str(value),
        Capture::Guid => item.guid.push_str(value),
    }
}

fn read_file(path: &Path) -> Result<RssFile> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(RssFile::default()),
        Err(err) => return Err(err).with_context(|| format!("could not read {}", path.display())),
    };
    Ok(serde_json::from_str(raw.trim_start_matches('\u{feff}')).unwrap_or_else(|err| {
        warn!("ignoring corrupt RSS settings: {err}");
        RssFile::default()
    }))
}

fn save_file(path: &Path, state: &RssFile) -> Result<()> {
    let data = serde_json::to_vec_pretty(state).context("could not serialize RSS settings")?;
    fs::write(path, data).with_context(|| format!("could not write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rss_enclosure() {
        let items = parse_items(
            r#"<rss><channel><item><title>Show S01E01</title><link>https://example/1</link><guid>one</guid><enclosure url="https://example/1.torrent" type="application/x-bittorrent" /></item></channel></rss>"#,
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key(), "one");
        assert_eq!(items[0].torrent_url(), Some("https://example/1.torrent"));
    }

    #[test]
    fn parses_atom_link() {
        let items = parse_items(
            r#"<feed><entry><title>Atom</title><id>atom-1</id><link href="https://example/a.torrent" /></entry></feed>"#,
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key(), "atom-1");
        assert_eq!(items[0].torrent_url(), Some("https://example/a.torrent"));
    }

    #[test]
    fn filters_include_and_exclude() {
        let feed = RssFeed {
            id: "x".to_owned(),
            name: "x".to_owned(),
            url: "https://example".to_owned(),
            folder: None,
            enabled: true,
            auto_download: true,
            paused: false,
            include: "show, series".to_owned(),
            exclude: "cam".to_owned(),
            created_at: 0,
            last_checked: None,
            last_error: None,
            last_items: 0,
            seen: Vec::new(),
        };
        assert!(matches_filters(&feed, "My Show S01"));
        assert!(!matches_filters(&feed, "Other S01"));
        assert!(!matches_filters(&feed, "My Show CAM"));
    }
}
