use crate::config::Config;
use crate::manifest::Manifest;
use crate::{paths, pkg, secrets, symlink};
use anyhow::{Context, Result};
use owo_colors::OwoColorize;

pub struct Options {
    pub modules: Vec<String>,
    pub tags: Vec<String>,
    pub dry_run: bool,
    pub skip_packages: bool,
    pub skip_secrets: bool,
}

/// Converge the machine to the manifest: packages, then symlinks, then
/// secrets. Every step is idempotent — running apply twice is a no-op.
pub fn run(opts: Options) -> Result<()> {
    let config = Config::load()?;
    let manifest = Manifest::load(&config.repo)?;
    let dry = opts.dry_run;
    if dry {
        println!("{}", "dry run — nothing will be changed".yellow());
    }

    if !opts.skip_packages && !manifest.packages.is_empty() {
        packages(&manifest, &opts)?;
    }
    links(&manifest, &config, &opts)?;
    if !opts.skip_secrets && !manifest.secrets.files.is_empty() {
        apply_secrets(&manifest, &config, dry)?;
    }

    println!("{} apply finished", "✓".green());
    Ok(())
}

fn packages(manifest: &Manifest, opts: &Options) -> Result<()> {
    let pm = pkg::detect()?;
    let wanted = pkg::resolve(manifest, pm.distro_key(), &opts.tags);
    let installed = pm.installed()?;
    let missing: Vec<String> = wanted.into_iter().filter(|p| !installed.contains(p)).collect();
    if missing.is_empty() {
        println!("{} packages: all present", "✓".green());
    } else if opts.dry_run {
        println!("{} would install {} packages via {}: {}", "→".cyan(), missing.len(), pm.name(), missing.join(" "));
    } else {
        println!("{} installing {} packages via {}...", "→".cyan(), missing.len(), pm.name());
        pm.install(&missing)?;
        println!("{} packages installed", "✓".green());
    }
    Ok(())
}

fn links(manifest: &Manifest, config: &Config, opts: &Options) -> Result<()> {
    let mut linked = 0usize;
    let mut created = 0usize;
    for (name, module) in &manifest.modules {
        if !opts.modules.is_empty() && !opts.modules.contains(name) {
            continue;
        }
        if !Manifest::tags_match(&module.tags, &opts.tags) {
            continue;
        }
        for rel in &module.files {
            let target = Manifest::module_file(&config.repo, name, rel);
            let dest = paths::from_home_relative(rel)?;
            if !target.exists() {
                println!(
                    "{} {}: {} missing from repo — skipped",
                    "!".red(),
                    name,
                    target.display()
                );
                continue;
            }
            if opts.dry_run {
                match symlink::check(&dest, &target) {
                    symlink::LinkState::Ok => linked += 1,
                    state => println!("{} would link ~/{} ({state:?})", "→".cyan(), rel.display()),
                }
                continue;
            }
            match symlink::ensure(&dest, &target)? {
                symlink::Action::AlreadyLinked => linked += 1,
                symlink::Action::Linked | symlink::Action::Relinked => created += 1,
                symlink::Action::BackedUpAndLinked(backup) => {
                    created += 1;
                    println!(
                        "{} existing ~/{} moved to {}",
                        "!".yellow(),
                        rel.display(),
                        backup.display()
                    );
                }
            }
        }
    }
    println!("{} symlinks: {created} created, {linked} already in place", "✓".green());
    Ok(())
}

fn apply_secrets(manifest: &Manifest, config: &Config, dry: bool) -> Result<()> {
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
