# nido

Declarative dotfiles + packages + secrets manager. Rebuild your whole setup on a clean machine with one command.

```sh
curl -fsSL https://raw.githubusercontent.com/itsplenum/nido/main/install.sh | sh
nido init git@github.com:you/dotfiles.git
nido apply
```

That's it: your packages installed, your configs symlinked, your SSH keys decrypted.

Prefer a truly fresh start? Restore only your secrets and build the rest from zero, adopting configs as you create them:

```sh
nido secret apply      # just your SSH keys, nothing else
nido add ~/.config/foo --module foo   # adopt new configs as you go
nido sync              # one-command commit + push
```

## Why

Every reinstall you lose hours rebuilding the same environment. nido turns your setup into a **declarative manifest** in a git repo — a single source of truth describing what your machine should look like — and makes any machine converge to it, **idempotently** (running `apply` twice changes nothing the second time).

nido is built for starting from scratch: no framework, no preset rice. You adopt the configs *you* choose, one by one, and the repo grows with you.

## How it works

Your dotfiles repo looks like this:

```
dotfiles/
├── nido.toml            # the manifest (modules, packages, secrets)
├── modules/
│   ├── shell/.bashrc    # real files live here...
│   └── git/.config/git/config
└── secrets/
    └── .ssh/id_ed25519.age   # age-encrypted, safe to push
```

- **Configs** — `nido add ~/.bashrc --module shell` moves the file into the repo and leaves a symlink behind. You keep editing the file at its normal path; changes land in the repo, ready to commit. `nido apply` recreates all symlinks on a new machine.
- **Packages** — the manifest declares package groups with canonical names; nido translates per distro (pacman on Arch, apt on Debian/Ubuntu) and installs what's missing. `nido pkg snapshot` bootstraps the list from what you have installed today.
- **Secrets** — `nido secret add ~/.ssh/id_ed25519` encrypts with [age](https://age-encryption.org) (passphrase, scrypt). Only ciphertext enters the repo; `apply` decrypts to the right path with `0600` permissions.
- **Tags** — mark modules and package groups `desktop` or `server` (or anything). On a server: `nido apply --tags server` — your shell and git arrive, Steam doesn't.

## Manifest reference

```toml
[modules.shell]
files = [".bashrc", ".config/starship.toml"]

[modules.hyprland]
tags = ["desktop"]                  # untagged modules apply everywhere
files = [".config/hypr"]            # directories work too

[packages.dev]
common = ["git", "tmux", "fd"]      # same name on every distro (see rename)
arch = ["base-devel"]               # distro-specific extras
debian = ["build-essential"]

[packages.desktop]
tags = ["desktop"]
common = ["steam"]

[rename.fd]
debian = "fd-find"                  # canonical name -> per-distro real name

[secrets]
files = [".ssh/id_ed25519", ".ssh/id_ed25519.pub", ".ssh/config"]
```

## Commands

| Command | What it does |
|---|---|
| `nido init [url] [--path DIR]` | Create or clone your dotfiles repo |
| `nido add <files> -m <module> [-t tags]` | Adopt configs into the repo (symlink back) |
| `nido apply [--tags t] [--modules m] [--dry-run]` | Converge: packages → symlinks → secrets |
| `nido pkg snapshot [-g group] [-t tags]` | Capture installed packages into the manifest |
| `nido pkg list` | Show wanted vs installed for this machine |
| `nido secret add <files>` | Encrypt secrets into the repo |
| `nido secret apply` | Decrypt only the secrets (minimal fresh-machine path) |
| `nido status` | Drift report: broken links, missing packages, dirty repo |
| `nido sync [-m msg]` | Commit + push the repo in one step |

Automation: set `NIDO_PASSPHRASE` to skip the interactive secrets prompt (CI, containers).

Anything `apply` would overwrite is backed up next to the original as `<name>.pre-nido` — nido never destroys a file it didn't create.

## Supported

Linux with pacman (Arch & derivatives) or apt (Debian, Ubuntu & derivatives). The `PackageManager` trait is small — PRs for other backends welcome.

## Build from source

```sh
cargo install --git https://github.com/itsplenum/nido
```

## License

MIT
