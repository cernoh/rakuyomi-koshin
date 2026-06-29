---
quick_id: 20260629-fish-dev-launch
slug: fish-dev-launch
date: 2026-06-29
status: complete
---

# 20260629-fish-dev-launch: Make `dev` (and `debug`) work in fish+direnv

## What was built

`dev` and `debug` were bash functions defined inside
`devShells.default.shellHook` of `flake.nix`. Under `use flake` / direnv,
the shellHook is **not** evaluated in fish — direnv's `use flake`
integration only sets environment variables and adds the devShell's
`bin/` to PATH; bash function bodies from `shellHook` are not visible
in the fish shell. The result: typing `dev` in a fish direnv session
returned "Unknown command" even though the same command worked in
`nix develop` interactive bash.

This task moves `dev` and `debug` out of `shellHook` and into
`pkgs.writeShellScriptBin` derivations, then puts the resulting
binaries on `nativeBuildInputs`. The wrappers:

1. `cd` to the repo root via `git rev-parse --show-toplevel` (works
   from the repo root or any subdir).
2. `exec bash <store-path-of-tools/run-koreader-with-plugin.sh>` (the
   `--debug` arg is forwarded for the `debug` binary).

`tools/run-koreader-with-plugin.sh` is **not** modified — its content
is baked into the wrapper at Nix-eval time via the `${./tools/…}`
path interpolation.

The original `dev()` / `debug()` bash function definitions in
`shellHook` are **kept** for back-compat. In `nix develop` interactive
bash, the function takes precedence over the binary on PATH, so
behavior there is unchanged. Fish, and any direnv-activated shell,
now sees a real binary and launches KOReader normally.

## Diff

```diff
@@ -141,6 +141,21 @@
           koreaderWithRakuyomiFrontend = pkgs.callPackage ./packages/koreader.nix {
             plugins = [ pluginFolderWithoutServer ];
           };
+          # dev / debug: real binaries (not bash shellHook functions) so they
+          # work in any shell — fish, bash, zsh, nushell — under
+          # `use flake` / direnv. The shellHook `dev()` / `debug()` bash
+          # functions are kept for back-compat with bash-only users; bash
+          # prefers the function, fish has only the binary.
+          rakuyomiDev = pkgs.writeShellScriptBin "dev" ''
+            set -e
+            cd "$(git rev-parse --show-toplevel)"
+            exec bash ${./tools/run-koreader-with-plugin.sh}
+          '';
+          rakuyomiDebug = pkgs.writeShellScriptBin "debug" ''
+            set -e
+            cd "$(git rev-parse --show-toplevel)"
+            exec bash ${./tools/run-koreader-with-plugin.sh} --debug
+          '';

           pkgsDev = import nixpkgs {
             inherit system;
@@ -228,6 +243,8 @@
             fontconfig
             koreader
             cargoDebugger
+            rakuyomiDev
+            rakuyomiDebug
           ] ++ lib.optionals pkgs.stdenv.isLinux [
             mold-wrapped
           ] ++ lib.optionals pkgs.stdenv.isDarwin [
```

17 insertions, 0 deletions, 1 file (`flake.nix`).

## Verification

### 1. Flake parses

```text
$ nix-instantiate --parse /home/nixos/rakuyomi-koshin/flake.nix > /dev/null
$ echo $?
0
```

### 2. devShell evaluates

```text
$ nix --extra-experimental-features 'nix-command flakes' eval \
      .#devShells.x86_64-linux.default.outPath
warning: Git tree '/home/nixos/rakuyomi-koshin' is dirty
"/nix/store/69znnxprl4k381j9fmnqvz9slrh76rcl-nix-shell"
```

The store path is returned (the dirty-tree warning is from the
user's pre-existing uncommitted `dev-linux` work, not from this
change). Wall time: 58.88s — one-time flake evaluation.

### 3. Binaries are produced on PATH

