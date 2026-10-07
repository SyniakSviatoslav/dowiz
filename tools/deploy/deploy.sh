#!/usr/bin/env bash
# DEPLOY THE dowiz WORKER FROM main's HEAD, AND PROVE THE LIVE CODE IS THAT COMMIT (W-DEPLOY 2026-09-30).
#
#   bash tools/deploy/deploy.sh             # the real deploy (main session / operator)
#   bash tools/deploy/deploy.sh --dry-run   # everything except the upload: builds, and greps the wasm for HEAD
#   bash tools/deploy/deploy.sh --verify    # only step 6 against what is live now (no build, no upload)
#
# WHAT IT REPLACES. /root/dowiz-deploy.sh ran `wrangler deploy` in the WORKING TREE (uncommitted lane
# merges shipped), beside whatever else was running (the box's 32-process cap killed it), and nobody
# checked that the live code was the commit. Each step below closes one of those:
#   1 REFUSE   HEAD must be the branch `main`, and the tracked tree must equal HEAD (staged or not).
#   2 COPY     $WORK/src := HEAD's tree exactly (tools/deploy/sync.py: checkout-index, rewritten only where a
#              blob changed so cargo keeps its build, re-hashed against HEAD, extras deleted). Everything
#              after this runs in the copy; the working tree is never read again.
#   3 GATES    tools/gates/run-all.sh (the no-compiler table: every *.prove.sh + gate) and
#              `cargo test --lib` in workers/api, on the copy. ALL of run-all rather than a subset: it is the
#              table CI and lanes already read, it takes minutes, and a subset is a second list to keep in step.
#   4 PREVIOUS `wrangler deployments status --json` -> the version serving 100 %, recorded BEFORE the upload
#              so the rollback command can be printed. No previous id, no deploy.
#   5 UPLOAD   `wrangler deploy` with DOWIZ_COMMIT/DOWIZ_BUILT_AT in the build's environment: they are compiled
#              into the wasm (workers/api/src/version.rs). Then the local wasm must CONTAIN the commit.
#   6 VERIFY   tools/deploy/verify.mjs: /api/version == HEAD, 3 admin JS files byte-equal to HEAD, then
#              e2e/kit-regression/_probe_crud.mjs on the QA hub. Any FAIL prints the exact rollback command.
#              It does NOT roll back by itself: a rollback is a production write a person decides.
# Every heavy step runs through bebop-lang/tools/slot.sh (one compute slot, waits under the process cap) --
# which is what stops the process-cap kill -- and every step prints its seconds.
#
# EXIT: 0 ok | 10 not main | 11 dirty tree | 12 copy | 20 run-all | 21 cargo test | 30 no previous version
#       | 31 upload / no Version ID | 32 wasm lacks the commit | 40 live != HEAD | 41 probe_crud | 42 flows (FLOWS_BLOCKING=0 makes it advisory)
#
# The token file is sourced ONLY inside the subshell that runs wrangler, and never printed.
# Overrides (tests use them to stub every external): DEPLOY_REPO DEPLOY_WORK DEPLOY_SLOT DEPLOY_WRANGLER
# DEPLOY_TOKEN_FILE DEPLOY_HOST DEPLOY_CARGO DEPLOY_VERIFY DEPLOY_PROBE DEPLOY_TOOLCHAIN DEPLOY_FILES.
set -u
MODE=deploy
case "${1:-}" in --dry-run) MODE=dry ;; --verify) MODE=verify ;; "") ;; *) echo "usage: deploy.sh [--dry-run|--verify]"; exit 2 ;; esac

