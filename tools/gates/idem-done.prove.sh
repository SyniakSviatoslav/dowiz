#!/bin/sh
# G1's proof: a gate is triggered before it is trusted. Runs `idem-done.sh`
# against scratch copies of the Worker. The green cases are half the proof:
#   clean tree                                         -> 0
#   harmless: `?` in a closure, `return` in a comment,
#             `?` inside an `async {}` block            -> 0
#   the unfixed tree of 37204f10 (D1 as the audit found it) -> 1
#   room/pay.rs refusal answered without its verdict   -> 1
#   booking/create.rs body ending without `answered`   -> 1
#   a verdict spent in ONE branch, an exit after it    -> 1
#   no guarded handler to measure                      -> 2
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
UNFIXED=37204f10

copy() {
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api"
  cp -r "$REPO/workers/api/src" "$SCRATCH/r/workers/api/src"
}
edit() { # edit FILE OLD NEW -- exactly one replacement, or the proof is stale
  python3 - "$@" <<'PY'
import sys
p, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
assert old in s, f'{p}: the text this proof edits moved; update the proof'
open(p, 'w').write(s.replace(old, new, 1))
PY
}
want() { # want RC LABEL
  sh "$HERE/idem-done.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
  [ $rc -eq "$1" ] || { echo "prove: $2 -> rc=$rc, WANTED $1:"; cat "$SCRATCH/out"; fail=1; }
  echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 'idem-done:' "$SCRATCH/out")"
}

copy
want 0 "clean"

copy
PAY="$SCRATCH/r/workers/api/src/services/orders/room/pay.rs"
edit "$PAY" '    let input = PayIn {' '    // a return in a comment is not an exit; neither is "?" in a string
    let _probe: Vec<i64> = vec![1].into_iter().filter_map(|x: i64| Some(x.checked_add(1)?)).collect();
    let _blk: Result<i64> = async { let v: i64 = "1".parse().map_err(|_| Error::RustError("x".into()))?; Ok(v) }.await;
    let input = PayIn {'
want 0 "harmless closure / comment / async-block exits"

if git -C "$REPO" cat-file -e "$UNFIXED^{commit}" 2>/dev/null; then
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r"
  git -C "$REPO" archive "$UNFIXED" workers/api/src | tar -x -C "$SCRATCH/r"
  want 1 "the unfixed tree at $UNFIXED"
else
  echo "prove: SKIPPED the unfixed-tree case -- $UNFIXED is not in this clone (shallow?); the three planted cases below still run"
fi

copy
PAY="$SCRATCH/r/workers/api/src/services/orders/room/pay.rs"
edit "$PAY" 'return idem.refused(&place, status, &said).await' 'return Response::error(said, status)'
want 1 "room/pay.rs refusal without its verdict"

copy
BK="$SCRATCH/r/workers/api/src/booking/create.rs"
edit "$BK" '    idem.answered(&place, res).await
}' '    res
}'
want 1 "booking/create.rs body ends without answered"

copy
PAY="$SCRATCH/r/workers/api/src/services/orders/room/pay.rs"
edit "$PAY" '    let input = PayIn {' '    if body.amount < 0 {
        return idem.refused(&place, 400, "negative").await;
    }
    if body.amount == 0 {
        return Response::error("zero", 400);
    }
    let input = PayIn {'
want 1 "a verdict in one branch excuses nothing after it"

rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api/src"
echo 'fn main() {}' > "$SCRATCH/r/workers/api/src/lib.rs"
want 2 "nothing guarded to measure"

[ $fail -eq 0 ] && echo "idem-done.prove: the gate fires, and stays quiet where it should" || echo "idem-done.prove: FAILED"
exit $fail
