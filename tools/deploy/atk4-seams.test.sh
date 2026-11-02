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
d=$(sh crates/dowiz-hub-wasm/tablet.sh --digest); s=$(cat "$W.src")
[ "$d" = "$s" ] && ok "tablet.wasm.src is its source" || no "tablet.wasm.src $s != source digest $d, and no gate compares them"
# THE DIGEST IS THE MODULE'S INPUTS (W-MG-TAB). A copy of the inputs, one edit at a time: each edit below reaches the
# module's bytes, so each must move the digest deploy.sh compares (exit 15); the test files must not. No compiler runs.
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
for c in dowiz-hub-wasm dowiz-hub dowiz-core einfile-crc einfile-sig-mldsa einstore einfile-image einfile-kv einfile-log einfile-io dowiz-canvas; do
  mkdir -p "$T/crates/$c"; cp -r "crates/$c/Cargo.toml" "crates/$c/src" "$T/crates/$c/"
done
cp crates/dowiz-hub-wasm/Cargo.lock crates/dowiz-hub-wasm/tablet.sh "$T/crates/dowiz-hub-wasm/"; cp crates/dowiz-canvas/Cargo.lock crates/dowiz-canvas/build.sh "$T/crates/dowiz-canvas/"
mkdir -p "$T/.cargo" "$T/tools/verify"; cp .cargo/config.toml "$T/.cargo/"; cp rust-toolchain.toml "$T/"; cp tools/verify/rustc-stable-id.sh "$T/tools/verify/" 2>/dev/null
TAB="$T/crates/dowiz-hub-wasm/tablet.sh"; BRD="$T/crates/dowiz-canvas/build.sh"
tab0=$(sh "$TAB" --digest 2>/dev/null); brd0=$(sh "$BRD" --digest 2>/dev/null)
[ -n "$tab0" ] && [ "$tab0" = "$d" ] && ok "the tablet digest does not depend on the directory" || no "tablet digest '$tab0' in a copy, '$d' here"
touched() { cp "$T/$2" "$T/.keep"; printf '\n' >> "$T/$2"; sh "$1" --digest 2>/dev/null; mv "$T/.keep" "$T/$2"; }
moves() { # <script> <baseline digest> <file>
  now=$(touched "$1" "$3"); [ -n "$2" ] && [ -n "$now" ] && [ "$now" != "$2" ] && ok "a change in $3 moves the digest" || no "a change in $3 does not trip the check (digest '$now')"
}
stays() { now=$(touched "$1" "$3"); [ -n "$2" ] && [ "$now" = "$2" ] && ok "a change in $3 leaves the digest" || no "$3 is not in the module and moved the digest to '$now'"; }
for f in crates/dowiz-core/src/lib.rs crates/einfile-crc/src/lib.rs crates/einfile-sig-mldsa/src/lib.rs crates/dowiz-hub/src/snn/weights.snn crates/einfile-log/Cargo.toml \
         crates/dowiz-hub-wasm/Cargo.lock crates/dowiz-hub-wasm/tablet.sh rust-toolchain.toml .cargo/config.toml tools/verify/rustc-stable-id.sh; do moves "$TAB" "$tab0" "$f"; done
stays "$TAB" "$tab0" crates/einfile-crc/src/tests.rs
stays "$TAB" "$tab0" crates/einstore/src/view_tests.rs
for f in crates/dowiz-canvas/Cargo.lock crates/dowiz-canvas/build.sh rust-toolchain.toml .cargo/config.toml; do moves "$BRD" "$brd0" "$f"; done
# A path crate added to the lockfile is in the digest with no edit to the script, and an input that is missing is a refusal.
printf '\n[[package]]\nname = "no-such-crate"\nversion = "0.1.0"\n' >> "$T/crates/dowiz-hub-wasm/Cargo.lock"
sh "$TAB" --digest > "$T/.out" 2>/dev/null; rc=$?
cp crates/dowiz-hub-wasm/Cargo.lock "$T/crates/dowiz-hub-wasm/"
[ $rc != 0 ] && [ ! -s "$T/.out" ] && ok "a path crate the digest cannot read refuses" || no "a path crate in the lockfile with no directory still printed a digest (rc=$rc)"
# No box path in a shipped module, and the module's bytes are this tree's.
for m in "$W" workers/api/public/room/canvas/board.wasm; do
  [ "$(grep -a -c -E '/(root|home|Users)/' "$m")" = 0 ] && ok "$m carries no box path" || no "$m carries a box path: rebuild it with the remap"
done
# The seed and the scheme are one call, and the log doors take the venue: a caller cannot forget.
grep -q 'pub async fn load_log_at(stub: &Stub, image: &str, venue: &str)' workers/api/src/platform_store.rs && grep -q 'scheme::arm(&mut log, venue)' workers/api/src/platform_store.rs \
  && ok "load_log_at requires the venue and arms" || no "platform_store::load_log_at does not arm"
[ "$(grep -rn 'set_entropy' workers/api/src --include=*.rs | grep -v 'tests\|scheme.rs\|lib.rs' | wc -l)" = 0 ] && ok "no seed outside scheme::arm" || no "a door seeds by hand"
grep -q 'None => list.trim() == "\*"' workers/api/src/scheme.rs && grep -q 'arm_named(log, self.state.own_name()' workers/api/src/hubdo/logmark.rs \
  && ok "no name fails closed" || no "arm_scheme: own_name() None leaves the crate default"
echo "atk4-seams: $pass ok, $bad HOLE"; [ $bad = 0 ]
