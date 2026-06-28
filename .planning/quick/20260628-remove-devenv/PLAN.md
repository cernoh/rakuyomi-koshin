# Remove devenv, use flake devShell directly

**Problem:** `devenv` doesn't work on the user's system. The project uses it as a layer over Nix flakes, adding complexity and a separate lock file.

**Solution:** Remove `devenv.nix`, `devenv.yaml`, `devenv.lock`. Add `devShells.default` to `flake.nix` with the same packages and scripts. Switch `.envrc.dist` to `use flake`.

**Design:**
- `devShells.default` in `flake.nix` replaces `devenv.nix` functionality:
  - Packages: koreader, cargo-debugger, Rust 1.95.0, lua-ls, busted, mdbook, python, sqlx-cli, etc.
  - `.cargo/config.toml` and `.luarc.json` generated via shellHook
  - All shell scripts (check-format, dev, debug, test-*, etc.) as shell functions
- `.envrc.dist` uses `use flake` to load the devShell via direnv
- All references in docs, AGENTS.md, and .planning/codebase/ updated

**Verification:** `nix flake show` eval succeeds, devShell evaluates as `nix-shell` with Rust 1.95.0.
