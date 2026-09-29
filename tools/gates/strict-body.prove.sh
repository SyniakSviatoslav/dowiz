#!/bin/sh
# strict-body's proof: a gate is triggered before it is trusted. Runs
# `strict-body.sh` against scratch copies of the Worker.
#   clean tree                                              -> 0
#   harmless: res.json() on a Response, req.json() in a
#             comment and in a string, a trait signature    -> 0
#   a handler parsing a deny_unknown_fields struct
#     through req.json() (the live defect, replanted)       -> 1
#   the same inside the Durable Object (`req.json().await?`) -> 1
#   a `let mut req = req;` rebinding does not hide it       -> 1
#   (the "fixed without lowering the baseline" case left with the last site:
#    the baseline is 0, the floor, so nothing can fall below it)
#   no function taking a Request at all                     -> 2
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
  sh "$HERE/strict-body.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
  [ $rc -eq "$1" ] || { echo "prove: $2 -> rc=$rc, WANTED $1:"; cat "$SCRATCH/out"; fail=1; }
  echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 'strict-body:' "$SCRATCH/out")"
}

copy
want 0 "clean"

copy
ACC="$SCRATCH/r/workers/api/src/accounts.rs"
edit "$ACC" '    let body: LoginIn = match crate::body::parse(&mut req).await {' '    // a handler may still call req.json() in a comment
    let _said = "and req.json() in a string";
    let _probe = async { let mut res = Response::ok("{}")?; let v: serde_json::Value = res.json().await?; Ok::<_, Error>(v) };
    let body: LoginIn = match crate::body::parse(&mut req).await {'
cat >> "$ACC" <<'RS'

#[allow(dead_code)]
trait Reads {
    fn read_it(req: Request) -> Result<Response>;
}
RS
want 0 "harmless: res.json(), a comment, a string, a trait signature"

copy
ACC="$SCRATCH/r/workers/api/src/accounts.rs"
edit "$ACC" '    let body: LoginIn = match crate::body::parse(&mut req).await {' '    let body: LoginIn = match req.json().await {'
want 1 "owner_login parsing its deny struct through req.json()"

copy
PR="$SCRATCH/r/workers/api/src/hubdo/print.rs"
edit "$PR" '                let input: AckIn = crate::body::parse(&mut req).await?;' '                let input: AckIn = req.json().await?;'
want 1 "the Durable Object: req.json().await?"

copy
HD="$SCRATCH/r/workers/api/src/hubdo.rs"
edit "$HD" '                    let input: crate::command::assign::AssignIn = crate::body::parse(&mut req).await?;' '                    let input: crate::command::assign::AssignIn = req.json().await?;'
want 1 "a let mut req = req; rebinding does not hide it"

rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api/src"
echo 'fn main() {}' > "$SCRATCH/r/workers/api/src/lib.rs"
want 2 "nothing taking a Request to measure"

[ $fail -eq 0 ] && echo "strict-body.prove: the gate fires, and stays quiet where it should" || echo "strict-body.prove: FAILED"
exit $fail
