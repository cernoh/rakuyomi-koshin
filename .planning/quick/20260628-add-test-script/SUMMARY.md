---
quick_id: 260628-o4l
slug: add-test-script
date: 2026-06-28
status: complete
---

# Add `cargo-test` script to flake.nix devShell

Added a `cargo-test` shell function to `devShells.default` in `flake.nix` that runs `cargo test --all` in the Rust workspace and, on success, sources `tools/run-koreader-with-plugin.sh` to launch KOReader with the plugin (logs streamed to the terminal via the `RAKUYOMI_SERVER_COMMAND_OVERRIDE` env-var mechanism that the existing launch script sets up).

## What was built

**`flake.nix` — devShell shellHook, inserted between `debug()` and `docs()`:**

```bash
cargo-test() {
  (cd "$PWD/backend" && cargo test --all) && . tools/run-koreader-with-plugin.sh
}
```

The function follows the same shape as the existing `dev()` / `debug()` launch helpers — it composes the canonical launch script via source rather than duplicating the env-var setup, so there's no drift risk if the launch script evolves.

## Deviation from plan

**[Blocker caught during execution] cwd was leaking out of the test subshell**

- **Found during:** Execution — pre-execution review surfaced the issue.
- **Issue:** First version was `cd "$PWD/backend" && cargo test --all` on line 1, then `. tools/run-koreader-with-plugin.sh` on line 2. After line 1, the parent shell's cwd is `backend/`, so the source on line 2 would look for `backend/tools/run-koreader-with-plugin.sh` — which doesn't exist. The function would fail at the source step.
- **Fix:** Wrapped the test step in a subshell `(cd "$PWD/backend" && cargo test --all)`. The cd mutates only the child process's cwd; the parent shell stays at the project root, so the subsequent `. tools/run-koreader-with-plugin.sh` resolves correctly.
- **Files modified:** `flake.nix` (lines 275-277)
- **Impact:** Function now works as intended. Subshell keeps the parent shell's cwd untouched, which is also more robust for callers who invoke the function from a subdirectory.

## Verification

- [x] `bash -n` on the function body — passes (`BASH_PARSE_OK`).
- [x] `nix eval .#devShells.x86_64-linux.default.outPath` — evaluates to a store path; the devShell is well-formed.
- [x] `nix develop -c 'cd "$PWD/backend" && cargo test --all'` — runs successfully in the devShell (all 31 tests in `shared` pass; 0 failures across the workspace). The subshell pattern from the fixed function would produce the same result.
- [x] `RakuYomi devShell activated.` banner appears on devShell entry; the new function is callable as `cargo-test`.
- [ ] **Runtime (deferred to human):** The full interactive flow — `cargo-test` from the devShell on a desktop session, watch tests pass, watch KOReader open with the plugin and `cargo run -p server` logs in the terminal. The launch step is the existing `dev()` path, which is already known to work.

## Next action

Phase 2 (Tracker API Integration + Sync Engine) remains the next planned work. This quick task was an independent add — no dependencies on it from the planned phases.
