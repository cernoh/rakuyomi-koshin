---
phase: quick/nix-test-cmd
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - flake.nix
autonomous: true
requirements: []
user_setup: []

must_haves:
  truths:
    - "A `test` shell function is callable from the devShell"
    - "`test` sources `tools/run-koreader-with-plugin.sh` (same as `dev`)"
    - "`test` does NOT run `cargo test`"
    - "`dev`, `debug`, `test`, `cargo-test` are all present and consistent"
  artifacts:
    - path: "flake.nix"
      provides: "test() function in shellHook"
      contains: "test()"
  key_links:
    - from: "test() function definition"
      to: "tools/run-koreader-with-plugin.sh"
      via: ". tools/run-koreader-with-plugin.sh"
      pattern: 'test\(\)'
---

<objective>
Add a `test()` shell function to `devShells.default.shellHook` in `flake.nix` that opens KOReader with the plugin installed, without running tests first.

Purpose: Provide a light launch-only entrypoint for manual plugin testing from the devShell, complementing the existing `dev()` (launch), `debug()` (launch with --debug), and `cargo-test()` (test-then-launch) functions.

Output: One `test()` function in `flake.nix` shellHook, adjacent to `cargo-test()`.
</objective>

<execution_context>
Uses the standard GSD execute-plan workflow. Single task, wave 1. No discovery needed — all context is in the existing shellHook.
</execution_context>

<context>
@./flake.nix:273-277
tools/run-koreader-with-plugin.sh
</context>

<tasks>

<task type="insert">
  <name>Add test() function to devShell shellHook</name>
  <files>flake.nix</files>
  <action>
Insert a `test()` function on its own line after the `cargo-test()` function (currently line 277). The function body MUST be identical to `dev()` on line 273: `test() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }`. Use a single `INS.POST 277:` edit targeting the `flake.nix` snapshot (re-read before editing). The line ends with `;` and a closing `}` — match exactly. Place the new function on the line immediately after the `cargo-test()` closing brace, and ensure a blank line exists between `test()` and the next function (`docs()` on current line 278) for visual grouping. Do NOT modify any other function.
  </action>
  <verify>
    <automated>
      nix eval .#devShells.x86_64-linux.default.outPath 2>&1 >/dev/null || nix eval .#devShells.aarch64-linux.default.outPath 2>&1 >/dev/null
    </automated>
  </verify>
  <done>
    - `nix eval .#devShells.x86_64-linux.default.outPath` (or aarch64) succeeds, proving the devShell parses without syntax errors.
    - `grep 'test()' flake.nix` returns exactly one match.
    - `grep 'cargo.test' flake.nix` returns zero matches in the `test()` function body (no accidental inclusion of test running).
    - The function follows the exact pattern of `dev()` on line 273: `test() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }`.
  </done>
</task>

</tasks>

<verification>
After the edit, verify:
1. `nix eval .#devShells.x86_64-linux.default.outPath` succeeds (flake parses).
2. `grep -c 'test()' flake.nix` returns 1 (exactly one `test()` definition).
3. The line containing `test()` has the form `test() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }` — grep with a pattern check against the `dev()` line to confirm structural parity.
4. Optional: `nix flake check --no-build` to confirm full flake integrity (not required but good hygiene; the devShell eval already catches syntax errors).
</verification>

<success_criteria>
- `test` shell function exists in `devShells.default.shellHook`.
- In a devShell session, running `test` opens KOReader with the plugin, with logs streaming to the terminal — identical behavior to `dev()`.
- No `cargo test` invocation occurs.
</success_criteria>

<output>
Create `.planning/quick/20260628-nix-test-cmd/260628-omg-SUMMARY.md` when done.
</output>
