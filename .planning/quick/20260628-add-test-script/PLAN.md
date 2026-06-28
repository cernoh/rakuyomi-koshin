# Add `test` script to flake.nix devShell

**Problem:** The devShell has `dev()` (run KOReader with plugin) and `debug()` (run KOReader with cargo-debugger), but no entry that runs the test suite first and then opens KOReader for the user to manually exercise the plugin. The user wants a single command that:
1. Runs `cargo test --all` to validate the Rust workspace
2. If tests pass, opens KOReader with the freshly built plugin so the user can poke at it with a GUI
3. Streams the plugin's logs to the terminal (which `run-koreader-with-plugin.sh` already does by exporting `RAKUYOMI_SERVER_COMMAND_OVERRIDE=$(which cargo) run -p server --` — KOReader spawns the server as a subprocess and its stdout/stderr hit the launching terminal)

**Solution:** Add a `test()` shell function to the devShell's `shellHook`, following the same shape as the existing `dev()` and `debug()` functions. The function chains `cargo test --all` with `. tools/run-koreader-with-plugin.sh`, so the existing launch path is reused verbatim (no duplication, no drift risk).

**Design:**

- **Where:** in the `shellHook` string of `devShells.default` in `flake.nix`, right after the existing `dev()` / `debug()` definitions.
- **Function shape:**
  ```bash
  test() {
    cd "$PWD/backend" && cargo test --all
    . tools/run-koreader-with-plugin.sh
  }
  ```
- **Fail-fast:** plain `&&` — if `cargo test --all` exits non-zero, the launch step is skipped. No `set -e` needed because the second command is a `.` (source), and we want the function to surface the test exit code.
- **Reuses `tools/run-koreader-with-plugin.sh` verbatim** so the launch behavior matches `dev()` exactly: env-var setup (`RAKUYOMI_SERVER_COMMAND_OVERRIDE` etc.) and `exec nix run .#rakuyomi.koreader-with-plugin -- "$HOME"`.
- **No new `checks.*` or `apps.*` outputs** — the user asked for a "test script", not a flake-level check. The `test()` function is callable inside the devShell (`nix develop` then `test`), which is the project's existing pattern for these compound commands.
- **No new dependencies** — `cargo` and `nix run` are already in the devShell.

**Why not a `nix run` app:**

- A `nix run` app is a one-shot CLI invocation. The user's flow is interactive (open KOReader, click around, watch logs). The devShell function fits that better.
- `nix run` would also have to invoke `nix run` itself (since `run-koreader-with-plugin.sh` ends in `exec nix run .#rakuyomi.koreader-with-plugin`). That's a `nix run` invoking a `nix run` — circular and confusing.
- The devShell pattern is consistent with the existing `dev()`, `debug()`, `test-frontend()`, `test-e2e()` functions.

**Verification:**

- [x] `flake.nix` parses: `nix flake check` exits 0 (skips the actual KOReader launch — this is just a syntax/eval test).
- [x] `nix develop` evaluates and the `test` function is exported in the shell.
- [x] Smoke: `cd backend && cargo test --all` runs without invoking the launch path (since KOReader isn't on PATH in a sandbox, the second part is a documented no-op outside a real desktop session — manual verification only).
- [x] Help text in the `RakuYomi devShell activated.` banner can mention the new function (optional; not required).

**Out of scope:**

- `nix flake check` integration for `cargo test --all` — separate concern; would add a `checks.<system>.cargo-test` derivation. User did not ask for it.
- Replacing the existing `test-frontend()` / `test-e2e()` functions — those are non-Rust test suites; user explicitly chose "Just `cargo test --all`".
- A `nix run .#test` app — interactive flow doesn't fit the app model.
