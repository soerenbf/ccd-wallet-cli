use anyhow::{Context, Result, bail};
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
    pub wallet_proxy: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            networks: BTreeMap::new(),
        }
    }
}

pub fn list_networks(config: &AppConfig) -> Vec<(String, NetworkEntry)> {
    config
        .networks
        .iter()
        .map(|(name, entry)| (name.clone(), entry.clone()))
        .collect()
}

pub fn rename_network(config: &mut AppConfig, old_name: &str, new_name: &str) -> Result<()> {
    if old_name == new_name {
        return Ok(());
    }
    if config.networks.contains_key(new_name) {
        bail!("network '{new_name}' is already registered");
    }
    let entry = config
        .networks
        .remove(old_name)
        .with_context(|| format!("network '{old_name}' is not registered"))?;
    config.networks.insert(new_name.to_owned(), entry);
    Ok(())
}

pub fn delete_networks(
    config: &mut AppConfig,
    names: &[String],
) -> Result<Vec<(String, NetworkEntry)>> {
    let mut removed = Vec::new();
    for name in names {
        let entry = config
            .networks
            .remove(name)
            .with_context(|| format!("network '{name}' is not registered"))?;
        removed.push((name.clone(), entry));
    }
    Ok(removed)
}

pub fn aliases_by_genesis_hash(config: &AppConfig, genesis_hash: &str) -> Vec<String> {
    config
        .networks
        .iter()
        .filter(|(_, entry)| entry.genesis_hash == genesis_hash)
        .map(|(name, _)| name.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

/// Returns `~/.config/ccd-wallet/config.json`, resolved from `$HOME`.
pub fn config_path() -> Result<PathBuf> {
    let home =
        std::env::var("HOME").context("could not determine home directory: $HOME is not set")?;
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

    let contents = serde_json::to_string_pretty(config).context("failed to serialise config")?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write config file at {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> NetworkEntry {
        NetworkEntry {
            node_endpoint: format!("https://{name}.example.com:20000"),
            genesis_hash: format!("hash-{name}"),
            wallet_proxy: Some(format!("https://wallet-proxy.{name}.example.com")),
        }
    }

    #[test]
    fn list_networks_returns_sorted_entries() {
        let mut config = AppConfig::default();
        config
            .networks
            .insert("testnet".to_owned(), entry("testnet"));
        config
            .networks
            .insert("mainnet".to_owned(), entry("mainnet"));

        let networks = list_networks(&config);
        assert_eq!(networks[0].0, "mainnet");
        assert_eq!(networks[1].0, "testnet");
    }

    #[test]
    fn rename_network_moves_key_and_preserves_entry() {
        let mut config = AppConfig::default();
        config
            .networks
            .insert("testnet".to_owned(), entry("testnet"));

        rename_network(&mut config, "testnet", "staging").unwrap();

        assert!(!config.networks.contains_key("testnet"));
        let staging = config.networks.get("staging").unwrap();
        assert_eq!(staging.genesis_hash, "hash-testnet");
    }

    #[test]
    fn rename_network_rejects_duplicate_target() {
        let mut config = AppConfig::default();
        config
            .networks
            .insert("testnet".to_owned(), entry("testnet"));
        config
            .networks
            .insert("mainnet".to_owned(), entry("mainnet"));

        let err = rename_network(&mut config, "testnet", "mainnet").unwrap_err();
        assert!(err.to_string().contains("already registered"));
    }

    #[test]
    fn delete_networks_removes_requested_aliases() {
        let mut config = AppConfig::default();
        config
            .networks
            .insert("testnet-a".to_owned(), entry("testnet"));
        config.networks.insert(
            "testnet-b".to_owned(),
            NetworkEntry {
                genesis_hash: "hash-testnet".to_owned(),
                ..entry("staging")
            },
        );

        let removed = delete_networks(
            &mut config,
            &["testnet-a".to_owned(), "testnet-b".to_owned()],
        )
        .unwrap();

        assert_eq!(removed.len(), 2);
        assert!(config.networks.is_empty());
    }

    #[test]
    fn aliases_by_genesis_hash_lists_matching_aliases() {
        let mut config = AppConfig::default();
        config
            .networks
            .insert("testnet-a".to_owned(), entry("testnet"));
        config.networks.insert(
            "testnet-b".to_owned(),
            NetworkEntry {
                genesis_hash: "hash-testnet".to_owned(),
                ..entry("staging")
            },
        );
        config
            .networks
            .insert("mainnet".to_owned(), entry("mainnet"));

        assert_eq!(
            aliases_by_genesis_hash(&config, "hash-testnet"),
            vec!["testnet-a".to_owned(), "testnet-b".to_owned()]
        );
    }
}
