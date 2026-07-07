use crate::config::Config;
use crate::manifest::{edit::ManifestEdit, Manifest};
use crate::{paths, symlink};
use anyhow::{bail, Result};
use owo_colors::OwoColorize;
use std::path::PathBuf;

pub fn run(files: Vec<PathBuf>, module: String, tags: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let mut manifest = ManifestEdit::open(&config.repo)?;

    for file in files {
        if !file.exists() && !file.is_symlink() {
            bail!("{} does not exist", file.display());
        }
        let rel = paths::to_home_relative(&file)?;
        let source = paths::from_home_relative(&rel)?;
        let target = Manifest::module_file(&config.repo, &module, &rel);

        if source.is_symlink() {
            let points_to = std::fs::read_link(&source)?;
            if points_to.starts_with(&config.repo) {
                println!("{} {} already managed by nido", "·".dimmed(), rel.display());
                continue;
            }
            bail!(
                "{} is a symlink to {}; adopt the real file instead",
                source.display(),
                points_to.display()
            );
        }
        if target.exists() {
            bail!(
                "{} already exists in the repo; remove it first if you want to re-adopt",
                target.display()
            );
        }

        symlink::adopt(&source, &target)?;
        manifest.add_module_file(&module, &rel.to_string_lossy(), &tags);
        println!(
            "{} {} → modules/{}/{} (symlinked back)",
            "✓".green(),
            rel.display(),
            module,
            rel.display()
        );
    }

    manifest.save()?;
    println!("Manifest updated. Review with `git -C {} diff`, then commit.", config.repo.display());
    Ok(())
}
