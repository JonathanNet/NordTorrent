use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

/// A named download destination visible in the web UI. The path itself stays inside
/// the configured download root, so a client can only select a discovered directory.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadFolder {
    pub name: String,
    /// Relative path used by the API. The root download folder is ".".
    pub path: String,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadBrowser {
    /// Relative path from the download root, using forward slashes ("" = root).
    pub path: String,
    /// Container-visible path, e.g. "/downloads/series4".
    pub display_path: String,
    /// None at the root, otherwise "" for the root or the parent relative path.
    pub parent: Option<String>,
    pub entries: Vec<DownloadBrowserEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadBrowserEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct DownloadFolders {
    base: PathBuf,
    preferred_default: Option<String>,
}

impl DownloadFolders {
    pub fn new(base: PathBuf, preferred_default: Option<String>) -> Self {
        Self {
            base,
            preferred_default,
        }
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    pub fn list(&self) -> Vec<DownloadFolder> {
        let names = self.discover();
        let default = self.default_name(&names);
        let root_name = folder_name(&self.base);
        names
            .into_iter()
            .map(|name| DownloadFolder {
                default: name == default,
                path: if name == root_name {
                    ".".to_owned()
                } else {
                    name.clone()
                },
                name,
            })
            .collect()
    }

    /// Resolve a selected relative path (or the configured default) to a real directory.
    pub fn resolve(&self, requested: Option<&str>) -> Result<PathBuf, String> {
        let relative = match requested.map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) => normalize_relative(value)?,
            None => self.default_relative()?,
        };
        self.resolve_relative(&relative)
    }

    /// List one directory level below the download root. The path is a relative,
    /// slash-separated path previously returned by this API.
    pub fn browse(&self, requested: Option<&str>, query: Option<&str>) -> Result<DownloadBrowser, String> {
        let relative = normalize_relative(requested.unwrap_or(""))?;
        let directory = self.resolve_relative(&relative)?;
        let needle = query.unwrap_or("").trim().to_lowercase();

        let mut entries = Vec::new();
        let read_dir = fs::read_dir(&directory)
            .map_err(|err| format!("Kunne ikke læse downloadmappen: {err}"))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            if !needle.is_empty() && !name.to_lowercase().contains(&needle) {
                continue;
            }
            entries.push(DownloadBrowserEntry {
                name: name.to_owned(),
                path: join_relative(&relative, name),
            });
        }
        entries.sort_by(|left, right| {
            left.name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then_with(|| left.name.cmp(&right.name))
        });

        Ok(DownloadBrowser {
            display_path: self.display_path(&relative),
            parent: if relative.is_empty() {
                None
            } else {
                Some(parent_relative(&relative))
            },
            path: relative,
            entries,
        })
    }

    pub fn default_relative(&self) -> Result<String, String> {
        let folders = self.list();
        let folder = folders
            .iter()
            .find(|folder| folder.default)
            .or_else(|| folders.first())
            .ok_or_else(|| "Der er ingen tilgængelige downloadmapper.".to_owned())?;
        if folder.path == "." {
            Ok(String::new())
        } else {
            Ok(folder.path.clone())
        }
    }

    pub fn display_path(&self, relative: &str) -> String {
        let base = self.base.to_string_lossy();
        if relative.is_empty() {
            base.trim_end_matches(['/', '\\']).to_owned()
        } else {
            format!(
                "{}/{}",
                base.trim_end_matches(['/', '\\']),
                relative
            )
        }
    }

    fn resolve_relative(&self, relative: &str) -> Result<PathBuf, String> {
        let base = fs::canonicalize(&self.base).map_err(|err| {
            format!(
                "Downloadroden {} kan ikke læses: {err}",
                self.base.display()
            )
        })?;
        let candidate = if relative.is_empty() {
            self.base.clone()
        } else {
            self.base.join(Path::new(relative))
        };
        let canonical = fs::canonicalize(&candidate)
            .map_err(|_| format!("Downloadmappen \"{relative}\" findes ikke."))?;
        if !canonical.starts_with(&base) {
            return Err("Downloadmappen ligger uden for downloadroden.".to_owned());
        }
        if !canonical.is_dir() {
            return Err("Den valgte sti er ikke en mappe.".to_owned());
        }
        Ok(candidate)
    }

    fn default_name(&self, names: &BTreeSet<String>) -> String {
        if let Some(preferred) = &self.preferred_default {
            if names.contains(preferred) {
                return preferred.clone();
            }
        }
        // A container stack usually mounts a dedicated "complete" folder; prefer it
        // over a torrent-created sibling when no explicit default was configured.
        if names.contains("complete") {
            return "complete".to_owned();
        }
        names
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| folder_name(&self.base))
    }

    fn discover(&self) -> BTreeSet<String> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(mountinfo) = std::fs::read_to_string("/proc/self/mountinfo") {
                let discovered = parse_mountinfo(&mountinfo, &self.base);
                if !discovered.is_empty() {
                    return discovered;
                }
            }
        }

        // Development fallback: on non-Linux, or when /proc is unavailable, expose the
        // immediate child directories. Docker always takes the mountinfo path above.
        let mut names = BTreeSet::new();
        if let Ok(entries) = std::fs::read_dir(&self.base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    if !name.starts_with('.') {
                        names.insert(name.to_owned());
                    }
                }
            }
        }
        if names.is_empty() {
            names.insert(folder_name(&self.base));
        }
        names
    }
}

