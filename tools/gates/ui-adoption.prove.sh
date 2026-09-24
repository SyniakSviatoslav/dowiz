#!/bin/sh
# ui-adoption's proof: a gate is triggered before it is trusted.
#
# Runs `ui-adoption.sh` against scratch copies of workers/api/public:
#   1. clean                                                  -> pass (exit 0)
#   2. the courier grows a hand-written <button class="cta">  -> refuse (exit 1)
#   3. the same button built with ui.button()'s classes       -> pass
#   4. a surface grows a private `const esc = s => ...`       -> refuse
#   5. a hand-built toast / empty state / skeleton / pill     -> refuse, each
#   6. an admin <input> migrated to `ui-input` (count FALLS)  -> refuse until
#      the baseline is lowered: the ratchet only moves down, on purpose
#
# The gate runs from a scratch copy with a baseline taken from the clean
# scratch tree, so the proof tests the gate's LOGIC and does not go red because
# some other uncommitted edit in the working tree moved a count (that is what
# running the gate itself is for).
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
mkdir -p "$SCRATCH/g"; cp "$HERE/ui-adoption.sh" "$SCRATCH/g/"
run() { sh "$SCRATCH/g/ui-adoption.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; echo $?; }
want() {
  rc=$(run)
  echo "prove: $2 -> rc=$rc (want $1): $(head -1 "$SCRATCH/out")"
  [ "$rc" -eq "$1" ] || { cat "$SCRATCH/out"; fail=1; }
}
add() { printf '%s\n' "$2" >> "$P/$1"; }

copy
want 0 "clean, no baseline yet (the gate writes one)"
grep -q "baseline written" "$SCRATCH/out" || { echo "prove: expected the first run to write a baseline"; fail=1; }
want 0 "clean, against its own baseline"

copy
add courier/screens.js "export const extra = () => '<button class=\"cta\" id=\"x\" type=\"button\">x</button>';"
want 1 "courier hand-writes a <button class=cta>"

copy
add courier/screens.js "export const extra = () => '<button class=\"ui-btn ui-btn--primary\" id=\"x\" type=\"button\">x</button>';"
want 0 "the same button with the component's classes"

copy
add room/till.js "const esc = s => String(s);"
want 1 "room grows a private escaper"

for snippet in 'toast' 'empty' 'skel skel-line' 'tag on'; do
  copy
  add store/ui.js "export const h = '<div class=\"$snippet\"></div>';"
  want 1 "store hand-builds class=\"$snippet\""
done

copy
f=$(grep -l '<input\b' "$P"/admin/*.js | head -1)
python3 - "$f" <<'PY'
import re, sys
p = sys.argv[1]; s = open(p, encoding='utf-8').read()
s = re.sub(r'<input\b(?![^>]*\bclass=["\'][^"\']*\bui-)', '<input class="ui-input"', s, count=1)
open(p, 'w', encoding='utf-8').write(s)
PY
want 1 "admin migrates one <input> and does not lower the baseline"

[ $fail -eq 0 ] && echo "ui-adoption.prove: the gate fires in both directions" || echo "ui-adoption.prove: FAILED"
exit $fail
