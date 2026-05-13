use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub networks: BTreeMap<String, NetworkEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEntry {
    pub node_endpoint: String,
    pub genesis_hash: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            networks: BTreeMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Returns `~/.config/ccd-wallet/config.json`, resolved from `$HOME`.
pub fn config_path() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .context("could not determine home directory: $HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("ccd-wallet")
        .join("config.json"))
}

// ---------------------------------------------------------------------------
// Load / save
// ---------------------------------------------------------------------------

/// Load the config file, initialising it with defaults if it does not exist.
pub fn load() -> Result<AppConfig> {
    let path = config_path()?;

    if !path.exists() {
        let config = AppConfig::default();
        save(&config)?;
        return Ok(config);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file at {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse config file at {}", path.display()))
}

/// Persist the config file, creating parent directories as needed.
pub fn save(config: &AppConfig) -> Result<()> {
    let path = config_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create config directory at {}", parent.display())
        })?;
    }

    let contents = serde_json::to_string_pretty(config)
        .context("failed to serialise config")?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write config file at {}", path.display()))
}
