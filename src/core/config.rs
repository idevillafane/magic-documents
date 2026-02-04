use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub vault: String,
    pub date: String,
    pub time: String,
    pub default_nametype: Option<String>,
    pub editor: Option<String>,
    pub editor_mode: Option<String>,
    pub timeprint: Option<bool>,
    #[serde(default = "default_notes_dir")]
    pub notes_dir: String,
    #[serde(default = "default_diary_dir")]
    pub diary_dir: String,
    #[serde(default = "default_templates_dir")]
    pub templates_dir: String,
    /// Directorio raíz desde el cual se derivan tags (excluido del path de tags)
    #[serde(default = "default_tag_root")]
    pub tag_root: String,
    /// Mapeo de directorios trabajo -> documentación (paths relativos a tag_root)
    /// Ejemplo: "/Users/usuario/Developer" = "developer"
    #[serde(default)]
    pub dir_mappings: HashMap<String, String>,
}

fn default_notes_dir() -> String {
    "Notas".to_string()
}

fn default_diary_dir() -> String {
    "Diario".to_string()
}

fn default_templates_dir() -> String {
    "Templates".to_string()
}

fn default_tag_root() -> String {
    "Notas".to_string()
}

impl Config {
    /// Returns the config directory path (~/.config/magic-documents or $XDG_CONFIG_HOME/magic-documents)
    pub fn config_dir() -> anyhow::Result<PathBuf> {
        let config_base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("No home directory found")
                    .join(".config")
            });
        Ok(config_base.join("magic-documents"))
    }

    /// Returns the config file path (~/.config/magic-documents/config.toml)
    pub fn config_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Returns the tags cache file path (~/.config/magic-documents/tags_cache.json)
    pub fn cache_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join("tags_cache.json"))
    }

    /// Returns the primary tags cache file path (~/.config/magic-documents/primary_tags_cache.json)
    pub fn primary_cache_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join("primary_tags_cache.json"))
    }

    /// Returns the last note file path (~/.config/magic-documents/.last_note)
    pub fn last_note_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join(".last_note"))
    }

    /// Returns the aliases file path (~/.config/magic-documents/aliases.json)
    pub fn aliases_path() -> anyhow::Result<PathBuf> {
        Ok(Self::config_dir()?.join("aliases.json"))
    }

    /// Loads the default config from ~/.config/magic-documents/config.toml
    pub fn load_default() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;
        if !config_path.exists() {
            return Err(anyhow::anyhow!(
                "Config file not found at: {}",
                config_path.display()
            ));
        }
        Self::read(&config_path)
    }

    pub fn read(config_path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
