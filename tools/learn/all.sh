#!/bin/sh
# The one command: every lesson that lacks an up-to-date video, one at a time, end to end:
# capture -> assemble (sq, en, uk) -> check.sh -> publish into the staging tree -> R2.
#
#   sh tools/learn/all.sh --out DIR [--only W3,O1a] [--role owner] [--lang sq,en,uk] [--ui live|local]
#                         [--dry-run] [--no-r2] [--no-retry]
#
# Run it through the slot:  bash bebop-lang/tools/slot.sh learn-all sh tools/learn/all.sh --out DIR
# ORDER, STATE, ESTIMATE: all-plan.mjs (tested). Read-only lessons first. DIR/.state.json records
# each lesson done/failed; a killed run started again skips what is done (and whose YAML and
# recorder did not change) and carries on. A lesson that fails is logged, marked failed, and the
# run moves on; the failed ones are tried once more at the end. Each finished lesson goes to R2
# at once (publish.sh --r2 uploads only what changed). A lesson that writes is recorded with every
# mutating call blocked; capture.mjs still compares the venue before and after and closes what is
# new -- its exit code is the count left open, and a non-zero one fails the lesson.
# Log: one line per stage, `all: <id> ...`, and `progress: n/N done` after every lesson.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
OUT= FILTER= LANGS=sq,en,uk UI=live DRY=0 R2=1 RETRY=1
while [ $# -gt 0 ]; do
  case "$1" in
    --out) OUT=$2; shift 2 ;;
    --only|--role) FILTER="$FILTER $1 $2"; shift 2 ;;
    --lang) LANGS=$2; shift 2 ;; --ui) UI=$2; shift 2 ;;
    --dry-run) DRY=1; shift ;; --no-r2) R2=0; shift ;; --no-retry) RETRY=0; shift ;;
    *) echo "all.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "$OUT" ] || { echo "usage: all.sh --out DIR [--only IDS] [--role ROLE] [--lang LANGS] [--ui live|local] [--dry-run] [--no-r2] [--no-retry]" >&2; exit 2; }
mkdir -p "$OUT"
# The cached wrangler 4, run directly: half the time of `npx wrangler@4` per object (publish.mjs).
[ -n "${LEARN_WRANGLER:-}" ] || LEARN_WRANGLER=$(ls -d "$HOME"/.npm/_npx/*/node_modules/wrangler/bin/wrangler.js 2>/dev/null | head -1)
export LEARN_WRANGLER
PLAN="node $HERE/all-plan.mjs --out $OUT $FILTER --lang $LANGS"
if [ $DRY = 1 ]; then $PLAN --plan; exit $?; fi

one() {   # one lesson, every stage; answers 0 or the name of the stage that failed
  id=$1
  h=$(node "$HERE/all-plan.mjs" --out "$OUT" --capture-hash "$id") || { echo "no lesson file"; return 1; }
  need=0
  [ "$(cat "$OUT/$id/.captured" 2>/dev/null)" = "$h" ] || need=1
  for l in $(echo "$LANGS" | tr ',' ' '); do [ -f "$OUT/$id/$l/marks.json" ] || need=1; done
  if [ $need = 1 ]; then
    rm -f "$OUT/$id/.captured"
    node "$HERE/capture.mjs" "$id" --out "$OUT/$id" --lang "$LANGS" --ui "$UI" > "$OUT/$id.capture.log" 2>&1
    rc=$?
    grep -E "ABSENT|blocked|wrote|artefacts|still open|OPEN|capture:|anchors found" "$OUT/$id.capture.log" | sed "s/^/  $id /"
    [ $rc -eq 0 ] || { echo "capture rc=$rc"; return 1; }
    echo "$h" > "$OUT/$id/.captured"
  fi
  for l in $(echo "$LANGS" | tr ',' ' '); do
    sh "$HERE/assemble.sh" "$OUT/$id" "$l" > "$OUT/$id.assemble-$l.log" 2>&1 || { tail -2 "$OUT/$id.assemble-$l.log"; echo "assemble $l"; return 1; }
  done
  sh "$HERE/check.sh" "$OUT/$id" $(echo "$LANGS" | tr ',' ' ') > "$OUT/$id.check.log" 2>&1 || { grep -E '^FAIL' "$OUT/$id.check.log" | head -5; echo "check"; return 1; }
  grep -E '^WARN' "$OUT/$id.check.log" | sed "s/^/  $id /"
  node "$HERE/publish.mjs" "$OUT" "$id" > "$OUT/$id.publish.log" 2>&1 || { cat "$OUT/$id.publish.log"; echo "publish"; return 1; }
  if [ $R2 = 1 ]; then
    sh "$HERE/publish.sh" --r2 > "$OUT/$id.r2.log" 2>&1
    rc=$?
    [ $rc -eq 0 ] || { tail -4 "$OUT/$id.r2.log"; echo "r2 rc=$rc"; return 1; }
    grep -E '^r2: [0-9]+ uploaded' "$OUT/$id.r2.log" | sed "s/^/  $id /"
  fi
  return 0
}

pass() {  # $1 = extra all-plan flags (--retry)
  for id in $($PLAN --list $1); do
    t0=$(date +%s)
    res=$(one "$id" 2>&1); rc=$?
    [ -n "$res" ] && printf '%s\n' "$res"
    why=$(printf '%s\n' "$res" | tail -1)
    if [ $rc -eq 0 ]; then node "$HERE/all-plan.mjs" --out "$OUT" --mark "$id" done
    else node "$HERE/all-plan.mjs" --out "$OUT" --mark "$id" failed "$why"; fi
    echo "all: $id $( [ $rc -eq 0 ] && echo done || echo "FAILED at $why" ) in $(( $(date +%s) - t0 )) s"
    $PLAN --summary
  done
}

pass ""
[ $RETRY = 1 ] && { echo "all: retrying the failed ones once"; pass --retry; }
$PLAN --summary
left=$($PLAN --list | wc -l)
echo "all: finished, $left lesson(s) still without an up-to-date video"
exit "$left"
