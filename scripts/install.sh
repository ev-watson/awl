#!/usr/bin/env bash
set -euo pipefail

REPO="ev-watson/awl"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
VERSION="${AWL_VERSION:-latest}"

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux) os="unknown-linux-gnu" ;;
        Darwin) os="apple-darwin" ;;
        *)
            echo "Unsupported operating system: $os" >&2
            echo "Build from source instead: cargo install --git https://github.com/$REPO awl --locked" >&2
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64)
            if [ "$os" = "unknown-linux-gnu" ]; then
                echo "Prebuilt Linux ARM releases are not published yet." >&2
                echo "Build from source instead: cargo install --git https://github.com/$REPO awl --locked" >&2
                exit 1
            fi
            arch="aarch64"
            ;;
        *)
            echo "Unsupported architecture: $arch" >&2
            echo "Build from source instead: cargo install --git https://github.com/$REPO awl --locked" >&2
            exit 1
            ;;
    esac

    printf '%s-%s\n' "$arch" "$os"
}

download_url() {
    local target asset tag
    target="$1"
    asset="awl-${target}.tar.gz"

    if [ "$VERSION" = "latest" ]; then
        printf 'https://github.com/%s/releases/latest/download/%s\n' "$REPO" "$asset"
        return
    fi

    tag="$VERSION"
    case "$tag" in
        v*) ;;
        *) tag="v$tag" ;;
    esac
    printf 'https://github.com/%s/releases/download/%s/%s\n' "$REPO" "$tag" "$asset"
}

main() {
    local target url checksum_url tmpdir archive checksum_file expected actual
    need_cmd curl
    need_cmd tar
    need_cmd install
    target="$(detect_target)"
    url="$(download_url "$target")"
    checksum_url="${url}.sha256"
    tmpdir="$(mktemp -d)"
    archive="$tmpdir/awl.tar.gz"
    checksum_file="$tmpdir/awl.tar.gz.sha256"

    trap 'rm -rf "$tmpdir"' EXIT

    mkdir -p "$BIN_DIR"

    echo "Downloading $url"
    curl -fsSL "$url" -o "$archive"
    if command -v shasum >/dev/null 2>&1; then
        curl -fsSL "$checksum_url" -o "$checksum_file"
        expected="$(tr -d '\n\r' < "$checksum_file")"
        actual="$(shasum -a 256 "$archive" | awk '{print $1}')"
        if [ "$expected" != "$actual" ]; then
            echo "Checksum verification failed for $url" >&2
            exit 1
        fi
    fi

    tar -xzf "$archive" -C "$tmpdir"
    install -m 755 "$tmpdir/awl" "$BIN_DIR/awl"

    echo "Installed awl to $BIN_DIR/awl"
    if ! command -v ollama >/dev/null 2>&1; then
        echo
        echo "Ollama was not found in PATH."
        echo "Install it from https://ollama.com/download before running awl."
    fi

    echo
    echo "Next steps:"
    echo "  ollama serve"
    echo "  awl init --profile lite --no-check"
    echo "  ollama pull qwen2.5-coder:7b-instruct-q4_K_M"
    echo "  ollama pull qwen2.5-coder:3b-instruct-q4_K_M"
    echo "  awl doctor"

    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *)
            echo
            echo "Add $BIN_DIR to your PATH if it is not already there."
            ;;
    esac
}

main "$@"
