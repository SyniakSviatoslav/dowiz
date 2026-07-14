#!/usr/bin/env bash
# post-edit-gates.sh — route the governance mode by CLASSIFICATION (Agent Operating Model §1).
# spike/challenge → relaxed (red lines + boundary only); build/audit → full discipline.
# Apply to .claude/hooks/ (protect-paths zone → manual approval).
set -euo pipefail

PROJECT_ROOT="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
cd "$PROJECT_ROOT"
MANIFEST="$PROJECT_ROOT/agent/CHANGE-MANIFEST.md"

# Edited file path is provided by the harness on stdin as JSON ({"tool_input":{"file_path":...}}).
INPUT="$(cat 2>/dev/null || true)"
FILE="$(printf '%s' "$INPUT" | grep -oE '"file_path"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed -E 's/.*"file_path"[[:space:]]*:[[:space:]]*"([^"]*)".*/\1/')"
REL="${FILE#"$PROJECT_ROOT"/}"

CLASS="build" # safe default: full discipline unless explicitly a spike/challenge
if [ -f "$MANIFEST" ]; then
  C="$(grep -ioE '^CLASSIFICATION:[[:space:]]*(spike|build|audit|challenge)' "$MANIFEST" | head -1 | sed -E 's/.*:[[:space:]]*//' | tr 'A-Z' 'a-z')"
  [ -n "$C" ] && CLASS="$C"
fi

red_lines() {
  # Red lines hold in EVERY mode. Cheap grep on the edited file.
  [ -n "$REL" ] && [ -f "$REL" ] || return 0
  # Scope to code: non-executable prose can't trip a RUNTIME red line. The regression
  # ledger/lessons literally cite these patterns (e.g. `Math.random()` + token) as the
  # thing to AVOID — matching them there is a false positive, not a violation.
  case "$REL" in *.md|*.mdx|*.markdown|*.txt|*.rst) return 0 ;; esac
  if grep -nE "document\.cookie|set-cookie|Math\.random\(\).*(token|otp|secret|nonce)|parseFloat.*(price|amount|total)|customerPhone|customer_phone" "$REL" >/dev/null 2>&1; then
    echo "RED-LINE: '$REL' trips a product red line (cookie / insecure-random secret / float money / raw PII). Holds in spike+challenge too." >&2
    exit 2
  fi
}

case "$CLASS" in
  spike|challenge)
    red_lines
    # Boundary: a spike change must live under spikes/; a challenge under docs/decisions/.
    if [ "$CLASS" = "spike" ] && [ -n "$REL" ] && ! printf '%s' "$REL" | grep -qE '^spikes/'; then
      echo "BOUNDARY: spike-classified change to '$REL' outside spikes/. Spike code lives only in spikes/." >&2
      exit 2
    fi
    node scripts/guardrail-spike-boundary.mjs >/dev/null 2>&1 || { echo "BOUNDARY: apps/packages import from spikes/." >&2; exit 2; }
    ;;
  build|audit)
    red_lines
    node scripts/guardrail-spike-boundary.mjs >/dev/null 2>&1 || { echo "BOUNDARY: apps/packages import from spikes/." >&2; exit 2; }
    # lint:gates only means something when a JS/TS/CSS surface exists to lint. The legacy
    # thin-layer was removed (no root package.json) → the script is absent and this failed
    # on EVERY edit. Run it precisely when a lintable web file is edited and pnpm+manifest exist.
    if [ -f package.json ] && command -v pnpm &>/dev/null && printf '%s' "$REL" | grep -qE '\.(ts|tsx|js|jsx|mjs|cjs|css|scss|astro|svelte|vue)$'; then
      pnpm -s lint:gates 2>&1 || { echo "lint:gates failed (hardcoded color / raw SQL / cookie?)" >&2; exit 2; }
    fi
    ;;
esac
