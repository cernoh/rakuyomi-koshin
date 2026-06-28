# CBZ Metadata Reader — CBZ Metadata Extraction Binary

## Purpose

Standalone binary for extracting metadata from CBZ (comic book ZIP) archive files. Used during import and library refresh.

## Ownership

Owns: CBZ metadata extraction binary, standalone executable.

## Local Contracts

- Single-purpose binary
- Reads CBZ file path as argument, outputs metadata as JSON
- Uses `zip` crate for archive reading and `shared` library types for metadata model

## Verification

- `cargo test -p cbz_metadata_reader`
- Tested via e2e test suite with sample CBZ files
