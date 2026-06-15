#!/usr/bin/env bash
# require-classification.sh — Stop: don't finish without a CHANGE-MANIFEST when code changed
set -euo pipefail

PROJECT_ROOT="/c/Users/Dell5/Documents/dowiz"

# Check if there are uncommitted changes to apps/ or packages/ (code directories)
if command -v git &>/dev/null; then
  if ! git -C "$PROJECT_ROOT" diff --quiet -- apps/ packages/ 2>/dev/null; then
    if [ ! -f "$PROJECT_ROOT/agent/CHANGE-MANIFEST.md" ]; then
      echo '{"decision":"block","reason":"Code changes detected without CHANGE-MANIFEST (FINDING-id + fix/improve classification + touched files). Create the manifest or revert changes."}'
      exit 0
    fi
  fi
fi
