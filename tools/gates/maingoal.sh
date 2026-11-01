#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# maingoal: THE DEMONSTRATION (W-MG-M7A; R-EMBED-STORE §12.2, R-MAINGOAL-A §2d, §7 row M7), the part
# that needs no merge and no network. From a clean temp dir, on REAL files the REAL library wrote:
#   (a) a device writes and seals with real ML-DSA-65; forget + cut + anchor through SealedHub
#       (crates/dowiz-hub/src/bin/maingoal_device.rs: every record, seal and mark is the library's);
#   (b) a third party with the file, the public keys and no repo checks it: `doc_verify` and
#       `anchor verify` for signatures, bebop-verify.py and .mjs for structure, and they agree on
#       every shared counter, head and anchor;
#   (c) a clean Node reads the archive with the reader embedded IN it (examples/read.mjs).
#   (d) two offline devices, each with its own key and its own file, one genesis, no server, merge
#       both ways (crates/dowiz-hub/src/bin/maingoal_merge.rs): every seal survives, the conflict
#       is named and nothing is dropped, and a third party agrees on the result.
# Every scenario has a kill: one changed byte -> the named refusal (tools/gates/maingoal.forge.py).
#
# WHAT THIS IS NOT. Not M7 whole: §7's row also wants the three scenarios in a browser on qa-durres
# (no deploys, operator 10-31). Level 1 here is one real ML-DSA-65 platform mark by THIS RUN'S key:
# no platform key exists in production and no Bitcoin attestation is involved. bebop-verify counts
# seals and checks none (checked=0); kill `seal-old` shows that as a fact, not a defect.
#
# PREBUILT BINARIES. run-all has no compiler time for a release build of dowiz-hub, so this gate runs
# binaries from $MAINGOAL_BIN (default crates/dowiz-hub/target/maingoal) and REFUSES (rc 2) any that
# does not match its stamp: sha256 over every source file of the crates they link, and over the
# binaries. `--build` builds them and writes the stamp; on GitHub a stale stamp builds by itself.
# On the box:  bash bebop-lang/tools/slot.sh maingoal sh tools/gates/maingoal.sh --build
#
# Usage: tools/gates/maingoal.sh [--build | --stamp]   (--stamp: bless $MAINGOAL_BIN as it stands,
#        for tools/gates/maingoal.prove.sh only).    Exit 0 GREEN; 1 RED; 2 REFUSED.
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HERE="$ROOT/tools/gates"
BIN="${MAINGOAL_BIN:-$ROOT/crates/dowiz-hub/target/maingoal}"
BINS="doc_verify anchor maingoal_device maingoal_merge"
FIX="$ROOT/crates/bebop-wasm/fixtures"
PHONE="+355691111111"
say() { echo "maingoal: $*"; }

stamp_text() {
  src=$(cd "$ROOT/crates" && find bebop-crc bebop-image bebop-kv bebop-log bebop-io bebop-store dowiz-core dowiz-hub \
    -type d -name target -prune -o -type f \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' \) -print \
    | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -d' ' -f1)
  bins=$(cd "$BIN" && sha256sum $BINS 2>/dev/null | sha256sum | cut -d' ' -f1)
  echo "src=$src bin=$bins"
}
build() {
  T="${MAINGOAL_TARGET:-$ROOT/crates/dowiz-hub/target}"
  CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"; [ -x "$CARGO" ] || CARGO=cargo
  offline="--offline"; [ -n "${GITHUB_ACTIONS:-}" ] && offline=""
  (cd "$ROOT/crates/dowiz-hub" && CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$T" "$CARGO" build --release $offline --bin doc_verify --bin anchor --bin maingoal_device --bin maingoal_merge) || { say "RED the build failed"; exit 1; }
  mkdir -p "$BIN"
  for b in $BINS; do cp "$T/release/$b" "$BIN/$b" || exit 1; done
  stamp_text > "$BIN/stamp"
  say "built $BINS into $BIN; $(cat "$BIN/stamp")"
}
case "${1:-}" in
  --build) build ;;
  --stamp) stamp_text > "$BIN/stamp"; say "stamped $BIN"; exit 0 ;;
  "") ;;
  *) say "unknown argument $1"; exit 2 ;;
esac
if [ ! -f "$BIN/stamp" ] || [ "$(cat "$BIN/stamp")" != "$(stamp_text)" ]; then
  if [ -n "${GITHUB_ACTIONS:-}" ]; then build
  else
    say "REFUSED the prebuilt binaries in $BIN are STALE or absent (stamp $(cat "$BIN/stamp" 2>/dev/null || echo none) vs now $(stamp_text)): bash bebop-lang/tools/slot.sh maingoal sh tools/gates/maingoal.sh --build"
    exit 2
  fi
fi

fail=0; held=0; kills=0
started=$(date +%s)
red() { say "RED $*"; fail=1; }
S=$(mktemp -d "${TMPDIR:-/tmp}/maingoal.XXXXXX")
trap 'rm -rf "${S:?}"' EXIT
DEV="$S/device"; AUD="$S/auditor"; NOD="$S/node"
mkdir -p "$DEV" "$AUD" "$NOD"
P="$(dirname "$(command -v node)"):$(dirname "$(command -v python3)")"
# every third-party step: from its own directory, no HOME, a PATH of two interpreters
clean() { d=$1; shift; (cd "$d" && env -i PATH="$P" "$@"); }
field() { printf '%s\n' "$1" | tr ' ' '\n' | sed -n "s/^$2=//p" | head -1; }

