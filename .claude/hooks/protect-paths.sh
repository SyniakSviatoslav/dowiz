#!/usr/bin/env bash
# protect-paths.sh — gate edits to protected zones. TWO buckets
# (protocol: docs/governance/self-ecosystem-evolution.md):
#   RED-LINE floor  — product safety / DB / migrations / infra / CI / contracts / env.
#                     ALWAYS human-gated (hard block). Never self-modifiable.
#   SELF-ECOSYSTEM  — the agent's own machinery under .claude/**. Modifiable UNDER a standing
#                     operator token (.claude/state/self-mod-enabled); every change AUDITED to
#                     .claude/logs/self-mod.log; kept honest by verify-safety-floor.sh (the floor
#                     invariant can only be strengthened/scope-corrected, never removed).
#   Authorization tokens are operator-only — the agent can NEVER grant itself the capability.
# Scope: files INSIDE the repo only. Absolute paths outside the repo (e.g. ~/.claude memory) pass.
set -euo pipefail

INPUT=$(cat)

# Parse JSON without requiring jq (jq not available on all platforms, e.g. Windows)
_extract_path() {
  if command -v jq &>/dev/null; then
    echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null
  elif command -v python3 &>/dev/null; then
    echo "$INPUT" | python3 -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('tool_input', {}).get('file_path', ''), end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v python &>/dev/null; then
    echo "$INPUT" | python -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('tool_input', {}).get('file_path', ''), end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v node &>/dev/null; then
    echo "$INPUT" | node -e "
let d = '';
process.stdin.on('data', c => d += c);
process.stdin.on('end', () => {
  try { const o = JSON.parse(d); process.stdout.write(o?.tool_input?.file_path || ''); } catch (e) {}
});
" 2>/dev/null || true
  fi
}

FILE=$(_extract_path)
[ -z "$FILE" ] && exit 0

# Only guard files inside the repo. An absolute path not under the repo root
# (e.g. ~/.claude/projects/.../memory) is out of scope.
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
case "$FILE" in
  "$ROOT"/*) REL="${FILE#"$ROOT"/}" ;;
  /*) exit 0 ;;
  *) REL="$FILE" ;;
esac

# ── RED-LINE floor: product safety / DB / migrations / infra / CI / contracts / env.
#    Human-gated always — not self-modifiable even with the capability ON. ──
RED_LINE='(^|/)(migrations|\.github)/|(^|/)(fly\.toml|Dockerfile|pnpm-lock\.yaml)$|/package\.json$|packages/shared-types/|packages/db/|/contracts/|\.contract\.|(^|/)\.env'
if echo "$REL" | grep -qE "$RED_LINE"; then
  echo "BLOCKED: '$REL' is a RED-LINE floor path (DB / migrations / infra / CI / contracts / env). Human-gated — not self-modifiable, even with the self-mod capability on." >&2
  exit 2
fi

# ── SELF-ECOSYSTEM: the agent's own machinery under .claude/**. ──
if echo "$REL" | grep -qE '(^|/)\.claude/'; then
  # Authorization tokens are operator-only. The agent must never grant itself the capability
  # or release a human gate. (self-mod-enabled / serious-override / redline-confirmed / serious-cleared)
  if echo "$REL" | grep -qE '(^|/)\.claude/state/(self-mod-enabled|serious-override|redline-confirmed|serious-cleared)$'; then
    echo "BLOCKED: '$REL' is an authorization token — operator-only. The agent cannot self-authorize or release a human gate." >&2
    exit 2
  fi
  if [ -f "$ROOT/.claude/state/self-mod-enabled" ]; then
    mkdir -p "$ROOT/.claude/logs" 2>/dev/null || true
    printf '%s SELF-MOD %s\n' "$(date -Iseconds 2>/dev/null || date)" "$REL" >> "$ROOT/.claude/logs/self-mod.log" 2>/dev/null || true
    exit 0   # capability ON — allow, audited. verify-safety-floor.sh keeps the floor honest.
  fi
  echo "BLOCKED: '$REL' is in your self-ecosystem but the self-mod capability is OFF. Operator enables the standing token:  echo '<reason>' > .claude/state/self-mod-enabled" >&2
  exit 2
fi

exit 0
