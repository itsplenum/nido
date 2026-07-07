use crate::config::Config;
use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use std::path::Path;
use std::process::Command;

/// `nido sync` — stage, commit and push the dotfiles repo in one step.
/// Editing configs is already automatic (they're symlinks into the repo);
/// this is the "save my setup to the cloud" button.
pub fn run(message: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let repo = &config.repo;

    git(repo, &["add", "-A"])?;
    let changes = git_output(repo, &["status", "--porcelain"])?;
    if changes.is_empty() {
        println!("{} nothing new to commit", "·".dimmed());
    } else {
        let n = changes.lines().count();
        let msg = message.unwrap_or_else(|| format!("sync: {n} file(s) updated"));
        git(repo, &["commit", "-m", &msg])?;
        println!("{} committed {n} change(s): {msg}", "✓".green());
    }

    let remotes = git_output(repo, &["remote"])?;
    if !remotes.lines().any(|r| r == "origin") {
        println!(
            "{} no 'origin' remote — create a PRIVATE repo on your forge and run:\n  git -C {} remote add origin <url>",
            "!".yellow(),
            repo.display()
        );
        return Ok(());
    }
    git(repo, &["push", "-u", "origin", "HEAD"])?;
    println!("{} pushed to origin", "✓".green());
    Ok(())
}

fn git(repo: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .context("failed to run git")?;
    if !status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(())
}

fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .context("failed to run git")?;
    if !out.status.success() {
        bail!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