# THE READER the archive embeds: the COMMITTED one, out of bebop-verify.mjs, against its own digest
python3 -I - "$ROOT/tools/verify/bebop-verify.mjs" "$DEV/reader.wasm" <<'PY' || { say "RED cannot take the reader out of bebop-verify.mjs"; exit 1; }
import base64, hashlib, re, sys
t = open(sys.argv[1]).read()
w = base64.b64decode(re.search(r'^const WASM_B64 = "([^"]*)";$', t, re.M).group(1))
if hashlib.sha256(w).hexdigest() != re.search(r'^const WASM_SHA256 = "([0-9a-f]*)";$', t, re.M).group(1):
    sys.exit("the embedded wasm does not hash to WASM_SHA256")
open(sys.argv[2], "wb").write(w)
PY
READER_SHA=$(sha256sum "$DEV/reader.wasm" | cut -d' ' -f1)
for b in $BINS; do cp "$BIN/$b" "$DEV/" || exit 1; done
cp "$ROOT/docs/FORMAT-BEBOP-IMAGE-v2.md" "$DEV/format.md"

# ── (a) THE DEVICE ───────────────────────────────────────────────────────────────────────────
SEED=$(od -An -tx1 -N32 /dev/urandom | tr -d ' \n')
say "seed $SEED (fresh each run; pass this seed to maingoal_device to replay a failure)"
out=$(clean "$DEV" ./maingoal_device . --seed "$SEED" --reader reader.wasm --format format.md 2>&1); rc=$?
printf '%s\n' "$out" | sed 's/^/maingoal: /'
[ "$rc" = 0 ] || { say "RED the device was refused rc=$rc"; exit 1; }
# the fixture is this program at seed 0^32: stale the moment the writer or the format moves
mkdir "$S/zero"
clean "$DEV" ./maingoal_device ../zero --seed 0000000000000000000000000000000000000000000000000000000000000000 --reader reader.wasm --format format.md > /dev/null 2>&1 || red "the seed-0 run was refused"
if cmp -s "$S/zero/hub.sealed" "$FIX/anch.store"; then say "fixture anch.store is the writer's output at seed 0, byte for byte"
else red "fixtures/anch.store is not the writer's output at seed 0 (STALE fixture; see anch.expected)"; fi
# NO MOCK IN THE PATH: the two programs of the path name none
if grep -nE 'Toy|Fake|Mock|cfg\(test\)' "$ROOT/crates/dowiz-hub/src/bin/maingoal_device.rs" "$ROOT/crates/dowiz-hub/src/bin/maingoal_merge.rs" "$ROOT/crates/dowiz-hub/src/bin/doc_verify.rs" > "$S/mock"; then red "a test double is in the path: $(cat "$S/mock")"; else say "no test double named in maingoal_device.rs, maingoal_merge.rs or doc_verify.rs"; fi
# THE ERASURE left the bytes: the control has the phone, nothing written after the forget does
if grep -qaF "$PHONE" "$DEV/before-forget.sealed"; then say "control: the phone is in before-forget.sealed"; else red "control: the phone is not in before-forget.sealed, this grep measures nothing"; fi
for f in hub.sealed history.sealed archive.store; do
  if grep -qaF "$PHONE" "$DEV/$f"; then red "the phone survived the forget in $f"; else say "the phone is gone from $f"; fi
done
[ "$fail" = 0 ] && { held=$((held + 1)); say "SCENARIO (a) held: real ML-DSA-65, forget + cut + anchor, phone gone"; }

# ── (b) THE THIRD PARTY: the files, the public keys, no repo ─────────────────────────────────
for f in doc_verify anchor; do cp "$BIN/$f" "$AUD/"; done
cp "$ROOT/tools/verify/bebop-verify.py" "$ROOT/tools/verify/bebop-verify.mjs" "$ROOT/examples/read.mjs" "$AUD/"
cp "$DEV/hub.sealed" "$DEV/history.sealed" "$DEV/archive.store" "$AUD/"
PPK=$(cat "$DEV/platform.pk"); ACTOR=$(cat "$DEV/device.actor")
THEIRS=$(python3 -I -c 'import hashlib,sys;print(hashlib.sha256(bytes.fromhex(open(sys.argv[1]).read())).hexdigest())' "$DEV/device.pk")
if [ "$THEIRS" = "$ACTOR" ]; then say "the auditor derives the actor id from the public key: $ACTOR"; else red "sha256(device.pk) $THEIRS is not device.actor $ACTOR"; fi
V_ARGS="--platform-pk $PPK --trust $ACTOR"
dv() { clean "$AUD" ./doc_verify "$1" $V_ARGS; }
av() { clean "$AUD" ./anchor verify "$1" --platform-key "$PPK"; }
py() { clean "$AUD" python3 -I ./bebop-verify.py "$@"; }
nd() { clean "$AUD" node ./bebop-verify.mjs "$@"; }
before=$fail
WANT_HUB="doc records=4 chained=4 legacy=0 redacted=0 broken=0 seals_ok=4 seals_bad=0 forks=0 keys=1 admitted=1 anchors=2 anchors_ok=1 anchors_bad=0 anchors_unchecked=1 anchors_cut=1 segments=1 cut=9 cut_redacted=2 declared=2 keys_disagree=0 revokes_unsettled=0 tombs_foreign=0 unsealed=0 tree_unsigned=0 merges=0 merges_bad=0"
WANT_HIST="doc records=12 chained=9 legacy=1 redacted=2 broken=0 seals_ok=6 seals_bad=0 forks=0 keys=1 admitted=1 anchors=1 anchors_ok=1 anchors_bad=0 anchors_unchecked=0 anchors_cut=0 segments=0 cut=0 cut_redacted=0 declared=2 keys_disagree=0 revokes_unsettled=0 tombs_foreign=0 unsealed=0 tree_unsigned=0 merges=0 merges_bad=0"
# `admitted` is not shared: doc_verify's is what the signed records admit, the readers print `genesis=`,
# the keys with no ADMIT record, which is all a reader with no signature code can count (a reader's
# keys_disagree is the same kind of number: the table against the REVOKEs' CLAIM, on their chain alone).
SHARED="records chained legacy redacted broken forks keys segments cut cut_redacted declared keys_disagree unsealed merges merges_bad"

