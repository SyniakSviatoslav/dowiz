#!/usr/bin/env bash
# pre-edit-lessons.sh — PreToolUse lesson injection (Tier-2 self-improvement).
# ADVISORY ONLY: injects a distilled lesson (ACTION + LINK) before an Edit/Write
# when the target path (or an error signature in context) matches a lesson
# TRIGGER in docs/lessons/INDEX.md. No match = zero output, zero noise.
# Never blocks (always exit 0) — enforcement is a separate deterministic gate.
set -euo pipefail

INPUT=$(cat)

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
INDEX="$ROOT/docs/lessons/INDEX.md"
[ -f "$INDEX" ] || exit 0

# Parse a JSON string field without requiring jq (same fallback chain as protect-paths.sh).
_extract() {
  local field="$1"
  if command -v jq &>/dev/null; then
    echo "$INPUT" | jq -r "$field // empty" 2>/dev/null
  elif command -v python3 &>/dev/null; then
    echo "$INPUT" | FIELD="$field" python3 -c "
import sys, json, os
f = os.environ['FIELD']
try:
    d = json.loads(sys.stdin.read())
    # support .tool_input.file_path and .tool_input.error / .tool_response.error style fields
    cur = d
    for k in f.strip('.').split('.'):
        if isinstance(cur, dict):
            cur = cur.get(k, '')
        else:
            cur = ''
    print(cur if isinstance(cur, str) else '', end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v python &>/dev/null; then
    echo "$INPUT" | FIELD="$field" python -c "
import sys, json, os
f = os.environ['FIELD']
try:
    d = json.loads(sys.stdin.read())
    cur = d
    for k in f.strip('.').split('.'):
        if isinstance(cur, dict):
            cur = cur.get(k, '')
        else:
            cur = ''
    print(cur if isinstance(cur, str) else '', end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v node &>/dev/null; then
    echo "$INPUT" | FIELD="$field" node -e "
let d = '';
process.stdin.on('data', c => d += c);
process.stdin.on('end', () => {
  try {
    const o = JSON.parse(d);
    const path = process.env.FIELD.replace(/^\./,'').split('.');
    let cur = o;
    for (const k of path) cur = (cur && typeof cur === 'object') ? cur[k] : '';
    process.stdout.write(typeof cur === 'string' ? cur : '');
  } catch (e) {}
});
" 2>/dev/null || true
  fi
}

FILE=$(_extract '.tool_input.file_path')
[ -z "$FILE" ] && exit 0

# Best-effort error signature from context (PreToolUse rarely has one; harmless when empty).
ERRSIG=$(_extract '.tool_input.error')
[ -z "$ERRSIG" ] && ERRSIG=$(_extract '.tool_response.error')

# Repo-relative path for glob matching (lesson TRIGGERs are repo-relative).
case "$FILE" in
  "$ROOT"/*) REL="${FILE#"$ROOT"/}" ;;
  *)         REL="$FILE" ;;
esac

# Glob match: translate a lesson TRIGGER glob into a bash extglob pattern.
# `**` -> match any chars incl '/'; `*` -> match any chars; the rest literal.
# Implemented by enabling globstar-like behavior via a simple regex translation.
_glob_match() {
  local glob="$1" target="$2"
  # Escape regex metacharacters, then re-expand glob tokens.
  local re
  re=$(printf '%s' "$glob" | sed -e 's/[.[\()+^${}|]/\\&/g')
  re=${re//\*\*/$'\x01'}     # placeholder for **
  re=${re//\*/[^\/]*}        # single * -> any non-slash run
  re=${re//$'\x01'/.*}       # ** -> any run incl slash
  [[ "$target" =~ ^${re}$ ]]
}

EMITTED=""

# INDEX.md table rows look like: | TRIGGER | file |
# Skip the header (| TRIGGER | file |) and the separator (|---|---|).
while IFS='|' read -r _ trig fpath _; do
  trig="$(echo "$trig" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  fpath="$(echo "$fpath" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  [ -z "$trig" ] && continue
  [ "$trig" = "TRIGGER" ] && continue
  case "$trig" in ---*) continue ;; esac
  [ -z "$fpath" ] && continue

  MATCH=0
  # Path glob match.
  if _glob_match "$trig" "$REL"; then MATCH=1; fi
  # Error-signature match: if a TRIGGER is not a path glob (no slash) and the
  # error context contains it as a substring, treat as a match too.
  if [ "$MATCH" -eq 0 ] && [ -n "$ERRSIG" ] && [[ "$trig" != */* ]]; then
    case "$ERRSIG" in *"$trig"*) MATCH=1 ;; esac
  fi
  [ "$MATCH" -eq 0 ] && continue

  LESSON="$ROOT/$fpath"
  [ -f "$LESSON" ] || continue

  # Pull ACTION (folded YAML scalar, may span lines until next top-level key) and LINK.
  ACTION=$(awk '
    /^ACTION:/ {grab=1; sub(/^ACTION:[[:space:]]*>?[[:space:]]*/,""); if(length($0))print; next}
    grab && /^[A-Z_]+:/ {grab=0}
    grab {sub(/^[[:space:]]+/,""); print}
  ' "$LESSON" | tr '\n' ' ' | sed -e 's/[[:space:]]\+/ /g' -e 's/^ //' -e 's/ $//')
  LINK=$(awk -F': ' '/^LINK:/ {sub(/^LINK:[[:space:]]*/,""); print; exit}' "$LESSON")

  [ -z "$ACTION" ] && continue
  EMITTED="${EMITTED}LESSON [$fpath]\n  ACTION: ${ACTION}\n  LINK: ${LINK}\n\n"
done < "$INDEX"

[ -z "$EMITTED" ] && exit 0

# Emit as PreToolUse additionalContext (JSON). The agent sees this distilled
# lesson before the edit. Advisory: permissionDecision stays 'allow'.
CONTEXT=$(printf "Relevant lesson(s) for this edit (advisory — read before editing):\n\n${EMITTED}")

if command -v jq &>/dev/null; then
  jq -n --arg ctx "$CONTEXT" '{
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      additionalContext: $ctx
    }
  }'
elif command -v python3 &>/dev/null; then
  CTX="$CONTEXT" python3 -c "
import json, os
print(json.dumps({'hookSpecificOutput': {'hookEventName': 'PreToolUse', 'additionalContext': os.environ['CTX']}}))
"
else
  # Fallback: emit to stderr so the agent still sees it (non-blocking).
  printf '%b' "$CONTEXT" >&2
fi

exit 0
