---
quick_id: 20260629-fish-dev-launch-all
slug: fish-dev-launch-all
date: 2026-06-29
status: planned
---

# 20260629-fish-dev-launch-all: Migrate all devShell helpers to fish-visible binaries

## Context

The previous quick task (`fish-dev-launch`, commit `529e8e3`) migrated
`dev` and `debug` to `writeShellScriptBin` binaries so they work in
fish+direnv. The same root cause — bash function definitions in
`devShells.default.shellHook` are invisible in fish under `use flake`
— applies to every other helper in that block. The user is on
fish+direnv, so the rest are also broken there.

Two additional sub-issues:

- **`dev-linux` / `dev-macos`** — the previous task could not wrap
  these as `writeShellScriptBin` because their `*.sh` scripts do
  `BASH_SOURCE[0]`-based self-path resolution:
  `REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"`.
  When the script is copied into `/nix/store/.../dev-linux.sh` (via
  Nix path interpolation), `BASH_SOURCE[0]` resolves to the store
  path and `REPO_ROOT` becomes `/nix/store/.../dev-linux`'s parent,
  not the repo. Wrapping requires the wrapper to pass the real repo
  root to the script via an env-var override.

## Goal

Make every user-facing devShell helper work in fish+direnv, by
exposing each as a real binary on PATH (same pattern as
`fish-dev-launch`).

## Approach

### A. Add `RAKUYOMI_REPO_ROOT` env-var override to the two scripts

`tools/dev-linux.sh` and `tools/dev-macos.sh` each compute
`REPO_ROOT` via `BASH_SOURCE[0]`. Add a one-line override: if
`RAKUYOMI_REPO_ROOT` is already set (i.e. a wrapper supplied it),
use it; otherwise fall back to the existing derivation. Purely
additive — direct invocation (`bash tools/dev-linux.sh` from the
repo) keeps the previous behavior.

```bash
REPO_ROOT="${RAKUYOMI_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
```

### B. Add 13 new `writeShellScriptBin` bindings in `flake.nix`

Eleven existing shellHook helpers + `dev-linux` + `dev-macos`.
Each wrapper resolves the repo root via
`git rev-parse --show-toplevel` and `exec`s the appropriate
command. Naming follows the previous task: `rakuyomi<Name>` Nix
binding, binary name matches the function name.

| Nix binding | Binary | Body |
|---|---|---|
| `rakuyomiCargoTest` | `cargo-test` | `cargo test --all` then `exec bash tools/run-koreader-with-plugin.sh` |
| `rakuyomiCheckFormat` | `check-format` | `cd backend && exec cargo fmt --check` |
| `rakuyomiCheckLint` | `check-lint` | `cd backend && cargo clippy -- -D warnings`; then `cd repo && python3 ci/lua-language-server-check.py frontend/` (no chaining — matches original function behavior) |
| `rakuyomiFixRustFormat` | `fix-rust-format` | `cd backend && exec cargo fmt --all` |
| `rakuyomiFixRustLint` | `fix-rust-lint` | `cd backend && exec cargo clippy --fix --allow-dirty -- -D warnings` |
| `rakuyomiDocs` | `docs` | `cd docs && exec mdbook serve --open` |
| `rakuyomiPrepareSqlQueries` | `prepare-sql-queries` | `cd repo && exec bash tools/prepare-sqlx-queries.sh` |
| `rakuyomiRemoteInstall` | `remote-install` | `cd repo && exec python3 tools/install-into-remote-koreader.py "$@"` |
| `rakuyomiRemoteSsh` | `remote-ssh` | `exec sshpass -p "" ssh -p "$REMOTE_KOREADER_SSH_PORT" -o StrictHostKeyChecking=no "root@$REMOTE_KOREADER_HOST" "$@"` (no `cd` needed — purely remote) |
| `rakuyomiTestFrontend` | `test-frontend` | `cd repo && exec busted -C frontend/rakuyomi.koplugin .` |
| `rakuyomiTestE2e` | `test-e2e` | `cd e2e-tests && poetry env use "$(which python)" && poetry install --no-root && exec poetry run pytest "$@"` |
| `rakuyomiDevLinux` | `dev-linux` | `cd repo && exec env RAKUYOMI_REPO_ROOT="$(pwd)" bash tools/dev-linux.sh "$@"` |
| `rakuyomiDevMacos` | `dev-macos` | `cd repo && exec env RAKUYOMI_REPO_ROOT="$(pwd)" bash tools/dev-macos.sh "$@"` |

Add all 13 to `devShells.default.nativeBuildInputs` (alongside the
existing `rakuyomiDev` / `rakuyomiDebug`).

### C. Don't touch the existing shellHook bash functions

Same rationale as the previous task: in `nix develop` interactive
bash, the function takes precedence over the binary on PATH. In
fish (and any non-interactive shellHook eval), the binary is
the only thing visible. Removing the functions would also conflict
with the user's in-progress uncommitted `dev-linux()` / `test()`
hunk in the same heredoc.

## Out of scope

- The `rakuyomiRemoteSsh` wrapper does not have an explicit
  `set -e` and does not need a `cd` (it's a single remote
  command). Verified against the original `remote-ssh()` bash
  function body.
- The `rakuyomiCheckLint` wrapper does not chain the two commands
  (no `&&` between them) — matches the original `check-lint()`
  bash function which runs the lua check regardless of clippy's
  exit status.
- Wrapping the launch helpers at a deeper level (e.g. re-doing
  `tools/run-koreader-with-plugin.sh` as a Nix derivation). Out
  of scope; current approach reuses the existing scripts as-is.

## Change

Three files:

- `flake.nix` — 13 new `let` bindings + 13 lines added to
  `nativeBuildInputs`.
- `tools/dev-linux.sh` — 1-line change to the `REPO_ROOT` line
  to honor `RAKUYOMI_REPO_ROOT` override.
- `tools/dev-macos.sh` — same 1-line change as `dev-linux.sh`.

## Verification

- `nix-instantiate --parse flake.nix` → exit 0
- `nix eval .#devShells.x86_64-linux.default.outPath` → store path
- `nix develop -c fish -c 'for b in cargo-test check-format check-lint fix-rust-format fix-rust-lint docs prepare-sql-queries remote-install remote-ssh test-frontend test-e2e dev-linux dev-macos; do command -v "$b" || echo "MISSING: $b"; done'`
  → all 13 resolve to `/nix/store/.../bin/<name>` (the
  "MISSING" sentinel must not appear)
- `grep -c RAKUYOMI_REPO_ROOT tools/dev-linux.sh tools/dev-macos.sh`
  → 1 match per file (the override line)
- Backward compat: original shellHook bash function bodies
  remain in `flake.nix` and still take precedence in
  `nix develop` interactive bash (sanity check, not a hard
  gate)

## Commit

Two commits:

1. **Implementation** (single atomic commit):
   ```
   feat(flake,tools): expose all devShell helpers as binaries for fish
   ```
   Files staged: `flake.nix`, `tools/dev-linux.sh`,
   `tools/dev-macos.sh`. The user's in-progress uncommitted
   changes (`M flake.nix` shellHook `dev-linux`/`test` hunk,
   `M tools/AGENTS.md`) are left untouched.

2. **Docs** (separate commit, follows project pattern):
   ```
   docs(quick-20260629-fish-dev-launch-all): migrate all devShell helpers
   ```
   Files staged: this `PLAN.md`, the eventual `SUMMARY.md`,
   and the new row in `STATE.md`.