agree() { # agree <file> <the signature verifier's output> [no-anchor]: both structure readers against it, field by field
  sig=$(printf '%s\n' "$2" | sed -n 1p)
  for who in python node; do
    if [ "$who" = python ]; then o=$(py "$1" --verify --dump 2>/dev/null); else o=$(nd "$1" --verify --dump 2>/dev/null); fi
    line=$(printf '%s\n' "$o" | sed -n '/^verify /p')
    for k in $SHARED; do
      a=$(field "$sig" "$k"); b=$(field "$line" "$k")
      [ -n "$a" ] && [ "$a" = "$b" ] || red "$1: $k: doc_verify '$a' vs $who '$b' ('$line') DISAGREE"
    done
    a=$(($(field "$sig" seals_ok) + $(field "$sig" seals_bad))); b=$(field "$line" seals)
    [ "$a" = "$b" ] || red "$1: seals: doc_verify ok+bad $a vs $who '$b' DISAGREE"
    [ "$(field "$line" checked)" = 0 ] || red "$1: $who claims it checked a seal"
    a=$(printf '%s\n' "$2" | grep '^head '); b=$(printf '%s\n' "$o" | grep '^head ')
    [ -n "$a" ] && [ "$a" = "$b" ] || red "$1: the heads: doc_verify '$a' vs $who '$b' DISAGREE"
    a=$(printf '%s\n' "$2" | sed -n 's/^anchor \(kind=[0-9]* over=[0-9a-f]* time_ms=[0-9]*\) .*/\1/p')
    b=$(printf '%s\n' "$o" | sed -n 's/^anchor \(kind=[0-9]* over=[0-9a-f]* time_ms=[0-9]*\) .*/\1/p')
    { [ -n "$a" ] || [ "${3:-}" = no-anchor ]; } && [ "$a" = "$b" ] || red "$1: the anchors: doc_verify '$a' vs $who '$b' DISAGREE"
  done
  a=$(py "$1" 2>/dev/null); b=$(nd "$1" 2>/dev/null)
  if [ -n "$a" ] && [ "$a" = "$b" ]; then say "$1: python and node: '$a'"; else red "$1: python '$a' node '$b' DISAGREE"; fi
}
ou=$(dv hub.sealed 2>"$S/err"); rc=$?
say "doc_verify hub.sealed rc=$rc: $(printf '%s\n' "$ou" | sed -n 1p)"
[ "$rc" = 0 ] && [ "$(printf '%s\n' "$ou" | sed -n 1p)" = "$WANT_HUB" ] && [ "$(printf '%s\n' "$ou" | tail -1)" = HOLDS ] || red "doc_verify hub.sealed rc=$rc, want '$WANT_HUB' + HOLDS; stderr '$(cat "$S/err")'"
printf '%s\n' "$ou" | grep -F 'Доказ **не** каже' > /dev/null || red "doc_verify did not print what the proof does not say"
EARLY=$(printf '%s\n' "$ou" | sed -n 's/^anchor .*over=\([0-9a-f]*\) .*under_cut=true -> unchecked: .*/\1/p')
if [ -n "$EARLY" ]; then say "the early anchor over $EARLY is under_cut: unchecked, the archive's to place"; else red "no anchor reads under_cut=true -> unchecked"; fi
printf '%s\n' "$ou" | grep -q '^anchor .*under_cut=false -> ok: ML-DSA-65 by platform key' || red "the tip's anchor is not ok under the platform key"
au=$(av hub.sealed 2>&1); rc=$?
# the same words on both doors (`anchor` has no --trust: the file's genesis key there, the same key)
if [ "$rc" = 0 ] && [ "$au" = "$ou" ]; then say "anchor verify prints what doc_verify prints, line for line"; else red "anchor verify rc=$rc differs from doc_verify: '$(printf '%s\n' "$au" | sed -n 1p)'"; fi
agree hub.sealed "$ou"
oh=$(dv history.sealed 2>"$S/err"); rc=$?
say "doc_verify history.sealed rc=$rc: $(printf '%s\n' "$oh" | sed -n 1p)"
[ "$rc" = 0 ] && [ "$(printf '%s\n' "$oh" | sed -n 1p)" = "$WANT_HIST" ] || red "doc_verify history.sealed rc=$rc, want '$WANT_HIST'; stderr '$(cat "$S/err")'"
if printf '%s\n' "$oh" | grep -q "^anchor kind=1 over=$EARLY .*names_record=true under_cut=false -> ok:"; then say "the history file places the early anchor: ok, under the platform key"; else red "the history file does not place the early anchor $EARLY"; fi
agree history.sealed "$oh"
# a REAL tombstone over a LEGACY record: the hub's mark is the kind byte's high bit, and the readers read it
cp "$DEV/legacy-forget.sealed" "$DEV/revoke.sealed" "$AUD/"
ol=$(dv legacy-forget.sealed 2>"$S/err"); rc=$?; l=$(printf '%s\n' "$ol" | sed -n 1p)
if [ "$rc" = 0 ] && [ "$(field "$l" redacted)" = 1 ] && [ "$(field "$l" legacy)" = 0 ] && [ "$(field "$l" broken)" = 0 ]; then say "doc_verify legacy-forget.sealed rc=0: a tombstone over a legacy record is redacted=1 broken=0"; else red "doc_verify legacy-forget.sealed rc=$rc: '$l' $(cat "$S/err")"; fi
agree legacy-forget.sealed "$ol" no-anchor
orv=$(dv revoke.sealed 2>"$S/err"); rc=$?; l=$(printf '%s\n' "$orv" | sed -n 1p)
if [ "$rc" = 0 ] && [ "$(field "$l" keys_disagree)" = 0 ] && [ "$(printf '%s\n' "$orv" | tail -1)" = HOLDS ]; then say "doc_verify revoke.sealed rc=0: a REVOKE and its table agree, HOLDS"; else red "doc_verify revoke.sealed rc=$rc: '$l' $(cat "$S/err")"; fi
agree revoke.sealed "$orv" no-anchor
# another auditor, another root of trust: its key signed nothing here, so no seal counts and the signed
# TRUNCATE declares nothing -- the cut file is refused by name, and the uncut history simply does not hold
OTHER=$(printf '%064d' 7)
e=$(clean "$AUD" ./doc_verify hub.sealed --trust "$OTHER" 2>&1 >/dev/null); r=$?
case "$r:$e" in 2:*"status=75 CUT_UNDECLARED"*) kills=$((kills + 1)); say "KILL (b) an actor the auditor trusts did not sign the cut -> doc_verify rc=2 status=75 CUT_UNDECLARED" ;; *) red "KILL another trust root FAILED: rc=$r '$e'" ;; esac
l=$(clean "$AUD" ./doc_verify history.sealed --trust "$OTHER" 2>/dev/null | sed -n '1p;$p'); r=$?
case "$l" in *"seals_ok=0 "*"DOES NOT HOLD") kills=$((kills + 1)); say "KILL (b) the same stranger on the uncut history -> seals_ok=0, DOES NOT HOLD" ;; *) red "KILL another trust root on history FAILED: '$l'" ;; esac
[ "$fail" = "$before" ] && { held=$((held + 1)); say "SCENARIO (b) held: signatures by doc_verify and anchor, structure by python and node, every shared counter equal"; }

