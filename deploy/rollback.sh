#!/usr/bin/env bash
# rollback.sh — P45-W1 one-command rollback (BLUEPRINT-P45 §4a.1).
#
# Runnable over SSH from a phone. Re-points the `current` symlink at a previous
# release sha and restarts the service. Fail-closed: refuses if the target sha
# release dir is absent (you cannot roll back to something never deployed).
set -euo pipefail

RELEASES_DIR="${RELEASES_DIR:-/var/lib/deliveryos/releases}"
CURRENT_LINK="${RELEASES_DIR}/current"

prev="${1:-}"
if [[ -z "$prev" ]]; then
  echo "usage: $0 <prev-sha>" >&2
  echo "available releases:" >&2
  ls -1 "$RELEASES_DIR" 2>/dev/null | grep -v '^current$' >&2 || true
  exit 2
fi

target="${RELEASES_DIR}/${prev}"
if [[ ! -d "$target" ]]; then
  echo "rollback: release dir not found: $target" >&2
  exit 3
fi

ln -sfn "$target" "$CURRENT_LINK"
echo "rollback: current -> ${prev}"

if command -v systemctl >/dev/null 2>&1; then
  systemctl restart native-spa-server || { echo "rollback: restart failed" >&2; exit 4; }
fi

# Verify the health endpoint responds after rollback.
HEALTHZ_URL="${HEALTHZ_URL:-http://127.0.0.1:8080/healthz}"
for _ in $(seq 1 10); do
  if curl -fsS --max-time 1 "$HEALTHZ_URL" >/dev/null 2>&1; then
    echo "rollback: ${prev} healthy"
    exit 0
  fi
  sleep 0.5
done
echo "rollback: WARN health endpoint not reachable after rollback" >&2
exit 5
