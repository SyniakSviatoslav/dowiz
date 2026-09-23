#!/bin/sh
# G4's proof: a gate is triggered before it is trusted. Runs `channel-closed.sh`
# against scratch copies of the Worker -- the clean tree and a tests-only
# unknown word (both must pass: THE GREEN CASES ARE HALF THE PROOF), then four
# deliberate defects (each must refuse): `"channel": "fax"` in a handler, the
# ebills mapper's hand copy of `"ebills"` come back, the storefront's
# `channel::stamp` call removed, and a second `Placed` append site.
# Exit 0 only when all six answer as they should.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
W="$SCRATCH/r/workers/api/src"

copy() {
  rm -rf "$SCRATCH/r"; mkdir -p "$SCRATCH/r/workers/api"
  cp -r "$REPO/workers/api/src" "$W"
}
# edit FILE OLD NEW -- replace once, and refuse if OLD is gone (the proof moved).
edit() {
  python3 - "$1" "$2" "$3" <<'PY'
import sys
p, old, new = sys.argv[1:4]; s = open(p).read()
assert old in s, 'the proof target moved: %r not in %s; update the proof' % (old, p)
open(p, 'w').write(s.replace(old, new, 1))
PY
}
run() { # want label
  sh "$HERE/channel-closed.sh" "$SCRATCH/r" >"$SCRATCH/out" 2>&1; rc=$?
  [ $rc -eq "$1" ] || { echo "prove: $2 -> rc=$rc (want $1):"; cat "$SCRATCH/out"; fail=1; }
  echo "prove: $2 -> rc=$rc (want $1): $(head -1 "$SCRATCH/out")"
}

copy
run 0 "clean tree"

copy
printf '%s\n' 'fn fixture() -> serde_json::Value { serde_json::json!({ "channel": "fax" }) }' \
  >> "$W/ebills/tests.rs"
run 0 "an unknown word inside a tests.rs only"

copy
printf '%s\n' 'pub fn scratch_handler() -> serde_json::Value { serde_json::json!({ "id": "o", "channel": "fax" }) }' \
  >> "$W/storefront.rs"
run 1 "\"channel\": \"fax\" in a scratch handler"

copy
edit "$W/ebills/mod.rs" '"channel": channel::EBILLS' '"channel": "ebills"'
run 1 "the ebills mapper's hand copy come back"

copy
edit "$W/storefront.rs" 'channel::stamp(&mut envelope, source)' 'channel::of(&envelope).map(|_| ())'
run 1 "the storefront's stamp removed"

copy
printf '%s\n' 'pub fn second(h: &mut dowiz_hub::Hub) { let _ = h.append(dowiz_hub::EventKind::Placed, "o", "{}", 1, [0u8; 32]); }' \
  >> "$W/hubstore.rs"
run 1 "a second Placed append site"

[ $fail -eq 0 ] && echo "channel-closed.prove: the gate fires in both directions" || echo "channel-closed.prove: FAILED"
exit $fail
