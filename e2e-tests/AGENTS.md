# E2E Tests — End-to-End Tests

## Purpose

End-to-end tests for RakuYomi using Python/Playwright. Tests exercise the full stack: KOReader → Lua plugin → Rust backend.

## Ownership

Owns: `e2e-tests/` directory with Playwright-based integration tests.

## Local Contracts

- Python 3 + Pytest + Playwright
- Test runner: `ci/run-e2e-tests.sh`
- KOReader driver abstraction in `koreader_driver.py`
- AI-assisted testing via agent pattern in `agent.py`

## Work Guidance

### Tests

| File | Purpose |
|---|---|
| `tests/test_library_view.py` | Library view interactions |
| `tests/test_open_chapter.py` | Chapter opening flow |
| `tests/test_search_view_modes.py` | Search modes and navigation |

### Key modules

- `koreader_driver.py` (11KB) — KOReader control via Playwright
- `agent.py` (5KB) — AI test agent for flexible assertions
- `fixtures.py` — test fixtures
- `conftest.py` — pytest configuration
- `phase_report_hook.py` — test phase reporting

## Verification

```sh
ci/run-e2e-tests.sh
```
