#!/bin/bash
# Cross-agent fallback mesh: when Claude Code / the Claude API is unreachable,
# tries the other installed agents in order — Hermes -> OpenCode -> Goose ->
# Aider -> OpenHands — until one picks up the task, so a single agent or
# provider outage never stalls work on dowiz.
#
# Shared credential pool: ~/.hermes/.env is sourced before every attempt, so
# adding ONE free-tier key there (GOOGLE_API_KEY / OPENROUTER_API_KEY) makes
# it available to every agent in the mesh, not just Hermes.
#
# Safety model — read before changing MESH_ALLOW_AUTO_APPROVE:
#   Every attempt runs with stdin closed (< /dev/null) and a hard timeout. A
#   tool that would need an interactive dangerous-command approval gets
#   instant EOF and fails through to the next agent — nothing is silently
#   auto-approved by default, matching this repo's never-bypass-human-gates
#   rule. Aider's one-shot mode and OpenHands's headless mode are structurally
#   unable to run at all without an auto-approve flag (--yes-always /
#   headless-always-approves) — they're skipped by default, not silently
#   downgraded to yolo. Set MESH_ALLOW_AUTO_APPROVE=1 for one deliberately
#   unattended run if you want them included; this is a per-run opt-in, not a
#   standing default, same posture as HERMES_FALLBACK_YOLO in
#   scripts/hermes-fallback.sh.
#
# Usage:
#   scripts/agents-mesh.sh "finish the checkout idempotency test"
#   scripts/agents-mesh.sh --dry-run "same task"     # show the order without running anything
#   MESH_ALLOW_AUTO_APPROVE=1 scripts/agents-mesh.sh "unattended cron-style task"

set -uo pipefail

REPO_ROOT="/root/dowiz"
cd "$REPO_ROOT" || exit 1

# Shared credential pool
if [ -f "$HOME/.hermes/.env" ]; then
  set -a
  # shellcheck disable=SC1091
  . "$HOME/.hermes/.env"
  set +a
fi

DRY_RUN=0
if [ "${1:-}" = "--dry-run" ]; then
  DRY_RUN=1
  shift
fi
TASK="${1:?Usage: $0 [--dry-run] \"<task description>\"}"

AUTO_APPROVE="${MESH_ALLOW_AUTO_APPROVE:-0}"
TIMEOUT_S="${MESH_TIMEOUT_SECONDS:-90}"
LOG="$REPO_ROOT/.agents-mesh.log"

have_key() {
  for v in "$@"; do
    [ -n "${!v:-}" ] && return 0
  done
  return 1
}

log() { echo "[mesh] $*" | tee -a "$LOG"; }

run_with_guard() {
  local label="$1"
  shift
  if [ "$DRY_RUN" = "1" ]; then
    log "(dry-run) would try $label: $*"
    return 2
  fi
  timeout "${TIMEOUT_S}" "$@" </dev/null
}

try_hermes() {
  have_key ANTHROPIC_API_KEY GOOGLE_API_KEY GEMINI_API_KEY OPENROUTER_API_KEY || return 1
  run_with_guard hermes hermes chat -q "$TASK" --checkpoints --source tool
}

try_opencode() {
  have_key ANTHROPIC_API_KEY OPENAI_API_KEY OPENROUTER_API_KEY GOOGLE_GENERATIVE_AI_API_KEY GEMINI_API_KEY \
    || [ -f "$HOME/.local/share/opencode/auth.json" ] || return 1
  run_with_guard opencode opencode run "$TASK"
}

try_goose() {
  have_key OPENAI_API_KEY ANTHROPIC_API_KEY GOOGLE_API_KEY OPENROUTER_API_KEY || return 1
  run_with_guard goose goose run --no-session -t "$TASK"
}

try_aider() {
  have_key ANTHROPIC_API_KEY OPENAI_API_KEY OPENROUTER_API_KEY GEMINI_API_KEY DEEPSEEK_API_KEY || return 1
  local extra=()
  if [ "$AUTO_APPROVE" = "1" ]; then
    extra+=(--yes-always)
  else
    log "aider one-shot mode needs --yes-always to run without a TTY — skipping (set MESH_ALLOW_AUTO_APPROVE=1 to include it)"
    return 1
  fi
  run_with_guard aider aider --message "$TASK" "${extra[@]}"
}

try_openhands() {
  have_key LLM_API_KEY ANTHROPIC_API_KEY OPENAI_API_KEY || [ -f "$HOME/.openhands/agent_settings.json" ] || return 1
  if [ "$AUTO_APPROVE" != "1" ]; then
    log "openhands headless mode always auto-approves actions (no non-yolo headless mode exists) — skipping (set MESH_ALLOW_AUTO_APPROVE=1 to include it)"
    return 1
  fi
  run_with_guard openhands openhands --headless -t "$TASK" --override-with-envs
}

ORDER=(hermes opencode goose aider openhands)
for agent in "${ORDER[@]}"; do
  log "trying $agent..."
  "try_$agent"
  rc=$?
  if [ "$rc" = "0" ]; then
    log "$agent handled the task"
    exit 0
  elif [ "$rc" = "2" ]; then
    continue
  else
    log "$agent unavailable or failed (exit $rc) — falling through"
  fi
done

log "no agent in the mesh could run — add ANTHROPIC_API_KEY / GOOGLE_API_KEY / OPENROUTER_API_KEY to ~/.hermes/.env to activate at least one tier"
exit 1
