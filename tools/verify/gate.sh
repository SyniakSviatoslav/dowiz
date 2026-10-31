#!/bin/sh
# SPDX-License-Identifier: MIT OR Apache-2.0
# THE CLEAN-MACHINE GATE for bebop-verify (W-MG-MR, W-MG-M5; R-MAINGOAL-A §7 M5 kill-test, -B §4):
# a Node with NO repo and a Python with NO repo each read a fixture from ONE file and answer the
# line conformance.sh's five readers answer, on all 21 of its fixtures, the log fixture, and the
# DOC fixture (W-MG-M5: the five refuse DOC by name, 12; these two READ it, and must agree with
# each other and with doc.expected's `doc_n` / `doc_root` / `doc_verify` lines).
#
# "No repo" is enforced, not assumed: the two verifier files and the fixtures are COPIED to an
# empty directory outside the checkout, the gate cd's there, and the verifiers run with that as
# the only path they see. A verifier that imported anything beside node:fs / the stdlib, or read
# any file but its argument, would fail there.
#
# WITH THE BUILD (the §7 M5 kill-test, heavy, under slot.sh): tools/verify/build.sh makes the reader;
# bebop-log's `write_the_archive_fixture_when_asked` writes an ARCHIVE (a DOC with SPEC first, the
# reader just built embedded, sha256 ids) into the clean dir; `examples/read.mjs` -- Node 22, no npm,
# ~30 lines -- reads that file WITH THE READER INSIDE IT and must print heads, records, counters and
# statuses in words; both verifiers must agree on it; and two REDs are shown: a flipped record cell
# -> status 10 in words, a flipped reader byte -> REFUSED exit 4 before anything runs. The archive is
# not committed (it would churn with the reader); its per-file cost is printed.
#
# --prove: the cell conformance.sh --prove names (bit 0 of the first value cell of kv3max's newest
# delta record) is flipped and BOTH verifiers must move from their clean answer to status 10.
# --no-build: skip the build and the archive (what tools/gates/verify-file.sh runs under run-all).
#
# Usage: tools/verify/gate.sh [--prove] [--no-build]   (heavy with the build: under slot.sh)
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
V="$ROOT/tools/verify"
FIX="$ROOT/crates/bebop-wasm/fixtures"
PROVE=0; BUILD=1
for a in "$@"; do
  case "$a" in
    --prove) PROVE=1 ;;
    --no-build) BUILD=0 ;;
    *) echo "verify-gate: unknown argument $a" >&2; exit 2 ;;
  esac
done
say() { echo "verify-gate: $*"; }
fail=0
sh "$V/assemble.sh" --check || { say "RED the committed bebop-verify.py is not the assembly of tools/verify/py"; exit 1; }
if [ "$BUILD" = 1 ]; then sh "$V/build.sh" || { say "RED build"; exit 1; }; fi
[ -f "$V/bebop-verify.mjs" ] || { say "RED tools/verify/bebop-verify.mjs is absent (run build.sh)"; exit 1; }
sha=$(sed -n 's/^const WASM_SHA256 = "\([0-9a-f]*\)";$/\1/p' "$V/bebop-verify.mjs")

# the same list conformance.sh reads (W-ES-P10), plus the one log fixture and the DOC fixture
FIXTURES="kv kv2 kv3 ptaway kv4stub kv3max kv3c torn ptbad empty kv0 kv4small kv4deep kv4del kv4zero kv4badn kv4badv kv4badf kv4badd kv4stale kv4old"
CLEAN="${VERIFY_SCRATCH:-${TMPDIR:-/tmp}/bebop-verify-clean.$$}"
rm -rf "$CLEAN"; mkdir -p "$CLEAN"
trap 'rm -rf "$CLEAN"' EXIT
cp "$V/bebop-verify.mjs" "$V/bebop-verify.py" "$ROOT/examples/read.mjs" "$CLEAN/"
for f in $FIXTURES proj4.log doc; do cp "$FIX/$f.store" "$CLEAN/"; done
say "clean dir $CLEAN holds $(ls "$CLEAN" | wc -l | tr -d ' ') files and no repo"

want() { awk -F= "/^$2=/{print \$2}" "$FIX/$1.expected"; }
# one verifier over one copied image, FROM the clean dir, HOME and PATH stripped to the interpreter
node_read() { (cd "$CLEAN" && env -i PATH="$(dirname "$(command -v node)")" node ./bebop-verify.mjs "$@" 2>/dev/null); }
py_read() { (cd "$CLEAN" && env -i PATH="$(dirname "$(command -v python3)")" python3 -I ./bebop-verify.py "$@" 2>/dev/null); }

