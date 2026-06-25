#!/usr/bin/env bash
# require-classification.sh — Stop: don't finish without a CHANGE-MANIFEST carrying a CLASSIFICATION
# (Agent Operating Model §1). Extends the prior CHANGE-MANIFEST gate to require one of the four
# labels: spike | build | audit | challenge. Apply to .claude/hooks/ (protect-paths zone → manual).
set -euo pipefail

PROJECT_ROOT="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
MANIFEST="$PROJECT_ROOT/agent/CHANGE-MANIFEST.md"

if command -v git &>/dev/null; then
  # Code changed in execution OR recon space → a manifest with a classification is required.
  if ! git -C "$PROJECT_ROOT" diff --quiet -- apps/ packages/ spikes/ 2>/dev/null; then
    if [ ! -f "$MANIFEST" ]; then
      echo '{"decision":"block","reason":"Code changed without agent/CHANGE-MANIFEST.md. Add it with a CLASSIFICATION line: spike | build | audit | challenge (+ FINDING-id + touched files)."}'
      exit 0
    fi
    if ! grep -qiE '^CLASSIFICATION:[[:space:]]*(spike|build|audit|challenge)\b' "$MANIFEST"; then
      echo '{"decision":"block","reason":"CHANGE-MANIFEST is missing a valid CLASSIFICATION line. Add exactly one of: CLASSIFICATION: spike | build | audit | challenge (§1)."}'
      exit 0
    fi
  fi
fi
