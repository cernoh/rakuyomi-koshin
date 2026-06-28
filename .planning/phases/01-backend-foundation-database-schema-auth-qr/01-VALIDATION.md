# Validation: Phase 1 — Backend Foundation

**Derived from:** 01-RESEARCH.md ## Validation Architecture
**Date:** 2026-06-28

## Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in Rust test harness) |
| Config file | none (workspace-level `cargo test` covers all crates) |
| Quick run command | `cargo test -p server -- track` |
| Full suite command | `cargo test --workspace` |

## Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | Wave |
|--------|----------|-----------|-------------------|------|
| DB-01/02 | Migrations create track + tracker_auth tables | integration | `cargo test -p shared -- test_tracking_tables` | Wave 1 |
| DB-03 | Rust types (TrackerService, TrackEntry, etc.) | unit | `cargo test -p shared -- track::types::test` | Wave 1 |
| AL-01 | AniList auth URL generation | unit | `cargo test -p server -- anilist::test_auth_url` | Wave 2 |
| ML-01 | MAL auth URL generation with PKCE params | unit | `cargo test -p server -- mal::test_auth_url` | Wave 2 |
| ML-02 | PKCE code_verifier/challenge generation | unit | `cargo test -p server -- pkce::test` | Wave 2 |
| QR-01/02 | QR code generation produces valid PNG | integration | `cargo test -p server -- qr::test_generate` | Wave 2 |
| API-01 | GET /track/services | integration | `cargo test -p server -- track::test_services` | Wave 3 |
| API-03 | POST /track/{tracker}/auth stores tokens | integration | `cargo test -p server -- track::test_auth_submit` | Wave 3 |
| API-04 | DELETE /track/{tracker}/auth clears credentials | integration | `cargo test -p server -- track::test_auth_delete` | Wave 3 |
| API-05 | GET /track/{tracker}/status returns login state | integration | `cargo test -p server -- track::test_status` | Wave 3 |

## Sampling Rate

- Unit tests: run always (cargo test default)
- Integration tests: run always (cargo test default)
- No snapshot testing (not applicable for backend HTTP API)

## Guardrails

| Guardrail | Method | Enforced At |
|-----------|--------|-------------|
| No tokens in logs | Code review + clippy | PR merge |
| Migration rollback | Add `-- DOWN` section in migration | Plan execution |
| sqlx prepare check | `cargo sqlx prepare` before commit | Plan execution |
