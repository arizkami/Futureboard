//! File browser state and filesystem walker for the left sidebar.
//!
//! Mirrors the Electron browser's behavior: a navigable directory listing
//! with audio-aware filtering. No globals — the layout owns one
//! `FileBrowserState` and passes it to the sidebar each render.
//!
//! Realtime / audio rules:
//! * filesystem scans are best-effort and run on the UI thread when the user
//!   navigates. They must not be triggered from audio paths.
//! * we never block the audio engine on a `read_dir` call — this module is
//!   pure UI state.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEntryKind {
    Folder,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileBrowserEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: FileEntryKind,
    /// Lowercased extension (without the dot), or empty for folders.
    pub extension: String,
}

impl FileBrowserEntry {
    pub fn is_audio(&self) -> bool {
        matches!(
            self.extension.as_str(),
            "wav" | "mp3" | "flac" | "ogg" | "aiff" | "aif"
        )
    }

    pub fn is_midi(&self) -> bool {
        matches!(self.extension.as_str(), "mid" | "midi")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserNodeKind {
    Section,
    Folder,
    File,
}

#[derive(Debug, Clone)]
pub struct BrowserRootSection {
    pub id: String,
    pub label: String,
    pub root_path: Option<PathBuf>,
    pub kind: BrowserNodeKind,
}

#[derive(Debug, Clone)]
pub struct BrowserVisibleNode {
    pub id: String,
    pub label: String,
    pub path: Option<PathBuf>,
    pub kind: BrowserNodeKind,
    pub depth: usize,
    pub extension: String,
    pub expandable: bool,
    pub expanded: bool,
    pub selected: bool,
    pub error: Option<String>,
}

impl BrowserVisibleNode {
    pub fn is_audio(&self) -> bool {
        matches!(
            self.extension.as_str(),
            "wav" | "mp3" | "flac" | "ogg" | "aiff" | "aif"
        )
    }

    pub fn is_midi(&self) -> bool {
        matches!(self.extension.as_str(), "mid" | "midi")
    }
}

#[derive(Debug, Clone)]
pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileBrowserEntry>,
    pub selected: Option<PathBuf>,
    pub expanded_paths: HashSet<PathBuf>,
    pub expanded_sections: HashSet<String>,
    pub root_sections: Vec<BrowserRootSection>,
    pub error: Option<String>,
}

impl Default for FileBrowserState {
    fn default() -> Self {
        let dir = default_directory();
        let (entries, error) = read_directory(&dir);
        let mut expanded_sections = HashSet::new();
        expanded_sections.insert("audio-files".to_string());
        Self {
            current_dir: dir,
            entries,
            selected: None,
            expanded_paths: HashSet::new(),
            expanded_sections,
            root_sections: default_root_sections(),
            error,
        }
    }
}

impl FileBrowserState {
    /// Navigate to `path` (must be an existing directory). Refreshes entries.
    pub fn navigate_to(&mut self, path: impl Into<PathBuf>) {
        let target = path.into();
        if !target.is_dir() {
            return;
        }
        let (entries, error) = read_directory(&target);
        self.current_dir = target;
        self.entries = entries;
        self.error = error;
        self.selected = None;
        self.expanded_paths.insert(self.current_dir.clone());
    }

    /// Move up one directory if a parent exists.
    pub fn navigate_up(&mut self) {
        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            self.navigate_to(parent);
        }
    }

    pub fn refresh(&mut self) {
        let (entries, error) = read_directory(&self.current_dir);
        self.entries = entries;
        self.error = error;
    }

    pub fn select(&mut self, path: PathBuf) {
        self.selected = Some(path);
    }

    pub fn toggle_node(&mut self, node_id: &str, path: Option<&Path>) {
        if let Some(path) = path {
            let path = path.to_path_buf();
            if self.expanded_paths.contains(&path) {
                self.expanded_paths.remove(&path);
            } else {
                self.expanded_paths.insert(path);
            }
            return;
        }

        if self.expanded_sections.contains(node_id) {
            self.expanded_sections.remove(node_id);
        } else {
            self.expanded_sections.insert(node_id.to_string());
        }
    }

    pub fn is_expanded_node(&self, node_id: &str, path: Option<&Path>) -> bool {
        if let Some(path) = path {
            return self.expanded_paths.contains(path);
        }
        self.expanded_sections.contains(node_id)
    }

    pub fn visible_nodes(&self) -> Vec<BrowserVisibleNode> {
        let mut nodes = Vec::new();
        for section in &self.root_sections {
            let expanded = self.expanded_sections.contains(&section.id);
            let selected = section
                .root_path
                .as_ref()
                .is_some_and(|p| self.selected.as_deref() == Some(p.as_path()));
            nodes.push(BrowserVisibleNode {
                id: section.id.clone(),
                label: section.label.clone(),
                path: section.root_path.clone(),
                kind: section.kind,
                depth: 0,
                extension: String::new(),
                expandable: section.root_path.is_some(),
                expanded,
                selected,
                error: section.root_path.as_ref().and_then(|p| {
                    if p.exists() {
                        None
                    } else {
                        Some("Missing folder".to_string())
                    }
                }),
            });

            if expanded {
                if let Some(root_path) = section.root_path.as_ref() {
                    self.append_directory_nodes(root_path, 1, &mut nodes);
                }
            }
        }
        nodes
    }

