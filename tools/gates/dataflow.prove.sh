#!/bin/sh
# R3's proof: the `dataflow` gate is triggered before it is trusted.
#
# Runs `dataflow.sh` against scratch copies of the Worker:
#   1. clean                                        -> must pass (exit 0)
#   2. one more `load_catalog(` on a request path   -> must refuse (exit 1)
#   3. one more `hubstore::load(`                   -> must refuse (exit 1)
#   4. the same call named only in a COMMENT        -> must pass (the epitaph trap)
#   5. a call after "https://" on the same line  -> must refuse (a string is not a comment)
#   6. the same call inside an allowlisted `seed_*` -> must pass
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0

copy() {
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api"
  cp -r "$REPO/workers/api/src" "$SCRATCH/r/workers/api/src"
}
run() { sh "$HERE/dataflow.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; echo $?; }
add() { printf '%s\n' "$1" >> "$SCRATCH/r/workers/api/src/eta.rs"; }

copy
rc=$(run); echo "prove: clean -> rc=$rc (want 0)"
[ "$rc" -eq 0 ] || { cat "$SCRATCH/out"; fail=1; }

copy
add 'async fn menu_again(place: &crate::hubstore::Place) { let _ = crate::hubstore::load_catalog(place).await; }'
rc=$(run); echo "prove: one more load_catalog( -> rc=$rc (want 1): $(grep REFUSED "$SCRATCH/out")"
[ "$rc" -eq 1 ] || fail=1

copy
add 'async fn log_again(place: &crate::hubstore::Place) { let _ = crate::hubstore::load(place).await; }'
rc=$(run); echo "prove: one more hubstore::load( -> rc=$rc (want 1): $(grep REFUSED "$SCRATCH/out")"
[ "$rc" -eq 1 ] || fail=1

copy
add '// was: crate::hubstore::load_catalog(place) -- moved to /fold/menu'
add '/* and crate::hubstore::load(place) before that */'
rc=$(run); echo "prove: the call named only in comments -> rc=$rc (want 0)"
[ "$rc" -eq 0 ] || { cat "$SCRATCH/out"; fail=1; }

copy
add 'async fn after_url(place: &crate::hubstore::Place) { let _u = "https://x"; let _ = crate::hubstore::load(place).await; }'
rc=$(run); echo "prove: a call after a // inside a string -> rc=$rc (want 1): $(grep REFUSED "$SCRATCH/out")"
[ "$rc" -eq 1 ] || fail=1

copy
add 'async fn seed_demo(place: &crate::hubstore::Place) { let _ = crate::hubstore::load_catalog(place).await; }'
rc=$(run); echo "prove: the call inside an allowlisted seed_* -> rc=$rc (want 0)"
[ "$rc" -eq 0 ] || { cat "$SCRATCH/out"; fail=1; }

[ $fail -eq 0 ] && echo "dataflow.prove: the gate fires, and not on comments or writers" || echo "dataflow.prove: FAILED"
exit $fail
