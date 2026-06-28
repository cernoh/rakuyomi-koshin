---
status: complete
---

# Remove devenv, use flake devShell directly

Replaced `devenv` (devenv.sh) development environment with a direct `devShells.default` in `flake.nix`, loaded via `.envrc` with `use flake`.

## Changes

- **`flake.nix`** — Added `devShells.default` with all packages, scripts, and shell hooks previously in `devenv.nix`: Rust 1.95.0 via rust-overlay, koreader, cargo-debugger, lua-language-server, busted, mdbook, python313, sqlx-cli, mold-wrapped, etc. Generated `.luarc.json` and `.cargo/config.toml` automatically. All convenience scripts (check-format, check-lint, dev, debug, docs, test-frontend, test-e2e, etc.) defined as shell functions in shellHook.
- **`.envrc.dist`** — Changed from `use devenv` to `use flake` for direct flake devShell loading.
- **Deleted** `devenv.nix`, `devenv.yaml`, `devenv.lock` (156 lines of lock file).
- **`.gitignore`** — Removed `.devenv*` and `devenv.local.nix` entries.
- **`tools/prepare-sqlx-queries.sh`** — Replaced `$DEVENV_ROOT` with `$(git rev-parse --show-toplevel)`.
- **`.github/workflows/test.yml%`** — Updated stale Nix workflow from `devenv shell` to `nix develop`.
- **AGENTS.md, docs, `.planning/codebase/`** — Updated all references to `devenv` to point to the flake devShell.
- **`flake.lock`** — Updated `rust-overlay` input to latest (2026-06-28) to gain Rust 1.95.0 availability.
