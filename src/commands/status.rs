use crate::config::Config;
use crate::manifest::Manifest;
use crate::{paths, pkg, symlink};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::process::Command;

/// `nido status` — drift report: what differs between the manifest (desired
/// state) and this machine (actual state). Read-only.
pub fn run() -> Result<()> {
    let config = Config::load()?;
    let manifest = Manifest::load(&config.repo)?;
    let mut drift = false;

    // Symlinks
    let mut ok = 0usize;
    for (name, module) in &manifest.modules {
        for rel in &module.files {
            let target = Manifest::module_file(&config.repo, name, rel);
            let dest = paths::from_home_relative(rel)?;
            match symlink::check(&dest, &target) {
                symlink::LinkState::Ok => ok += 1,
                state => {
                    drift = true;
                    println!("{} ~/{} [{}]: {state:?}", "✗".red(), rel.display(), name);
                }
            }
        }
    }
    println!("{} symlinks: {ok} in place", "✓".green());

    // Packages
    match pkg::detect() {
        Ok(pm) => {
            let wanted = pkg::resolve(&manifest, pm.distro_key(), &[]);
            let installed = pm.installed()?;
            let missing: Vec<&String> = wanted.iter().filter(|p| !installed.contains(*p)).collect();
            if missing.is_empty() {
                println!("{} packages: all {} present", "✓".green(), wanted.len());
            } else {
                drift = true;
                println!(
                    "{} packages: {} missing: {}",
                    "✗".red(),
                    missing.len(),
                    missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")
                );
            }
        }
        Err(e) => println!("{} packages: {e}", "!".yellow()),
    }

    // Secrets (only presence of the encrypted blobs; content needs the passphrase)
    for rel in &manifest.secrets.files {
        let enc = Manifest::secret_file(&config.repo, rel);
        if !enc.exists() {
            drift = true;
            println!("{} secret ~/{}: missing {}", "✗".red(), rel.display(), enc.display());
        }
    }
    if !manifest.secrets.files.is_empty() {
        println!("{} secrets: {} declared", "✓".green(), manifest.secrets.files.len());
    }

    // Repo cleanliness
    let out = Command::new("git")
        .arg("-C")
        .arg(&config.repo)
        .args(["status", "--porcelain"])
        .output()?;
    if out.status.success() && !out.stdout.is_empty() {
        drift = true;
        println!("{} repo has uncommitted changes ({})", "!".yellow(), config.repo.display());
    }

    if !drift {
        println!("{}", "everything converged — machine matches the manifest".green());
    }
    Ok(())
}
