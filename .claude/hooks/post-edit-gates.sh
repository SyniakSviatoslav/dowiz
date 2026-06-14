#!/usr/bin/env bash
# post-edit-gates.sh — Quick lint gate immediately after any code edit
set -euo pipefail

# Run fast lint gate to catch hardcoded colors, raw SQL, cookies, etc.
if command -v pnpm &>/dev/null; then
  pnpm -s lint:gates 2>&1 || {
    echo "lint:gates failed — fix before continuing (hardcoded color / raw SQL / cookie?)" >&2
    exit 2
  }
fi
