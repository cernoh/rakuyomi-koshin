# RakuYomi KOReader Plugin

## Purpose

KOReader plugin providing the manga reading UI. Communicates with the Rust backend via HTTP/JSON over TCP or UDS.

## Ownership

Owns: all Lua source files for UI views, platform dispatch, async jobs, widgets, patch menus, extension handlers, and translations.

## Local Contracts

- `Backend.lua` — central API, all server communication
- `Platform.lua` — platform dispatch (android vs generic_unix)
- `platform/` — platform implementations (TCP for Android/linux bridge, UDS + fork/exec for Unix)
- `main.lua` — plugin entry, registers menu & Dispatcher
- `Settings.lua` — plugin settings management
- `Paths.lua` — data directory paths
- `Icons.lua` — icon definitions

### UI Views

- `LibraryView.lua` (38KB) — manga library grid/list
- `ChapterListing.lua` (40KB) — chapter list for a manga
- `MangaSearchResults.lua` (13KB) — search results view
- `MangaInfoWidget.lua` (18KB) — manga detail/info widget
- `MangaReader.lua` (8KB) — manga reading view
- `AvailableSourcesListing.lua` (7KB) — source browser
- `InstalledSourcesListing.lua` (5KB) — installed source manager
- `UpdateChecker.lua` (4KB) — update notifications
- `PlaylistDialog.lua` (10KB) — playlist management
- `Settings.lua` (14KB) — settings dialog
- `SourceSettings.lua` (8KB) — per-source settings

### Jobs (async operations)

- `DownloadChapter.lua` — single chapter download
- `DownloadScanlatorChapters.lua` — scanlator batch download
- `DownloadUnreadChapters.lua` — unread batch download
- `RefreshLibraryChapters.lua` — chapter refresh
- `RefreshLibraryDetails.lua` — detail refresh
- `Job.lua` — base job abstraction
- `BasicJobDialog.lua` — job progress dialog

### Platform

- `platform/android_platform.lua` — TCP connection via Android
- `platform/generic_unix_platform.lua` — UDS + fork/exec server
- `platform/util.lua` — platform utilities
- `platform/_meta.lua` — platform metadata (excluded from luacheck)

### Widgets & Patches

- `widgets/` — reusable UI widgets (Menu, SettingItem, etc.)
- `patch/` — KOReader menu patches (MenuItemCover, MenuItemGrid, MenuCustom)
- `extensions/` — document extensions (CbzDocument.lua)
- `chapters/` — chapter navigation logic
- `utils/` — utility functions
- `handlers/` — event handlers (addToPlaylist.lua)
- `l10n/` — translations (40+ languages via gettext)
- `gettext+.lua` — gettext library extension

## Verification

- Tests via `busted`: `chapters/findNextChapter_spec.lua`
- Lua lint via `.github/workflows/luacheck.yml`
- CI tests via E2E test suite
- Manual testing in KOReader
