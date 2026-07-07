use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// State of a destination path relative to the symlink we want there.
#[derive(Debug, PartialEq)]
pub enum LinkState {
    /// Symlink exists and points at the repo file. Nothing to do.
    Ok,
    /// Nothing at the destination.
    Missing,
    /// A symlink exists but points somewhere else.
    WrongTarget(PathBuf),
    /// A real file/directory sits there (needs backing up first).
    Occupied,
}

pub fn check(dest: &Path, target: &Path) -> LinkState {
    match std::fs::symlink_metadata(dest) {
        Err(_) => LinkState::Missing,
        Ok(meta) if meta.is_symlink() => match std::fs::read_link(dest) {
            Ok(current) if current == target => LinkState::Ok,
            Ok(current) => LinkState::WrongTarget(current),
            Err(_) => LinkState::WrongTarget(PathBuf::new()),
        },
        Ok(_) => LinkState::Occupied,
    }
}

/// What `ensure` did, so callers can report honestly.
#[derive(Debug, PartialEq)]
pub enum Action {
    AlreadyLinked,
    Linked,
    Relinked,
    /// Existing real file moved to `.pre-nido` sibling, then linked.
    BackedUpAndLinked(PathBuf),
}

/// Idempotently make `dest` a symlink to `target`.
pub fn ensure(dest: &Path, target: &Path) -> Result<Action> {
    let state = check(dest, target);
    if state == LinkState::Ok {
        return Ok(Action::AlreadyLinked);
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    let action = match state {
        LinkState::Ok => unreachable!(),
        LinkState::Missing => Action::Linked,
        LinkState::WrongTarget(_) => {
            std::fs::remove_file(dest)?;
            Action::Relinked
        }
        LinkState::Occupied => {
            let mut backup = dest.as_os_str().to_os_string();
            backup.push(".pre-nido");
            let backup = PathBuf::from(backup);
            std::fs::rename(dest, &backup)
                .with_context(|| format!("cannot back up {}", dest.display()))?;
            Action::BackedUpAndLinked(backup)
        }
    };
    std::os::unix::fs::symlink(target, dest)
        .with_context(|| format!("cannot link {} -> {}", dest.display(), target.display()))?;
    Ok(action)
}

/// Move a real file/dir into the repo and leave a symlink behind (nido add).
pub fn adopt(source: &Path, repo_target: &Path) -> Result<()> {
    if let Some(parent) = repo_target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // rename() is atomic but fails across filesystems; fall back to copy.
    if std::fs::rename(source, repo_target).is_err() {
        copy_recursive(source, repo_target)?;
        if source.is_dir() {
            std::fs::remove_dir_all(source)?;
        } else {
            std::fs::remove_file(source)?;
        }
    }
    std::os::unix::fs::symlink(repo_target, source).with_context(|| {
        format!(
            "moved {} into the repo but could not create the symlink back — restore it from {}",
            source.display(),
            repo_target.display()
        )
    })?;
    Ok(())
}

fn copy_recursive(from: &Path, to: &Path) -> Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        std::fs::copy(from, to)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nido-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ensure_is_idempotent_and_backs_up() {
        let dir = tmpdir("ensure");
        let target = dir.join("repo/file");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "repo content").unwrap();
        let dest = dir.join("home/file");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(&dest, "old content").unwrap();

        assert!(matches!(
            ensure(&dest, &target).unwrap(),
            Action::BackedUpAndLinked(_)
        ));
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "repo content");
        assert_eq!(
            std::fs::read_to_string(dir.join("home/file.pre-nido")).unwrap(),
            "old content"
        );
        // second run: no-op
        assert_eq!(ensure(&dest, &target).unwrap(), Action::AlreadyLinked);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adopt_moves_and_links() {
        let dir = tmpdir("adopt");
        let source = dir.join("home/.gitconfig");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "[user]").unwrap();
        let repo_target = dir.join("repo/modules/git/.gitconfig");

        adopt(&source, &repo_target).unwrap();
        assert!(source.is_symlink());
        assert_eq!(std::fs::read_link(&source).unwrap(), repo_target);
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "[user]");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
