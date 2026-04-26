#!/bin/bash
set -euo pipefail

REPO="geronimo-iia/llm-wiki"
BINARY="llm-wiki"
INSTALL_DIR="${LLM_WIKI_INSTALL_DIR:-/usr/local/bin}"

# ── Colors ─────────────────────────────────────────────────────────────────────

red() { printf "\033[31m%s\033[0m\n" "$1"; }
green() { printf "\033[32m%s\033[0m\n" "$1"; }
dim() { printf "\033[2m%s\033[0m\n" "$1"; }

# ── Prerequisites ──────────────────────────────────────────────────────────────

check_prereqs() {
    if ! command -v git &>/dev/null; then
        red "error: git is required but not installed"
        echo "Install git: https://git-scm.com/downloads"
        exit 1
    fi

    if ! command -v curl &>/dev/null; then
        red "error: curl is required but not installed"
        exit 1
    fi
}

# ── Platform detection ─────────────────────────────────────────────────────────

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="unknown-linux-gnu" ;;
        Darwin*) os="apple-darwin" ;;
        *)
            red "error: unsupported OS: $(uname -s)"
            echo "Use 'cargo install llm-wiki-engine' instead"
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)
            red "error: unsupported architecture: $(uname -m)"
            echo "Use 'cargo install llm-wiki-engine' instead"
            exit 1
            ;;
    esac

    TARGET="${arch}-${os}"
}

# ── Version ────────────────────────────────────────────────────────────────────

get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    VERSION=$(curl -fsSL "$url" | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        red "error: could not determine latest version"
        exit 1
    fi
}

# ── Download and install ───────────────────────────────────────────────────────

install() {
    local url="https://github.com/${REPO}/releases/download/v${VERSION}/${TARGET}.tar.gz"
    local tmpdir
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    echo "Installing ${BINARY} v${VERSION} (${TARGET})"
    dim "  downloading ${url}"

    curl -fsSL "$url" -o "${tmpdir}/archive.tar.gz"
    tar xzf "${tmpdir}/archive.tar.gz" -C "$tmpdir"

    if [ ! -f "${tmpdir}/${BINARY}" ]; then
        red "error: binary not found in archive"
        exit 1
    fi

    chmod +x "${tmpdir}/${BINARY}"

    if [ -w "$INSTALL_DIR" ]; then
        mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    else
        dim "  installing to ${INSTALL_DIR} (requires sudo)"
        sudo mv "${tmpdir}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
    fi
}

# ── Verify ─────────────────────────────────────────────────────────────────────

verify() {
    if command -v "$BINARY" &>/dev/null; then
        green "✓ ${BINARY} v${VERSION} installed to ${INSTALL_DIR}/${BINARY}"
        dim "  $($BINARY --version)"
    else
        echo ""
        echo "${BINARY} installed to ${INSTALL_DIR}/${BINARY}"
        echo "but it's not on your PATH. Add this to your shell profile:"
        echo ""
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi
}

# ── Main ───────────────────────────────────────────────────────────────────────

main() {
    check_prereqs
    detect_platform
    get_latest_version
    install
    verify
}

main
