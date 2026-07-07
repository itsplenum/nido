use crate::config::Config;
use crate::manifest::{MANIFEST_FILE, MODULES_DIR};
use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::process::Command;

const TEMPLATE: &str = r#"# nido manifest — the declarative description of your machine.
# Docs: https://github.com/itsplenum/nido
#
# Modules group config files. Paths are relative to $HOME; the real file
# lives at modules/<name>/<path> in this repo and ~/<path> becomes a symlink.
# Add files with:  nido add ~/.bashrc --module shell
#
# [modules.shell]
# files = [ ".bashrc" ]
#
# Tag modules/package groups to target machine kinds ("desktop", "server"):
# apply everything with `nido apply`, or a subset with `nido apply --tags server`.
#
# [packages.dev]
# common = [ "git", "tmux", "fd" ]   # same name on every distro (see [rename])
# arch   = [ "base-devel" ]          # distro-specific
# debian = [ "build-essential" ]
#
# [rename.fd]
# debian = "fd-find"
#
# Secrets are age-encrypted into secrets/<path>.age and decrypted to ~/<path>
# with 0600 permissions. Add them with:  nido secret add ~/.ssh/id_ed25519
"#;

pub fn run(url: Option<String>, path: Option<PathBuf>) -> Result<()> {
    let repo = match path {
        Some(p) => std::path::absolute(p)?,
        None => crate::paths::home()?.join("dotfiles"),
    };

    if let Some(url) = url {
        if repo.exists() {
            bail!("{} already exists; refusing to clone over it", repo.display());
        }
        let status = Command::new("git")
            .args(["clone", &url])
            .arg(&repo)
            .status()
            .context("failed to run git clone")?;
        if !status.success() {
            bail!("git clone failed");
        }
        if !repo.join(MANIFEST_FILE).exists() {
            bail!("cloned repo has no {MANIFEST_FILE}; is it a nido repo?");
        }
    } else {
        std::fs::create_dir_all(repo.join(MODULES_DIR))?;
        let manifest = repo.join(MANIFEST_FILE);
        if !manifest.exists() {
            std::fs::write(&manifest, TEMPLATE)?;
        }
        if !repo.join(".git").exists() {
            let status = Command::new("git")
                .arg("-C")
                .arg(&repo)
                .arg("init")
                .status()
                .context("failed to run git init")?;
            if !status.success() {
                bail!("git init failed");
            }
        }
    }

    Config { repo: repo.clone() }.save()?;
    println!("{} dotfiles repo ready at {}", "✓".green(), repo.display());
    println!("Next: adopt your first config, e.g. `nido add ~/.bashrc --module shell`");
    Ok(())
}