agree=0
for f in $FIXTURES; do
  st=$(want "$f" status)
  if [ -n "$st" ]; then exp="status=$st"; else exp="status=0 n=$(want "$f" n) root=$(want "$f" root)"; fi
  n=$(node_read "./$f.store"); nrc=$?
  p=$(py_read "./$f.store"); prc=$?
  bad=""
  if [ -n "$st" ]; then
    case "$n" in "kv status=$st "*) ;; *) bad="$bad node" ;; esac
    case "$p" in "kv status=$st "*) ;; *) bad="$bad python" ;; esac
    [ "$nrc" = "$st" ] || bad="$bad node-rc=$nrc"
    [ "$prc" = "$st" ] || bad="$bad python-rc=$prc"
  else
    [ "$n" = "kv $exp" ] && [ "$nrc" = 0 ] || bad="$bad node(rc=$nrc)"
    [ "$p" = "kv $exp" ] && [ "$prc" = 0 ] || bad="$bad python(rc=$prc)"
  fi
  if [ -n "$bad" ]; then say "$f: want '$exp' -- node '$n' python '$p' DISAGREE:$bad"; fail=1
  else say "$f: want '$exp' -- node and python agree"; agree=$((agree + 1)); fi
done
# the log fixture: the log fold `proj.expected` records for the same chain (DG5)
exp="log status=0 n=$(want proj n) root=$(want proj root)"
n=$(node_read ./proj4.log.store); p=$(py_read ./proj4.log.store)
if [ "$n" = "$exp" ] && [ "$p" = "$exp" ]; then say "proj4.log: want '$exp' -- node and python agree"
else say "proj4.log: want '$exp' -- node '$n' python '$p' DISAGREE"; fail=1; fi

# the DOC fixture (W-MG-M5): the status line AND the verify line, both verifiers, against doc.expected
doc_check() { # doc_check <label> <image> <want status line> <want verify line>
  n=$(node_read "$2" --verify); p=$(py_read "$2" --verify)
  n1=$(printf '%s\n' "$n" | sed -n 1p); n2=$(printf '%s\n' "$n" | sed -n 2p)
  p1=$(printf '%s\n' "$p" | sed -n 1p); p2=$(printf '%s\n' "$p" | sed -n 2p)
  if [ "$n1" = "$3" ] && [ "$p1" = "$3" ] && [ "$n2" = "$4" ] && [ "$p2" = "$4" ]; then say "$1: '$3' + '$4' -- node and python agree"
  else say "$1: want '$3' + '$4' -- node '$n1' / '$n2' python '$p1' / '$p2' DISAGREE"; fail=1; fi
}
doc_check doc ./doc.store "doc status=0 n=$(want doc doc_n) root=$(want doc doc_root)" "verify $(sed -n 's/^doc_verify=//p' "$FIX/doc.expected")"

