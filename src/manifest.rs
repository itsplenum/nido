use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILE: &str = "nido.toml";
pub const MODULES_DIR: &str = "modules";
pub const SECRETS_DIR: &str = "secrets";

/// The manifest is the heart of nido: a declarative description of the
/// desired state of a machine. The tool's job is only to make reality
/// converge to it (idempotently).
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default)]
    pub modules: BTreeMap<String, Module>,
    #[serde(default)]
    pub packages: BTreeMap<String, PackageGroup>,
    /// Canonical name -> distro key -> real package name.
    /// e.g. `fd = { debian = "fd-find" }`
    #[serde(default)]
    pub rename: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub secrets: Secrets,
}

/// A module is a named group of config files (git, shell, nvim...).
/// Files are stored as paths relative to $HOME; on disk they live at
/// `<repo>/modules/<name>/<rel>` and get symlinked to `~/<rel>`.
#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Module {
    /// Machine kinds this applies to (e.g. "desktop", "server").
    /// Empty = applies everywhere.
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub files: Vec<PathBuf>,
}

#[derive(Deserialize, Default, Debug)]
pub struct PackageGroup {
    /// Empty = applies everywhere.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Packages whose name is the same on every distro (after `rename`).
    #[serde(default)]
    pub common: Vec<String>,
    /// Distro-specific lists, keyed by distro key ("arch", "debian", ...).
    #[serde(flatten)]
    pub per_distro: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize, Default, Debug)]
#[serde(deny_unknown_fields)]
pub struct Secrets {
    /// $HOME-relative paths. Stored encrypted at `<repo>/secrets/<rel>.age`,
    /// decrypted to `~/<rel>` with 0600 permissions on apply.
    #[serde(default)]
    pub files: Vec<PathBuf>,
}

impl Manifest {
    pub fn load(repo: &Path) -> Result<Self> {
        let path = repo.join(MANIFEST_FILE);
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read manifest {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("invalid manifest {}", path.display()))
    }

    /// Does a module pass the tag filter? No filter or untagged module = yes.
    pub fn tags_match(tags: &[String], filter: &[String]) -> bool {
        filter.is_empty() || tags.is_empty() || tags.iter().any(|t| filter.contains(t))
    }

    /// Where the real file for `rel` of module `module` lives inside the repo.
    pub fn module_file(repo: &Path, module: &str, rel: &Path) -> PathBuf {
        repo.join(MODULES_DIR).join(module).join(rel)
    }

    /// Where the encrypted blob for secret `rel` lives inside the repo.
    pub fn secret_file(repo: &Path, rel: &Path) -> PathBuf {
        let mut name = rel.as_os_str().to_os_string();
        name.push(".age");
        repo.join(SECRETS_DIR).join(name)
    }
}

/// Mutations to nido.toml go through toml_edit so user comments and
/// formatting survive. Reading uses serde (above) for a typed view.
pub mod edit {
    use super::MANIFEST_FILE;
    use anyhow::{Context, Result};
    use std::path::Path;
    use toml_edit::{Array, DocumentMut, Item, Table, Value};

    pub struct ManifestEdit {
        path: std::path::PathBuf,
        doc: DocumentMut,
    }

    impl ManifestEdit {
        pub fn open(repo: &Path) -> Result<Self> {
            let path = repo.join(MANIFEST_FILE);
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("cannot read manifest {}", path.display()))?;
            Ok(Self {
                path,
                doc: raw.parse().context("invalid manifest")?,
            })
        }

        pub fn save(&self) -> Result<()> {
            std::fs::write(&self.path, self.doc.to_string())?;
            Ok(())
        }

