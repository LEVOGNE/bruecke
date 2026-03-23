#!/usr/bin/env bash
# Fast rebuild — only when src/engine.js or src/shell.html changes (no WASM recompile)
set -e
cd "$(dirname "$0")"

echo "▶ 1/2  Building server..."
cargo build --bin server --release --quiet

echo "▶ 2/2  Copying to dist/..."
cp target/release/server dist/server

echo "✓ done!  →  cd dist && ./server"
