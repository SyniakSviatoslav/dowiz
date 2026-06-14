#!/usr/bin/env bash
# require-classification.sh — Stop: don't finish without a CHANGE-MANIFEST when code changed
set -euo pipefail

# Check if there are uncommitted changes to apps/ or packages/ (code directories)
if command -v git &>/dev/null; then
  if ! git diff --quiet -- apps/ packages/ 2>/dev/null; then
    if [ ! -f agent/CHANGE-MANIFEST.md ]; then
      echo '{"decision":"block","reason":"Code changes detected without CHANGE-MANIFEST (FINDING-id + fix/improve classification + touched files). Create the manifest or revert changes."}'
      exit 0
    fi
  fi
fi
