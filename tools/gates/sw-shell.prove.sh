#!/bin/sh
# sw-shell's proof: a gate is triggered before it is trusted.
#
# Runs `sw-shell.sh` against scratch copies of workers/api/public:
#   1. clean                                             -> must pass (exit 0)
#   2. the G2 defect put back: '/lib/vocab.js' dropped
#      from the courier's shell                          -> must refuse (exit 1)
#   3. a room module gains a static import of a new file
#      the room's shell does not list                    -> must refuse
#   4. the same new import, with the file added to the
#      shell                                             -> must pass again
#   5. a shell names a file that is not on disk          -> must refuse
#   6. a new service worker the gate does not know       -> must refuse
#   7. a DYNAMIC import() of an unlisted file            -> must pass (not
#      counted on purpose: the map and live.js need the network anyway)
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
P="$SCRATCH/r/workers/api/public"

copy() {
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api"
  cp -r "$REPO/workers/api/public" "$P"
}
run() { sh "$HERE/sw-shell.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; echo $?; }
want() { # want <rc> <label>
  rc=$(run)
  echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 '  /' "$SCRATCH/out" || tail -1 "$SCRATCH/out")"
  [ "$rc" -eq "$1" ] || { cat "$SCRATCH/out"; fail=1; }
}

copy
want 0 "clean"

copy
grep -v "^  '/lib/vocab.js',$" "$P/courier/sw.js" > "$SCRATCH/sw" && cp "$SCRATCH/sw" "$P/courier/sw.js"
grep -q "'/lib/vocab.js'" "$P/courier/sw.js" && { echo "prove: could not remove vocab.js"; fail=1; }
want 1 "G2 put back (courier shell without /lib/vocab.js)"

copy
echo "export const x = 1;" > "$P/room/extra.js"
printf "import { x } from '/room/extra.js';\n" >> "$P/room/net.js"
want 1 "room/net.js imports an unlisted module"

sed "s#^  '/room/app.js',\$#  '/room/app.js',\n  '/room/extra.js',#" "$P/room/sw.js" > "$SCRATCH/sw" && cp "$SCRATCH/sw" "$P/room/sw.js"
want 0 "the same import, listed"

copy
sed "s#^  '/courier/app.js',\$#  '/courier/app.js',\n  '/courier/gone.js',#" "$P/courier/sw.js" > "$SCRATCH/sw" && cp "$SCRATCH/sw" "$P/courier/sw.js"
want 1 "courier shell names a file not on disk"

copy
printf "const SHELL = [\n  '/platform/',\n];\n" > "$P/platform/sw.js"
want 1 "an unknown service worker"

copy
echo "export const y = 1;" > "$P/room/lazy.js"
printf "const lazy = () => import('/room/lazy.js');\n" >> "$P/room/net.js"
want 0 "a dynamic import() of an unlisted file"

[ $fail -eq 0 ] && echo "sw-shell.prove: the gate fires in both directions" || echo "sw-shell.prove: FAILED"
exit $fail