    fn append_directory_nodes(
        &self,
        dir: &Path,
        depth: usize,
        nodes: &mut Vec<BrowserVisibleNode>,
    ) {
        let (entries, error) = read_directory(dir);
        if let Some(error) = error {
            nodes.push(BrowserVisibleNode {
                id: format!("error:{}", dir.display()),
                label: error,
                path: None,
                kind: BrowserNodeKind::File,
                depth,
                extension: String::new(),
                expandable: false,
                expanded: false,
                selected: false,
                error: Some("Cannot read folder".to_string()),
            });
            return;
        }

        for entry in entries {
            let entry_path = entry.path.clone();
            let entry_name = entry.name;
            let entry_extension = entry.extension;
            let is_folder = entry.kind == FileEntryKind::Folder;
            let expanded = is_folder && self.expanded_paths.contains(&entry_path);
            let selected = self.selected.as_deref() == Some(entry_path.as_path());
            nodes.push(BrowserVisibleNode {
                id: entry_path.to_string_lossy().to_string(),
                label: entry_name,
                path: Some(entry_path.clone()),
                kind: if is_folder {
                    BrowserNodeKind::Folder
                } else {
                    BrowserNodeKind::File
                },
                depth,
                extension: entry_extension,
                expandable: is_folder,
                expanded,
                selected,
                error: None,
            });

            if expanded {
                self.append_directory_nodes(&entry_path, depth + 1, nodes);
            }
        }
    }
}

/// Resolve a sensible starting directory: user Music dir, then home, then cwd.
pub fn default_directory() -> PathBuf {
    if let Some(p) = dirs::audio_dir() {
        if p.is_dir() {
            return p;
        }
    }
    if let Some(p) = dirs::home_dir() {
        if p.is_dir() {
            return p;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn default_root_sections() -> Vec<BrowserRootSection> {
    let audio_root = default_directory();
    let home = dirs::home_dir();
    let documents = dirs::document_dir().or_else(|| home.clone());
    let projects = documents
        .as_ref()
        .map(|d| d.join("Futureboard Studio").join("Projects"))
        .filter(|p| p.is_dir())
        .or_else(|| documents.clone());
    let samples = documents
        .as_ref()
        .map(|d| d.join("Futureboard Studio").join("Samples"))
        .filter(|p| p.is_dir())
        .or_else(|| Some(audio_root.clone()));

    let plugin_root = default_plugin_root();

    vec![
        BrowserRootSection {
            id: "audio-files".to_string(),
            label: "Audio Files".to_string(),
            root_path: Some(audio_root),
            kind: BrowserNodeKind::Section,
        },
        BrowserRootSection {
            id: "plugins".to_string(),
            label: "Plug-ins (VST3/CLAP)".to_string(),
            root_path: plugin_root,
            kind: BrowserNodeKind::Section,
        },
        BrowserRootSection {
            id: "instruments".to_string(),
            label: "Instruments".to_string(),
            root_path: default_instrument_root(),
            kind: BrowserNodeKind::Section,
        },
        BrowserRootSection {
            id: "projects".to_string(),
            label: "Projects".to_string(),
            root_path: projects,
            kind: BrowserNodeKind::Section,
        },
        BrowserRootSection {
            id: "samples".to_string(),
            label: "Samples".to_string(),
            root_path: samples,
            kind: BrowserNodeKind::Section,
        },
        BrowserRootSection {
            id: "user-library".to_string(),
            label: "User Library".to_string(),
            root_path: home,
            kind: BrowserNodeKind::Section,
        },
    ]
}

fn default_plugin_root() -> Option<PathBuf> {
    let candidates = [
        #[cfg(target_os = "windows")]
        PathBuf::from(r"C:\Program Files\Common Files\VST3"),
        #[cfg(target_os = "windows")]
        PathBuf::from(r"C:\Program Files\Common Files\CLAP"),
        #[cfg(target_os = "macos")]
        PathBuf::from("/Library/Audio/Plug-Ins/VST3"),
        #[cfg(target_os = "macos")]
        PathBuf::from("/Library/Audio/Plug-Ins/CLAP"),
        #[cfg(target_os = "linux")]
        PathBuf::from("/usr/lib/vst3"),
        #[cfg(target_os = "linux")]
        PathBuf::from("/usr/lib/clap"),
    ];
    candidates.into_iter().find(|p| p.is_dir())
}

fn default_instrument_root() -> Option<PathBuf> {
    dirs::audio_dir()
        .map(|p| p.join("Instruments"))
        .filter(|p| p.is_dir())
        .or_else(default_plugin_root)
}

/// Read a directory into a sorted entry list. Folders first, then files,
/// each block alphabetical (case-insensitive). Hidden entries (`.foo`) are
/// skipped — they almost never matter inside a DAW browser.
pub fn read_directory(path: &Path) -> (Vec<FileBrowserEntry>, Option<String>) {
    let read = match std::fs::read_dir(path) {
        Ok(r) => r,
        Err(e) => return (Vec::new(), Some(e.to_string())),
    };

    let mut folders: Vec<FileBrowserEntry> = Vec::new();
    let mut files: Vec<FileBrowserEntry> = Vec::new();

    for ent in read.flatten() {
        let p = ent.path();
        let name = match p.file_name().and_then(|s| s.to_str()) {
            Some(n) if !n.starts_with('.') => n.to_string(),
            _ => continue,
        };
        let meta = match ent.file_type() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if meta.is_dir() {
            folders.push(FileBrowserEntry {
                name,
                path: p,
                kind: FileEntryKind::Folder,
                extension: String::new(),
            });
        } else if meta.is_file() {
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            files.push(FileBrowserEntry {
                name,
                path: p,
                kind: FileEntryKind::File,
                extension: ext,
            });
        }
    }

    folders.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    folders.extend(files);
    (folders, None)
}
