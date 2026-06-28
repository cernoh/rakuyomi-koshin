# Docs — mdBook Documentation

## Purpose

User-facing documentation site built with mdBook. Includes a user guide and contributing guide.

## Ownership

Owns: `docs/src/` (markdown sources), `docs/theme/` (custom JS), `docs/book.toml` (mdBook config), `docs/mdbook-admonish.css`.

## Local Contracts

- Built with mdBook
- `docs/src/SUMMARY.md` defines the book structure
- Two sections: `user-guide/` and `contributing/`
- Custom theme in `docs/theme/book.js`
- Admonition blocks styled via `mdbook-admonish.css`
- Images in `docs/src/images/`

## Work Guidance

### Build

```sh
cd docs
mdbook build    # output in docs/book/
mdbook serve    # dev server at http://localhost:3000
```

### Deploy

Published via GitHub Pages workflow: `.github/workflows/deploy-pages.yml`

## Verification

- `mdbook build` succeeds with no broken links
- GitHub Pages deployment workflow validates on push to main