# ── (c) A CLEAN NODE READS THE ARCHIVE WITH THE READER INSIDE IT ─────────────────────────────
cp "$ROOT/examples/read.mjs" "$AUD/archive.store" "$NOD/"
before=$fail
or=$(clean "$NOD" node ./read.mjs ./archive.store 2>&1); rc=$?
printf '%s\n' "$or" | grep -E '^(embedded reader|verify|anchor|doc status)' | cut -c1-150 | sed 's/^/maingoal: read.mjs | /'
[ "$rc" = 0 ] || red "read.mjs rc=$rc: $or"
printf '%s\n' "$or" | grep -q "^embedded reader: .* sha256 $READER_SHA " || red "read.mjs did not name the committed reader $READER_SHA"
printf '%s\n' "$or" | grep -q '^verify(whole arena) = OK' || red "read.mjs: the whole-arena verify is not OK"
printf '%s\n' "$or" | grep -q '^seals COUNTED, not CHECKED' || red "read.mjs does not say the seals are counted, not checked"
line=$(printf '%s\n' "$or" | sed -n '/^verify records=/p')
for k in $SHARED; do
  a=$(field "$(printf '%s\n' "$ou" | sed -n 1p)" "$k"); b=$(field "$line" "$k")
  [ -n "$a" ] && [ "$a" = "$b" ] || red "archive: $k: doc_verify on hub.sealed '$a' vs read.mjs on the archive '$b' DISAGREE"
done
[ "$(printf '%s\n' "$or" | grep -c '^anchor kind=1 ')" = 2 ] || red "read.mjs did not list the two anchors"
oa=$(dv archive.store 2>"$S/err"); rc=$?
if [ "$rc" = 0 ] && [ "$(printf '%s\n' "$oa" | sed -n 1p)" = "$WANT_HUB" ]; then say "doc_verify on the archive: the same line, HOLDS (every seal survived the copy, none re-signed)"; else red "doc_verify archive.store rc=$rc '$(printf '%s\n' "$oa" | sed -n 1p)' $(cat "$S/err")"; fi
[ "$fail" = "$before" ] && { held=$((held + 1)); say "SCENARIO (c) held: Node, no npm, no repo, the file's own reader"; }

