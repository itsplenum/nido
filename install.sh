#!/bin/sh
# nido installer — downloads the latest release binary, or builds from
# source with cargo if no binary matches this platform.
set -eu

REPO="harpeblue/nido"
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
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: no prebuilt binary and cargo not found." >&2
        echo "install rust (https://rustup.rs) and re-run, or download a binary from" >&2
        echo "https://github.com/$REPO/releases" >&2
        exit 1
    fi
    cargo install --git "https://github.com/$REPO" --locked
}

main "$@"
