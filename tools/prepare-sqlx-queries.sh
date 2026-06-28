#!/usr/bin/env bash
set -ev

export BACKEND_SHARED_DIR="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PWD")/backend/shared"
export DATABASE_URL="sqlite:/tmp/rakuyomi.db"

cd "$BACKEND_SHARED_DIR"

cargo sqlx db create
cargo sqlx migrate run
cargo sqlx prepare

echo "sqlx queries prepared successfully!"