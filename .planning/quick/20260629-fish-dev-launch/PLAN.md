---
quick_id: 20260629-fish-dev-launch
slug: fish-dev-launch
date: 2026-06-29
status: planned
---

# 20260629-fish-dev-launch: Make `dev` (and `debug`) work in fish

## Context

The `dev()` and `debug()` launch helpers are currently defined as bash
functions in `devShells.default.shellHook` of `flake.nix` (lines 273–274):

```bash
dev()   { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }
debug() { cd "$PWD" && . tools/run-koreader-with-plugin.sh --debug; }
```

`shellHook` is bash syntax. When `use flake` activates the devShell in
**fish** (via direnv), the standard `use flake` integration sets environment
variables and adds the devShell's `bin/` to `PATH`, but bash function
definitions in `shellHook` are **not** visible in fish. The functions do not
exist in the fish shell, so `dev` and `debug` fail with `command not found`
(or, in fish, "Unknown command") — they "do nothing" from the user's POV.

This is a long-standing latent issue: every helper in that block
(`dev`, `debug`, `cargo-test`, `check-format`, `check-lint`,
`fix-rust-format`, `fix-rust-lint`, `docs`, `prepare-sql-queries`,
`remote-install`, `remote-ssh`, `test-frontend`, `test-e2e`) is
fish-invisible. The user is on fish + direnv and only needs the launch
helpers, so this task targets the user-facing launchers (`dev`, `debug`)
only.

## Goal

Expose `dev` and `debug` as real binaries on `PATH` so they work in **any
shell** — fish, bash, zsh, nushell — without requiring the consumer to read
`shellHook`. The implementation lives entirely in `flake.nix` and is a
declarative Nix expression, not a user-side fish config tweak.

## Approach

Add a `let` binding in `flake.nix` for each launch binary, using
`pkgs.writeShellScriptBin`. Each wrapper:

1. Resolves the repo root via `git rev-parse --show-toplevel` so the
   command works whether the user is in the repo root or a subdir.
2. `cd`s into it (the underlying script reads `pwd` and uses
   `backend/Cargo.toml` and `nix run .#…` with a relative path).
3. `exec`s `bash <tools/run-koreader-with-plugin.sh>` (or with `--debug` for
   the debug variant). The script keeps its existing behavior unchanged.

Add both bindings to `devShells.default.nativeBuildInputs` so they are on
`PATH` in the activated devShell.

**Do not** touch the existing bash function definitions in `shellHook` —
they are inert in fish (no-op) and keep working unchanged in bash. Removing
them is a separate cleanup that would also conflict with the user's
in-progress uncommitted work in the same heredoc.

## Out of scope

- The other 11 fish-invisible shell functions (`cargo-test`,
  `check-format`, `check-lint`, `fix-rust-format`, `fix-rust-lint`, `docs`,
  `prepare-sql-queries`, `remote-install`, `remote-ssh`, `test-frontend`,
  `test-e2e`) — not requested, will be flagged for a follow-up.
- `dev-linux` / `dev-macos` — their `*.sh` scripts do
  `BASH_SOURCE`-based self-path resolution, so wrapping them via
  `writeShellScriptBin` needs a `RAKUYOMI_REPO_ROOT` env-var override
  thread-through. Worth doing but a separate change; will be flagged.
- Cleaning up the existing bash `dev()`/`debug()` in `shellHook` — see
  approach rationale above.

## Change

`flake.nix` only:

1. In the outer `let` block (next to `koreaderWithRakuyomiFrontend`),
   add:
   ```nix
   rakuyomiDev = pkgs.writeShellScriptBin "dev" ''
     set -e
     cd "$(git rev-parse --show-toplevel)"
     exec bash ${./tools/run-koreader-with-plugin.sh}
   '';
   rakuyomiDebug = pkgs.writeShellScriptBin "debug" ''
     set -e
     cd "$(git rev-parse --show-toplevel)"
     exec bash ${./tools/run-koreader-with-plugin.sh} --debug
   '';
   ```
2. Add `rakuyomiDev` and `rakuyomiDebug` to
   `devShells.default.nativeBuildInputs`.

## Verification

- `nix --extra-experimental-features 'nix-command flakes' eval .#devShells.x86_64-linux.default.outPath`
  — must return a store path (proves the new bindings parse and the
  devShell evaluates).
- `nix --extra-experimental-features 'nix-command flakes' build .#devShells.x86_64-linux.default --no-link --print-out-paths`
  followed by
  `ls -la $(…)/bin/dev $(…)/bin/debug` — proves the binaries are produced
  with executable bits and the expected names.
- `nix develop -c bash -c 'type dev; type debug'`
  — in bash, `type` reports the function (still wins) AND
  `command -v` shows the binary. Both exist; bash prefers the function.
- `nix develop -c fish -c 'command -v dev; command -v debug'`
  — in fish, only the binary exists. `command -v` returns the store path.
  This is the actual user scenario: fish now sees `dev`.

## Commit

Single atomic commit on the current branch:

```
feat(flake): expose dev and debug as real binaries for fish+direnv
```

Files staged: `flake.nix` only (the user's pending uncommitted changes
are left untouched).
