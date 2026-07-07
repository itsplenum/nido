use crate::config::Config;
use crate::manifest::{edit::ManifestEdit, Manifest};
use crate::{paths, secrets};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use std::path::PathBuf;

/// `nido secret apply` — decrypt only the secrets, touching nothing else.
/// This is the minimal fresh-machine path: get your SSH keys working and
/// build the rest of the system from scratch.
pub fn apply(dry: bool) -> Result<()> {
    let config = Config::load()?;
    let manifest = Manifest::load(&config.repo)?;
    if manifest.secrets.files.is_empty() {
        println!("no secrets declared in the manifest");
        return Ok(());
    }
    apply_files(&manifest, &config, dry)
}

/// Shared by `apply` and `secret apply`.
pub fn apply_files(manifest: &Manifest, config: &Config, dry: bool) -> Result<()> {
    if dry {
        println!(
            "{} would decrypt {} secret(s): {}",
            "→".cyan(),
            manifest.secrets.files.len(),
            manifest
                .secrets
                .files
                .iter()
                .map(|p| format!("~/{}", p.display()))
                .collect::<Vec<_>>()
                .join(" ")
        );
        return Ok(());
    }
    let pass = secrets::passphrase(false)?;
    for rel in &manifest.secrets.files {
        let encrypted_path = Manifest::secret_file(&config.repo, rel);
        let ciphertext = std::fs::read(&encrypted_path)
            .with_context(|| format!("missing encrypted secret {}", encrypted_path.display()))?;
        let plaintext = secrets::decrypt(&ciphertext, &pass)?;
        let dest = paths::from_home_relative(rel)?;
        if std::fs::read(&dest).map(|cur| cur == plaintext).unwrap_or(false) {
            println!("{} ~/{} already up to date", "✓".green(), rel.display());
            continue;
        }
        secrets::write_secret(&dest, &plaintext)?;
        println!("{} ~/{} decrypted (0600)", "✓".green(), rel.display());
    }
    Ok(())
}

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
