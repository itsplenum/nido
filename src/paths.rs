use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

pub fn home() -> Result<PathBuf> {
    dirs::home_dir().context("could not determine home directory")
}

/// Convert any user-supplied path into a path relative to $HOME.
/// This relative path is the canonical identity of a file in the repo:
/// `modules/<mod>/<rel>` in the repo maps to `~/<rel>` on disk.
pub fn to_home_relative(path: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(path)
        .with_context(|| format!("cannot absolutize {}", path.display()))?;
    let home = home()?;
    match abs.strip_prefix(&home) {
        Ok(rel) if rel.as_os_str().is_empty() => bail!("refusing to manage $HOME itself"),
        Ok(rel) => Ok(rel.to_path_buf()),
        Err(_) => bail!(
            "{} is outside your home directory; nido only manages files under ~",
            abs.display()
        ),
    }
}

/// `~/<rel>` — where a managed file lives on the real system.
pub fn from_home_relative(rel: &Path) -> Result<PathBuf> {
    Ok(home()?.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_roundtrip() {
        let home = home().unwrap();
        let p = home.join(".config/git/config");
        let rel = to_home_relative(&p).unwrap();
        assert_eq!(rel, PathBuf::from(".config/git/config"));
        assert_eq!(from_home_relative(&rel).unwrap(), p);
    }

    #[test]
    fn rejects_outside_home() {
        assert!(to_home_relative(Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn rejects_home_itself() {
        assert!(to_home_relative(&home().unwrap()).is_err());
    }
}
