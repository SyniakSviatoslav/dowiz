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

# ── Mutation detection — TARGET-based (fable-audit finding #1) ───────────────
# The old gate greped the WHOLE command text for PROTECTED, so a commit message, banner, or curl
# URL that merely MENTIONED `.env`/`migrations/` next to any `>` false-positived (~83% FP rate). We
# now match PROTECTED/OVERRIDES against actual WRITE TARGETS only:
#   • SCRUBBED  = CMD minus harmless stderr/null redirects (2>&1, >/dev/null) — inspecting a hook
#                 with them attached never trips.
#   • SKELETON  = SCRUBBED with quoted strings removed — a quoted commit message / banner / prose
#                 contributes NO shell tokens (the core FP source).
#   • TARGETS   = per ';|&'-segment: redirect destinations (token after >/>>) + the path args of an
#                 actual mutator in command position (sed -i, tee, cp, mv, rm, truncate, touch,
#                 chmod, chown, ln, dd, install, perl -i). READ sources are NOT targets — so
#                 `cat protected > /tmp/x` and read-only `curl .../contracts/..` pass.
#   • Scratchpad /tmp/claude-* targets are whitelisted (the agent's own working dir).
# Inline-interpreter writes (python/node/ruby -c/-e / heredoc) carry the path IN the code string,
# not as a shell token, so for those we still match PROTECTED against the raw CMD.
SCRUBBED=$(printf '%s' "$CMD" | sed -E 's/[0-9]*>+[[:space:]]*(&[0-9]+|\/dev\/null)//g')
SKELETON=$(printf '%s' "$SCRUBBED" | sed -E "s/'[^']*'//g; s/\"[^\"]*\"//g")

MUTATOR_CMD='(^|[[:space:]])(sed[[:space:]]+-[a-zA-Z]*i|perl[[:space:]]+-[a-zA-Z]*i|tee([[:space:]]|$)|mv[[:space:]]|cp[[:space:]]|rm[[:space:]]|touch[[:space:]]|truncate[[:space:]]|chmod[[:space:]]|chown[[:space:]]|ln[[:space:]]|dd[[:space:]]|install[[:space:]])'
shell_mut=0
printf '%s' "$SKELETON" | grep -qE "$MUTATOR_CMD|>{1,2}" && shell_mut=1

# Collect write targets, segment by segment (so a read-source in one segment is not attributed to a
# mutator in another). Process substitution keeps TARGETS in this shell.
TARGETS=""
while IFS= read -r seg; do
  [ -z "$seg" ] && continue
  reds=$(printf '%s' "$seg" | grep -oE '>>?[[:space:]]*[^[:space:]<>]+' | sed -E 's/^[[:space:]]*>>?[[:space:]]*//')
  margs=""
  if printf '%s' "$seg" | grep -qE "$MUTATOR_CMD"; then
    margs=$(printf '%s' "$seg" | tr ' \t' '\n' | grep -E '/|^\.[A-Za-z]' || true)
  fi
  TARGETS="$TARGETS
$reds
$margs"
done < <(printf '%s\n' "$SKELETON" | tr ';|&' '\n')
# Whitelist the agent scratchpad (its own working dir); everything else stays checkable.
TARGETS=$(printf '%s' "$TARGETS" | grep -v '/tmp/claude-' || true)

# Inline-interpreter file-writes: only an ACTUAL inline-exec context (python/ruby -c/-e, node -e,
# deno eval, or a heredoc) + a clear file-mutation op counts — prose that merely mentions
# `write_text` does not (it lacks -c/<<). The path literal is in the code, so match the raw CMD.
INTERP_WRITE='(python[0-9.]*[[:space:]]+-[A-Za-z]*c|node[[:space:]]+-e|ruby[[:space:]]+-e|deno[[:space:]]+eval|<<[^<|;&]*[[:alnum:]_]).*(write_text|write_bytes|writeFileSync|appendFileSync|os\.(remove|unlink|rename|makedirs)|shutil\.(move|copy|rmtree)|fs\.(writeFile|appendFile|unlink|rm|rename|mkdir)|open\([^)]*,[^)]*[aw])'
interp=0
printf '%s' "$CMD" | grep -qE "$INTERP_WRITE" && interp=1

OVERRIDES='\.claude/state/(serious-override|redline-confirmed|fable-override)'
# --- 1) HUMAN-ONLY gate-override files: the agent must never write its own bypass ---
if { [ "$shell_mut" -eq 1 ] && printf '%s' "$TARGETS" | grep -qE "$OVERRIDES"; } \
   || { [ "$interp" -eq 1 ] && printf '%s' "$CMD" | grep -qE "$OVERRIDES"; }; then
  _block "'.claude/state/{serious-override,redline-confirmed,fable-override}' are HUMAN-ONLY release files. The human types the override themselves (e.g. via '! <command>'). An agent writing its own gate bypass defeats the gate."
fi

# --- 2) protect-paths parity: Bash must not mutate what Edit/Write cannot ---
# (.claude/state and .claude/logs stay agent-writable by design — state files are TTL'd by
# the gates that own them.)
PROTECTED='\.github/|(^|[^A-Za-z0-9_])migrations/|fly\.toml|Dockerfile|pnpm-lock\.yaml|packages/(db|shared-types)/|/contracts/|\.contract\.|(^|[^A-Za-z0-9_])\.env([^A-Za-z0-9_]|$)'
if { [ "$shell_mut" -eq 1 ] && printf '%s' "$TARGETS" | grep -qE "$PROTECTED"; } \
   || { [ "$interp" -eq 1 ] && printf '%s' "$CMD" | grep -qE "$PROTECTED"; }; then
  _block "Bash mutation WRITING a protect-paths zone (.github/migrations/infra/db/contracts/.env). These require manual human approval — same rule as the Edit/Write gate (protect-paths.sh). Read-only access (cat/grep/curl) is fine, including piping a protected file's contents to /tmp/claude-*."
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
  _block "dependency add/remove mutates pnpm-lock.yaml (protected). New deps go through the human (plain 'pnpm install' restore is allowed)."
fi

exit 0
