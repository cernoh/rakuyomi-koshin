# Debug Session: dev-server-startup-failure

Created: 2026-06-29
Status: resolved

## Issue

User reports: "there seems to be an issue when i run the dev command and open
rakuyomi? I have had to end it prematurely". Symptom: opening Rakuyomi in
KOReader triggers repeated `Requesting to /health-check` log lines (one per
second) with the server never responding. User Ctrl+C's after ~20s.

## Symptoms (from koreader stdout)

```
06/29/26-20:28:43 INFO  Loading Rakuyomi plugin...
/nix/store/rlq03x4cwf8zn73hxaxnx0zn5q9kifls-bash-5.3p3/bin/bash
/nix/store/rlq03x4cwf8zn73hxaxnx0zn5q9kifls-bash-5.3p3/bin/sh
/home/nixos/.nix-profile/bin/zsh
06/29/26-20:29:03 INFO  Requesting to /health-check
06/29/26-20:29:04 INFO  Requesting to /health-check
... (one per second, 20s of polling, then Ctrl+C)
```

The three unprefixed shell paths are an unrelated koreader startup artifact
(write to stdout by some pre-init diagnostic in the koreader build). They are
NOT from the plugin or the cargo subprocess. Verified by reading the plugin
sources and the koreader source: no `print`/`io.popen` of these paths in the
plugin's startup path.

The actual signal: plugin polls `/health-check` for 20s+ with no success
response, which means the Rust server never bound to `/tmp/rakuyomi.sock`.

## Hypotheses Tested

1. **Server can't bind to UDS** — REJECTED. Server's `pick_listener()` only
   fails if the path is in use or the dir is unwritable. `/tmp` is world-
   writable and the path was free at startup.

2. **Plugin isn't using the override env vars** — REJECTED. `Platform.lua`
   reads `RAKUYOMI_SERVER_COMMAND_OVERRIDE` at module load and
   `unix_platform:startServer()` splits it on spaces and passes the
   resulting array (plus `Paths.getHomeDirectory()`) to `fork`+`execl`.
   The split is well-tested.

3. **`fork`/`execl` is broken in the plugin** — REJECTED. The fork happens
   and returns a PID (no `interrupted!` at fork time); the only interruption
   is during `waitUntilHttpServerIsReady`'s `ffiutil.sleep(1)`.

4. **The `cargo run` subprocess can't find rustc in the koreader env** —
   CONFIRMED. Reproduced by stripping PATH to koreader's runtime
   (`/nix/store/.../luajit-2.1.../bin:/nix/store/.../sdl2-compat.../bin:/nix/store/.../gnutar/bin`) and
   invoking `cargo run --manifest-path backend/Cargo.toml -p server`:
   ```
   error: could not execute process `rustc -vV` (never executed)
   Caused by: No such file or directory (os error 2)
   ```
   cargo exits 101, the plugin's `waitUntilHttpServerIsReady` keeps polling
   /health-check on the UDS, gets connection-refused (uds_http_request
   is itself a fresh process per request, also unable to talk to a
   non-existent server), and the loop continues until timeout or user
   Ctrl+C.

## Root Cause

`tools/run-koreader-with-plugin.sh` sets
`RAKUYOMI_SERVER_COMMAND_OVERRIDE="$(which cargo) run --manifest-path
backend/Cargo.toml -p server --"`. The full path to cargo is captured in
the dev shell (where the toolchain is on PATH), but `cargo run` then
invokes `rustc` via PATH lookup at runtime. When the plugin fork+execl's
that command from inside the koreader process — which is started by
`nix run .#rakuyomi.koreader-with-plugin` and inherits ONLY the
koreader package's runtime env (no rustc, no cc, no mold) — cargo fails
with `rustc -vV: No such file or directory`. The error is printed to
stderr of the cargo subprocess, captured by the plugin's
`SubprocessOutputCapturer`, and surfaces as a per-iteration log line
that is buried in the noise. The plugin keeps polling /health-check
indefinitely (up to `RAKUYOMI_SERVER_STARTUP_TIMEOUT=600s` set by the
dev script).

Same problem for `uds_http_request` and `cbz_metadata_reader` overrides,
though those binaries are usually already built (cargo run is a no-op
build) so the failure is intermittent and only fires after a clean
`cargo clean`.

## Fix

Pre-build the three server binaries once in the dev script and point
the override env vars at the binary paths instead of `cargo run`.
The plugin fork+execl's the binary directly, which doesn't need
rustc on PATH.

Trade-off: lose hot-reload of the server. To pick up server source
changes during development, run `cargo build -p server` and restart
KOReader. This matches the pattern already used by `tools/dev-linux.sh`
for `uds_http_request` and `cbz_metadata_reader` (it pre-builds those
two; the server remains `cargo run` because that script launches the
system koreader, which inherits the dev shell's PATH and so can find
rustc). The `run-koreader-with-plugin.sh` script cannot use that
trick because it launches koreader via `nix run`, which strips PATH.

## Files Modified

- `tools/run-koreader-with-plugin.sh` — pre-build server/uds/cbz,
  point overrides at the resulting binaries.

## Verification

- Server binary launched directly from a stripped nix env (koreader-style
  PATH, no rustc) starts and binds to `/tmp/rakuyomi.sock` in <1s.
  Log:
  ```
  INFO server::app: starting rakuyomi, version: unknown
  INFO server::app: settings file not found at ..., creating default
  INFO server::app: starting rakuyomi unknown on unix:/tmp/rakuyomi.sock
  ```
  Confirms the fix path works; the previous failure was cargo-specific,
  not a binary-load issue.

## Advisor Followup (--debug path)

Advisor flagged that the original `--debug` branch
(`RAKUYOMI_SERVER_COMMAND_OVERRIDE="$(which cargo) debugger --manifest-path ... -p server --"`)
has the same rustc-on-PATH failure mode as `cargo run`: `cargo debugger`
shells out to `rust-gdb`, which needs `rustc` and a system debugger
(`gdb`/`lldb`) on PATH. Neither is in the koreader env, and the dev shell
also lacks `gdb`/`lldb`, so the original `--debug` was already broken
under `nix run .#rakuyomi.koreader-with-plugin` — the fix above only
addressed `dev`, not `debug`.

### Decision

Apply the same pre-built-binary fix to `--debug` (the override is the
same as `dev`). The server starts; the user attaches a debugger
manually from another terminal:

    gdb -p $(pgrep -f target/debug/server)

### Alternatives Considered (Rejected)

- `lldb -- $SERVER_BIN` — would work if lldb were in the koreader env.
  It isn't.
- `nix develop -c koreader ...` for `--debug` — would give the koreader
  process the dev shell's env (rustc, but no gdb/lldb without adding
  them to the dev shell). Also requires rsync'ing the plugin into the
  data dir like `dev-linux.sh` does. Full `cargo debugger` workflow
  preserved, but bigger change (flake.nix + dev script) and out of
  scope for this debug session.
