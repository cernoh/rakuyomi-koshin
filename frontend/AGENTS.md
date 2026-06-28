# Frontend — Lua Plugin Frontend

## Purpose

Frontend of RakuYomi — KOReader Lua plugin and associated configuration files. Provides the complete UI within KOReader for browsing, searching, and reading manga.

## Ownership

Owns: `rakuyomi.koplugin/` (the plugin itself), `.luarc.ci.json` (Lua language server config), `.editorconfig`.

## Local Contracts

- LuaJIT 5.1 compatibility
- KOReader widget pattern (`InputContainer:extend`)
- EmmyLua annotations on all public APIs
- Lua lint via `luacheck` (`.luacheckrc` at repo root)
- Plugin entry at `rakuyomi.koplugin/main.lua`

## Work Guidance

### Lua conventions

- CamelCase for module names/classes, snake_case for locals/functions
- Require-based modules returning tables
- UI via `UIManager:show()`, frame containers
- Configuration via `Settings.lua`

### CI

- `.luarc.ci.json` sets LuaJIT runtime, `G_reader_settings` global
- `.editorconfig` standard at `frontend/`

## Verification

- `luacheck frontend/rakuyomi.koplugin/` from repo root
- `.github/workflows/luacheck.yml` runs `ci/lua-language-server-check.py`
- E2E tests in `e2e-tests/` exercise the plugin through KOReader

## Child DOX Index

| Path | Scope | Owner |
|---|---|---|
| `rakuyomi.koplugin/` | KOReader plugin: UI views, platform, jobs, widgets, l10n | Plugin |
