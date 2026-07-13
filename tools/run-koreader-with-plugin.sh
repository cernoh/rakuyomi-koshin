#!/usr/bin/env bash
# Launches KOReader with the rakuyomi plugin and a pre-built Rust server.
#
# The plugin uses fork+execl to start the server. We point the override
# env vars at the binary paths in backend/target/debug so the koreader
# process can run them directly. (We cannot use `cargo run` here: the
# koreader process started by `nix run .#rakuyomi.koreader-with-plugin`
# inherits a stripped env without the Rust toolchain, so `cargo run`
# would fail with `rustc -vV: No such file or directory` and the server
# would never start.)
#
# The `build_if_stale` helper below checks source mtimes vs the binary
# and rebuilds automatically when sources are newer. Just restart
# KOReader after making server changes — no manual `cargo build` needed.
#
# --debug: the original `cargo debugger --manifest-path ... -p server --`
# path has the same rustc-on-PATH problem as `cargo run` — `cargo
# debugger` shells out to `rust-gdb`, which needs both `rustc` and a
# system debugger (`gdb`/`lldb`) on PATH. Neither is in the koreader
# env, and the dev shell also lacks `gdb`/`lldb`, so the original
# `--debug` was already broken under `nix run .#rakuyomi.koreader-with-plugin`.
# Falling back to the pre-built binary for now: the server starts, and
# you can attach a debugger manually from another terminal, e.g.
#   gdb -p $(pgrep -f target/debug/server)
# A full fix (preserve the `cargo debugger` workflow) needs `gdb` in
# the dev shell and `nix develop -c koreader ...` for the --debug
# launch path; out of scope for this fix.

set -e

REPO_ROOT="$(git rev-parse --show-toplevel)"
BACKEND="$REPO_ROOT/backend"
SERVER_BIN="$BACKEND/target/debug/server"
UDS_BIN="$BACKEND/target/debug/uds_http_request"
CBZ_BIN="$BACKEND/target/debug/cbz_metadata_reader"

build_if_stale() {
    local bin_path="$1"
    local pkg="$2"
    if [[ ! -x "$bin_path" ]] || [[ $(find "$BACKEND/$pkg/src" -newer "$bin_path" -print -quit 2>/dev/null) ]]; then
        echo "==> Building $pkg (target/debug/$(basename "$bin_path"))..."
        cargo build --manifest-path "$BACKEND/Cargo.toml" -p "$pkg"
    fi
}

build_if_stale "$SERVER_BIN" server
build_if_stale "$UDS_BIN" uds_http_request
build_if_stale "$CBZ_BIN" cbz_metadata_reader

export RAKUYOMI_SERVER_COMMAND_OVERRIDE="$SERVER_BIN"

export RAKUYOMI_SERVER_WORKING_DIRECTORY="$REPO_ROOT"
[ -z "${RAKUYOMI_SERVER_STARTUP_TIMEOUT+x}" ] && export RAKUYOMI_SERVER_STARTUP_TIMEOUT="600"

export RAKUYOMI_UDS_HTTP_REQUEST_COMMAND_OVERRIDE="$UDS_BIN"
export RAKUYOMI_UDS_HTTP_REQUEST_WORKING_DIRECTORY="$REPO_ROOT"

export RAKUYOMI_CBZ_METADATA_READER_COMMAND_OVERRIDE="$CBZ_BIN"
export RAKUYOMI_CBZ_METADATA_READER_WORKING_DIRECTORY="$REPO_ROOT"

exec nix run .#rakuyomi.koreader-with-plugin -- "$HOME"
