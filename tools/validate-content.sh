#!/usr/bin/env sh
# Check the course files (content/<lang>/) the way the app reads them.
# Usage: tools/validate-content.sh [DIR]   (default: content)
# See docs/CONTENT-UPDATES.md.
cd "$(dirname "$0")/.." || exit 2
exec cargo run -q --release --bin shorashim-ctl -- validate-content "${1:-content}"
