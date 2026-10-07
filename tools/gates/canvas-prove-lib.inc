# Shared by the canvas-*.prove.sh scripts: a scratch copy of /room/canvas/, a mutation, and
# `want <rc> <label> <gate> [args]` that runs the gate on the copy and checks its exit code.
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SRC="$REPO/workers/api/public/room/canvas"
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
C="$SCRATCH/canvas"
fail=0
copy() { rm -rf "$C"; cp -r "$SRC" "$C"; }
# mutate <file> <python-expr old> <new>: replace one literal; refuse when it is not there.
mutate() {
  python3 - "$C/$1" "$2" "$3" <<'PY' || { echo "prove: mutation not applied to $1"; fail=1; }
import sys
p, old, new = sys.argv[1:]
s = open(p).read()
if old not in s: sys.exit(1)
open(p, 'w').write(s.replace(old, new, 1))
PY
}
want() { # want <rc> <label> <gate-script> [args...]
  exp=$1; label=$2; shift 2
  sh "$@" > "$SCRATCH/out" 2>&1; rc=$?
  echo "prove: $label -> rc=$rc (want $exp): $(grep -m1 'RED --' "$SCRATCH/out" || tail -1 "$SCRATCH/out")"
  [ "$rc" -eq "$exp" ] || { tail -5 "$SCRATCH/out"; fail=1; }
}
