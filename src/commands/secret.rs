use crate::config::Config;
use crate::manifest::{edit::ManifestEdit, Manifest};
use crate::{paths, secrets};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::path::PathBuf;

/// `nido secret add` — encrypt a file into the repo. The original stays in
/// place untouched (it's your live key); `apply` on another machine recreates
/// it with 0600 permissions.
pub fn add(files: Vec<PathBuf>) -> Result<()> {
    let config = Config::load()?;
    let pass = secrets::passphrase(true)?;
    let mut manifest = ManifestEdit::open(&config.repo)?;

    for file in files {
        let rel = paths::to_home_relative(&file)?;
        let source = paths::from_home_relative(&rel)?;
        let plaintext =
            std::fs::read(&source).with_context(|| format!("cannot read {}", source.display()))?;
        let encrypted = secrets::encrypt(&plaintext, &pass)?;
        let dest = Manifest::secret_file(&config.repo, &rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, encrypted)?;
        manifest.add_secret_file(&rel.to_string_lossy());
        println!(
            "{} ~/{} encrypted → secrets/{}.age (original untouched)",
            "✓".green(),
            rel.display(),
            rel.display()
        );
    }

    manifest.save()?;
    println!("{}", "Only the encrypted .age files enter the repo. Never commit the originals.".yellow());
    Ok(())
}
