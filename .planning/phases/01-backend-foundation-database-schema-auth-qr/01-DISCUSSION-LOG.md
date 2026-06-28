# Discussion Log: Phase 01 — Backend Foundation

## Areas Discussed

### 1. DB Schema Design — Track table FK strategy

- **Question**: Composite FK (source_id + manga_id) vs dedicated manga_rowid FK
- **Options**: Composite FK matching existing MangaId pattern (chosen) / Dedicated rowid FK
- **Decision**: Composite FK. Avoids extra query on every write. Matches existing manga table patterns.

- **Question**: Score storage format
- **Options**: Normalize 0-10 INTEGER (chosen) / Raw per-tracker / REAL for precision
- **Decision**: 0-10 INTEGER. Normalize at Rust API boundary. Slight precision loss for AniList (1-decimal) is acceptable.

- **Question**: TrackStatus enum shape
- **Options**: Mihon-compatible 6-variant (chosen) / Minimal AniList+MAL union
- **Decision**: Mihon-compatible: CURRENTLY_READING, COMPLETED, ON_HOLD, DROPPED, PLAN_TO_READ, REPEATING.

- **Question**: SyncDirection enum
- **Options**: Push/Pull/TwoWay (chosen) / Just Push/Pull as flags
- **Decision**: Three variants. TwoWay = push then pull. Clearer intent in sync API.

### 2. OAuth Module Design — Abstraction vs separate

- **Question**: Trait-based vs standalone functions
- **Options**: TrackerAuth trait (chosen) / Standalone per-service functions
- **Decision**: TrackerAuth trait with generate_auth_url(), exchange_code(), refresh_token(). Benefits Phase 2.

- **Question**: Where client code lives
- **Options**: shared::track::client / server::track (chosen)
- **Decision**: Auth + HTTP code in server::track. Only types in shared::track::types.

### 3. PKCE State Management

- **Question**: Where to store code_verifier between /auth-url and /auth
- **Options**: In-memory HashMap (chosen) / Pre-insert into tracker_auth / JWT-encode in state param
- **Decision**: Arc<Mutex<HashMap<String, PkceSession>>>. State param as key. 15-min TTL. Simple and appropriate for short-lived OAuth flow.

### 4. QR Code Delivery Format

- **Question**: How to serve PNG to Lua frontend
- **Options**: base64 in JSON body / Separate binary endpoint (chosen)
- **Decision**: POST returns URL + qr_id, GET /track/qr/{qr_id} → image/png. Enables caching, keeps JSON compact.
