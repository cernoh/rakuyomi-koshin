# UDS HTTP Request — UDS HTTP Proxy Binary

## Purpose

Standalone binary that bridges HTTP requests over Unix Domain Sockets. Used on Unix platforms (Kindle, Kobo) where the Lua plugin cannot connect directly to a TCP socket.

## Ownership

Owns: the UDS-to-HTTP proxy binary, standalone executable.

## Local Contracts

- Single-purpose binary with minimal dependencies
- Reads HTTP request from stdin, sends over UDS to `server` socket
- Returns HTTP response on stdout
- Socket path: `/tmp/rakuyomi.sock` (configurable)

## Work Guidance

Operates as part of the Unix platform architecture:
1. Lua plugin executes `uds_http_request` binary via `io.popen` or similar
2. Binary forwards the request to the `server` UDS listener
3. Binary returns the response to the Lua plugin

## Verification

- `cargo test -p uds_http_request`
- Integration test via the full stack: Lua → uds_http_request → server → response