/// Convert an absolute output folder into a path relative to the download root.
pub fn relative_output_folder(base: &Path, output: &str) -> String {
    let output_path = Path::new(output.trim_end_matches(|c| c == '/' || c == '\\'));
    let canonical_base = fs::canonicalize(base).unwrap_or_else(|_| base.to_path_buf());
    let canonical_output =
        fs::canonicalize(output_path).unwrap_or_else(|_| output_path.to_path_buf());
    if let Ok(relative) = canonical_output.strip_prefix(&canonical_base) {
        let relative = relative.to_string_lossy().replace('\\', "/");
        if !relative.is_empty() {
            return relative;
        }
    }
    output_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloads")
        .to_owned()
}

fn normalize_relative(raw: &str) -> Result<String, String> {
    let raw = raw.trim().replace('\\', "/");
    if raw.starts_with('/') {
        return Err("Stien skal være relativ til downloadroden.".to_owned());
    }
    let mut parts = Vec::new();
    for part in raw.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err("Stien må ikke gå op over downloadroden.".to_owned());
        }
        if part.contains('\0') {
            return Err("Stien indeholder ugyldige tegn.".to_owned());
        }
        parts.push(part);
    }
    Ok(parts.join("/"))
}

fn parent_relative(relative: &str) -> String {
    relative
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_owned())
        .unwrap_or_default()
}

fn join_relative(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}

fn folder_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloads")
        .to_owned()
}

#[cfg(any(target_os = "linux", test))]
fn parse_mountinfo(content: &str, base: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in content.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 {
            continue;
        }
        let mountpoint = PathBuf::from(unescape_mount_path(fields[4]));
        if mountpoint == base {
            names.insert(folder_name(base));
            continue;
        }
        let Ok(relative) = mountpoint.strip_prefix(base) else {
            continue;
        };
        let components: Vec<std::path::Component<'_>> = relative.components().collect();
        if components.len() != 1 {
            continue;
        }
        let std::path::Component::Normal(name) = components[0] else {
            continue;
        };
        if let Some(name) = name.to_str() {
            if !name.starts_with('.') {
                names.insert(name.to_owned());
            }
        }
    }
    names
}

#[cfg(any(target_os = "linux", test))]
fn unescape_mount_path(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 3 < bytes.len() {
            let octal = &bytes[index + 1..index + 4];
            if octal.iter().all(|byte| (b'0'..=b'7').contains(byte)) {
                let value = (octal[0] - b'0') * 64 + (octal[1] - b'0') * 8 + (octal[2] - b'0');
                output.push(value);
                index += 4;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_base() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!("nordtorrent-downloads-{stamp}"));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(base.join("series2").join("Carrie (2026)")).unwrap();
        fs::create_dir_all(base.join("gamle_serier")).unwrap();
        base
    }

    #[test]
    fn parses_direct_mountpoints_and_unescapes_names() {
        let content = "\
36 35 98:0 / /downloads/movies1 rw - ext4 /dev/sda1 rw
37 35 98:0 / /downloads/gamle_serier rw - ext4 /dev/sda2 rw
38 35 98:0 / /downloads/musik\\040videoer rw - ext4 /dev/sda3 rw
39 35 98:0 / /downloads/nested/ignored rw - ext4 /dev/sda4 rw
40 35 98:0 / /other rw - ext4 /dev/sda5 rw
";
        let names = parse_mountinfo(content, Path::new("/downloads"));
        assert_eq!(
            names.into_iter().collect::<Vec<_>>(),
            vec!["gamle_serier", "movies1", "musik videoer"]
        );
    }

    #[test]
    fn browses_nested_folders_and_filters() {
        let base = temp_base();
        let folders = DownloadFolders::new(base.clone(), None);
        let root = folders.browse(None, None).unwrap();
        assert_eq!(root.path, "");
        assert!(root.entries.iter().any(|entry| entry.name == "series2"));

        let series = folders.browse(Some("series2"), None).unwrap();
        assert_eq!(series.display_path, format!("{}/series2", base.to_string_lossy()));
        assert_eq!(series.parent.as_deref(), Some(""));
        assert_eq!(series.entries.len(), 1);

        let filtered = folders
            .browse(Some("series2"), Some("carrie"))
            .unwrap();
        assert_eq!(filtered.entries[0].name, "Carrie (2026)");
        assert_eq!(filtered.entries[0].path, "series2/Carrie (2026)");
    }

    #[test]
    fn reports_nested_relative_output_folder() {
        let base = temp_base();
        let output = base.join("series2").join("Carrie (2026)");
        assert_eq!(
            relative_output_folder(&base, &output.to_string_lossy()),
            "series2/Carrie (2026)"
        );
    }

    #[test]
    fn resolves_only_directories_inside_root() {
        let base = temp_base();
        let folders = DownloadFolders::new(base.clone(), Some("series2".to_owned()));
        assert_eq!(
            folders.resolve(Some("series2/Carrie (2026)")).unwrap(),
            base.join("series2").join("Carrie (2026)")
        );
        assert!(folders.resolve(Some("../etc")).is_err());
        assert!(folders.resolve(Some("/etc")).is_err());
        assert!(folders.resolve(Some("series3")).is_err());
    }
}

