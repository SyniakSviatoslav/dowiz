#!/usr/bin/env bash
# guard-bash.sh — PreToolUse gate on Bash: the Bash lane must not bypass the edit-governance gates.
#
# 2026-07-02 (P0 gate-rearm): re-registered after removal in 43a018c1. The old version blocked ALL
# fly deploys — which broke Ship Discipline (staging deploys are the agent's job) and is why it was
# unregistered, leaving the entire Bash lane ungoverned: a heredoc / sed -i / node script via Bash
# could mutate every protect-paths zone unseen. This version:
#   1. mirrors protect-paths.sh for Bash MUTATIONS of protected zones (read-only access stays free);
#   2. makes the gate-override state files HUMAN-ONLY (the agent must never write its own bypass);
#   3. blocks irreversible git commands (push to main / force-push / hard-reset to origin);
#   4. blocks PROD fly deploys (CI/human-only) while explicitly allowing staging deploys;
#   5. blocks dependency mutations (pnpm add/remove — lockfile is a protected zone).
# Blocking = exit 2 (same convention as protect-paths.sh). Fail-open on parse errors.
set -uo pipefail

INPUT=$(cat)

# Parse JSON without requiring jq (jq not available on all platforms, e.g. Windows)
_extract_cmd() {
  if command -v jq &>/dev/null; then
    echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null
  elif command -v python3 &>/dev/null; then
    echo "$INPUT" | python3 -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('tool_input', {}).get('command', ''), end='')
except Exception:
    pass
" 2>/dev/null || true
  elif command -v node &>/dev/null; then
    echo "$INPUT" | node -e "
let d = '';
process.stdin.on('data', c => d += c);
process.stdin.on('end', () => {
  try { const o = JSON.parse(d); process.stdout.write(o?.tool_input?.command || ''); } catch (e) {}
});
" 2>/dev/null || true
  fi
}

CMD=$(_extract_cmd)
[ -z "$CMD" ] && exit 0

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"

# P1 telemetry (meta-loop 2026-07-02): one JSONL line per decision — the advisory layer was
# unmeasurable (no lesson hit-counts, no deny/allow rates), so pruning/promotion ran blind.
# Advisory: never fails the hook.
HEV_LOG="$ROOT/.claude/logs/harness-events.jsonl"
_hev() {
  mkdir -p "$(dirname "$HEV_LOG")" 2>/dev/null || true
  printf '{"ts":"%s","hook":"%s","event":"%s","target":"%s","detail":"%s"}\n' \
    "$(date -Iseconds)" "$1" "$2" \
    "$(printf '%s' "${3:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    "$(printf '%s' "${4:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    >>"$HEV_LOG" 2>/dev/null || true
}


_block() {
  _hev guard-bash block "$(printf '%s' "$CMD" | cut -c1-160)" "$1"
  echo "BLOCKED (guard-bash): $1" >&2
  exit 2
}

# Mutating shell constructs. Harmless stderr/null redirects (2>&1, >/dev/null) are scrubbed first
# so running/inspecting a hook with them does not false-positive.
SCRUBBED=$(printf '%s' "$CMD" | sed -E 's/[0-9]*>+[[:space:]]*(&[0-9]+|\/dev\/null)//g')
MUTATORS='(^|[|;&][[:space:]]*|[[:space:]])(sed[[:space:]]+-[a-zA-Z]*i|tee([[:space:]]|$)|mv[[:space:]]|cp[[:space:]]|rm[[:space:]]|touch[[:space:]]|truncate[[:space:]]|chmod[[:space:]]|chown[[:space:]]|ln[[:space:]]|dd[[:space:]]|install[[:space:]]|perl[[:space:]]+-[a-zA-Z]*i)|>{1,2}'
mutates=0
printf '%s' "$SCRUBBED" | grep -qE "$MUTATORS" && mutates=1

# --- 1) HUMAN-ONLY gate-override files: the agent must never write its own bypass ---
OVERRIDES='\.claude/state/(serious-override|redline-confirmed)'
if [ "$mutates" -eq 1 ] && printf '%s' "$CMD" | grep -qE "$OVERRIDES"; then
  _block "'.claude/state/{serious-override,redline-confirmed}' are HUMAN-ONLY release files. The human types the override themselves (e.g. via '! <command>'). An agent writing its own gate bypass defeats the gate."
fi

# --- 2) protect-paths parity: Bash must not mutate what Edit/Write cannot ---
# (.claude/state and .claude/logs stay agent-writable — the council flow appends serious-cleared
# there by design; the clearance itself is TTL'd by serious-gate.sh.)
PROTECTED='\.claude/(hooks|commands|agents|settings)|\.github/|(^|[^A-Za-z0-9_])migrations/|fly\.toml|Dockerfile|pnpm-lock\.yaml|packages/(db|shared-types)/|/contracts/|\.contract\.|(^|[^A-Za-z0-9_])\.env([^A-Za-z0-9_]|$)'
if [ "$mutates" -eq 1 ] && printf '%s' "$CMD" | grep -qE "$PROTECTED"; then
  _block "Bash mutation touching a protect-paths zone (hooks/settings/agents/.github/migrations/infra/db/contracts/.env). These require manual human approval — same rule as the Edit/Write gate (protect-paths.sh). Read-only access is fine without redirects."
fi

# --- 3) irreversible git: push to main / force-push / hard-reset to origin ---
if printf '%s' "$CMD" | grep -qiE 'git[[:space:]]+push[[:space:]]+[^|;&]*(--force|-f[[:space:]]|[[:space:]](main|master)([[:space:]]|$|:))'; then
  _block "git push to main / force-push is human+CI territory (CI deploys prod on main). Push the feature branch instead."
fi
if printf '%s' "$CMD" | grep -qiE 'git[[:space:]]+reset[[:space:]]+--hard[[:space:]]+origin'; then
  _block "git reset --hard origin/* discards local work irreversibly. Manual only."
fi
if printf '%s' "$CMD" | grep -qE '(^|[|;&][[:space:]]*|[[:space:]])rm[[:space:]]+-[a-zA-Z]*r[a-zA-Z]*[[:space:]]+/([[:space:]*]|$)'; then
  _block "recursive delete at filesystem root. No."
fi

# --- 4) PROD deploy is CI/human-only; staging deploys are the agent's job (Ship Discipline) ---
if printf '%s' "$CMD" | grep -qiE 'fly(ctl)?[[:space:]]+deploy' && ! printf '%s' "$CMD" | grep -qi 'staging'; then
  _block "prod fly deploy is CI-on-main / human-only. Staging: flyctl deploy -a dowiz-staging --remote-only (allowed)."
fi
if printf '%s' "$CMD" | grep -qiE 'fly(ctl)?[[:space:]]+secrets[[:space:]]+(set|unset)' && ! printf '%s' "$CMD" | grep -qi 'staging'; then
  _block "prod fly secrets mutation is human-only."
fi

# --- 5) dependency mutations: lockfile is a protected zone ---
if printf '%s' "$CMD" | grep -qE '(^|[|;&][[:space:]]*|[[:space:]])(pnpm[[:space:]]+(add|remove)[[:space:]]|npm[[:space:]]+(install|i)[[:space:]]+[^-[:space:]]|yarn[[:space:]]+add[[:space:]])'; then
  _block "dependency add/remove mutates pnpm-lock.yaml (protected). New deps go through the council/human (plain 'pnpm install' restore is allowed)."
fi

exit 0
