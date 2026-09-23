#!/bin/sh
# G2's proof: a gate is triggered before it is trusted. Runs `record.sh`
# against three scratch copies of the Worker -- clean (must pass), a card write
# that stores the customer's name (must refuse), and an allow-list that gained
# `spent` (must refuse). Exit 0 only when all three answer as they should.
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

copy
sh "$HERE/record.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
[ $rc -eq 0 ] || { echo "prove: CLEAN copy refused (rc=$rc):"; cat "$SCRATCH/out"; fail=1; }
echo "prove: clean -> rc=$rc (want 0)"

copy
f="$SCRATCH/r/workers/api/src/services/customers/at_placement.rs"
python3 - "$f" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
old = 't.put(KIND, &key, &rec, &[], &[])'
assert old in s, 'the card write moved; update the proof'
s = s.replace(old, 'let rec = serde_json::json!({"name": "Arben", "created_at_ms": now_ms}).to_string(); ' + old, 1)
open(p, 'w').write(s)
PY
sh "$HERE/record.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
[ $rc -eq 1 ] || { echo "prove: a card storing a NAME passed (rc=$rc)"; fail=1; }
echo "prove: name written onto the card -> rc=$rc (want 1): $(sed -n 2p "$SCRATCH/out")"

copy
f="$SCRATCH/r/workers/api/src/services/customers/record.rs"
python3 - "$f" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
old = '"created_at_ms", "updated_at_ms",\n];'
assert old in s, 'FIELDS moved; update the proof'
s = s.replace(old, '"created_at_ms", "updated_at_ms", "spent",\n];', 1)
open(p, 'w').write(s)
PY
sh "$HERE/record.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
[ $rc -eq 1 ] || { echo "prove: FIELDS holding 'spent' passed (rc=$rc)"; fail=1; }
echo "prove: 'spent' added to the allow-list -> rc=$rc (want 1): $(sed -n 2p "$SCRATCH/out")"

[ $fail -eq 0 ] && echo "record.prove: the gate fires in both directions" || echo "record.prove: FAILED"
exit $fail
