mod commands;
mod config;
mod manifest;
mod paths;
mod pkg;
mod secrets;
mod symlink;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Declarative dotfiles + packages + secrets. Your whole setup, one command.
#[derive(Parser)]
#[command(name = "nido", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new dotfiles repo, or clone an existing one
    Init {
        /// Git URL of an existing nido repo (omit to start fresh)
        url: Option<String>,
        /// Where the repo lives (default: ~/dotfiles)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Move files into the repo and symlink them back
    Add {
        /// Files or directories under $HOME
        #[arg(required = true)]
        files: Vec<PathBuf>,
        /// Module to file them under (e.g. shell, git, nvim)
        #[arg(short, long)]
        module: String,
        /// Tags for the module if it's new (e.g. desktop, server)
        #[arg(short, long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    /// Converge this machine to the manifest (packages + symlinks + secrets)
    Apply {
        /// Only these modules
        #[arg(long, value_delimiter = ',')]
        modules: Vec<String>,
        /// Only modules/package groups with these tags (untagged = always)
        #[arg(short, long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Show what would happen without changing anything
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        skip_packages: bool,
        #[arg(long)]
        skip_secrets: bool,
    },
    /// Package list management
    #[command(subcommand)]
    Pkg(PkgCmd),
    /// Age-encrypted secrets management
    #[command(subcommand)]
    Secret(SecretCmd),
    /// Show drift between the manifest and this machine
    Status,
}

#[derive(Subcommand)]
enum PkgCmd {
    /// Capture this machine's explicitly installed packages into the manifest
    Snapshot {
        /// Package group name in the manifest
        #[arg(short, long, default_value = "snapshot")]
        group: String,
        /// Tags for the group if it's new
        #[arg(short, long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    /// List the packages the manifest wants on this machine
    List {
        #[arg(short, long, value_delimiter = ',')]
        tags: Vec<String>,
    },
}

#[derive(Subcommand)]
enum SecretCmd {
    /// Encrypt files into the repo (originals stay in place)
    Add {
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Init { url, path } => commands::init::run(url, path),
        Cmd::Add { files, module, tags } => commands::add::run(files, module, tags),
        Cmd::Apply {
            modules,
            tags,
            dry_run,
            skip_packages,
            skip_secrets,
        } => commands::apply::run(commands::apply::Options {
            modules,
            tags,
            dry_run,
            skip_packages,
            skip_secrets,
        }),
        Cmd::Pkg(PkgCmd::Snapshot { group, tags }) => commands::pkg::snapshot(group, tags),
        Cmd::Pkg(PkgCmd::List { tags }) => commands::pkg::list(tags),
        Cmd::Secret(SecretCmd::Add { files }) => commands::secret::add(files),
        Cmd::Status => commands::status::run(),
    }
}
