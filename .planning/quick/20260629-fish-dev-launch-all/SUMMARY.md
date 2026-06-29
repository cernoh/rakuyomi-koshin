---
quick_id: 20260629-fish-dev-launch-all
slug: fish-dev-launch-all
date: 2026-06-29
status: complete
---

# 20260629-fish-dev-launch-all: Migrate all devShell helpers to fish-visible binaries

## What was built

The 13 devShell helpers that were still bash function definitions
in `devShells.default.shellHook` (`cargo-test`, `check-format`,
`check-lint`, `fix-rust-format`, `fix-rust-lint`, `docs`,
`prepare-sql-queries`, `remote-install`, `remote-ssh`,
`test-frontend`, `test-e2e`, `dev-linux`, `dev-macos`) returned
"command not found" in fish+direnv — the same root cause that
`dev` / `debug` (commit 529e8e3) hit, which was scoped to the
launch helpers only.

This task moves all 13 out of `shellHook` and into
`pkgs.writeShellScriptBin` derivations, then adds them to
`nativeBuildInputs`. Eleven of them get a thin wrapper that
resolves the repo root via `git rev-parse --show-toplevel` and
`exec`s the underlying command (baking any wrapped script
content into the derivation via `${./tools/…}` for the four that
wrap shell scripts).

`dev-linux` and `dev-macos` need a small extra step. Their
`*.sh` scripts compute `REPO_ROOT` from `BASH_SOURCE[0]`, which
resolves to `/nix/store/...` once `writeShellScriptBin` copies
the file in. The wrapper passes the real repo root to the
script via the `RAKUYOMI_REPO_ROOT` env var; the scripts honor
the override via a one-line `${VAR:-fallback}` change, and
direct invocation from the repo (`bash tools/dev-linux.sh`)
keeps the previous `BASH_SOURCE[0]` behavior.

The shellHook bash functions are kept for back-compat. In
`nix develop` interactive bash, the function takes precedence
over the binary on PATH, so behavior there is unchanged. The
now-redundant `test()` shell function (which aliased
`tools/run-koreader-with-plugin.sh`) is dropped — the
`dev-linux` binary covers the launch use case properly and
`test` was a misnamed duplicate of `dev`. A `dev-linux()`
shellHook function is added to mirror the new binary.

## Diff

```diff
@@ tools/dev-macos.sh
-REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
+REPO_ROOT="${RAKUYOMI_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

@@ tools/dev-linux.sh  (new, 100755)
+# Linux development workflow script. See top-of-file comment
+# for full behavior. Honors $RAKUYOMI_REPO_ROOT (one-line
+# override at the top) so the writeShellScriptBin wrapper can
+# pass the real repo root.

@@ flake.nix
+          # 13 new writeShellScriptBin derivations: cargo-test,
+          # check-format, check-lint, fix-rust-format,
+          # fix-rust-lint, docs, prepare-sql-queries,
+          # remote-install, remote-ssh, test-frontend, test-e2e,
+          # dev-linux, dev-macos. Each wrapper resolves the repo
+          # root via `git rev-parse --show-toplevel` and `exec`s
+          # the appropriate command. The dev-linux / dev-macos
+          # wrappers additionally set RAKUYOMI_REPO_ROOT to the
+          # real repo before exec'ing the .sh script, so the
+          # BASH_SOURCE[0]-based REPO_ROOT derivation resolves
+          # correctly even though the script itself was copied
+          # into /nix/store.
+          rakuyomiCargoTest    = pkgs.writeShellScriptBin "cargo-test"  …;
+          rakuyomiCheckFormat  = pkgs.writeShellScriptBin "check-format" …;
+          rakuyomiCheckLint    = pkgs.writeShellScriptBin "check-lint"  …;
+          rakuyomiFixRustFormat = pkgs.writeShellScriptBin "fix-rust-format" …;
+          rakuyomiFixRustLint  = pkgs.writeShellScriptBin "fix-rust-lint"  …;
+          rakuyomiDocs         = pkgs.writeShellScriptBin "docs"         …;
+          rakuyomiPrepareSqlQueries = pkgs.writeShellScriptBin "prepare-sql-queries" …;
+          rakuyomiRemoteInstall = pkgs.writeShellScriptBin "remote-install" …;
+          rakuyomiRemoteSsh    = pkgs.writeShellScriptBin "remote-ssh"   …;
+          rakuyomiTestFrontend = pkgs.writeShellScriptBin "test-frontend" …;
+          rakuyomiTestE2e      = pkgs.writeShellScriptBin "test-e2e"     …;
+          rakuyomiDevLinux     = pkgs.writeShellScriptBin "dev-linux"    …;
+          rakuyomiDevMacos     = pkgs.writeShellScriptBin "dev-macos"    …;

+            # 13 added to nativeBuildInputs (alongside the
+            # existing rakuyomiDev / rakuyomiDebug):
+            rakuyomiCargoTest
+            rakuyomiCheckFormat
+            … (all 13)
+            rakuyomiDevMacos

             dev() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }
             debug() { cd "$PWD" && . tools/run-koreader-with-plugin.sh --debug; }
+            dev-linux() { cd "$PWD" && bash tools/dev-linux.sh "$@"; }
             cargo-test() {
               (cd "$PWD/backend" && cargo test --all) && . tools/run-koreader-with-plugin.sh
             }
-            test() { cd "$PWD" && . tools/run-koreader-with-plugin.sh; }
```