if [ "$BUILD" = 1 ]; then
  # THE ARCHIVE: SPEC first, the reader just built inside, sha256 ids; read by the file's own reader.
  WASM="${BWR_TARGET_DIR:-$ROOT/crates/bebop-wasm-reader/target}/wasm32-unknown-unknown/release/bebop_wasm_reader.wasm"
  CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"; [ -x "$CARGO" ] || CARGO=cargo
  if ! (cd "$ROOT/crates/bebop-log" && BEBOP_ARCHIVE_FIXTURE="$CLEAN/archive.store" BEBOP_CUT_FIXTURE="$CLEAN/cut.store" BEBOP_CUT_UNDECLARED_FIXTURE="$CLEAN/cut-undeclared.store" BEBOP_READER_WASM="$WASM" BEBOP_FORMAT_TEXT="$ROOT/docs/FORMAT-BEBOP-IMAGE-v2.md" \
      CARGO_INCREMENTAL=0 "$CARGO" test --offline --target-dir "${BLOG_TARGET_DIR:-$ROOT/crates/bebop-log/target}" write_the_) > "$CLEAN/archive.log" 2>&1 || [ ! -s "$CLEAN/archive.store" ] || [ ! -s "$CLEAN/cut.store" ] || [ ! -s "$CLEAN/cut-undeclared.store" ]; then
    say "RED the archive fixture was not written -- $(grep -m1 -E 'error|panicked|FAILED' "$CLEAN/archive.log")"; fail=1
  else
    say "archive: $(wc -c < "$CLEAN/archive.store" | tr -d ' ') bytes = doc.store $(wc -c < "$FIX/doc.store" | tr -d ' ') + reader $(wc -c < "$WASM" | tr -d ' ') + FORMAT $(wc -c < "$ROOT/docs/FORMAT-BEBOP-IMAGE-v2.md" | tr -d ' ') + headers (the per-file cost of SPEC, §2f)"
    doc_check archive ./archive.store "doc status=0 n=3 root=$(py_read ./archive.store | sed 's/.*root=//')" "verify records=3 chained=3 legacy=0 redacted=0 broken=0 forks=0 seals=3 checked=0 heads=1 keys=1 admitted=1 segments=0 cut=0 cut_redacted=0 declared=0"
    out=$(cd "$CLEAN" && env -i PATH="$(dirname "$(command -v node)")" node ./read.mjs ./archive.store 2>&1); rc=$?
    printf '%s\n' "$out" | sed 's/^/verify-gate: read.mjs | /'
    ok=1
    printf '%s\n' "$out" | grep -q "^embedded reader: $(wc -c < "$WASM" | tr -d ' ') bytes, sha256 $sha " || ok=0
    printf '%s\n' "$out" | grep -q '^doc status=0 n=3 root=' || ok=0
    [ "$(printf '%s\n' "$out" | grep -c '^head actor=')" = 1 ] || ok=0
    [ "$(printf '%s\n' "$out" | grep -c '^#[0-9]* seq=')" = 3 ] || ok=0
    printf '%s\n' "$out" | grep -q '^verify records=3 chained=3 legacy=0 redacted=0 broken=0 forks=0 seals=3 checked=0 heads=1 keys=1 admitted=1 segments=0 cut=0 cut_redacted=0 declared=0$' || ok=0
    printf '%s\n' "$out" | grep -q '^verify(whole arena) = OK' || ok=0
    printf '%s\n' "$out" | grep -q '^seals COUNTED, not CHECKED' || ok=0
    if [ "$rc" = 0 ] && [ "$ok" = 1 ]; then say "read.mjs: a clean Node read the archive with the reader embedded in it (rc=0, every line present)"
    else say "RED read.mjs rc=$rc -- a wanted line is missing above"; fail=1; fi
    # RED 1: the newest record's first payload cell -> the crc refuses (10). RED 2: one byte of the
    # embedded reader -> REFUSED before it runs. RED 3 (the counter's own red): the OLDEST record's
    # payload edited AND ITS CRC RE-SEALED, as a forger would -- the bytes are whole, status stays 0,
    # and the id no longer hashes: `chained` 3 -> 2, `broken` 0 -> 1. A verifier that still said
    # chained=3 would be calling a broken file healthy. (The oldest record is outside the last
    # commit's anchor range, so the anchor is not what catches it.)
    python3 -I - "$CLEAN/archive.store" "$CLEAN/flip-rec.store" "$CLEAN/flip-wasm.store" "$CLEAN/forged.store" <<'PY' || { say "RED could not name the archive cells"; fail=1; }
import struct, sys, zlib
b = bytearray(open(sys.argv[1], "rb").read())
c = lambda i: struct.unpack_from("<q", b, 8 * i)[0]
sb = max((at for at in (0, 512) if c(at) == 3554557610294396226), key=lambda at: c(at + 2))
def reanchor(x, at):
    # a forger inside the LAST commit also re-anchors it (FORMAT §7): cell at+9 = (mark << 32) | crc32(cells[mark..used)),
    # then the superblock's own crc (cell at+15, low 32) -- else the anchor, not the counter, is what refuses (41)
    g = lambda i: struct.unpack_from("<Q", x, 8 * i)[0]
    anc = g(at + 9)
    if anc:
        mark, used = anc >> 32, g(at + 4)
        struct.pack_into("<Q", x, 8 * (at + 9), (mark << 32) | zlib.crc32(x[8 * mark: 8 * used]))
        struct.pack_into("<Q", x, 8 * (at + 15), (g(at + 15) & ~0xFFFFFFFF) | zlib.crc32(x[8 * at: 8 * (at + 15)]))
