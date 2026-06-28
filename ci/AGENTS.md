# CI — CI Helper Scripts

## Purpose

Scripts used by CI workflows for linting and testing.

## Ownership

Owns: `ci/` directory with CI-specific scripts.

## Work Guidance

| Script | Purpose |
|---|---|
| `lua-language-server-check.py` (3.5KB) | Lua language server CI check — validates Lua code quality |
| `run-e2e-tests.sh` (344B) | E2E test runner — launches KOReader and runs Playwright tests |

## Verification

- `lua-language-server-check.py` run by `.github/workflows/luacheck.yml`
- `run-e2e-tests.sh` run by `.github/workflows/test.yml`
