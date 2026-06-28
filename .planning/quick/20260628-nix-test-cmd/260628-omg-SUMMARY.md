---
quick_id: 260628-omg
slug: nix-test-cmd
date: 2026-06-28
status: complete
---

# 260628-omg: Add `test` devShell function for launching KOReader with plugin

## What was built

Added a new `test()` shell function to `devShells.default.shellHook` in `flake.nix`. It is a launch-only entrypoint — identical body to the existing `dev()` — that sources `tools/run-koreader-with-plugin.sh` to open KOReader with the freshly built plugin, without first running `cargo test`. It complements the existing `dev()` / `debug()` / `cargo-test()` launch helpers.

## Diff

```diff
@@ -275,6 +275,8 @@
             cargo-test() {
               (cd "$PWD/backend" && cargo test --all) && . tools/run-koreader-with-plugin.sh
             }
+            test() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }
+
             docs() { cd "$PWD/docs" && exec mdbook serve --open; }
```

`test()` is tightly grouped with `dev() / debug() / cargo-test()` (no blank line between them), and a blank line separates it from the next concern (`docs()`) for visual grouping — consistent with the existing layout pattern.

## Verification

### 1. `nix eval` — flake parses

```text
$ nix --extra-experimental-features 'nix-command flakes' eval .#devShells.x86_64-linux.default.outPath
warning: Git tree '/home/nixos/rakuyomi-koshin' is dirty
"/nix/store/ilhr8m2n6cm4m7giw8jw2xkvpmp0jhz2-nix-shell"
```

The `outPath` was returned successfully, proving the devShell (and its `shellHook` string containing the new `test()` function) parses without Nix syntax errors. Wall time: 267.5s (one-time flake evaluation; the warning about the dirty tree is informational only — the eval itself returned a valid store path).

### 2. `test()` definition exists exactly once

```text
$ grep -nE '^[[:space:]]+test\(\) ' flake.nix
278:            test() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }
```

Exactly one `test()` function definition at line 278. (The plan's literal `^test()` regex was a no-op here because every `shellHook` function is indented 12 spaces inside the Nix multi-line string — `^[[:space:]]+test()` is the equivalent match against the actual layout.)

### 3. No `cargo` invocation in `test()` body

```text
$ sed -n '278p' flake.nix | grep -c cargo
0
```

The new function launches KOReader without running `cargo test` (or any other cargo subcommand) — matches the plan's "no `cargo test` step" requirement.

### 4. Structural parity with `dev()`

```text
$ grep -nE '^[[:space:]]+(dev|test)\(\)' flake.nix
273:            dev() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }
278:            test() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }
```

`test()` and `dev()` are structurally identical (same body, same `; }` terminator, same indentation), confirming parity.

## Notes

- **Verification regex adjustment:** The plan's verification step `grep -c '^test()'` returns 0 in this file because every shellHook function is indented 12 spaces inside the Nix multi-line string. The semantic check is the same — exactly one `test()` function definition exists — and `^[[:space:]]+test() ` returns 1, as expected. No change to the underlying requirement.
- **Layout decision:** The plan's body section said both "ensure a blank line between `test()` and `docs()`" and "tightly grouped with the existing three". Both are honored: no blank line between `cargo-test()` and `test()` (tight group), and a blank line between `test()` and `docs()` (visual separation from the next concern). This matches the existing pattern where the `test-e2e` group also has a blank line before the activation `echo`.
- **Commit scope:** Only `flake.nix` was staged and committed. No docs artifacts (`SUMMARY.md`, `STATE.md`, `PLAN.md`) were committed per the quick-task workflow — those are the orchestrator's responsibility.

## Commit

```text
6950eb2 feat(flake): add `test` devShell function for launching KOReader with plugin
```
