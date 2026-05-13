use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub version: u32,
    pub active_network: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            version: 1,
            active_network: None,
        }
    }
}

/// Returns `~/.config/ccd-wallet/state.json`, resolved from `$HOME`.
pub fn state_path() -> Result<PathBuf> {
    let home =
        std::env::var("HOME").context("could not determine home directory: $HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("ccd-wallet")
        .join("state.json"))
}

/// Load the state file, returning defaults if it does not exist.
pub fn load() -> Result<AppState> {
    let path = state_path()?;

    if !path.exists() {
        return Ok(AppState::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read state file at {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse state file at {}", path.display()))
}

/// Persist the state file, creating parent directories as needed.
pub fn save(state: &AppState) -> Result<()> {
    let path = state_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create state directory at {}", parent.display()))?;
    }

    let contents = serde_json::to_string_pretty(state).context("failed to serialise state")?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write state file at {}", path.display()))
}