REPO=${DEPLOY_REPO:-/root/dowiz}
WORK=${DEPLOY_WORK:-/root/.cache/dowiz-deploy}
SRC=$WORK/src
SLOT=${DEPLOY_SLOT:-bash $REPO/bebop-lang/tools/slot.sh}
WRANGLER=${DEPLOY_WRANGLER:-node --no-warnings $REPO/workers/node_modules/wrangler/wrangler-dist/cli.js}
TOKEN_FILE=${DEPLOY_TOKEN_FILE:-/root/.cf_deploy_token}
HOST=${DEPLOY_HOST:-https://qa-durres.dowiz.org}
CARGO=${DEPLOY_CARGO:-cargo}
TOOLCHAIN=${DEPLOY_TOOLCHAIN:-1.96.1-aarch64-unknown-linux-gnu}
FILES=${DEPLOY_FILES:-admin/app.js,admin/core.js,admin/i18n.js}
HERE=$(cd "$(dirname "$0")" && pwd)
VERIFY=${DEPLOY_VERIFY:-node $HERE/verify.mjs}
PROBE=${DEPLOY_PROBE:-node --import $SRC/e2e/kit-regression/_retry-fetch.mjs $SRC/e2e/kit-regression/_probe_crud.mjs}
export PATH="$HOME/.cargo/bin:$PATH" CARGO_TARGET_DIR="$WORK/target"
mkdir -p "$WORK/logs"

T0=$(date +%s)
step() { STEP=$1; TS=$(date +%s); echo "== $1"; }
took() { echo "   $STEP: $(( $(date +%s) - TS )) s (total $(( $(date +%s) - T0 )) s)"; }
fail() { echo "deploy: FAIL $2"; [ -n "${ROLLBACK:-}" ] && echo "deploy: to undo, run: $ROLLBACK"; exit "$1"; }

# ── 1 REFUSE ────────────────────────────────────────────────────────────────
step refuse
BR=$(git -C "$REPO" symbolic-ref -q HEAD) || fail 10 "not-main: HEAD is detached ($(git -C "$REPO" rev-parse --short HEAD)); deploy only from the branch main"
[ "$BR" = refs/heads/main ] || fail 10 "not-main: HEAD is $BR; deploy only from refs/heads/main"
SHA=$(git -C "$REPO" rev-parse HEAD) || fail 10 "not-main: no HEAD"
# `git diff` reads the index without rewriting it (`git status` would take index.lock under other lanes).
git -C "$REPO" diff --quiet HEAD -- || fail 11 "dirty-tree: tracked files differ from HEAD $SHA: $(git -C "$REPO" diff --name-only HEAD -- | head -5 | tr '\n' ' ')-- commit or drop them first"
git -C "$REPO" diff --cached --quiet || fail 11 "dirty-tree: the index differs from HEAD $SHA (staged changes)"
echo "   main @ $SHA ($(git -C "$REPO" log -1 --format=%s "$SHA" | cut -c1-70))"
took

if [ $MODE != verify ]; then
  # ── 2 COPY ────────────────────────────────────────────────────────────────
  step copy
  python3 "$HERE/sync.py" "$REPO" "$SRC" || fail 12 "copy: $SRC could not be made equal to HEAD"
  took

  # ── 3 GATES ───────────────────────────────────────────────────────────────
  # A commit whose gates AND cargo test already passed here is not re-gated: the copy was just re-hashed
  # equal to HEAD, so it is the very tree that passed. It saves 20-50 min when only the upload failed
  # (a slow uplink). DEPLOY_REGATE=1 runs them anyway.
  PASSED=$WORK/passed/$SHA
fi
if [ $MODE != verify ] && [ -f "$PASSED" ] && [ -z "${DEPLOY_REGATE:-}" ]; then
  step gates
  echo "   already passed on $SHA at $(cat "$PASSED") -- not re-run (DEPLOY_REGATE=1 re-runs them)"
  took
elif [ $MODE != verify ]; then
  step gates
  $SLOT deploy sh "$SRC/tools/gates/run-all.sh" > "$WORK/logs/gates.log" 2>&1; rc=$?
  grep -E '^(FAIL|run-all:)' "$WORK/logs/gates.log" | tail -12 | sed 's/^/   /'
  [ $rc = 0 ] || fail 20 "gates: tools/gates/run-all.sh rc=$rc on $SHA (log $WORK/logs/gates.log); nothing was uploaded"
  took
  step cargo-test
  ( cd "$SRC/workers/api" && RUSTUP_TOOLCHAIN=$TOOLCHAIN $SLOT deploy $CARGO test --lib ) > "$WORK/logs/cargo-test.log" 2>&1; rc=$?
  grep -E '^test result:' "$WORK/logs/cargo-test.log" | tail -3 | sed 's/^/   /'
  [ $rc = 0 ] || fail 21 "cargo-test: workers/api cargo test --lib rc=$rc on $SHA (log $WORK/logs/cargo-test.log); nothing was uploaded"
  mkdir -p "$WORK/passed" && date -u +%Y-%m-%dT%H:%M:%SZ > "$PASSED"
  took
fi

# The box's TCP connect to Cloudflare takes 5-16 s and wrangler's fetch (undici) gives up at 10 s, so any
# wrangler call fails at random after a good build: "fetch failed" (upload) or "The request to Cloudflare's
# API timed out" (deployments status). net_flake <log> says the log shows one of those two and nothing else
# is retried: an auth or build error fails at once.
net_flake() {
  NET_ERR=
  grep -q 'fetch failed' "$1" && NET_ERR="fetch failed (connect timeout)"
  grep -q "request to Cloudflare's API timed out" "$1" && NET_ERR="Cloudflare API timed out"
  [ -n "$NET_ERR" ]
}

wr() { # wr <wrangler args...> -- in the copy's workers/api, token sourced only in this subshell
  ( cd "$SRC/workers/api" && set -a && . "$TOKEN_FILE" && set +a && $WRANGLER "$@" )
}

if [ $MODE = deploy ]; then
  # ── 4 PREVIOUS ────────────────────────────────────────────────────────────
  step previous
  for try in 1 2 3; do
    wr deployments status --json > "$WORK/logs/previous.json" 2>&1; rc=$?
    [ $rc != 0 ] && [ $try -lt 3 ] && net_flake "$WORK/logs/previous.json" || break
    echo "   previous try $try: $NET_ERR, retrying"
  done
  PREV=$(python3 - "$WORK/logs/previous.json" <<'PY'
import json, sys
try: d = json.load(open(sys.argv[1]))
except Exception: sys.exit(0)
v = [x["version_id"] for x in d.get("versions", []) if x.get("percentage") == 100]
print(v[0] if len(v) == 1 else "")
PY
)
  [ $rc = 0 ] && [ -n "$PREV" ] || fail 30 "previous: could not read the version serving 100 % (rc=$rc, $WORK/logs/previous.json); refusing to deploy without a rollback target"
  # rollback.sh compacts every venue's KV images back to v2 BEFORE it switches (an older Worker refuses v3; W-ATOMIC).
  ROLLBACK="bash $REPO/tools/deploy/rollback.sh $PREV 'deploy.sh: $SHA failed verification'"
  echo "   serving now: $PREV"
  took
fi

if [ $MODE != verify ]; then
  # ── 5 UPLOAD (or the dry-run build) ───────────────────────────────────────
  step upload
  BUILT_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
  if [ $MODE = dry ]; then args="deploy --dry-run --outdir $WORK/dry"; else args=deploy; fi
  # Retried only on net_flake with no Version ID printed (the rebuild is cached); anything else fails at once.
  for try in 1 2 3; do
    ( cd "$SRC/workers/api" && export RUSTUP_TOOLCHAIN=$TOOLCHAIN DOWIZ_COMMIT=$SHA DOWIZ_BUILT_AT=$BUILT_AT &&
      if [ $MODE = dry ]; then $SLOT deploy $WRANGLER $args
      else $SLOT deploy bash -c 'set -a && . "$0" && set +a && exec "$@"' "$TOKEN_FILE" $WRANGLER $args; fi
    ) > "$WORK/logs/upload.log" 2>&1; rc=$?
    [ $rc != 0 ] && [ $try -lt 3 ] && net_flake "$WORK/logs/upload.log" \
      && ! grep -q 'Current Version ID' "$WORK/logs/upload.log" || break
    echo "   upload try $try: $NET_ERR, retrying"
  done
  tail -6 "$WORK/logs/upload.log" | sed 's/^/   /'
  [ $rc = 0 ] || fail 31 "upload: wrangler rc=$rc (log $WORK/logs/upload.log)"
  NEW=$(grep -o 'Current Version ID: [0-9a-f-]*' "$WORK/logs/upload.log" | tail -1 | cut -d' ' -f4)
  [ $MODE = dry ] || [ -n "$NEW" ] || fail 31 "upload: wrangler exited 0 but printed no 'Current Version ID' (log $WORK/logs/upload.log)"
  WASM=$SRC/workers/api/build/index_bg.wasm
  hits=$(grep -c -a "$SHA" "$WASM" 2>/dev/null); hits=${hits:-0}
  echo "   version: ${NEW:-dry-run}; $WASM carries $SHA: $hits"
  [ "$hits" -gt 0 ] || fail 32 "wasm-commit: the built $WASM does not contain $SHA -- /api/version will not say HEAD"
  took
  [ $MODE = dry ] && { echo "deploy: DRY-RUN OK $SHA (built, gated, not uploaded) in $(( $(date +%s) - T0 )) s"; exit 0; }
fi

# ── 6 VERIFY ──────────────────────────────────────────────────────────────
step verify
if [ $MODE = verify ]; then python3 "$HERE/sync.py" "$REPO" "$SRC" || fail 12 "copy: $SRC could not be made equal to HEAD"; fi
$VERIFY --host "$HOST" --commit "$SHA" --src "$SRC" --files "$FILES"; rc=$?
[ $rc = 0 ] || fail 40 "live-is-not-head: $HOST is not serving $SHA (see FAIL lines above). If the upload finished under a minute ago, run 'deploy.sh --verify' once before rolling back"
took
step probe-crud
HOST=$HOST $PROBE > "$WORK/logs/probe_crud.log" 2>&1; rc=$?
tail -3 "$WORK/logs/probe_crud.log" | sed 's/^/   /'
[ $rc = 0 ] || fail 41 "probe-crud: e2e/kit-regression/_probe_crud.mjs rc=$rc on $HOST (log $WORK/logs/probe_crud.log)"
# Browser flows (W-FLOWS): BLOCKING since 2026-10-02 (the rename-back bug was fixed in 4d2d4f33, and on 2026-10-02
# the advisory run found a live 500 on /api/live that every other step passed). FLOWS_BLOCKING=0 makes it advisory.
if [ "${DEPLOY_FLOWS:-1}" = 1 ]; then
  # probe_crud has just created and deleted a dish; the public menu is edge-cached for 30 s
  # (storefront.rs `max-age=30`), so F1 started at once counted the deleted dish (6 cards vs
  # 5 on sale, twice on 2026-10-02) and never reached the status page's socket. Let it expire.
  sleep "${FLOWS_SETTLE_S:-40}"
  bash "$SRC/tools/gates/flows.sh" > "$WORK/logs/flows.log" 2>&1; rc=$?
  grep '^flows:' "$WORK/logs/flows.log" | tail -1 | sed 's/^/   /'
  if [ $rc != 0 ]; then
    [ "${FLOWS_BLOCKING:-1}" = 1 ] && fail 42 "flows: the browser flows gate is RED on $HOST (log $WORK/logs/flows.log)"
    echo "   flows: ADVISORY red (rc=$rc), not blocking -- log $WORK/logs/flows.log"
  fi
fi
took
# A rollback pinned every venue's KV images to v2 (tools/deploy/rollback.sh); this build reads v3, so lift it.
# Advisory: a failure leaves the pin (catalogue saves stay compacted, slower, never wrong) and says so.
if [ $MODE != verify ] && [ -f "${ROLLBACK_PIN_MARK:-/root/.cache/dowiz-deploy/kv-pinned}" ]; then
  step unpin
  bash "$HERE/rollback.sh" --unpin 2>&1 | sed 's/^/   /'
  [ "${PIPESTATUS[0]}" = 0 ] || echo "   unpin: FAILED -- venues stay pinned v2; run 'bash tools/deploy/rollback.sh --unpin'"
fi
echo "deploy: OK $SHA live on $HOST${NEW:+ as $NEW}${PREV:+ (previous $PREV)} in $(( $(date +%s) - T0 )) s"
