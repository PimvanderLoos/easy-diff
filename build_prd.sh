#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

if ! command -v cargo-tauri &>/dev/null; then
    echo "tauri-cli not found, installing..."
    cargo install tauri-cli
fi

cd frontend && npm install && cd ..

cargo tauri build --features gui
