use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// nido's own tiny config: it only remembers where your dotfiles repo lives.
/// Everything else is declared in the repo's nido.toml, so the repo stays
/// the single source of truth and this file is disposable.
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub repo: PathBuf,
}

fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("could not determine config directory")?;
    Ok(base.join("nido/config.toml"))
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        let raw = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "no nido config at {} — run `nido init` first",
                path.display()
            )
        })?;
        toml::from_str(&raw).context("invalid nido config")
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}