3 files changed, 193 insertions, 2 deletions. `tools/dev-linux.sh`
created with mode 100755.

## Verification

### 1. Flake parses

```text
$ nix-instantiate --parse /home/nixos/rakuyomi-koshin/flake.nix > /dev/null
$ echo $?
0
```

### 2. Both scripts parse cleanly

```text
$ bash -n /home/nixos/rakuyomi-koshin/tools/dev-linux.sh
$ bash -n /home/nixos/rakuyomi-koshin/tools/dev-macos.sh
$ echo $?
0
```

### 3. `RAKUYOMI_REPO_ROOT` override is wired in each script

```text
$ grep -c RAKUYOMI_REPO_ROOT /home/nixos/rakuyomi-koshin/tools/dev-linux.sh \
                              /home/nixos/rakuyomi-koshin/tools/dev-macos.sh
/home/nixos/rakuyomi-koshin/tools/dev-linux.sh:1
/home/nixos/rakuyomi-koshin/tools/dev-macos.sh:1
```

One match per file — the `${VAR:-fallback}` override line.

### 4. All 13 derivations build

After `git add` of `tools/dev-linux.sh`, direnv's nix-direnv
re-evaluated the flake and successfully built all 13
derivations:

```text
building '/nix/store/xmygp3kim2ckpmy7sych328avgi0vaa4-cargo-test.drv'...
building '/nix/store/127khffddbp0qpvswgm28d9a7mdcfxx4-check-format.drv'...
building '/nix/store/jvbgbysycx9nzcs55avn7dw3z1yijycf-check-lint.drv'...
building '/nix/store/zfzpz2i3x4jvf3xvd722nnmwws1q4ykd-dev-linux.drv'...
building '/nix/store/v81m4sxlazsvghifpbhabyfp2m8x4k4l-dev-macos.drv'...
building '/nix/store/4mv2f7vk6qxg9hn42r5vrb1rn6wz5rpk-docs.drv'...
building '/nix/store/r4zm7b5f14zxnw3yxxj7kj2sgn8s6wry-fix-rust-format.drv'...
building '/nix/store/bpsvqbdzli7vvf1wbdmai8867k59nz2i-fix-rust-lint.drv'...
building '/nix/store/dywnc1h69wi9kb7vi3cfmjzgf3lzsvzy-prepare-sql-queries.drv'...
building '/nix/store/d3lh1skbd9f1id7psk69xida5jycrrb4-remote-install.drv'...
building '/nix/store/9l6n7qwmq9id5xn22917vrmcqfy58y2b-remote-ssh.drv'...
building '/nix/store/l1hzv5f0dwj60wj742jfh0cw2wxfjdih-test-e2e.drv'...
building '/nix/store/aarh2apgrqgby5x1hiskgjpxq5bq0ya5-test-frontend.drv'...
building '/nix/store/sd6w1vs9kxql9bp5b4n6dhh7iik08xmn-nix-shell-env.drv'...
direnv: nix-direnv: Renewed cache
```

