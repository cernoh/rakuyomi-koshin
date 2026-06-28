# Packages — Nix Packages and Patches

## Purpose

Nix packages and patches for building KOReader with RakuYomi dependencies in a Nix environment.

## Ownership

Owns: `packages/` directory with Nix expressions and patches.

## Work Guidance

| File | Purpose |
|---|---|
| `koreader.nix` (3.4KB) | Nix derivation for KOReader with RakuYomi plugin |
| `patches/` | Patches for KOReader or dependencies |

## Verification

- Build tested via `nix build` against flakes
- `flake.nix` at repo root references these packages
