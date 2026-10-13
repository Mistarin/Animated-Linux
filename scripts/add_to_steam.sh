#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "Registering AetherPaper in Steam shortcuts..."

cd "$DIR/src-tauri"
cargo run --bin aetherpaper -- --register-steam 2>/dev/null || true

echo "Done! Please restart Steam to see AetherPaper in your Library with full artwork."