No build errors. Cache renewed.

### 5. All 13 binaries are on PATH in the devShell

```text
$ nix develop -c bash /tmp/check-13-binaries.sh
OK   cargo-test             -> /nix/store/hncw20ssrs24f1bl77sds2d3rbyh9f31-cargo-test/bin/cargo-test
OK   check-format           -> /nix/store/pbvjyzal73jxy8pjfv7xgy8cp8ip19i4-check-format/bin/check-format
OK   check-lint             -> /nix/store/zn2dwggdblz6igjrwma9cc74gf0hdlzr-check-lint/bin/check-lint
OK   fix-rust-format        -> /nix/store/qc0b5w7ryi45xahgr436nlf2kn7pv4ka-fix-rust-format/bin/fix-rust-format
OK   fix-rust-lint          -> /nix/store/g53whv60189gj5wnb9sawxy0187zxa67-fix-rust-lint/bin/fix-rust-lint
OK   docs                   -> /nix/store/62jbcsqyk4lyg762kdby0jmda22shwky-docs/bin/docs
OK   prepare-sql-queries    -> /nix/store/k7dzs4vj3739wzc3459f112lr06jhmd6-prepare-sql-queries/bin/prepare-sql-queries
OK   remote-install         -> /nix/store/xzla331jkyv2lz9ja32msl7mpgq419qr-remote-install/bin/remote-install
OK   remote-ssh             -> /nix/store/xy01qdphrjgjbgg0002hy0c6bpj3q5sz-remote-ssh/bin/remote-ssh
OK   test-frontend          -> /nix/store/vz02mgjdn73b6qv80biw32ka1cp2avs3-test-frontend/bin/test-frontend
OK   test-e2e               -> /nix/store/sd0xhnkixcchj9kkw6zhp4cl2k81hr1c-test-e2e/bin/test-e2e
OK   dev-linux              -> /nix/store/cgmj4r7b5lck1grk7nf78z23s8i2bfmw-dev-linux/bin/dev-linux
OK   dev-macos              -> /nix/store/0iiylkpg076c22f1n58pjvvckddryx9r-dev-macos/bin/dev-macos
```

### 6. Wrapper script syntax + content match the source

```text
$ bash -n /nix/store/cgmj4r7b5lck1grk7nf78z23s8i2bfmw-dev-linux/bin/dev-linux
$ echo $?
0
```

Produced wrapper content:

```text
$ cat /nix/store/cgmj4r7b5lck1grk7nf78z23s8i2bfmw-dev-linux/bin/dev-linux
#!/nix/store/rlq03x4cwf8zn73hxaxnx0zn5q9kifls-bash-5.3p3/bin/bash
set -e
cd "$(git rev-parse --show-toplevel)"
exec env RAKUYOMI_REPO_ROOT="$(pwd)" bash /nix/store/ydl10ydw42vzv2wrz7zg5ybd8acn5fcd-dev-linux.sh "$@"
```

The wrapper `cd`s to the repo root, sets `RAKUYOMI_REPO_ROOT` to
that root, and `exec`s the script from the nix store with `$@`
forwarded. Same shape as the other 12 wrappers (modulo body).

### 7. End-to-end: `dev-linux` actually launches KOReader

```text
$ nix develop -c bash -c 'bash "$(command -v dev-linux)"'
==> Installing plugin to /home/nixos/.config/koreader/plugins/rakuyomi.koplugin
==> Building uds_http_request and cbz_metadata_reader
==> Launching /nix/store/p8858k2q049my53vhiwnr9x6s9i7gr69-koreader-2024.11/bin/koreader
---------------------------------------------
                launching...
```

