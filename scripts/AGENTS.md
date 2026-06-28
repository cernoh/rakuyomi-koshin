# Scripts — Build and Dev Scripts

## Purpose

Shell scripts for building, cross-compiling, and packaging RakuYomi.

## Ownership

Owns: build scripts in `scripts/` directory.

## Local Contracts

- All scripts are bash/shell
- Target argument convention: `<target-triple>` for cross-compile scripts
- Scripts are run from repository root

## Work Guidance

### Scripts

| Script | Purpose |
|---|---|
| `build-all.sh` | Cross-compile Rust + package plugin for a target |
| `build-rust-android.sh` | Build Rust `.so` for Android (via cross) |
| `build-plugin.sh` | Package the Lua plugin |
| `generate-settings-schema.sh` | Generate settings schema JSON from Rust |

## Verification

- Scripts tested as part of CI workflow (`.github/workflows/build.yml`)
