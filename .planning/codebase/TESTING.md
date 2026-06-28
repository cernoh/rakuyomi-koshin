# Testing Patterns

**Analysis Date:** 2026-06-28

## Test Framework: Rust

**Runner:**
- cargo test (built-in)
- Config: `RUSTFLAGS`, workspace members in `backend/Cargo.toml`

**Benchmarking:**
- Criterion 0.5.1 with async_tokio feature
- pprof 0.15 for flamegraph profiling

**Run Commands:**
```bash
cargo test --all              # Run all unit + integration tests in workspace
cargo bench                   # Run criterion benchmarks (from workspace root)
```

## Test Framework: Lua

**Runner:**
- busted (Lua BDD test framework)
- luacheck (static analysis)

**Run Commands:**
```bash
luacheck frontend/rakuyomi.koplugin/       # Static analysis from repo root
busted frontend/rakuyomi.koplugin/         # Run busted tests (install busted first)
python3 ci/lua-language-server-check.py frontend/  # Lua language server diagnostics
```

## Test Framework: E2E

**Runner:**
- pytest 8.3.4 with pytest-asyncio 0.25.3
- Config: `e2e-tests/pyproject.toml` — `asyncio_mode = "auto"`

**Run Commands:**
```bash
cd e2e-tests && poetry run pytest tests/          # Run all E2E tests
cd e2e-tests && poetry run pytest -k test_library_view  # Single test file
bash ci/run-e2e-tests.sh                           # CI runner (uses xvfb + fluxbox)
```

## Test File Organization: Rust

**Location:**
- Unit tests: `#[cfg(test)] mod tests { ... }` at the bottom of implementation files
- Benchmarks: `backend/shared/benches/<name>_benchmark.rs` with `harness = false`

**Test placement by file:**

| File | Coverage |
|------|----------|
| `backend/shared/src/arima_light.rs:612-737` | ARIMA model fitting, forecast, preprocessing |
| `backend/shared/src/chapter_storage.rs:693-750+` | Image transcoding to JPEG |
| `backend/shared/src/source/html_element.rs:389-570+` | CSS selector `:contains()` normalization, HTML element traversal |
| `backend/shared/src/source/source_settings.rs:116-165+` | Setting retrieval with defaults vs stored values |
| `backend/shared/src/source/wasm_imports/std.rs:969-1020+` | Swift date format to strptime conversion |

**Naming:**
- Test functions: `snake_case`, descriptive — `image_is_transcoded_to_jpeg`, `test_difference_and_simple_fit`
- Module: `mod tests` in each source file

## Test Structure: Rust

**Helper pattern (factory functions before tests):**
```rust
fn make_storage() -> ChapterStorage { ... }
fn make_rgb_jpeg(width: u32, height: u32) -> Vec<u8> { ... }
fn build_chapters(nums: Vec<Option<f32>>, ts: Vec<Option<i64>>) -> Vec<ChapterInformation> { ... }
```

**Assertion pattern:**
- `assert!(expr)` — boolean assertions
- `assert_eq!(left, right)` — equality
- `result.is_ok() / result.unwrap()` — Result checking
- `diff_abs < threshold` — approximate comparisons

## Benchmarks: Rust

**Benchmark files:**

| File | Harness | Description |
|------|---------|-------------|
| `backend/shared/benches/chapter_downloader_benchmark.rs` | False (criterion) | Downloads chapter pages as CBZ. Requires env vars: `BENCHMARK_SOURCE_PATH`, `BENCHMARK_MANGA_ID`, `BENCHMARK_CHAPTER_ID` |
| `backend/shared/benches/search_mangas_benchmark.rs` | False (criterion) | Search manga performance benchmark |

**Benchmark pattern:**
```rust
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = chapter_downloader_benchmark
}
criterion_main!(benches);
```

## Test File Organization: Lua

**Location:**
- Single busted spec file: `frontend/rakuyomi.koplugin/chapters/findNextChapter_spec.lua`
- Test module in plugin: `frontend/rakuyomi.koplugin/testing.lua` (provides IPC-based test hooks for E2E)

**Naming:**
- Spec files: `<name>_spec.lua`
- No other busted spec files found — test coverage is sparse

## Test Structure: Lua

**BDD-style suite:**
```lua
describe('findNextChapter', function()
  it('should be nil when there is a single chapter', function()
    local chapters = { makeChapter({ volume_num = 1, chapter_num = 1 }) }
    local current_chapter = chapters[1]
    local next_chapter = findNextChapter(chapters, current_chapter)
    assert.is_nil(next_chapter)
  end)

  it('should find chapter with the closest chapter number...', function()
    ...
    assert.is_not_nil(next_chapter)
    assert.equal(1.5, next_chapter.chapter_num)
  end)
end)
```

**Patterns:**
- `describe(name, fn)` / `it(name, fn)` — BDD structure
- Helper factory: `local function makeChapter(fields) ... end` — creates test data
- `--- @type Chapter[]` — type annotation for arrays
- `---@diagnostic disable-next-line: need-check-nil` — suppressing nil-check after `assert.is_not_nil`
- Assertions: `assert.is_nil`, `assert.is_not_nil`, `assert.equal`, `assert.is_true`

## Testing Module (`testing.lua`)

- Located at `frontend/rakuyomi.koplugin/testing.lua`
- **Not a test file** — it's a test *harness* for E2E tests
- Provides IPC socket at `/tmp/rakuyomi_testing_ipc.sock`
- Emits events like `'initialized'`, handles keypress hooks
- Gated by `RAKUYOMI_IS_TESTING=1` env var; returns `NullTesting` in production

## E2E Test Infrastructure

**Framework:** pytest with async IO (`pytest-asyncio`)