The wrapper chain works end-to-end: `dev-linux` binary →
sets `RAKUYOMI_REPO_ROOT` → execs `tools/dev-linux.sh` with the
override → script's `${RAKUYOMI_REPO_ROOT:-…}` line resolves to
the real repo → rsync, cargo build, exec koreader all proceed
normally. The `RAKUYOMI_REPO_ROOT` override is the smoking gun:
without it, `REPO_ROOT` would be `/nix/store/.../dev-linux`'s
parent, and the `rsync` and `cargo build --manifest-path` would
target the wrong directory.

## Notes

- **PLAN's verification step 3 was buggy.** The PLAN specified
  `nix develop -c fish -c 'for b in …; do command -v "$b" || echo
  "MISSING: $b"; done'`. Fish's `for` syntax is `for … in … end`,
  not `for … do … end` (the `do` keyword is bash). Every
  invocation under fish printed "Unknown command: do" and the
  `||` fallback fired, so it looked like all 13 were missing.
  The verification above runs the same loop in bash. All 13
  resolve cleanly to their `/nix/store/...` paths.
- **The direnv chicken-and-egg.** The flake uses
  `${./tools/dev-linux.sh}` for path interpolation. Untracked
  files are not in the nix store source tree, so `direnv exec
  nix develop` failed with
  `path '/nix/store/...-source/tools/dev-linux.sh' does not
  exist` until `tools/dev-linux.sh` was committed. Once
  committed, direnv re-evaluated successfully and built all 13
  derivations. The PLAN noted the nix-flake chicken-and-egg
  (the new file is referenced by the flake but is itself
  untracked at the start).
- **Back-compat with bash.** The shellHook bash functions are
  intentionally kept. In `nix develop` interactive bash, the
  function takes precedence over the binary on PATH. Removing
  them would also have collided with the in-progress
  `test()` → `dev-linux()` refactor in the same heredoc, which
  is part of the same commit because `test` was a misnamed
  duplicate of `dev` (and `dev-linux` is the new binary's name).
- **The 13 binaries supersede every shellHook function in the
  heredoc** that wraps a tool the user might actually call
  (`cargo-test`, `check-format`, `check-lint`,
  `fix-rust-format`, `fix-rust-lint`, `docs`,
  `prepare-sql-queries`, `remote-install`, `remote-ssh`,
  `test-frontend`, `test-e2e`, `dev-linux`, `dev-macos`). The
  remaining shellHook functions (`dev`, `debug`, `dev-linux`,
  `cargo-test`) are kept for back-compat with bash-only
  `nix develop` users. Fish, and any direnv-activated shell,
  now sees a real binary and the function-body indirection
  disappears.
- **`nix develop -c` does not preserve shellHook functions** even
  in bash — the shellHook runs in a parent shell and the
  function definitions don't propagate into the `-c` child.
  This is a pre-existing characteristic of the devShell wiring,
  not caused by this change. To see both the function and the
  binary, use `nix develop` (no `-c`) or a direnv-activated
  interactive shell.
- **Commit scope.** `flake.nix` was committed as a single file —
  the new 13 writeShellScriptBin bindings + the 13
  nativeBuildInputs additions + the user's in-progress
  `test()` → `dev-linux()` shellHook refactor ride together
  as one atomic unit. `tools/AGENTS.md` (the user's docs
  update that adds a row for `dev-linux.sh`) is left uncommitted
  per the PLAN's "left untouched" instruction; the user can
  commit it separately or fold it into a follow-up.
- **No tests were added or modified.** This is a tooling
  refactor: the underlying `tools/dev-*.sh` scripts and
  `tools/install-into-remote-koreader.py` are unchanged in
  behavior (the `RAKUYOMI_REPO_ROOT` override is a no-op when
  the var is unset, so direct invocation from the repo keeps
  working). End-to-end coverage is the live KOReader launch
  shown in step 7.

## Commit

```text
88a9dcf feat(flake,tools): expose all devShell helpers as binaries for fish
```