# ── THE KILLS: one changed byte per scenario, each to its named refusal ──────────────────────
forge() { python3 -I "$HERE/maingoal.forge.py" "$1" "$AUD/$2" "$AUD/$3" || red "forge $1 failed"; }
killed() { kills=$((kills + 1)); say "KILL $*"; }
CRC="BAD_CRC: an object the read reached fails its crc"
UND="CUT_UNDECLARED: a log segment left this file"

# (a) a bit of the newest record, no repair: all three refuse by the SAME words
forge rec-bit hub.sealed k1.sealed
e1=$(dv k1.sealed 2>&1 >/dev/null); r1=$?
e2=$(py k1.sealed 2>&1 >/dev/null); r2=$?
e3=$(nd k1.sealed 2>&1 >/dev/null); r3=$?
w=$(printf '%s\n' "$e2" | grep -m1 "^$CRC")
if [ "$r1" = 2 ] && [ "$r2" = 10 ] && [ "$r3" = 10 ] && [ -n "$w" ] && [ "$e2" = "$e3" ]; then
  case "$e1" in *"status=10 $w"*) killed "(a) rec-bit -> doc_verify rc=2 'status=10 BAD_CRC', python rc=10, node rc=10, one text on all three" ;; *) red "KILL rec-bit FAILED: doc_verify's words '$e1' are not '$w'" ;; esac
else red "KILL rec-bit FAILED: doc_verify rc=$r1 '$e1'; python rc=$r2 '$e2'; node rc=$r3 '$e3'"; fi

# (a) the oldest kept record, crc and anchor resealed: the bytes are whole, the id no longer hashes
forge kept-bit hub.sealed k2.sealed
o=$(dv k2.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
if [ "$r" = 3 ] && [ "$(field "$l" broken)" = 1 ] && [ "$(field "$l" chained)" = 3 ] && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ]; then killed "(a) kept-bit resealed -> rc=3 broken=1 chained=3 DOES NOT HOLD"
else red "KILL kept-bit FAILED: rc=$r '$l'"; fi
agree k2.sealed "$o"

# (b) a signature byte of an older seal, resealed: ONLY ML-DSA can see it, and the readers say what they can
forge seal-old hub.sealed k3.sealed
o=$(dv k3.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
p=$(py k3.sealed --verify 2>/dev/null); pr=$?
if [ "$r" = 3 ] && [ "$(field "$l" seals_bad)" = 1 ] && [ "$pr" = 0 ] && [ "$(field "$(printf '%s\n' "$p" | sed -n 2p)" checked)" = 0 ]; then killed "(b) seal-old resealed -> doc_verify rc=3 seals_bad=1; python rc=0 checked=0, the limit it states"
else red "KILL seal-old FAILED: rc=$r '$l' python rc=$pr '$p'"; fi

# (b) the newest seal covers the TRUNCATE: forged, the cut is no longer declared by any valid seal
forge seal-new hub.sealed k4.sealed
e=$(dv k4.sealed 2>&1 >/dev/null); r=$?
case "$r:$e" in 2:*"status=75 $UND"*) killed "(b) seal-new resealed -> doc_verify rc=2 status=75 CUT_UNDECLARED" ;; *) red "KILL seal-new FAILED: rc=$r '$e'" ;; esac

# (b) a signature byte of the tip's platform mark, resealed
forge proof-byte hub.sealed k5.sealed
o=$(dv k5.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
if [ "$r" = 3 ] && [ "$(field "$l" anchors_bad)" = 1 ] && printf '%s\n' "$o" | grep -q '^anchor .* -> BAD: the ML-DSA-65 signature by platform key'; then killed "(b) proof-byte resealed -> rc=3 anchors_bad=1 '-> BAD'"
else red "KILL proof-byte FAILED: rc=$r '$l'"; fi

# (b) the segment's range moved, resealed: every reader names the undeclared cut, in one text
forge seg-range hub.sealed k6.sealed
e1=$(dv k6.sealed 2>&1 >/dev/null); r1=$?
e2=$(py k6.sealed --verify 2>&1 >/dev/null); r2=$?
e3=$(nd k6.sealed --verify 2>&1 >/dev/null); r3=$?
w=$(printf '%s\n' "$e2" | grep -m1 "^$UND")
if [ "$r1" = 2 ] && [ "$r2" = 75 ] && [ "$r3" = 75 ] && [ -n "$w" ]; then
  case "$e1|$e3" in *"status=75 $w"*"|"*"$w"*) killed "(b) seg-range resealed -> doc_verify rc=2, python and node rc=75, CUT_UNDECLARED in one text" ;; *) red "KILL seg-range FAILED: the words differ: doc_verify '$e1' node '$e3'" ;; esac
else red "KILL seg-range FAILED: doc_verify rc=$r1 '$e1'; python rc=$r2 '$e2'; node rc=$r3 '$e3'"; fi

# (b) THE TABLE ZEROED, resealed: a key revoked by a signed record reads as never revoked to anyone who
# believes KEYS. doc_verify refuses the verdict; python and node need no signature code to name it.
forge zero-revoked revoke.sealed k9.sealed
o=$(dv k9.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
pl=$(py k9.sealed --verify 2>/dev/null | sed -n '/^verify /p'); nl=$(nd k9.sealed --verify 2>/dev/null | sed -n '/^verify /p')
if [ "$r" = 3 ] && [ "$(field "$l" keys_disagree)" = 1 ] && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ] && [ "$(field "$pl" keys_disagree)" = 1 ] && [ "$(field "$nl" keys_disagree)" = 1 ] && [ "$(field "$pl" checked)" = 0 ] && [ "$(field "$nl" checked)" = 0 ]; then killed "(b) zero-revoked resealed -> doc_verify rc=3 keys_disagree=1 DOES NOT HOLD; python and node keys_disagree=1 checked=0"
else red "KILL zero-revoked FAILED: doc_verify rc=$r '$l'; python '$pl'; node '$nl'"; fi

