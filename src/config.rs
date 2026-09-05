use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "cfg.json";
pub const CACHE_FILE: &str = "cache.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Title {
    pub displayed_folders: Option<u32>,
    pub show_program_name: bool,
}

impl Default for Title {
    fn default() -> Self {
        Self { displayed_folders: Some(1), show_program_name: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigWindow {
    pub dark: bool,
    pub show_bottom_bar: bool,
    pub antialias: bool,
    pub start_fullscreen: bool,
}

impl Default for ConfigWindow {
    fn default() -> Self {
        Self { dark: true, show_bottom_bar: true, antialias: true, start_fullscreen: false }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
}

impl Channel {
    pub fn accepts_prerelease(self) -> bool {
        matches!(self, Self::Beta)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Stable => "Stable",
            Self::Beta => "Beta",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Updates {
    pub check: bool,
    pub auto_install: bool,
    pub channel: Channel,
}

impl Default for Updates {
    fn default() -> Self {
        Self { check: true, auto_install: false, channel: Channel::default() }
    }
}

impl Updates {
    pub fn should_check(&self) -> bool {
        self.check
    }

    pub fn should_auto_install(&self) -> bool {
        self.check && self.auto_install
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Configuration {
    pub window: ConfigWindow,
    pub title: Title,
    pub bindings: Option<std::collections::BTreeMap<String, Vec<String>>>,
    pub commands: Vec<CommandDefinition>,
    pub updates: Updates,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub input: Vec<String>,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub envs: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CacheWindow {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub dark: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub show_bottom_bar: bool,
}

impl Default for CacheWindow {
    fn default() -> Self {
        Self {
            x: 64,
            y: 64,
            w: 1280,
            h: 800,
            dark: true,
            maximized: false,
            fullscreen: false,
            show_bottom_bar: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cache {
    pub window: CacheWindow,
    pub current_dir: Option<PathBuf>,
    pub last_update_check: Option<u64>,
    pub update_skipped_version: Option<String>,
    pub update_pending_version: Option<String>,
    pub last_launched_version: Option<String>,
}

fn project_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "reveal").map(|d| d.config_dir().to_path_buf())
}

pub fn config_path() -> Option<PathBuf> {
    project_dir().map(|d| d.join(CONFIG_FILE))
}

pub fn cache_path() -> Option<PathBuf> {
    project_dir().map(|d| d.join(CACHE_FILE))
}

fn load_json<T: Default + for<'de> Deserialize<'de>>(path: &Path) -> T {
    let Ok(text) = std::fs::read_to_string(path) else {
        return T::default();
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(e) => {
            log::warn!("{}: {e}; using defaults", path.display());
            T::default()
        }
    }
}

fn save_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    text.push('\n');
    std::fs::write(path, text)
}

impl Configuration {
    pub fn load_from(path: &Path) -> Self {
        load_json(path)
    }

    pub fn load() -> Self {
        config_path().map(|p| Self::load_from(&p)).unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        save_json(path, self)
    }

    pub fn save(&self) -> std::io::Result<()> {
        match config_path() {
            Some(p) => self.save_to(&p),
            None => Ok(()),
        }
    }
}

impl Cache {
    pub fn load_from(path: &Path) -> Self {
        load_json(path)
    }

    pub fn load() -> Self {
        cache_path().map(|p| Self::load_from(&p)).unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        save_json(path, self)
    }

    pub fn save(&self) -> std::io::Result<()> {
        match cache_path() {
            Some(p) => self.save_to(&p),
            None => Ok(()),
        }
    }
}