```text
$ nix --extra-experimental-features 'nix-command flakes' develop \
      --command bash -c 'command -v dev; command -v debug'
…
/nix/store/lzlnf36s6l45sy5lkp7fk2s6hj6pnk8k-dev/bin/dev
/nix/store/mimvrd2slak72fl39ssn60jr069imfbn-debug/bin/debug
```

### 4. `dev` resolves to the binary in fish (the actual user scenario)

```text
$ nix --extra-experimental-features 'nix-command flakes' develop \
      --command fish -c 'command -v dev; command -v debug; type dev'
…
/nix/store/lzlnf36s6l45sy5lkp7fk2s6hj6pnk8k-dev/bin/dev
/nix/store/mimvrd2slak72fl39ssn60jr069imfbn-debug/bin/debug
dev is /nix/store/lzlnf36s6l45sy5lkp7fk2s6hj6pnk8k-dev/bin/dev
```

`type dev` reports the binary, not a function — proves fish has no
`dev` function in scope (the previous broken state) but the binary
is available on PATH.

### 5. Wrapper script syntax + content match the source

```text
$ bash -n /nix/store/lzlnf36s6l45sy5lkp7fk2s6hj6pnk8k-dev/bin/dev
$ echo $?
0

$ diff /nix/store/x5sy8glrsw9fff8y65fqd4ii8jd62fdm-run-koreader-with-plugin.sh \
        /home/nixos/rakuyomi-koshin/tools/run-koreader-with-plugin.sh
$ echo $?
0
```

The wrapper parses cleanly and the script content baked into the
derivation is byte-identical to the source file.

### 6. Produced wrapper content

```text
$ cat /nix/store/lzlnf36s6l45sy5lkp7fk2s6hj6pnk8k-dev/bin/dev
#!/nix/store/rlq03x4cwf8zn73hxaxnx0zn5q9kifls-bash-5.3p3/bin/bash
set -e
cd "$(git rev-parse --show-toplevel)"
exec bash /nix/store/x5sy8glrsw9fff8y65fqd4ii8jd62fdm-run-koreader-with-plugin.sh
```

## Notes

- **Back-compat with bash:** the `dev()` and `debug()` shellHook
  functions are intentionally not removed. In `nix develop`
  interactive bash, the function is defined and takes precedence
  over the binary on PATH. Removing them would touch the same
  shellHook heredoc as the user's in-progress uncommitted
  `dev-linux()` / `test()` cleanup — kept out of scope to avoid
  colliding with that work.
- **Scope was deliberately tight:** only the user-facing launch
  helpers (`dev`, `debug`) are exposed as binaries. The other 11
  fish-invisible helpers in shellHook (`cargo-test`, `check-format`,
  `check-lint`, `fix-rust-format`, `fix-rust-lint`, `docs`,
  `prepare-sql-queries`, `remote-install`, `remote-ssh`,
  `test-frontend`, `test-e2e`) have the same root cause but are not
  part of the "open KOReader with the plugin installed" use case.
  They should be migrated the same way in a follow-up.
- **Same root cause applies to `dev-linux` / `dev-macos`:** their
  scripts do `BASH_SOURCE`-based self-path resolution, so wrapping
  them via `writeShellScriptBin` needs a `RAKUYOMI_REPO_ROOT`
  env-var override thread-through. Out of scope here; flagged for
  a follow-up.
- **`nix develop -c` does not preserve shellHook functions** even
  in bash — the shellHook runs in a parent shell and the function
  definitions don't propagate into the `-c` child. This is a
  pre-existing characteristic of the devShell wiring, not caused
  by this change. Bash users who want the function semantics
  should use `nix develop` (no `-c`) or a direnv-activated
  interactive shell.
- **Commit scope:** only `flake.nix` was staged and committed
  (17 insertions, 0 deletions). The user's in-progress uncommitted
  changes (`M flake.nix` shellHook dev-linux/test hunk,
  `M tools/AGENTS.md`, `?? tools/dev-linux.sh`) were left
  untouched.

## Commit

```text
529e8e3 feat(flake): make dev and debug work in fish+direnv
```
