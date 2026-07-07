#!/bin/sh
# nido installer — downloads the latest release binary, or builds from
# source with cargo if no binary matches this platform.
set -eu

REPO="itsplenum/nido"
INSTALL_DIR="${NIDO_INSTALL_DIR:-$HOME/.local/bin}"

main() {
    os="$(uname -s)"
    arch="$(uname -m)"
    if [ "$os" != "Linux" ] || [ "$arch" != "x86_64" ]; then
        echo "no prebuilt binary for $os/$arch, trying cargo..."
        build_from_source
        return
    fi

    url="https://github.com/$REPO/releases/latest/download/nido-x86_64-linux"
    mkdir -p "$INSTALL_DIR"
    echo "downloading $url"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$INSTALL_DIR/nido" "$url" || { echo "download failed, trying cargo..."; build_from_source; return; }
    else
        wget -qO "$INSTALL_DIR/nido" "$url" || { echo "download failed, trying cargo..."; build_from_source; return; }
    fi
    chmod +x "$INSTALL_DIR/nido"
    echo "installed to $INSTALL_DIR/nido"
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *) echo "note: add $INSTALL_DIR to your PATH" ;;
    esac
}

build_from_source() {
    # a C linker is required to build Rust code
    if ! command -v cc >/dev/null 2>&1; then
        echo "error: no C compiler found. On Ubuntu/Debian run:" >&2
        echo "    sudo apt install -y build-essential curl git" >&2
        echo "then re-run this script." >&2
        exit 1
    fi
    if ! command -v cargo >/dev/null 2>&1; then
        # distro cargo is often too old (Ubuntu 24.04 ships 1.75); use rustup
        echo "installing rust via rustup (minimal profile)..."
        curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
        . "$HOME/.cargo/env"
    fi
    cargo install --git "https://github.com/$REPO" --locked
    echo "installed to $HOME/.cargo/bin/nido (open a new shell if not found)"
}

main "$@"