doc = c(c(sb + 3) + 18); log = doc + c(doc + 2 + 1); rec = log + c(log + 2 + 1)
r = bytearray(b); r[8 * (rec + 2 + 16)] ^= 1; open(sys.argv[2], "wb").write(r)
w = 1024 + c(1027); x = bytearray(b); x[(w + 3) * 8 + 100] ^= 1; open(sys.argv[3], "wb").write(x)
old = rec
while c(old + 2 + 2): old = old + c(old + 2 + 2)
f = bytearray(b); f[8 * (old + 2 + 16)] ^= 1
ln = c(old) & 0xFFFFFFFF; crc = zlib.crc32(f[8 * (old + 2): 8 * (old + 2 + ln)])
struct.pack_into("<Q", f, 8 * (old + 1), (crc << 32) | (c(old + 1) & 0xFFFFFFFF)); reanchor(f, sb); open(sys.argv[4], "wb").write(f)
print("verify-gate: archive DOC %d log %d newest record %d -> flipped bit 0 of cell %d; reader blob %d -> flipped byte 100; oldest record %d -> payload bit flipped and crc re-sealed to %08x" % (doc, log, rec, rec + 18, w, old, crc))
PY
    o1=$(cd "$CLEAN" && node ./read.mjs ./flip-rec.store 2>&1); r1=$?
    o2=$(cd "$CLEAN" && node ./read.mjs ./flip-wasm.store 2>&1); r2=$?
    o3=$(cd "$CLEAN" && node ./read.mjs ./forged.store 2>&1); r3=$?
    case "$o1" in *"doc status=10 n=0 root=0"*"BAD_CRC:"*) say "RED proof 1: flipped record -> rc=$r1 '$(printf '%s' "$o1" | tail -1 | cut -c1-80)'" ;; *) say "RED proof 1 FAILED: rc=$r1 '$o1'"; fail=1 ;; esac
    case "$r2:$o2" in 4:*"REFUSED: the embedded reader hashes to"*) say "RED proof 2: flipped reader byte -> rc=4, not run" ;; *) say "RED proof 2 FAILED: rc=$r2 '$o2'"; fail=1 ;; esac
    forged="verify records=3 chained=2 legacy=0 redacted=0 broken=1 forks=0 seals=3 checked=0 heads=1 keys=1 admitted=1 segments=0 cut=0 cut_redacted=0 declared=0"
    case "$r3:$o3" in 0:*"doc status=0 n=3 root="*"$forged"*) say "RED proof 3: forged record (crc re-sealed) -> rc=0 and '$forged'" ;; *) say "RED proof 3 FAILED: rc=$r3 '$o3'"; fail=1 ;; esac
    doc_check forged ./forged.store "$(py_read ./forged.store | sed -n 1p)" "$forged"

    # THE CUT (W-MG-M5b, M3's rule through the readers): five sha256 commits cut to the newest two, the
    # TRUNCATE committed, archived with SPEC first. Three readers agree: segments=1 cut=3, the kept
    # chain continues the frontier (chained=3, broken=0). RED 4: the same cut archived BEFORE the
    # TRUNCATE -> every reader says 75 CUT_UNDECLARED by name. RED 5: the oldest KEPT record forged
    # (payload bit, crc re-sealed) -> chained 2, broken 1, the segment still declared.
    cutline="verify records=3 chained=3 legacy=0 redacted=0 broken=0 forks=0 seals=3 checked=0 heads=1 keys=1 admitted=1 segments=1 cut=3 cut_redacted=0 declared=0"
    doc_check cut ./cut.store "$(py_read ./cut.store | sed -n 1p)" "$cutline"
    o4=$(cd "$CLEAN" && env -i PATH="$(dirname "$(command -v node)")" node ./read.mjs ./cut.store 2>&1); r4=$?
    case "$r4:$o4" in 0:*"$cutline"*) say "cut: read.mjs read the cut archive with its own reader: '$cutline'" ;; *) say "RED cut: read.mjs rc=$r4 '$o4'"; fail=1 ;; esac
    doc_check cut-undeclared ./cut-undeclared.store "$(py_read ./cut-undeclared.store | sed -n 1p)" "verify status=75"
    o5=$(cd "$CLEAN" && node ./read.mjs ./cut-undeclared.store 2>&1); r5=$?
    case "$r5:$o5" in 75:*"verify status=75 -- CUT_UNDECLARED:"*) say "RED proof 4: the cut archived without its TRUNCATE -> rc=75 'verify status=75 -- CUT_UNDECLARED: ...' on read.mjs, node and python" ;; *) say "RED proof 4 FAILED: rc=$r5 '$o5'"; fail=1 ;; esac
    python3 -I - "$CLEAN/cut.store" "$CLEAN/cut-forged.store" <<'PY2' || { say "RED could not name the cut cells"; fail=1; }