        fn table_at<'a>(doc: &'a mut DocumentMut, keys: &[&str]) -> &'a mut Table {
            let mut table = doc.as_table_mut();
            for key in keys {
                table = table
                    .entry(key)
                    .or_insert(Item::Table(Table::new()))
                    .as_table_mut()
                    .expect("nido-managed key is not a table");
            }
            table
        }

        fn push_unique(arr: &mut Array, value: &str) -> bool {
            if arr.iter().any(|v| v.as_str() == Some(value)) {
                return false;
            }
            arr.push_formatted(Value::from(value).decorated("\n  ", ""));
            arr.set_trailing("\n");
            arr.set_trailing_comma(true);
            true
        }

        /// Register `rel` under [modules.<name>].files; returns false if it
        /// was already there.
        pub fn add_module_file(&mut self, module: &str, rel: &str, tags: &[String]) -> bool {
            let table = Self::table_at(&mut self.doc, &["modules", module]);
            if !tags.is_empty() && table.get("tags").is_none() {
                let mut arr = Array::new();
                arr.extend(tags.iter().map(|t| t.as_str()));
                table["tags"] = toml_edit::value(arr);
            }
            let files = table
                .entry("files")
                .or_insert(toml_edit::value(Array::new()))
                .as_array_mut()
                .expect("files is not an array");
            Self::push_unique(files, rel)
        }

        /// Register `rel` under [secrets].files; returns false if already there.
        pub fn add_secret_file(&mut self, rel: &str) -> bool {
            let table = Self::table_at(&mut self.doc, &["secrets"]);
            let files = table
                .entry("files")
                .or_insert(toml_edit::value(Array::new()))
                .as_array_mut()
                .expect("files is not an array");
            Self::push_unique(files, rel)
        }

        /// Replace [packages.<group>].<distro_key> with the given list.
        pub fn set_package_list(
            &mut self,
            group: &str,
            distro_key: &str,
            packages: &[String],
            tags: &[String],
        ) {
            let table = Self::table_at(&mut self.doc, &["packages", group]);
            if !tags.is_empty() && table.get("tags").is_none() {
                let mut arr = Array::new();
                arr.extend(tags.iter().map(|t| t.as_str()));
                table["tags"] = toml_edit::value(arr);
            }
            let mut arr = Array::new();
            for pkg in packages {
                arr.push_formatted(Value::from(pkg.as_str()).decorated("\n  ", ""));
            }
            arr.set_trailing("\n");
            arr.set_trailing_comma(true);
            table[distro_key] = toml_edit::value(arr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_manifest() {
        let raw = r#"
            [modules.shell]
            files = [".bashrc", ".config/starship.toml"]

            [modules.desktop-only]
            tags = ["desktop"]
            files = [".config/ghostty/config"]

            [packages.dev]
            common = ["git", "tmux", "fd"]
            arch = ["base-devel"]
            debian = ["build-essential"]

            [rename.fd]
            debian = "fd-find"

            [secrets]
            files = [".ssh/id_ed25519"]
        "#;
        let m: Manifest = toml::from_str(raw).unwrap();
        assert_eq!(m.modules["shell"].files.len(), 2);
        assert_eq!(m.packages["dev"].per_distro["debian"], vec!["build-essential"]);
        assert_eq!(m.rename["fd"]["debian"], "fd-find");
        assert_eq!(m.secrets.files, vec![PathBuf::from(".ssh/id_ed25519")]);
    }

    #[test]
    fn tag_filtering() {
        let desktop = vec!["desktop".to_string()];
        assert!(Manifest::tags_match(&[], &[]));
        assert!(Manifest::tags_match(&[], &desktop)); // untagged = universal
        assert!(Manifest::tags_match(&desktop, &[])); // no filter = everything
        assert!(Manifest::tags_match(&desktop, &desktop));
        assert!(!Manifest::tags_match(&desktop, &["server".to_string()]));
    }

    #[test]
    fn secret_path_gets_age_suffix() {
        let p = Manifest::secret_file(Path::new("/repo"), Path::new(".ssh/id_ed25519"));
        assert_eq!(p, PathBuf::from("/repo/secrets/.ssh/id_ed25519.age"));
    }
}
