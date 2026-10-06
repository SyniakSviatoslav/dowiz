#!/bin/sh
# S7c's proof: the gate is triggered before it is trusted. Runs `migration-law.sh` against scratch
# copies -- clean (must pass), a stock kind that gained a field with no law registered (must
# refuse), a block schema string edited (must refuse), a register naming a test nobody wrote
# (must refuse), and the field change done RIGHT -- a new law test plus a v+1 line (must pass).
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0

copy() {
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/crates/dowiz-hub" "$SCRATCH/r/workers/api" "$SCRATCH/r/tools/gates"
  cp -r "$REPO/crates/dowiz-hub/src" "$SCRATCH/r/crates/dowiz-hub/src"
  cp -r "$REPO/workers/api/src" "$SCRATCH/r/workers/api/src"
  cp "$HERE/migration-law.registry" "$SCRATCH/r/tools/gates/"
}
edit() { # file old new
  python3 - "$1" "$2" "$3" <<'PY'
import sys
p, old, new = sys.argv[1:]
s = open(p, encoding='utf-8').read()
assert old in s, f'{old!r} moved in {p}; update the proof'
open(p, 'w', encoding='utf-8').write(s.replace(old, new, 1))
PY
}
run() { # label want
  sh "$HERE/migration-law.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
  [ $rc -eq "$2" ] || { echo "prove: $1 -> rc=$rc, want $2:"; cat "$SCRATCH/out"; fail=1; }
  echo "prove: $1 -> rc=$rc (want $2): $(grep -m1 REFUSED "$SCRATCH/out" || tail -1 "$SCRATCH/out")"
}

copy; run "clean" 0

copy
edit "$SCRATCH/r/crates/dowiz-hub/src/stock/codec.rs" \
  'r#"{{"k":"received","item":"{}","qty":{qty}}}"#' 'r#"{{"k":"received","item":"{}","qty":{qty},"lot":""}}"#'
run "received gained a field, no law" 1

copy
edit "$SCRATCH/r/crates/dowiz-hub/src/block/schema.rs" 'qty:i64:0,gen:i64:7' 'qty:i64:0,gen:i64:7,lot:i64:0'
run "a block schema string changed, no law" 1

copy
edit "$SCRATCH/r/tools/gates/migration-law.registry" 'stock::migration_tests::wasted_gained_by_and_the_laws_hold' 'stock::migration_tests::nobody_wrote_this'
run "a register naming a test nobody wrote" 1

copy
edit "$SCRATCH/r/tools/gates/migration-law.registry" 'stock::migration_tests::stocktake_gained_by_and_the_laws_hold' 'stock::migration_tests::every_kind_round_trips'
run "two versions reusing one law test" 1

# Done right: the field, a law test for it, and the v2 line.
copy
edit "$SCRATCH/r/crates/dowiz-hub/src/stock/codec.rs" \
  'r#"{{"k":"received","item":"{}","qty":{qty}}}"#' 'r#"{{"k":"received","item":"{}","qty":{qty},"lot":""}}"#'
printf '\n#[test]\nfn received_gained_lot_and_the_laws_hold() {}\n' >> "$SCRATCH/r/crates/dowiz-hub/src/stock/migration_tests.rs"
printf 'stock.received 2 item,qty,lot stock::migration_tests::received_gained_lot_and_the_laws_hold\n' >> "$SCRATCH/r/tools/gates/migration-law.registry"
run "the field shipped with its law and a v2 line" 0

[ $fail -eq 0 ] && echo "migration-law.prove: the gate fires in both directions" || echo "migration-law.prove: FAILED"
exit $fail
