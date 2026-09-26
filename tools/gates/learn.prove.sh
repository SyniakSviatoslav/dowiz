#!/bin/sh
# learn's proof: a gate is triggered before it is trusted.
#
# Runs `learn.sh` against scratch copies of the public tree, the lesson YAML
# and the build tool:
#   1. clean                                               -> must pass (exit 0)
#   2. a courier anchor RENAMED in the markup
#      ('pick.take' -> 'pick.grab' in courier/screens.js)  -> must refuse (item 2 and item 1)
#   3. a room anchor renamed ('pay.tip' in room/pay.js)    -> must refuse
#   4. a new More tile no lesson covers                    -> must refuse (item 3)
#   5. a caption's language deleted from a YAML file,
#      lessons.json not rebuilt                            -> must refuse (item 4)
#   6. a new data-tour in the courier no lesson names      -> must refuse (item 1)
#   7. a step marked `pending: yes` whose anchor is absent,
#      rebuilt                                             -> must pass (pending is printed, not red)
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
R="$SCRATCH/r"
P="$R/workers/api/public"

copy() {
  rm -rf "$R"; mkdir -p "$R/workers/api" "$R/docs" "$R/tools"
  cp -r "$REPO/workers/api/public" "$P"
  cp -r "$REPO/docs/learn" "$R/docs/learn"
  cp -r "$REPO/tools/learn" "$R/tools/learn"
}
run() { sh "$HERE/learn.sh" "$R" >"$SCRATCH/out" 2>&1; echo $?; }
want() { # want <rc> <label>
  rc=$(run)
  echo "prove: $2 -> rc=$rc (want $1): $(grep -m1 '^  [0-9] ' "$SCRATCH/out" || tail -1 "$SCRATCH/out")"
  [ "$rc" -eq "$1" ] || { cat "$SCRATCH/out"; fail=1; }
}
edit() { # edit <file> <python-literal-old> <python-literal-new>: exactly one replacement, or the proof fails
  python3 - "$1" "$2" "$3" <<'PY' || { echo "prove: could not edit $1"; fail=1; }
import sys
p, old, new = sys.argv[1:4]
s = open(p, encoding='utf-8').read()
assert s.count(old) == 1, f'{old!r} occurs {s.count(old)} times in {p}'
open(p, 'w', encoding='utf-8').write(s.replace(old, new))
PY
}

copy
want 0 "clean"

copy
edit "$P/courier/screens.js" "tour: 'pick.take'" "tour: 'pick.grab'"
want 1 "courier anchor pick.take renamed to pick.grab"

copy
edit "$P/room/pay.js" "'pay.tip'" "'pay.gratuity'"
want 1 "room anchor pay.tip renamed"

copy
edit "$P/admin/more.js" "['dpa', 'shield-check', openDpa]" "['dpa', 'shield-check', openDpa], ['loyalty', 'star', openDpa]"
want 1 "a More tile (loyalty) no lesson covers"

copy
edit "$R/docs/learn/lessons/courier/C3.yaml" '      uk: "Час до дверей"
' ''
want 1 "C3 lost a Ukrainian title, lessons.json not rebuilt"

copy
edit "$P/courier/index.html" 'id="themeBtn"' 'id="themeBtn" data-tour="hud.theme"'
want 1 "a new courier anchor (hud.theme) no lesson names"

copy
edit "$R/docs/learn/lessons/courier/C5.yaml" "anchor: hud.gps
    pending: no" "anchor: hud.compass
    pending: yes"
edit "$R/docs/learn/lessons/courier/C5.yaml" "selector: '[data-tour=\"hud.gps\"]'" "selector: '[data-tour=\"hud.compass\"]'"
edit "$P/courier/index.html" ' data-tour="hud.gps"' ''
# The list names the line the anchor is on; the UI moves it, so drop the entry by its name.
grep -q '^hud.gps ' "$R/docs/learn/anchors-courier.txt" && sed -i '/^hud.gps /d' "$R/docs/learn/anchors-courier.txt" || { echo "prove: hud.gps is not in anchors-courier.txt"; fail=1; }
node "$R/tools/learn/build-lessons.mjs" --root "$R" >/dev/null 2>&1 || { echo "prove: rebuild failed"; fail=1; }
want 0 "a pending anchor that does not exist yet"

# 8. item 5 (a WARN, never red): a video recorded from W3's YAML is current until the YAML moves.
copy
python3 - "$R" <<'PY'
import hashlib, json, os, sys
r = sys.argv[1]
y = os.path.join(r, 'docs/learn/lessons/waiter/W3.yaml')
m = {'version': 1, 'lessons': {'W3': {'role': 'waiter', 'source': hashlib.sha256(open(y, 'rb').read()).hexdigest(), 'cuts': {}}}}
os.makedirs(os.path.join(r, 'workers/api/public/learn/media'), exist_ok=True)
json.dump(m, open(os.path.join(r, 'workers/api/public/learn/media/manifest.json'), 'w'))
PY
want 0 "W3's video recorded from the current YAML"
grep -q 'stale video: W3' "$SCRATCH/out" && { echo "prove: a current video was called stale"; fail=1; }
grep -q 'videos: 1/.* 0 stale' "$SCRATCH/out" || { echo "prove: the video count is missing"; fail=1; }
printf '# edited after the recording\n' >> "$R/docs/learn/lessons/waiter/W3.yaml"
node "$R/tools/learn/build-lessons.mjs" --root "$R" >/dev/null 2>&1 || { echo "prove: rebuild failed"; fail=1; }
want 0 "W3's YAML edited after its video (warned, not refused)"
grep -q '5 WARN stale video: W3' "$SCRATCH/out" && echo "prove: item 5 warns: $(grep -m1 'stale video: W3' "$SCRATCH/out" | cut -c1-60)" || { echo "prove: item 5 did not warn on a stale W3"; fail=1; }

[ $fail -eq 0 ] && echo "learn.prove: the gate fires in both directions" || echo "learn.prove: FAILED"
exit $fail
