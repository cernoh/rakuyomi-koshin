#!/usr/bin/env bash
# Launches KOReader with the rakuyomi plugin installed for development/testing.
#
# - The server is started via `cargo run` (recompiles on restart when source
#   changes). With --debug, the server is launched via `cargo debugger`
#   (requires `cargo install cargo-debugger`).
# - uds_http_request and cbz_metadata_reader are pre-built once and copied into
#   the plugin directory. Rebuild them manually with:
#     cargo build -p uds_http_request -p cbz_metadata_reader
#   if you change their source.
#
# Designed to work both inside the flake dev shell (`nix develop` or direnv)
# and on a plain machine with cargo + KOReader installed (e.g. via the
# upstream .deb). Override the install path with KOREADER_DATA_DIR if KOReader
# is configured to use a non-default data directory.

set -euo pipefail

REPO_ROOT="${RAKUYOMI_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
BACKEND="$REPO_ROOT/backend"
PLUGIN_SRC="$REPO_ROOT/frontend/rakuyomi.koplugin"

# Pick up cargo from rustup if it isn't on PATH yet.
if ! command -v cargo >/dev/null 2>&1; then
    if [[ -f "$HOME/.cargo/env" ]]; then
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    fi
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: cargo not found." >&2
        echo "  Install rustup (https://rustup.rs) or run inside 'nix develop'." >&2
        exit 1
    fi
fi

# Locate the KOReader binary: PATH first, then a couple of common system
# install locations from the upstream .deb / AppImage.
KOREADER_BIN=""
if path_koreader="$(command -v koreader 2>/dev/null)"; then
    KOREADER_BIN="$path_koreader"
else
    for candidate in /usr/bin/koreader /usr/local/bin/koreader /opt/koreader/bin/koreader; do
        if [[ -x "$candidate" ]]; then
            KOREADER_BIN="$candidate"
            break
        fi
    done
fi
if [[ -z "$KOREADER_BIN" ]]; then
    echo "error: koreader not found." >&2
    echo "  Install KOReader (https://koreader.rocks) or run inside 'nix develop'." >&2
    exit 1
fi

# Resolve the plugin install directory from KOReader's data dir. On Linux
# datastorage:getFullDataDir() defaults to $XDG_CONFIG_HOME/koreader, i.e.
# ~/.config/koreader for a typical user.
KOREADER_DATA_DIR="${KOREADER_DATA_DIR:-$HOME/.config/koreader}"
PLUGIN_DEST="$KOREADER_DATA_DIR/plugins/rakuyomi.koplugin"

echo "==> Installing plugin to $PLUGIN_DEST"
mkdir -p "$(dirname "$PLUGIN_DEST")"
rm -rf "$PLUGIN_DEST"
mkdir -p "$PLUGIN_DEST"
# Skip *_spec.lua test files — they pull in busted at load time on KOReader.
rsync -a --exclude='*_spec.lua' "$PLUGIN_SRC/" "$PLUGIN_DEST/"

echo "==> Building uds_http_request and cbz_metadata_reader"
cargo build --manifest-path "$BACKEND/Cargo.toml" \
    -p uds_http_request -p cbz_metadata_reader -q
cp -f "$BACKEND/target/debug/uds_http_request" "$PLUGIN_DEST/uds_http_request"
cp -f "$BACKEND/target/debug/cbz_metadata_reader" "$PLUGIN_DEST/cbz_metadata_reader"

# The server is started by the plugin via fork+execl, so it inherits these
# env vars cleanly. `cargo run` gives hot recompilation on server source
# changes.
if [[ "${1:-}" == "--debug" ]]; then
    if ! command -v cargo-debugger >/dev/null 2>&1; then
        echo "error: --debug requires cargo-debugger (cargo install cargo-debugger)." >&2
        exit 1
    fi
    export RAKUYOMI_SERVER_COMMAND_OVERRIDE="$(command -v cargo) debugger --manifest-path $BACKEND/Cargo.toml -p server --"
else
    export RAKUYOMI_SERVER_COMMAND_OVERRIDE="$(command -v cargo) run --manifest-path $BACKEND/Cargo.toml -p server --"
fi
export RAKUYOMI_SERVER_WORKING_DIRECTORY="$REPO_ROOT"
export RAKUYOMI_SERVER_STARTUP_TIMEOUT="${RAKUYOMI_SERVER_STARTUP_TIMEOUT:-600}"

# uds_http_request and cbz_metadata_reader are invoked through io.popen /
# execute_binary_fast. The env overrides point at the freshly built binaries
# inside the plugin dir.
export RAKUYOMI_UDS_HTTP_REQUEST_COMMAND_OVERRIDE="$PLUGIN_DEST/uds_http_request"
export RAKUYOMI_UDS_HTTP_REQUEST_WORKING_DIRECTORY="$REPO_ROOT"
export RAKUYOMI_CBZ_METADATA_READER_COMMAND_OVERRIDE="$PLUGIN_DEST/cbz_metadata_reader"
export RAKUYOMI_CBZ_METADATA_READER_WORKING_DIRECTORY="$REPO_ROOT"

echo "==> Launching $KOREADER_BIN"
exec "$KOREADER_BIN" "$HOME"
