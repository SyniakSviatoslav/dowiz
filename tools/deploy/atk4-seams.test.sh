#!/usr/bin/env bash
# W-MG-ATK4: the seams no crate test reaches. From the repo root, read-only, no compiler.
#   bash tools/deploy/atk4-seams.test.sh
set -u
cd "$(dirname "$0")/../.." || exit 2
pass=0; bad=0
ok() { pass=$((pass + 1)); echo "ok   $1"; }
no() { bad=$((bad + 1)); echo "HOLE $1"; }
W=workers/api/public/room/canvas/tablet.wasm
# The promoted module is prod's: it replays a nonced reply under FNV and refuses it, so a flip stops every tablet's sync.
[ "$(grep -a -c 'dowiz/nonce/derived' "$W")" -gt 0 ] && ok "tablet.wasm knows sha256 records" || no "tablet.wasm has no sha256 reader: every flipped venue's tablet sync refuses"
d=$(sh crates/bebop-wasm/tablet.sh --digest); s=$(cat "$W.src")
[ "$d" = "$s" ] && ok "tablet.wasm.src is its source" || no "tablet.wasm.src $s != source digest $d, and no gate compares them"
# The seed and the scheme are one call, and the log doors take the venue: a caller cannot forget.
grep -q 'pub async fn load_log_at(stub: &Stub, image: &str, venue: &str)' workers/api/src/platform_store.rs && grep -q 'scheme::arm(&mut log, venue)' workers/api/src/platform_store.rs \
  && ok "load_log_at requires the venue and arms" || no "platform_store::load_log_at does not arm"
[ "$(grep -rn 'set_entropy' workers/api/src --include=*.rs | grep -v 'tests\|scheme.rs\|lib.rs' | wc -l)" = 0 ] && ok "no seed outside scheme::arm" || no "a door seeds by hand"
grep -q 'None => list.trim() == "\*"' workers/api/src/scheme.rs && grep -q 'arm_named(log, self.state.own_name()' workers/api/src/hubdo/logmark.rs \
  && ok "no name fails closed" || no "arm_scheme: own_name() None leaves the crate default"
echo "atk4-seams: $pass ok, $bad HOLE"; [ $bad = 0 ]
