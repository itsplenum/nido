use crate::manifest::Manifest;
use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::process::Command;

/// Abstraction over the system package manager. This is what lets the same
/// dotfiles repo provision an Arch desktop today and an Ubuntu server
/// tomorrow: the manifest speaks canonical names, each backend translates.
pub trait PackageManager {
    fn name(&self) -> &'static str;
    /// Key used in the manifest ("arch", "debian").
    fn distro_key(&self) -> &'static str;
    /// Every package currently installed (for drift/status checks).
    fn installed(&self) -> Result<HashSet<String>>;
    /// Packages the user installed explicitly (for `pkg snapshot`).
    fn explicit(&self) -> Result<Vec<String>>;
    fn install(&self, packages: &[String]) -> Result<()>;
}

pub struct Pacman;
pub struct Apt;

fn run_lines(cmd: &mut Command) -> Result<Vec<String>> {
    let out = cmd.output().with_context(|| format!("failed to run {cmd:?}"))?;
    if !out.status.success() {
        bail!("{cmd:?} failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// Prefix with sudo when not root (a fresh container is root, a desktop isn't).
fn privileged(program: &str) -> Command {
    let is_root = run_lines(Command::new("id").arg("-u"))
        .map(|l| l.first().map(|u| u == "0").unwrap_or(false))
        .unwrap_or(false);
    if is_root {
        Command::new(program)
    } else {
        let mut cmd = Command::new("sudo");
        cmd.arg(program);
        cmd
    }
}

fn run_install(mut cmd: Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("failed to run {cmd:?}"))?;
    if !status.success() {
        bail!("package installation failed ({status})");
    }
    Ok(())
}

impl PackageManager for Pacman {
    fn name(&self) -> &'static str {
        "pacman"
    }
    fn distro_key(&self) -> &'static str {
        "arch"
    }
    fn installed(&self) -> Result<HashSet<String>> {
        Ok(run_lines(Command::new("pacman").arg("-Qq"))?.into_iter().collect())
    }
    fn explicit(&self) -> Result<Vec<String>> {
        run_lines(Command::new("pacman").arg("-Qqe"))
    }
    fn install(&self, packages: &[String]) -> Result<()> {
        let mut cmd = privileged("pacman");
        cmd.args(["-S", "--needed", "--noconfirm"]).args(packages);
        run_install(cmd)
    }
}

impl PackageManager for Apt {
    fn name(&self) -> &'static str {
        "apt"
    }
    fn distro_key(&self) -> &'static str {
        "debian"
    }
    fn installed(&self) -> Result<HashSet<String>> {
        let lines = run_lines(
            Command::new("dpkg-query").args(["-W", "-f", "${binary:Package}\t${Status}\n"]),
        )?;
        Ok(lines
            .into_iter()
            .filter(|l| l.ends_with("install ok installed"))
            .filter_map(|l| {
                // "pkg:arch\t..." -> "pkg"
                let name = l.split('\t').next()?;
                Some(name.split(':').next().unwrap_or(name).to_string())
            })
            .collect())
    }
    fn explicit(&self) -> Result<Vec<String>> {
        run_lines(Command::new("apt-mark").arg("showmanual"))
    }
    fn install(&self, packages: &[String]) -> Result<()> {
        let mut cmd = privileged("apt-get");
        cmd.args(["install", "-y", "--no-install-recommends"]).args(packages);
        cmd.env("DEBIAN_FRONTEND", "noninteractive");
        run_install(cmd)
    }
}

/// Detect the package manager from /etc/os-release (ID and ID_LIKE).
pub fn detect() -> Result<Box<dyn PackageManager>> {
    let raw = std::fs::read_to_string("/etc/os-release").context("cannot read /etc/os-release")?;
    let mut ids: Vec<String> = Vec::new();
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("ID=").or_else(|| line.strip_prefix("ID_LIKE=")) {
            ids.extend(
                v.trim_matches('"')
                    .split_whitespace()
                    .map(|s| s.to_lowercase()),
            );
        }
    }
    if ids.iter().any(|id| id == "arch") {
        Ok(Box::new(Pacman))
    } else if ids.iter().any(|id| id == "debian" || id == "ubuntu") {
        Ok(Box::new(Apt))
    } else {
        bail!("unsupported distro (os-release ids: {ids:?}); nido v1 supports pacman and apt")
    }
}

/// Resolve the manifest into the concrete package list for this machine:
/// filter groups by tag, merge `common` (through `rename`) with the
/// distro-specific list, dedupe preserving order.
pub fn resolve(manifest: &Manifest, distro_key: &str, tag_filter: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for group in manifest.packages.values() {
        if !Manifest::tags_match(&group.tags, tag_filter) {
            continue;
        }
        let common = group.common.iter().map(|name| {
            manifest
                .rename
                .get(name)
                .and_then(|m| m.get(distro_key))
                .unwrap_or(name)
                .clone()
        });
        let distro = group.per_distro.get(distro_key).cloned().unwrap_or_default();
        for pkg in common.chain(distro) {
            if seen.insert(pkg.clone()) {
                result.push(pkg);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        toml::from_str(
            r#"
            [packages.dev]
            common = ["git", "fd"]
            arch = ["base-devel"]
            debian = ["build-essential"]

            [packages.desktop]
            tags = ["desktop"]
            common = ["steam"]

            [rename.fd]
            debian = "fd-find"
        "#,
        )
        .unwrap()
    }

    #[test]
    fn resolves_with_rename_and_distro_lists() {
        let m = manifest();
        assert_eq!(
            resolve(&m, "debian", &["server".into()]),
            vec!["git", "fd-find", "build-essential"]
        );
        // groups iterate in BTreeMap (alphabetical) order: desktop, dev
        assert_eq!(
            resolve(&m, "arch", &[]),
            vec!["steam", "git", "fd", "base-devel"]
        );
    }
}
