# GitHub — GitHub Configuration

## Purpose

GitHub workflows, issue templates, and command configurations for the RakuYomi repository.

## Ownership

Owns: `.github/workflows/`, `.github/commands/`, `.github/ISSUE_TEMPLATE/`, `.github/FUNDING.yml`.

## Local Contracts

### Workflows

| Workflow | Purpose |
|---|---|
| `build.yml` | Cross-compile + package plugin (5 targets via cross + Podman) |
| `test.yml` / `test.yml%` | Lua lint and test workflow |
| `luacheck.yml` | Lua lint check on PRs |
| `deploy-pages.yml` | Deploy mdBook docs to GitHub Pages |
| `gemini-dispatch.yml` | Gemini AI dispatch workflow |
| `gemini-invoke.yml` | Gemini AI invocation |
| `gemini-review.yml` | Gemini AI code review |
| `gemini-triage.yml` | Gemini issue triage |
| `gemini-scheduled-triage.yml` | Scheduled Gemini triage |
| `issue-label-flow.yml%` | Issue label automation |

### Commands

`.github/commands/` contains Gemini AI command configurations (TOML files for invoke, review, triage, scheduled-triage).

### Issue Templates

- `bug_report.md` — bug report template
- `feature_request.md` — feature request template

## Verification

- Workflows verified by GitHub Actions on push/PR
- Issue templates validated when creating new issues