# (b) A TAIL NO SEAL COVERS: the newest record stays, its seal is dropped, HEADS points at the seal before
# it (resealed), so every seal left is ok and nothing is broken. Only the walk from the seals' tips sees it.
cp "$DEV/before-forget.sealed" "$AUD/"
forge unsealed-tail before-forget.sealed k10.sealed
o=$(dv k10.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
pl=$(py k10.sealed --verify 2>/dev/null | sed -n '/^verify /p'); nl=$(nd k10.sealed --verify 2>/dev/null | sed -n '/^verify /p')
if [ "$r" = 3 ] && [ "$(field "$l" unsealed)" = 1 ] && [ "$(field "$l" seals_bad)" = 0 ] && [ "$(field "$l" broken)" = 0 ] && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ] && [ "$(field "$pl" unsealed)" = 1 ] && [ "$(field "$nl" unsealed)" = 1 ]; then killed "(b) unsealed-tail resealed -> doc_verify rc=3 unsealed=1 seals_bad=0 DOES NOT HOLD; python and node unsealed=1"
else red "KILL unsealed-tail FAILED: doc_verify rc=$r '$l'; python '$pl'; node '$nl'"; fi

# W-MG-READ: ATK's holes as a script forges them, no key and no repo. The flag is inside id v2: with it
# stripped the id stops hashing, where M0's id stayed right and the body moved (hole 2).
forge flag-strip hub.sealed k11.sealed
o=$(dv k11.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
if [ "$r" = 3 ] && [ "$(field "$l" broken)" = 1 ] && [ "$(field "$l" chained)" = 3 ] && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ]; then killed "(b) flag-strip resealed -> rc=3 broken=1 chained=3 DOES NOT HOLD"
else red "KILL flag-strip FAILED: rc=$r '$l'"; fi
agree k11.sealed "$o"

# hole 1: the root's pk changed in the unsigned KEY row. A script holds no ML-DSA, so it cannot re-sign:
# this kill lands on the signatures as well as on `actor_of`. atk_holes hole 1 is the one that re-signs.
forge root-pk history.sealed k12.sealed
o=$(dv k12.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
if [ "$r" = 3 ] && [ "$(field "$l" seals_ok)" = 0 ] && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ]; then killed "(b) root-pk resealed -> rc=3 seals_ok=0 DOES NOT HOLD"
else red "KILL root-pk FAILED: rc=$r '$l'"; fi

# hole 5: a frontier row moved. The six fields of the TRUNCATE still name the segment; only the signed
# chain hash covers the row, so a verdict is not reached: all three refuse the file by name.
forge frontier-row hub.sealed k13.sealed
o=$(dv k13.sealed 2>&1 >/dev/null); r=$?
p=$(py k13.sealed --verify 2>&1 >/dev/null); pr=$?
n=$(nd k13.sealed --verify 2>&1 >/dev/null); nr=$?
if [ "$r" = 2 ] && [ "$pr" = 75 ] && [ "$nr" = 75 ]; then
  case "$o|$p|$n" in *"status=75 $UND"*"|"*"$UND"*"|"*"$UND"*) killed "(b) frontier-row resealed -> doc_verify rc=2, python and node rc=75, CUT_UNDECLARED" ;; *) red "KILL frontier-row FAILED: the words: doc_verify '$o' python '$p' node '$n'" ;; esac
else red "KILL frontier-row FAILED: doc_verify rc=$r '$o'; python rc=$pr '$p'; node rc=$nr '$n'"; fi

# hole 3: two tombstones trade payloads. Law 9 still balances (redacted == declared) and every id and
# link stands; only FORGOTTEN's digests see it. doc_verify and python both count it, node has no cell.
forge tomb-swap history.sealed k14.sealed
o=$(dv k14.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
pl=$(py k14.sealed --verify 2>&1 >/dev/null | sed -n 's/^tombs_foreign=\([0-9]*\) .*/\1/p')
if [ "$r" = 3 ] && [ "$(field "$l" tombs_foreign)" = 2 ] && [ "$(field "$l" redacted)" = 2 ] && [ "$(field "$l" declared)" = 2 ] && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ] && [ "$pl" = 2 ]; then killed "(b) tomb-swap resealed -> rc=3 tombs_foreign=2 redacted=2 declared=2 DOES NOT HOLD; python tombs_foreign=2"
else red "KILL tomb-swap FAILED: rc=$r '$l'; python tombs_foreign='$pl'"; fi

