# Tools — Development Helper Scripts

## Purpose

Developer utility scripts for installation, setup, and development workflow.

## Ownership

Owns: `tools/` directory.

## Work Guidance

| Script | Purpose |
|---|---|
| `install-into-remote-koreader.py` | Install plugin into a remote KOReader device |
| `run-koreader-with-plugin.sh` | Launch KOReader with the plugin for testing |
| `prepare-sqlx-queries.sh` | Prepare SQLx offline query data |
| `setup-macos.sh` | macOS development environment setup |
| `dev-macos.sh` | macOS development workflow script |
| `dev-linux.sh` | Linux development workflow script (uses system koreader + cargo, no nix run) |

## Verification

- Scripts are manual/dev tools, tested on an as-needed basis
- `prepare-sqlx-queries.sh` must be run after any SQL query changes
