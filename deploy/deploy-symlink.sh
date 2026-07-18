#!/usr/bin/env bash
# deploy-symlink.sh — P45-W1 deploy path (BLUEPRINT-P45 §4a.1, OPS-13).
#
# Atomic release: build -> local health-check (fail-closed) -> ln -sfn swap
# (one rename() syscall, no half-deployed state) -> POST_SWAP_WATCH_S watch
# window; >=3 health failures inside it => auto-rollback + S0 page.
#
# Rollback is ONE command runnable over SSH from a phone:
#   ./deploy/rollback.sh <prev-sha>
#
# Fail-closed: a health-check failure at swap time leaves prod UNTOUCHED and
# emits an S1 "deploy refused" alert. No auto-deploy touches live state without
# an explicit human `deploy` invocation (D5-F2 lesson — never reintroduce it).
#
# Secrets: none read from here. TELEGRAM_* are injected by CI / the box env;
# if absent the script degrades to local exit codes + stderr (never panics).
#
# Self-reaping: any child we spawn is killed on exit so the script never leaves
# an orphaned listener holding a port/pipe.
set -euo pipefail

# Reap our own background children on any exit.
_cleanup() { [[ -n "${_test_pid:-}" ]] && kill "$_test_pid" 2>/dev/null || true; }
trap _cleanup EXIT

RELEASES_DIR="${RELEASES_DIR:-/var/lib/deliveryos/releases}"
CURRENT_LINK="${RELEASES_DIR}/current"
HEALTHZ_URL="${HEALTHZ_URL:-http://127.0.0.1:8080/healthz}"
BIN_NAME="native-spa-server"
RELEASES_KEPT_MIN="${RELEASES_KEPT_MIN:-5}"
POST_SWAP_WATCH_S="${POST_SWAP_WATCH_S:-30}"
RESTART_PAGE_THRESHOLD="${RESTART_PAGE_THRESHOLD:-3}"
LOG_DIR="${LOG_DIR:-/var/log/deliveryos}"

sha="${1:-}"
if [[ -z "$sha" ]]; then
  echo "usage: $0 <git-sha>" >&2
  exit 2
fi

BIN_PATH="${RELEASES_DIR}/${sha}/${BIN_NAME}"
if [[ ! -x "$BIN_PATH" ]]; then
  echo "deploy: binary not built at $BIN_PATH (run build first)" >&2
  exit 3
fi

mkdir -p "$LOG_DIR"
ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
log="${LOG_DIR}/deploy-${sha}-${ts}.log"

log() { echo "$*" | tee -a "$log"; }

alert() {
  local sev="$1" msg="$2"
  if [[ -n "${TELEGRAM_BOT_TOKEN:-}" && -n "${TELEGRAM_CHAT_ID:-}" ]]; then
    # Non-blocking: a failed/network-less send must never stall the deploy.
    ( curl -sS --max-time 5 "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
        --data-urlencode "chat_id=${TELEGRAM_CHAT_ID}" \
        ${OPS_ALERTS_TOPIC:+--data-urlencode "message_thread_id=${OPS_ALERTS_TOPIC}"} \
        --data-urlencode "text=${sev} ${msg}" >/dev/null 2>&1 || true ) &
  fi
  echo "[alert:${sev}] ${msg}" >&2
}

prev="$(readlink -f "$CURRENT_LINK" 2>/dev/null || true)"

# 2) Local pre-swap health-check: start the NEW binary on a throwaway port,
#    probe /healthz, then kill it. Refuses swap if the new build is broken.
TEST_PORT=$((8080 + $$ % 1000))
TEST_URL="http://127.0.0.1:${TEST_PORT}/healthz"
"$BIN_PATH" --root "${SPA_ROOT:-/var/www}" --port "$TEST_PORT" --bind 127.0.0.1 >/dev/null 2>&1 &
_test_pid=$!
for _ in $(seq 1 20); do
  if curl -fsS --max-time 1 "$TEST_URL" >/dev/null 2>&1; then break; fi
  sleep 0.25
done
if ! curl -fsS --max-time 2 "$TEST_URL" >/dev/null 2>&1; then
  kill "$_test_pid" 2>/dev/null || true; _test_pid=""
  wait "$_test_pid" 2>/dev/null || true
  alert "S1" "deploy refused: health-check failed for ${sha} (prod untouched)"
  exit 4
fi
kill "$_test_pid" 2>/dev/null || true; _test_pid=""
wait "$_test_pid" 2>/dev/null || true

# 3) Atomic swap — one rename() syscall, no half-deployed state.
ln -sfn "$RELEASES_DIR/$sha" "$CURRENT_LINK"
log "deploy: swapped current -> ${sha}"

# 4) Restart the service to pick up the new symlink target.
if command -v systemctl >/dev/null 2>&1; then
  systemctl restart native-spa-server || {
    alert "S0" "deploy: systemctl restart failed for ${sha} — rolling back"
    if [[ -n "$prev" ]]; then ln -sfn "$prev" "$CURRENT_LINK"; systemctl restart native-spa-server || true; fi
    exit 5
  }
fi

# 5) Post-swap watch window — catch crash-loops a swap-time gate is blind to.
restarts=0
for _ in $(seq 1 "$POST_SWAP_WATCH_S"); do
  if ! curl -fsS --max-time 1 "$HEALTHZ_URL" >/dev/null 2>&1; then
    restarts=$((restarts + 1))
    if [[ $restarts -ge $RESTART_PAGE_THRESHOLD ]]; then
      alert "S0" "deploy: ${restarts} health failures within ${POST_SWAP_WATCH_S}s for ${sha} — AUTO-ROLLBACK"
      if [[ -n "$prev" ]]; then
        ln -sfn "$prev" "$CURRENT_LINK"
        systemctl restart native-spa-server || true
        alert "S0" "deploy: auto-rolled-back to $(basename "$prev")"
      fi
      exit 6
    fi
  else
    restarts=0
  fi
  sleep 1
done

# 6) Prune old releases, keep >= RELEASES_KEPT_MIN.
if [[ -d "$RELEASES_DIR" ]]; then
  ls -1dt "$RELEASES_DIR"/*/ 2>/dev/null | tail -n +$((RELEASES_KEPT_MIN + 1)) | while read -r old; do
    rm -rf "$old" || true
  done
fi

alert "S2" "deploy: ${sha} live and healthy (post-swap watch passed)"
log "deploy: ${sha} complete"
exit 0