# (c) a byte of the embedded reader: refused before it runs. A byte of a record: its own reader says 10.
forge wasm-byte archive.store k7.store; cp "$AUD/k7.store" "$NOD/"
o=$(clean "$NOD" node ./read.mjs ./k7.store 2>&1); r=$?
case "$r:$o" in 4:*"REFUSED: the embedded reader hashes to"*) killed "(c) wasm-byte -> read.mjs rc=4 REFUSED, not run" ;; *) red "KILL wasm-byte FAILED: rc=$r '$o'" ;; esac
forge rec-bit archive.store k8.store; cp "$AUD/k8.store" "$NOD/"
o=$(clean "$NOD" node ./read.mjs ./k8.store 2>&1); r=$?
case "$r:$o" in 10:*"doc status=10 n=0 root=0"*"$CRC"*) killed "(c) rec-bit on the archive -> read.mjs rc=10 BAD_CRC" ;; *) red "KILL archive rec-bit FAILED: rc=$r '$o'" ;; esac

# ── (d) TWO OFFLINE DEVICES, ONE GENESIS, NO SERVER ──────────────────────────────────────────
# Each device runs in its own directory holding its own file and the one it was handed. Neither
# has the other's seed in any file; `--who` picks which key the shared run seed derives.
before=$fail
MA="$S/device-a"; MB="$S/device-b"; MW="$S/writer"
mkdir -p "$MA" "$MB" "$MW"
for d in "$MA" "$MB" "$MW"; do cp "$BIN/maingoal_merge" "$d/"; done
MSEED=$(od -An -tx1 -N32 /dev/urandom | tr -d ' \n')
say "merge seed $MSEED"
out=$(clean "$MW" ./maingoal_merge write . --seed "$MSEED" 2>&1); rc=$?
printf '%s\n' "$out" | sed 's/^/maingoal: /'
[ "$rc" = 0 ] || { say "RED the merge device was refused rc=$rc"; exit 1; }
cp "$MW"/*.sealed "$MA/"; cp "$MW"/*.sealed "$MB/"; cp "$MW"/*.sealed "$AUD/"
mg() { d=$1; w=$2; ms=$3; shift 3; clean "$d" ./maingoal_merge merge "$@" --seed "$MSEED" --who "$w" --ms "$ms"; }
MV_ARGS="--trust $(cat "$MW/merge.actor")"
mv_() { clean "$AUD" ./doc_verify "$1" $MV_ARGS; }
held_line() { # held_line <file> <what>: rc 0, HOLDS; sets $o and $l
  o=$(mv_ "$1" 2>"$S/err"); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
  [ "$r" = 0 ] && [ "$(printf '%s\n' "$o" | tail -1)" = HOLDS ] || red "doc_verify $1 ($2) rc=$r '$l' $(cat "$S/err")"
}

# commuting tails
r=$(mg "$MA" a 1000 calm-a.sealed calm-b.sealed calm-m.sealed 2>&1); rc=$?
[ "$rc" = 0 ] && [ "$r" = "merged records=1 conflicts=0" ] || red "the calm merge rc=$rc '$r'"
cp "$MA/calm-m.sealed" "$AUD/"; held_line calm-m.sealed "commuting tails"
[ "$(field "$l" merges)" = 1 ] && [ "$(field "$l" merges_bad)" = 0 ] && [ "$(field "$l" seals_ok)" = 5 ] || red "calm-m: '$l'"
printf '%s\n' "$o" | grep -q '^merge .*state=commutes .*conflicts=0 recheck=same$' || red "calm-m: no commuting MERGE line"
agree calm-m.sealed "$o" no-anchor

# the last salmon, sold on both: each device merges the other, then each takes in the other's MERGE
r=$(mg "$MA" a 1000 salmon-a.sealed salmon-b.sealed merged-a.sealed 2>&1); rc=$?
[ "$rc" = 0 ] && [ "$r" = "merged records=1 conflicts=1" ] || red "A's merge rc=$rc '$r'"
r=$(mg "$MB" b 1001 salmon-b.sealed salmon-a.sealed merged-b.sealed 2>&1); rc=$?
[ "$rc" = 0 ] && [ "$r" = "merged records=2 conflicts=1" ] || red "B's merge rc=$rc '$r'"
cp "$MB/merged-b.sealed" "$MA/"; cp "$MA/merged-a.sealed" "$MB/"
r=$(mg "$MA" a 1002 merged-a.sealed merged-b.sealed final-a.sealed 2>&1); rc=$?
[ "$rc" = 0 ] && [ "$r" = "adopted records=1" ] || red "A taking B's MERGE rc=$rc '$r': a third MERGE would never end"
r=$(mg "$MB" b 1003 merged-b.sealed merged-a.sealed final-b.sealed 2>&1); rc=$?
[ "$rc" = 0 ] && [ "$r" = "adopted records=1" ] || red "B taking A's MERGE rc=$rc '$r'"
cp "$MA/merged-a.sealed" "$MA/final-a.sealed" "$MB/final-b.sealed" "$AUD/"
cp "$MB/final-b.sealed" "$MA/"
for f in final-a.sealed final-b.sealed; do
  held_line "$f" "both MERGEs"
  [ "$(field "$l" records)" = 8 ] && [ "$(field "$l" seals_ok)" = 6 ] && [ "$(field "$l" merges)" = 2 ] && [ "$(field "$l" forks)" = 0 ] || red "$f: '$l'"
  printf '%s\n' "$o" | grep -q '^conflict .*"kind":"oversold","item":"salmon".*salmon: на двох пристроях' || red "$f: the conflict is not named"
  printf '%s\n' "$o" | grep -q '^Цей файл звів власник ключа K' || red "$f: the MERGE's sentence is not printed"
  [ "$(printf '%s\n' "$o" | grep -c 'recheck=same$')" = 2 ] || red "$f: the MERGEs do not re-derive"
  agree "$f" "$o" no-anchor
done
ha=$(mv_ final-a.sealed | grep '^head '); hb=$(mv_ final-b.sealed | grep '^head ')
if [ -n "$ha" ] && [ "$ha" = "$hb" ]; then say "the two devices end on one set of heads"; else red "the heads differ: '$ha' vs '$hb'"; fi
r=$(mg "$MA" a 1004 final-a.sealed final-b.sealed end.sealed 2>&1); rc=$?
[ "$rc" = 0 ] && [ "$r" = nothing ] && cmp -s "$MA/final-a.sealed" "$MA/end.sealed" || red "the two ends still differ rc=$rc '$r'"
[ "$fail" = "$before" ] && { held=$((held + 1)); say "SCENARIO (d) held: two devices, one genesis, no server, every seal real, the conflict named, nothing dropped"; }

# the kills: one flipped byte, two MERGEs a real key signed, and each refusal by name, nothing written
forge merge-bit merged-a.sealed k15.sealed
o=$(mv_ k15.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
if [ "$r" = 3 ] && [ "$(field "$l" broken)" = 1 ] && [ "$(field "$l" merges)" = 0 ] && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ]; then killed "(d) merge-bit resealed -> rc=3 broken=1 merges=0 DOES NOT HOLD"
else red "KILL merge-bit FAILED: rc=$r '$l'"; fi
agree k15.sealed "$o" no-anchor
o=$(mv_ lie.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
pl=$(py lie.sealed --verify 2>/dev/null | sed -n '/^verify /p'); nl=$(nd lie.sealed --verify 2>/dev/null | sed -n '/^verify /p')
if [ "$r" = 3 ] && [ "$(field "$l" merges_bad)" = 1 ] && [ "$(field "$l" seals_bad)" = 0 ] && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ] && [ "$(field "$pl" merges_bad)" = 1 ] && [ "$(field "$nl" merges_bad)" = 1 ]; then killed "(d) a signed MERGE over a record no file holds -> rc=3 merges_bad=1 seals_bad=0; python and node merges_bad=1"
else red "KILL lie FAILED: rc=$r '$l'; python '$pl'; node '$nl'"; fi
o=$(mv_ hidden.sealed 2>&1); r=$?; l=$(printf '%s\n' "$o" | sed -n 1p)
if [ "$r" = 3 ] && [ "$(field "$l" merges_bad)" = 0 ] && [ "$(field "$l" seals_bad)" = 0 ] && printf '%s\n' "$o" | grep -q '^merge .*conflicts=0 recheck=DIFFERS$' && [ "$(printf '%s\n' "$o" | tail -1)" = "DOES NOT HOLD" ]; then killed "(d) a signed MERGE hiding the conflict -> rc=3 recheck=DIFFERS DOES NOT HOLD"
else red "KILL hidden FAILED: rc=$r '$l'"; fi
refused_by() { # refused_by <name> <who> <ours> <theirs>
  e=$(mg "$MA" "$2" 2000 "$3" "$4" gone.sealed 2>&1 >/dev/null); r=$?
  case "$r:$e" in
    "4:REFUSED $1"*) if [ -e "$MA/gone.sealed" ]; then red "KILL $1 wrote a file"; else killed "(d) $3 + $4 -> rc=4 REFUSED $1, nothing written"; fi ;;
    *) red "KILL $1 FAILED: rc=$r '$e'" ;;
  esac
  rm -f "$MA/gone.sealed"
}
refused_by FOREIGN_ROOT a calm-a.sealed other.sealed
refused_by FORK a fork-a.sealed fork-b.sealed
refused_by REVOKED_TAIL a revoke-a.sealed revoke-b.sealed
refused_by REVOKED_TAIL b revoke-b.sealed revoke-a.sealed
refused_by FORGET_ONE_SIDE a forget-a.sealed forget-b.sealed
refused_by FORGET_ONE_SIDE b forget-b.sealed forget-a.sealed
refused_by CUT_ONE_SIDE a cut-a.sealed cut-b.sealed
refused_by NOT_WHOLE a calm-a.sealed lie.sealed
if grep -qaF "+355692222222" "$MW/forget-b.sealed" && ! grep -qaF "+355692222222" "$MW/forget-a.sealed"; then say "control: the phone is in forget-b.sealed and not in forget-a.sealed, and no merge of the two exists"; else red "control: the erasure pair is not what the kill says"; fi
for pair in "calm-a.sealed calm-a.sealed itself" "merged-a.sealed salmon-b.sealed replay"; do
  set -- $pair
  r=$(mg "$MA" a 2000 "$1" "$2" same.sealed 2>&1); rc=$?
  if [ "$rc" = 0 ] && [ "$r" = nothing ] && cmp -s "$MA/$1" "$MA/same.sealed"; then killed "(d) $3 -> 'nothing', the file byte for byte"; else red "KILL $3 FAILED: rc=$rc '$r'"; fi
done

took=$(($(date +%s) - started))
if [ "$fail" != 0 ]; then say "RED ($held of 4 scenarios held, $kills kills named; ${took}s)"; exit 1; fi
say "GREEN ($held of 4 scenarios held from a clean dir, $kills kills named; ${took}s)"
exit 0
