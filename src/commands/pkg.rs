use crate::config::Config;
use crate::manifest::{edit::ManifestEdit, Manifest};
use crate::pkg;
use anyhow::Result;
use owo_colors::OwoColorize;

/// `nido pkg snapshot` — capture the explicitly installed packages of this
/// machine into the manifest, under the distro-specific key. The list is a
/// starting point: the whole point of nido is *curating* it afterwards.
pub fn snapshot(group: String, tags: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let pm = pkg::detect()?;
    let explicit = pm.explicit()?;
    let mut manifest = ManifestEdit::open(&config.repo)?;
    manifest.set_package_list(&group, pm.distro_key(), &explicit, &tags);
    manifest.save()?;
    println!(
        "{} {} packages captured into [packages.{}].{}",
        "✓".green(),
        explicit.len(),
        group,
        pm.distro_key()
    );
    println!("Now curate: open nido.toml and delete what you don't want to carry to the next machine.");
    Ok(())
}

/// `nido pkg list` — what `apply` would want on this machine.
pub fn list(tags: Vec<String>) -> Result<()> {
    let config = Config::load()?;
    let manifest = Manifest::load(&config.repo)?;
    let pm = pkg::detect()?;
    let wanted = pkg::resolve(&manifest, pm.distro_key(), &tags);
    let installed = pm.installed()?;
    for p in &wanted {
        if installed.contains(p) {
            println!("{} {p}", "✓".green());
        } else {
            println!("{} {p} (missing)", "✗".red());
        }
    }
    println!("{} packages for {} ({})", wanted.len(), pm.distro_key(), pm.name());
    Ok(())
}
