#!/usr/bin/env bash
# ROLL THE WORKER BACK -- AFTER EVERY VENUE'S KV IMAGES ARE v2 AGAIN (W-ATOMIC row 3, 2026-10-06).
#
#   bash tools/deploy/rollback.sh <version-id> [message]    # compact + pin every venue, then switch
#   bash tools/deploy/rollback.sh --unpin                   # lift the pin (after a later forward deploy)
#
# WHY A STEP BEFORE THE SWITCH. W-DELTA's catalogue saves write bebop-store v3 images (a delta chain), and a
# Worker built before W-DELTA REFUSES v3. Switching first would leave every venue that saved its menu since the
# deploy with a catalogue the old Worker cannot read. So:
#   1 COMPACT  POST <platform>/api/platform/compact as a platform administrator (workers/api/src/hubdo/compact/fan.rs):
#              every venue's KV images rewritten v2 AND pinned v2 -- each object compacts any later KV write itself,
#              so the new Worker cannot write a fresh delta in the seconds before the switch. Anything but 200 (a
#              venue failed, or the route is missing) -> exit 50 and NO switch.
#   2 SWITCH   wrangler rollback <version-id> -m <message>
# ROLLBACK_SKIP_COMPACT=1 skips step 1 and says so: only for a target build that reads v3 itself.
#
# The administrator: ROLLBACK_ADMIN_TOKEN, or ADMIN_EMAIL/ADMIN_PASSWORD from $ROLLBACK_ADMIN_FILE signed in at
# <platform>/api/auth/login. The token file is sourced only in the subshell that runs wrangler, never printed.
#
# EXIT: 0 ok | 2 usage | 50 compact failed | 51 no administrator token | 52 wrangler rollback failed
# Overrides (rollback.test.sh): ROLLBACK_PLATFORM ROLLBACK_CURL ROLLBACK_WRANGLER ROLLBACK_TOKEN_FILE
# ROLLBACK_ADMIN_FILE ROLLBACK_REPO.
set -u
REPO=${ROLLBACK_REPO:-/root/dowiz}
PLATFORM=${ROLLBACK_PLATFORM:-https://dowiz.org}
CURL=${ROLLBACK_CURL:-curl}
WRANGLER=${ROLLBACK_WRANGLER:-node --no-warnings $REPO/workers/node_modules/wrangler/wrangler-dist/cli.js}
TOKEN_FILE=${ROLLBACK_TOKEN_FILE:-/root/.cf_deploy_token}
ADMIN_FILE=${ROLLBACK_ADMIN_FILE:-/root/.dowiz_admin}
# Written once every venue is pinned v2; deploy.sh lifts the pin (rollback.sh --unpin) after its next
# successful forward deploy and removes this file (main's decision 2026-10-07).
PIN_MARK=${ROLLBACK_PIN_MARK:-/root/.cache/dowiz-deploy/kv-pinned}
WORK=$(mktemp -d "${TMPDIR:-/tmp}/rollback.XXXXXX"); trap 'rm -rf "$WORK"' EXIT

fail() { echo "rollback: FAIL $2"; exit "$1"; }
case "${1:-}" in
  "") echo "usage: rollback.sh <version-id> [message] | --unpin"; exit 2 ;;
  --unpin) MODE=unpin ;;
  -*) echo "usage: rollback.sh <version-id> [message] | --unpin"; exit 2 ;;
  *) MODE=rollback; VERSION=$1; MSG=${2:-rollback.sh} ;;
esac

json_field() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get(sys.argv[2], ""))' "$1" "$2" 2>/dev/null; }

admin_token() {
  [ -n "${ROLLBACK_ADMIN_TOKEN:-}" ] && { echo "$ROLLBACK_ADMIN_TOKEN"; return; }
  [ -r "$ADMIN_FILE" ] || return 1
  ( set -a; . "$ADMIN_FILE"; set +a
    python3 -c 'import json,os; print(json.dumps({"email": os.environ.get("ADMIN_EMAIL",""), "password": os.environ.get("ADMIN_PASSWORD","")}))' \
      > "$WORK/login.json" )
  code=$($CURL -sS -o "$WORK/login.out" -w '%{http_code}' -X POST -H 'content-type: application/json' \
    --data-binary @"$WORK/login.json" "$PLATFORM/api/auth/login") || return 1
  rm -f "$WORK/login.json"
  [ "$code" = 200 ] || return 1
  json_field "$WORK/login.out" access_token
}

compact() { # compact <query>
  TOKEN=$(admin_token); [ -n "$TOKEN" ] || fail 51 "admin: no platform administrator token (ROLLBACK_ADMIN_TOKEN, or ADMIN_EMAIL/ADMIN_PASSWORD in $ADMIN_FILE)"
  code=$($CURL -sS -o "$WORK/compact.out" -w '%{http_code}' -X POST -H "authorization: Bearer $TOKEN" \
    "$PLATFORM/api/platform/compact$1"); rc=$?
  echo "   compact$1: HTTP $code, failed venues: $(json_field "$WORK/compact.out" failed)"
  [ $rc = 0 ] && [ "$code" = 200 ] || { sed 's/^/   | /' "$WORK/compact.out" | head -20
    fail 50 "compact: $PLATFORM/api/platform/compact$1 answered ${code:-nothing} (curl rc=$rc); the Worker was NOT switched"; }
}

if [ $MODE = unpin ]; then
  echo "== unpin"; compact "?pin=0"; rm -f "$PIN_MARK"; echo "rollback: UNPINNED every venue"; exit 0
fi

echo "== compact (every venue's KV images to v2, pinned)"
if [ "${ROLLBACK_SKIP_COMPACT:-}" = 1 ]; then
  echo "   SKIPPED by ROLLBACK_SKIP_COMPACT=1 -- a target built before W-DELTA will REFUSE any v3 catalogue"
else
  compact ""; mkdir -p "$(dirname "$PIN_MARK")" && date -u +%FT%TZ > "$PIN_MARK"
fi
echo "== switch to $VERSION"
( cd "$REPO/workers/api" && set -a && . "$TOKEN_FILE" && set +a && $WRANGLER rollback "$VERSION" -m "$MSG" ) || fail 52 "switch: wrangler rollback $VERSION failed"
echo "rollback: OK -> $VERSION"
