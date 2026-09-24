#!/bin/sh
# G6's proof: a gate is triggered before it is trusted. Runs `principal-binds.sh`
# (a copy, beside a copy of its baseline) against scratch copies of the Worker.
# The green cases are half the proof:
#   clean tree                                                  -> 0
#   harmless: the binder words only in a comment elsewhere       -> 0
#   the unfixed tree of 37204f10 (D13, D18, D40 as the audit found them) -> 1
#   lib.rs `/api/order/:id`: the customer arm answers `true`    -> 1
#   social::messages without its `thread_party` binder          -> 1
#   room pay spends the typed wallet id, no `payer`              -> 1
#   a baseline exception that is bound now (ratchet must fall)   -> 1
#   no authenticated handler to measure                          -> 2
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
UNFIXED=37204f10
mkdir -p "$SCRATCH/g"
cp "$HERE/principal-binds.sh" "$HERE/principal-binds.baseline" "$SCRATCH/g/"

copy() {
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api"
  cp -r "$REPO/workers/api/src" "$SCRATCH/r/workers/api/src"
  cp "$HERE/principal-binds.baseline" "$SCRATCH/g/"
}
edit() { # edit FILE OLD NEW -- exactly one replacement, or the proof is stale
  python3 - "$@" <<'PY'
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
assert s.count(old) == 1, f'{p}: the text this proof edits moved; update the proof'
open(p, 'w').write(s.replace(old, new, 1))
PY
}
want() { # want RC LABEL
  sh "$SCRATCH/g/principal-binds.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
  [ $rc -eq "$1" ] || { echo "prove: $2 -> rc=$rc, WANTED $1:"; cat "$SCRATCH/out"; fail=1; }
  echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 -E 'principal-binds:|NEW|GONE' "$SCRATCH/out")"
}

copy
want 0 "clean"

copy
edit "$SCRATCH/r/workers/api/src/social.rs" '/// Who may read or speak in a thread (D18).' '/// Who may read or speak in a thread (D18). Not a binder: "order_id == id",
// body.wallet, principal_at(&req) -- words in a comment and a string.'
want 0 "harmless: binder words in a comment"

if git -C "$REPO" cat-file -e "$UNFIXED^{commit}" 2>/dev/null; then
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r"
  git -C "$REPO" archive "$UNFIXED" workers/api/src | tar -x -C "$SCRATCH/r"
  want 1 "the unfixed tree at $UNFIXED"
else
  echo "prove: SKIPPED the unfixed-tree case -- $UNFIXED is not in this clone (shallow?)"
fi

copy
edit "$SCRATCH/r/workers/api/src/lib.rs" 'Ok(auth::Principal::Customer { order_id, .. }) => order_id == id,' 'Ok(auth::Principal::Customer { .. }) => true,'
want 1 "lib.rs /api/order/:id customer arm answers true"

copy
edit "$SCRATCH/r/workers/api/src/social.rs" '    if let Err((code, why)) = party::thread_party(&p, &id, false) {
        return Response::error(why, code);
    }' '    let _ = &p;'
want 1 "social::messages without thread_party"

copy
edit "$SCRATCH/r/workers/api/src/services/orders/room/pay.rs" 'match whose::payer(is_owner, body.wallet.as_deref(), shown.as_deref()) {' 'match Ok::<String, (u16, &str)>(body.wallet.clone().unwrap_or_default()) {'
want 1 "room pay spends the typed wallet"

copy
echo 'workers/api/src/social.rs::messages' >> "$SCRATCH/g/principal-binds.baseline"
want 1 "a bound handler left in the baseline"

rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api/src"
echo 'fn main() {}' > "$SCRATCH/r/workers/api/src/lib.rs"
want 2 "nothing to measure"

[ $fail -eq 0 ] && echo "principal-binds.prove: the gate fires, and stays quiet where it should" || echo "principal-binds.prove: FAILED"
exit $fail