**Directory structure:**
```
e2e-tests/
├── pyproject.toml           # Poetry project config
├── tests/
│   ├── conftest.py          # Fixture registration
│   ├── fixtures.py          # agent & koreader_driver fixtures
│   ├── koreader_driver.py   # KOReader process manager + IPC
│   ├── agent.py             # OpenAI-based UI query agent
│   ├── phase_report_hook.py # Test phase reporting
│   └── queries/             # Reusable query helpers
│       ├── count_listing_pages.py
│       ├── describe_dialog.py
│       ├── list_available_sources.py
│       ├── list_mangas.py
│       ├── locate_button.py
│       └── __init__.py
├── test_library_view.py     # Library view + search + add to library E2E
├── test_open_chapter.py     # Open chapter flow E2E
└── test_search_view_modes.py # Search view mode switching E2E
```

**E2E Test Files:**

| File | Description |
|------|-------------|
| `e2e-tests/tests/test_library_view.py` | Full library flow: install source, search, add to library, view library, open chapter listing, verify unread count |
| `e2e-tests/tests/test_open_chapter.py` | Chapter listing -> download -> open in reader flow |
| `e2e-tests/tests/test_search_view_modes.py` | Search result view mode switching (base/cover/grid) |

**Driver Architecture:**
- `KOReaderDriver` manages KOReader subprocess, IPC socket, screenshots
  - `__aenter__` sets up IPC, writes settings, starts KOReader
  - Screenshots on test failure saved to `e2e-tests/screenshots/`
- `Agent` wraps OpenAI API for AI-powered UI understanding
  - Sends Lua-table serialized UI description with JSON schema query
  - Returns structured Pydantic model responses
  - Environment vars: `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_MODEL`

**Key Dependencies:**
- `openai >=1.60.1` — AI query agent
- `pydantic >=2.10.6` — typed response models
- `pyautogui >=0.9.54` — mouse/keyboard automation
- `pywinctl >=0.4.1` — cross-platform window control
- `pillow >=11.1.0` — screenshot capture
- `requests >=2.32.3` — HTTP client
- `curlify >=2.2.1` — curl command logging

## CI Coverage

### GitHub Actions Workflows

**`.github/workflows/test.yml`:**
- Trigger: push to main, pull_request, workflow_dispatch, workflow_call
- Runs `cargo test --all` in `backend/`
- Cargo cache for registry + git + target
- Installs system deps: `pkg-config`, `libfontconfig1-dev`, `libfreetype6-dev`
- Rust components: rustfmt, clippy

**`.github/workflows/luacheck.yml`:**
- Trigger: push/PR to main, paths matching `frontend/**`
- Runs `luacheck frontend/` via system lua5.1 + luarocks
- Reusable via `workflow_call` (used by build.yml)

**`.github/workflows/build.yml`:**
- Matrix build: aarch64, desktop, kindle, kindlehf, kindlea9, android
- Called sub-workflows: `luacheck`, `test`
- Generates settings.schema.json via `scripts/generate-settings-schema.sh`
- **E2E tests are NOT run in CI** — no E2E step exists in build.yml or test.yml
- Release step uses semantic-release on main branch with GitHub release creation

**`.github/workflows/gemini-*.yml`:**
- Gemini-powered workflows for automated code review and issue triage (dispatch, invoke, review, scheduled-triage, triage) — exist but not standard CI testing

## Coverage Gaps

| Area | Status | Risk |
|------|--------|------|
| Rust unit tests | Present in 5 source files, 63+ test functions | Moderate — core DB and use cases have no tests |
| Rust integration tests | Not found — no `tests/` directory in any crate | High — no end-to-end Rust tests |
| Rust benchmark tests | 2 criterion benchmarks | Low — benchmarks exist but require manual env setup |
| Lua busted tests | 1 spec file with 5 test cases | High — only findNextChapter tested |
| Lua luacheck static analysis | Running in CI | Low — covers style/global issues |
| Lua language server check | Python script exists (`ci/lua-language-server-check.py`) | Low — checks Lua type safety |
| E2E tests | 3 Playwright-based test files | Moderate — only exercises basic navigation flows |
| E2E in CI | NOT enabled — no E2E step in any workflow | High — E2E tests can drift from working state |
| SQLx compile-time checking | Enabled via `sqlx::query!` in `database.rs` | Low — catches SQL errors at compile time |
| Cargo clippy | Installed as component | Low — not explicitly run in CI (only installed) |

## Mocking

**Framework:** Not used — Rust codebase avoids mocking frameworks

**Pattern:** Real implementations with test helpers:
- `tempfile::tempdir()` for filesystem isolation (chapter_storage tests)
- Factory functions returning test data structs
- No mock HTTP servers or database mocks in tests

**Rust Dev-Dependencies:**
- `criterion` — benchmarks
- `pprof` — CPU profiling with flamegraph output

## Fixtures: E2E Tests

**Test data:**
```python
@pytest.fixture
def agent() -> Agent:
    return Agent()

@pytest.fixture
async def koreader_driver(request, agent, tmp_path) -> AsyncGenerator[KOReaderDriver]:
    async with KOReaderDriver(agent, tmp_path) as driver:
        yield driver
        # Screenshot on failure
        if test_call is not None and test_call.failed:
            screenshot_path = screenshot_folder / f'{request.node.name}.png'
            await driver.screenshot(screenshot_path)
```

**Setup:**
- KOReader starts fresh per-test (temp directory via `tmp_path`)
- Pre-configures `settings.reader.lua` and `rakuyomi/settings.json`
- Installs source via `koreader_driver.install_source()`
- IPC socket at `/tmp/rakuyomi_testing_ipc.sock`

**Teardown:**
- KOReader process killed via context manager exit
- Failure screenshots saved to `e2e-tests/screenshots/`

---

*Testing analysis: 2026-06-28*
