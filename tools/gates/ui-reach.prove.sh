#!/bin/sh
# ui-reach's proof: the gate is heard before it is trusted.
#   1. the tree as it is                                        -> must pass (exit 0)
#   2. a new route in a scratch lib.rs that no screen calls     -> must refuse (exit 1, ORPHAN named)
#   3. the same route, called from a scratch screen             -> must pass
#   4. the call only in a line comment                          -> must refuse (a comment is not a caller)
#   5. a scratch screen that calls an existing route's LITERAL
#      segment through a one-literal parameter (`/owner/${x}`)  -> must not reach it (the orphan in 2 stays)
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
LIB="$SCRATCH/lib.rs"
PUB="$SCRATCH/public"
cp "$REPO/workers/api/src/lib.rs" "$LIB"
cp -r "$REPO/workers/api/public" "$PUB"

want() { # want <rc> <label>
  UI_REACH_LIB="$LIB" UI_REACH_PUBLIC="$PUB" sh "$HERE/ui-reach.sh" >"$SCRATCH/out" 2>&1
  rc=$?
  echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 '^ORPHAN' "$SCRATCH/out" || tail -1 "$SCRATCH/out")"
  [ "$rc" -eq "$1" ] || { cat "$SCRATCH/out"; fail=1; }
}

want 0 "the tree as it is"
python3 - "$LIB" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
anchor = '.get("/healthz"'
assert anchor in s, 'no anchor in lib.rs'
s = s.replace(anchor, '.post_async("/api/owner/prove/nobody-calls-this", prove::x)\n        ' + anchor, 1)
open(p, 'w').write(s)
PY
want 1 "a route no screen calls"
printf '%s\n' "export const x = () => post('/owner/prove/nobody-calls-this', {});" > "$PUB/admin/prove-caller.js"
want 0 "the same route, called from a screen"
printf '%s\n' "// post('/owner/prove/nobody-calls-this')" > "$PUB/admin/prove-caller.js"
want 1 "the call only in a comment"
printf '%s\n' 'export const y = x => api(`/owner/${x}/nobody-calls-this`);' > "$PUB/admin/prove-caller.js"
want 1 "a one-literal parameter does not stand for a literal segment"

[ "$fail" -eq 0 ] && echo "ui-reach.prove: the gate fires on an orphan and clears when a screen calls it" || echo "ui-reach.prove: FAILED"
exit "$fail"
