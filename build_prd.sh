#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

if ! cargo tauri --version &>/dev/null; then
    echo "tauri-cli not found, installing..."
    cargo install tauri-cli
fi

(cd frontend && npm install)

cargo tauri build