import struct, sys, zlib
b = bytearray(open(sys.argv[1], "rb").read())
c = lambda i: struct.unpack_from("<q", b, 8 * i)[0]
sb = max((at for at in (0, 512) if c(at) == 3554557610294396226), key=lambda at: c(at + 2))
def reanchor(x, at):
    # a forger inside the LAST commit also re-anchors it (FORMAT §7): cell at+9 = (mark << 32) | crc32(cells[mark..used)),
    # then the superblock's own crc (cell at+15, low 32) -- else the anchor, not the counter, is what refuses (41)
    g = lambda i: struct.unpack_from("<Q", x, 8 * i)[0]
    anc = g(at + 9)
    if anc:
        mark, used = anc >> 32, g(at + 4)
        struct.pack_into("<Q", x, 8 * (at + 9), (mark << 32) | zlib.crc32(x[8 * mark: 8 * used]))
        struct.pack_into("<Q", x, 8 * (at + 15), (g(at + 15) & ~0xFFFFFFFF) | zlib.crc32(x[8 * at: 8 * (at + 15)]))
doc = c(c(sb + 3) + 18); log = doc + c(doc + 2 + 1); old = log + c(log + 2 + 1)
while c(old + 2 + 2): old = old + c(old + 2 + 2)
b[8 * (old + 2 + 16)] ^= 1
ln = c(old) & 0xFFFFFFFF; crc = zlib.crc32(b[8 * (old + 2): 8 * (old + 2 + ln)])
struct.pack_into("<Q", b, 8 * (old + 1), (crc << 32) | (c(old + 1) & 0xFFFFFFFF)); reanchor(b, sb); open(sys.argv[2], "wb").write(b)
print("verify-gate: cut archive oldest KEPT record %d -> payload bit flipped and crc re-sealed to %08x" % (old, crc))
PY2
    cutforged="verify records=3 chained=2 legacy=0 redacted=0 broken=1 forks=0 seals=3 checked=0 heads=1 keys=1 admitted=1 segments=1 cut=3 cut_redacted=0 declared=0"
    doc_check cut-forged ./cut-forged.store "$(py_read ./cut-forged.store | sed -n 1p)" "$cutforged"
    o6=$(cd "$CLEAN" && node ./read.mjs ./cut-forged.store 2>&1); r6=$?
    case "$r6:$o6" in 0:*"$cutforged"*) say "RED proof 5: forged kept record in the cut archive -> rc=0 and '$cutforged'" ;; *) say "RED proof 5 FAILED: rc=$r6 '$o6'"; fail=1 ;; esac
  fi
fi

if [ "$PROVE" = 1 ]; then
  python3 -I - "$FIX/kv3max.store" "$CLEAN/flip.store" <<'PY' || { say "RED --prove: could not name the cell"; exit 1; }
import struct, sys
b = bytearray(open(sys.argv[1], "rb").read())
c = lambda i: struct.unpack_from("<q", b, 8 * i)[0]
sb = max((at for at in (0, 512) if c(at) == 3554557610294396226), key=lambda at: c(at + 2))
pt = c(sb + 3); root = c(pt + 18); rec = root + c(root + 2 + 6)
cell = rec + 2 + 4 + (c(rec + 4) + 7) // 8
b[8 * cell] ^= 1
open(sys.argv[2], "wb").write(b)
print("verify-gate --prove: kv3max.store newest record %d -> flipped bit 0 of cell %d" % (rec, cell))
PY
  for r in node python; do
    if [ "$r" = node ]; then a=$(node_read ./kv3max.store); b=$(node_read ./flip.store); brc=$?; else a=$(py_read ./kv3max.store); b=$(py_read ./flip.store); brc=$?; fi
    say "prove $r: clean '$a' -> flipped '$b' rc=$brc"
    case "$b" in "kv status=10 "*) [ "$a" != "$b" ] || { say "RED --prove: $r did not move"; fail=1; } ;; *) say "RED --prove: $r did not land on status 10"; fail=1 ;; esac
  done
fi

bytes=$(cat "$V/bytes.baseline" 2>/dev/null || echo "?")
if [ "$fail" -ne 0 ]; then say "RED"; exit 1; fi
say "GREEN (node and python, from a clean dir, agree with conformance on $agree of 21 fixtures + the log + doc$([ "$BUILD" = 1 ] && echo ' + the archive and the cut archive read by their own reader, 5 REDs'); reader wasm $bytes bytes, sha256 $sha)"
exit 0
