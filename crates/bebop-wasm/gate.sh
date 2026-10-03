#!/bin/sh
# THE FOUR-WAY FOLD, RUN. (DG5 adds a third image, `fixtures/proj.store`: a log with a
# projection memo, step 6 below.) Two images -- `fixtures/kv.store` (v1, one byte per
# cell, FROZEN: every image written before DG3 looks like it) and
# `fixtures/kv2.store` (v2, eight bytes to a cell, DG3 2026-09-28); four
# readers; one number each. The gate is green only when every reader that
# COULD run agrees with the fixture's `.expected` on BOTH images, and for each
# image it prints how many readers agreed and NAMES each one that did not.
#
#   bebop.bin  -- selfhost/std/kv.bp, compiled and run by bebop.bin. AArch64
#                 only, so on an x86 CI runner it is NOT MEASURED and said so;
#                 a skip is never counted as agreement.
#   native     -- this crate's `cargo test` (tests/parity.rs reads the same file).
#   wasm32     -- this crate built for wasm32-unknown-unknown, run by node.
#   python     -- oracle.py, from the bytes alone.
#
# AND A BYTE RATCHET. `bytes.baseline` holds the wasm module's size; a build
# that grows it is refused, a build that shrinks it is announced so the
# baseline can be lowered in the same commit. The number is the deliverable:
# it is what the format's reader costs a Worker, and nothing else is in it.
#
# `--prove` shows the gate can fail: it flips one bit of a COPY of the fixture,
# runs the wasm reader on it, and expects a refusal or a moved root. A gate
# whose number cannot move is not measuring anything (bebop-lang AGENTS.md).
#
# Toolchain: the wasm32 target is in the rustup toolchain the repo pins
# (rust-toolchain.toml -> 1.96.1), not in the distro cargo on PATH, so cargo
# is taken from $HOME/.cargo/bin explicitly -- the same rule wrangler.toml states.
set -u
cd "$(dirname "$0")"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
WASM=target/wasm32-unknown-unknown/release/bebop_wasm.wasm
BASELINE=bytes.baseline
FIXTURES="kv kv2"
SCRATCH="${BEBOP_WASM_SCRATCH:-${TMPDIR:-/tmp}/bebop-wasm-gate.$$}"
mkdir -p "$SCRATCH"
trap 'rm -rf "$SCRATCH"' EXIT
fail=0

say() { echo "bebop-wasm: $*"; }
want() { awk -F= "/^$2=/{print \$2}" "fixtures/$1.expected"; }

# ---- BLOCKS (DG9, SPEC-DATALOG-AND-CODEC §B.6): the machinery, used by step 7 and by --prove ----
# Four readers of one block file, each DECODING it and printing what it decoded:
#   block <schema> n=<n> nnz=<nnz> vals=<fnv64 of the decoded columns> rt=<ok|diff> k256=<sha256>
#   block refused=<code> col=<column|->
# `rt` is encode(decode(b)) == b byte for byte and `k256` is the sha256 of that RE-ENCODING, so
# a reader that never decoded cannot print either. The readers:
#   native    -- DG7's own decode/encode (crates/dowiz-hub/src/block), run on the file by
#                crates/dowiz-hub/examples/block_read.rs
#   wasm32    -- src/block.rs, a decoder/encoder of its own, built for wasm32 and run by node
#   python    -- oracle.py --block, re-derived from the spec text
#   bebop.bin -- bebop-lang/selfhost/std/block.bp via bench/vs_rust/std_tests/block_rt.bp
#                (AArch64 only: elsewhere it is NOT MEASURED, said so, never counted)
# A reader that is absent is NAMED absent and counts as a disagreement, never as a skip.
HUB=../dowiz-hub
BLOCKFIX=$(cd "$HUB/fixtures/blocks" && pwd)
BLOCKSEED=${BLOCK_SEED:-2609300009}
BLOCKCOUNT=${BLOCK_COUNT:-1000}
BLOCKWASM=target/block/wasm32-unknown-unknown/release/bebop_wasm.wasm
SEED=../../bebop-lang/seed/build/seed
BIN=../../bebop-lang/bebop.bin
BLOCKREADERS="native wasm32 python"
HUBEX=""
blocks_setup() {
  if "$CARGO" build --offline --examples --target-dir "$(pwd)/target/hub" --manifest-path "$HUB/Cargo.toml" > "$SCRATCH/hubex.txt" 2>&1; then
    HUBEX=target/hub/debug/examples
  else
    say "blocks native: dowiz-hub examples did not build rc=$? -- $(grep -m1 '^error' "$SCRATCH/hubex.txt")"
    fail=1
  fi
  # The block reader's module: `bw_block` is exported only under `--cfg bw_block`, built apart
  # like key/proj, so the Worker's module (and bytes.baseline) carry none of it.
  if RUSTFLAGS="--cfg bw_block" "$CARGO" build --release --target wasm32-unknown-unknown --offline --target-dir target/block > "$SCRATCH/blockwasm.txt" 2>&1; then
    say "wasm32 block: $(wc -c < "$BLOCKWASM" | tr -d ' ') bytes with bw_block exported (not in the ratchet)"
  else
    say "wasm32 block: build FAILED rc=$? -- $(grep -m1 'error' "$SCRATCH/blockwasm.txt")"
    fail=1
  fi
  BRTBIN=""
  if [ "$(uname -m)" = "aarch64" ] && [ -x "$SEED" ] && [ -f "$BIN" ]; then
    BLOCKREADERS="$BLOCKREADERS bebop.bin"
    if (cd ../../bebop-lang && ./seed/build/seed ./bebop.bin compile bench/vs_rust/std_tests/block_rt.bp "$SCRATCH/block_rt.bin") > "$SCRATCH/brtc.txt" 2>&1 && [ -s "$SCRATCH/block_rt.bin" ]; then
      BRTBIN="$SCRATCH/block_rt.bin"
    else
      say "blocks bebop.bin: block_rt.bp did not compile rc=$? -- $(tail -1 "$SCRATCH/brtc.txt")"
    fi
  else
    say "blocks bebop.bin: box-only, NOT MEASURED here (uname -m = $(uname -m)) -- not counted as agreement"
  fi
  cat > "$SCRATCH/block.mjs" <<'EOFJS'
// node block.mjs <module.wasm> <list>: one bw_block line per file of the list.
import { readFileSync } from "node:fs";
const [, , mod, list] = process.argv;
const { instance } = await WebAssembly.instantiate(readFileSync(mod), {});
const { memory, bw_alloc, bw_free, bw_block } = instance.exports;
if (typeof bw_block !== "function") { console.log("wasm32 ABSENT: the module exports no bw_block"); process.exit(3); }
const cap = 512;
const out = bw_alloc(cap);
for (const p of readFileSync(list, "utf8").split("\n").filter((x) => x)) {
  const img = readFileSync(p);
  const at = bw_alloc(Math.max(img.length, 1));
  new Uint8Array(memory.buffer, at, img.length).set(img);
  const n = bw_block(at, img.length, out, cap);
  console.log(n < 0 ? `block status=${n}` : new TextDecoder().decode(new Uint8Array(memory.buffer, out, n)));
  bw_free(at, Math.max(img.length, 1));
}
bw_free(out, cap);
EOFJS
  cat > "$SCRATCH/view.mjs" <<'EOFJS'
// node view.mjs <module.wasm> view <list>            : one bw_view line per file of the list
// node view.mjs <module.wasm> menu <prices> <names>  : the bw_menu line of the pair
import { readFileSync } from "node:fs";
const [, , mod, mode, a, b] = process.argv;
const { instance } = await WebAssembly.instantiate(readFileSync(mod), {});
const { memory, bw_alloc, bw_free, bw_view, bw_menu } = instance.exports;
if (typeof bw_view !== "function" || typeof bw_menu !== "function") { console.log("wasm32 ABSENT: the module exports no bw_view/bw_menu"); process.exit(3); }
const cap = 512;
const out = bw_alloc(cap);
const put = (img) => { const at = bw_alloc(Math.max(img.length, 1)); new Uint8Array(memory.buffer, at, img.length).set(img); return at; };
const say = (n) => console.log(n < 0 ? `status=${n}` : new TextDecoder().decode(new Uint8Array(memory.buffer, out, n)));
if (mode === "view") {
  for (const p of readFileSync(a, "utf8").split("\n").filter((x) => x)) {
    const img = readFileSync(p);
    const at = put(img);
    say(bw_view(at, img.length, out, cap));
    bw_free(at, Math.max(img.length, 1));
  }
} else {
  const [pi, ni] = [readFileSync(a), readFileSync(b)];
  say(bw_menu(put(pi), pi.length, put(ni), ni.length, out, cap));
}
EOFJS
}

# view_judge <tag> <list> (BN3, src/block_view.rs): the IN-PLACE view over every block of the list,
# held line by line against the decode readers' agreed line minus its ` rt=.. k256=..` half (taken
# from the python reader, which blocks_judge has just held to the others). A view that reads one
# element wrong moves `vals`; one that refuses differently names another check.
view_judge() {
  if [ ! -f "$BLOCKWASM" ]; then say "view $1: ABSENT -- no block module"; fail=1; return; fi
  node "$SCRATCH/view.mjs" "$BLOCKWASM" view "$2" > "$SCRATCH/view.$1" 2>&1
  sed 's/ rt=.*$//' "$SCRATCH/blk.$1.python" > "$SCRATCH/view.$1.want"
  total=$(grep -c . "$2")
  same=$(paste -d '\n' "$SCRATCH/view.$1" "$SCRATCH/view.$1.want" | awk 'NR % 2 { a = $0; next } a == $0 { n++ } END { print n + 0 }')
  if [ "$same" -eq "$total" ] && [ "$(grep -c . "$SCRATCH/view.$1")" -eq "$total" ]; then
    say "view $1: $total blocks; the in-place view (wasm32 bw_view) prints the decode readers' line on $same of $total"
  else
    say "view $1: DISAGREES on $((total - same)) of $total -- first: $(paste -d '|' "$SCRATCH/view.$1" "$SCRATCH/view.$1.want" | awk -F'|' '$1 != $2 { print; exit }')"
    fail=1
  fi
}

# blocks_run <tag> <list>: every reader over the list, into $SCRATCH/blk.<tag>.<reader>.
blocks_run() {
  for r in $BLOCKREADERS; do
    o="$SCRATCH/blk.$1.$r"
    case $r in
      native) if [ -n "$HUBEX" ]; then "$HUBEX/block_read" "@$2" > "$o" 2>&1; else echo "native ABSENT: the dowiz-hub examples did not build" > "$o"; fi ;;
      wasm32) if [ -f "$BLOCKWASM" ]; then node "$SCRATCH/block.mjs" "$BLOCKWASM" "$2" > "$o" 2>&1; else echo "wasm32 ABSENT: no block module" > "$o"; fi ;;
      python) python3 oracle.py --block "@$2" > "$o" 2>&1 ;;
      bebop.bin)
        if [ -n "$BRTBIN" ]; then
          "$SEED" "$BRTBIN" "$2" > "$o.raw" 2>&1
          echo "rc=$?" >> "$o.raw"
          grep '^block ' "$o.raw" > "$o"
          # the program returns how many blocks it read: the seed prints it last, before our rc line
          say "blocks $1 bebop.bin: returned '$(tail -2 "$o.raw" | head -1)', $(tail -1 "$o.raw")"
        else
          echo "bebop.bin ABSENT: block_rt.bp did not compile" > "$o"
        fi ;;
    esac
  done
}

# blocks_judge <tag> <list> <mode> [expect]: compare the readers' lines, line i = block i.
#   ok     -- every reader prints the SAME accepted `rt=ok` line (valid blocks)
#   same   -- every reader prints the same line, accepted (rt=ok) or refused (corrupted blocks)
#   refuse -- every reader prints exactly line i of <expect>, a named refusal
# Names every reader that does not, with its first bad line; writes the agreeing count to
# $SCRATCH/blk.<tag>.agree. A reader with fewer lines than blocks disagrees.
blocks_judge() {
  python3 - "$SCRATCH" "$1" "$2" "$3" "${4:-}" $BLOCKREADERS <<'EOFPY'
import sys
scratch, tag, lst, mode, expect, readers = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5], sys.argv[6:]
paths = [p for p in open(lst).read().split("\n") if p]
lines = {r: open("%s/blk.%s.%s" % (scratch, tag, r)).read().split("\n") for r in readers}
want = [x for x in open(expect).read().split("\n") if x] if mode == "refuse" else None
def at(r, i):
    return lines[r][i] if i < len(lines[r]) else "<no line>"
absent = [r for r in readers if " ABSENT" in at(r, 0)]
present = [r for r in readers if r not in absent]
agree, bad = [], []
for r in present:
    first = None
    for i, p in enumerate(paths):
        g = at(r, i)
        if mode == "refuse":
            ok, why = g == want[i], " (want '%s')" % want[i]
        else:
            others = [at(q, i) for q in present]
            accepted = g.startswith("block ") and " n=" in g
            ok = others.count(g) * 2 > len(present) and (" rt=ok " in g if accepted else mode == "same" and g.startswith("block refused="))
            why = ""
        if not ok:
            first = "%s: '%s'%s" % (p.rsplit("/", 1)[-1], g, why)
            break
    (bad if first else agree).append((r, first))
for r in absent:
    print("bebop-wasm: blocks %s %s: ABSENT -- '%s'" % (tag, r, at(r, 0)))
for r, f in bad:
    print("bebop-wasm: blocks %s %s: DISAGREES -- first: %s" % (tag, r, f))
print("bebop-wasm: blocks %s: %d blocks; readers agreeing on every line: %d of %d (%s)%s%s" % (
    tag, len(paths), len(agree), len(readers), " ".join(r for r, _ in agree),
    "" if not bad else " -- DISAGREE: " + " ".join(r for r, _ in bad),
    "" if not absent else " -- ABSENT: " + " ".join(absent)))
open("%s/blk.%s.agree" % (scratch, tag), "w").write("%d\n" % len(agree))
sys.exit(1 if bad or absent else 0)
EOFPY
}

# blocks_prove: one corrupted byte per header region of a COPY of a fixture (the crc re-sealed
# except for the crc's own region, so the check under test is the one reached), each with the
# refusal the spec names. Every reader must print exactly that line; a reader that accepts, or
# names another region, FAILS. Then a sweep: every header byte of every fixture xor'd three
# ways, sealed and not -- the readers must print the same line for each.
blocks_prove() {
  python3 - "$BLOCKFIX" "$SCRATCH/prove" <<'EOFPY'
import os, struct, sys, zlib
fix, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
def rd(n):
    return bytearray(open("%s/%s.dwb" % (fix, n), "rb").read())
def seal(b):
    struct.pack_into("<I", b, len(b) - 4, zlib.crc32(bytes(b[:-4])) & 0xFFFFFFFF)
    return b
def desc(b, i):
    return 24 + 16 * i
def coloff(b, i):
    return struct.unpack_from("<I", b, desc(b, i) + 8)[0]
cases = []
def case(region, name, b, want, sealed=True):
    cases.append((region, name, bytes(seal(b) if sealed else b), want))
def flip(name, at, x, region, want, sealed=True):
    b = rd(name); b[at] ^= x; case(region, name, b, want, sealed)
M = "menu_prices"
flip(M, 0, 0x01, "magic", "block refused=bad_magic col=-")
flip(M, 4, 0x01, "version", "block refused=bad_version col=-")
flip(M, len(rd(M)) - 1, 0x01, "crc", "block refused=bad_crc col=-", sealed=False)
flip(M, 16, 0x01, "schema", "block refused=unknown_schema col=-")
flip(M, 6, 0x01, "ncols", "block refused=bad_ncols col=-")
flip(M, 8, 0x01, "n", "block refused=bad_length col=dish")
flip(M, 12, 0x01, "nnz", "block refused=bad_length col=mods_col")
flip(M, desc(0, 1) + 0, 0x01, "types", "block refused=bad_type col=price")
flip(M, desc(0, 1) + 1, 0x01, "flags", "block refused=bad_type col=price")
flip(M, desc(0, 1) + 2, 0x01, "units", "block refused=bad_unit col=price")
flip(M, desc(0, 1) + 3, 0x01, "reserved", "block refused=bad_reserved col=price")
flip(M, desc(0, 1) + 12, 0x01, "reserved2", "block refused=bad_reserved col=price")
flip(M, desc(0, 1) + 8, 0x01, "alignment", "block refused=bad_alignment col=price")
flip(M, desc(0, 1) + 8, 0x08, "offsets", "block refused=bad_offsets col=price")
flip(M, desc(0, 1) + 4, 0x08, "length", "block refused=bad_length col=price")
m = rd(M); pad = coloff(m, 4) + struct.unpack_from("<I", m, desc(m, 4) + 4)[0]
assert pad % 8 != 0, "menu_prices fixture has no padding before mods_val"
flip(M, pad, 0x01, "padding", "block refused=bad_padding col=mods_val")
b = rd(M); struct.pack_into("<I", b, coloff(b, 3) + 4, 0xFFFFFFFF); case("row_ptr", M, b, "block refused=bad_offsets col=mods_ptr")
case("too_short", M, rd(M)[:27], "block refused=too_short col=-", sealed=False)
N = "names"
b = rd(N); b[coloff(b, 1)] = 0xFF; case("utf8", N, b, "block refused=not_utf8 col=bytes")
b = rd(N); b[coloff(b, 1)] = 0x00; case("nul", N, b, "block refused=not_utf8 col=bytes")
b = rd(N); struct.pack_into("<I", b, coloff(b, 2) + 4, 0x7FFFFFFF); case("text_off", N, b, "block refused=bad_offsets col=off")
flip("stock_levels", 12, 0x01, "nnz_no_csr", "block refused=bad_nnz col=-")
case("too_big", M, bytearray(b"DWB1") + bytearray(96 * 1024 + 4), "block refused=too_big col=-", sealed=False)
with open(out + "/list", "w") as L, open(out + "/expect", "w") as E, open(out + "/regions", "w") as R:
    for i, (region, name, b, want) in enumerate(cases):
        p = "%s/%02d_%s.dwb" % (out, i, region)
        open(p, "wb").write(b)
        L.write(p + "\n"); E.write(want + "\n"); R.write("%s %s\n" % (region, name))
sw = []
for f in sorted(os.listdir(fix)):
    b0 = bytearray(open(os.path.join(fix, f), "rb").read())
    hdr = 24 + 16 * struct.unpack_from("<H", b0, 6)[0]
    for i in range(min(hdr, len(b0) - 4)):
        for x in (0x01, 0x80, 0xFF):
            for sealed in (False, True):
                b = bytearray(b0); b[i] ^= x
                p = "%s/sweep_%05d.dwb" % (out, len(sw)); open(p, "wb").write(seal(b) if sealed else b); sw.append(p)
open(out + "/sweep", "w").write("\n".join(sw) + "\n")
print("bebop-wasm: prove blocks: %d named regions, %d sweep blocks" % (len(cases), len(sw)))
EOFPY
  pf=0
  blocks_run prove "$SCRATCH/prove/list"
  i=0
  while read -r region name; do
    i=$((i + 1))
    for r in $BLOCKREADERS; do
      say "prove block $region ($name.dwb): $r -> '$(sed -n "${i}p" "$SCRATCH/blk.prove.$r")'"
    done
  done < "$SCRATCH/prove/regions"
  blocks_judge prove "$SCRATCH/prove/list" refuse "$SCRATCH/prove/expect" || pf=1
  blocks_run sweep "$SCRATCH/prove/sweep"
  blocks_judge sweep "$SCRATCH/prove/sweep" same || pf=1
  view_judge prove "$SCRATCH/prove/list"
  view_judge sweep "$SCRATCH/prove/sweep"
  return $pf
}


# 1. native: one parity test per fixture, so a failure names the image ------
# One cargo run (not --quiet, so every test prints its own `... ok` line).
if "$CARGO" test --offline > "$SCRATCH/native.txt" 2>&1; then
  say "native: cargo test rc=0"
else
  say "native: cargo test FAILED rc=$? -- $(grep -m1 'panicked\|error' "$SCRATCH/native.txt")"
  fail=1
fi

# 2. wasm32 ---------------------------------------------------------------
if ! "$CARGO" build --release --target wasm32-unknown-unknown --offline > "$SCRATCH/wasm.txt" 2>&1; then
  say "wasm32: build FAILED rc=$? -- $(grep -m1 'error' "$SCRATCH/wasm.txt")"
  say "wasm32: the target lives in the rustup toolchain, not /usr/bin/cargo; CARGO=$CARGO"
  exit 1
fi
bytes=$(wc -c < "$WASM" | tr -d ' ')
gz=$(gzip -9c "$WASM" | wc -c | tr -d ' ')
say "wasm32: $bytes bytes ($gz gzip)"
if [ ! -f "$BASELINE" ]; then
  echo "$bytes" > "$BASELINE"
  say "wasm32: baseline written at $bytes"
else
  b=$(cat "$BASELINE")
  if [ "$bytes" -gt "$b" ]; then
    say "wasm32: REFUSED -- the module grew $b -> $bytes. The ratchet only falls."
    fail=1
  elif [ "$bytes" -lt "$b" ]; then
    say "wasm32: the ratchet has fallen $b -> $bytes. Lower $BASELINE in this commit."
  fi
fi
# The node-key reader's module (DG2): the same crate with `bw_key` exported,
# built apart so the ratchet above measures the Worker's reader and nothing else.
KEYWASM=target/key/wasm32-unknown-unknown/release/bebop_wasm.wasm
if "$CARGO" build --release --target wasm32-unknown-unknown --offline --features key --target-dir target/key > "$SCRATCH/keywasm.txt" 2>&1; then
  kb=$(wc -c < "$KEYWASM" | tr -d ' ')
  say "wasm32 key: $kb bytes with bw_key exported (+$((kb - bytes)) over the reader; not in the ratchet)"
else
  say "wasm32 key: build FAILED rc=$? -- $(grep -m1 'error' "$SCRATCH/keywasm.txt")"
  fail=1
fi
# The projection reader's module (DG5): `bw_proj` exported, built apart like the key's.
PROJWASM=target/proj/wasm32-unknown-unknown/release/bebop_wasm.wasm
if "$CARGO" build --release --target wasm32-unknown-unknown --offline --features proj --target-dir target/proj > "$SCRATCH/projwasm.txt" 2>&1; then
  pb=$(wc -c < "$PROJWASM" | tr -d ' ')
  say "wasm32 proj: $pb bytes with bw_proj exported (+$((pb - bytes)) over the reader; not in the ratchet)"
else
  say "wasm32 proj: build FAILED rc=$? -- $(grep -m1 'error' "$SCRATCH/projwasm.txt")"
  fail=1
fi
if [ "${1:-}" = "--prove" ]; then
  for f in $FIXTURES; do
    # One bit, in a copy: the LAST payload byte of the image, which is the
    # newest value blob.
    python3 - "fixtures/$f.store" "$SCRATCH/broken.store" <<'EOF2'
import sys
b = bytearray(open(sys.argv[1], 'rb').read())
b[-8] ^= 1
open(sys.argv[2], 'wb').write(b)
EOF2
    out=$(node harness.mjs "$WASM" "$SCRATCH/broken.store" 2>&1)
    rc=$?
    say "prove $f: one flipped payload bit -> '$out' rc=$rc"
    case "$out" in
      *"root=$(want "$f" root)"*) say "prove $f: FAILED -- the root did not move"; exit 1 ;;
    esac
  done
  say "prove: the number moved on every fixture; the gate measures the bytes"
  blocks_setup
  if blocks_prove && [ "$fail" -eq 0 ]; then
    say "prove blocks: every reader refused every corrupted region by name, and the sweep agreed"
    exit 0
  fi
  say "prove blocks: FAILED"
  exit 1
fi

# 3. bebop.bin, compiled once -----------------------------------------------
SEED=../../bebop-lang/seed/build/seed
BIN=../../bebop-lang/bebop.bin
KVBIN=""
NKBIN=""
NKTRY=0
SPBIN=""
if [ "$(uname -m)" = "aarch64" ] && [ -x "$SEED" ] && [ -f "$BIN" ]; then
  if (cd ../../bebop-lang && ./seed/build/seed ./bebop.bin compile selfhost/std/kv.bp "$SCRATCH/kv.bin") > "$SCRATCH/kvc.txt" 2>&1; then
    KVBIN="$SCRATCH/kv.bin"
  else
    say "bebop.bin: kv.bp did not compile rc=$? -- $(tail -1 "$SCRATCH/kvc.txt")"; fail=1
  fi
  NKTRY=1
  if (cd ../../bebop-lang && ./seed/build/seed ./bebop.bin compile bench/vs_rust/std_tests/sproj.bp "$SCRATCH/sproj.bin") > "$SCRATCH/spc.txt" 2>&1; then
    SPBIN="$SCRATCH/sproj.bin"
  else
    say "bebop.bin: sproj.bp did not compile rc=$? -- $(tail -1 "$SCRATCH/spc.txt")"; fail=1
  fi
  if (cd ../../bebop-lang && ./seed/build/seed ./bebop.bin compile bench/vs_rust/std_tests/nodekey.bp "$SCRATCH/nodekey.bin") > "$SCRATCH/nkc.txt" 2>&1; then
    NKBIN="$SCRATCH/nodekey.bin"
  else
    say "bebop.bin: nodekey.bp did not compile rc=$? -- $(tail -1 "$SCRATCH/nkc.txt")"; fail=1
  fi
else
  say "bebop.bin: NOT MEASURED (uname -m = $(uname -m); seed present: $([ -x "$SEED" ] && echo yes || echo no)) -- a skip is not an agreement"
fi

# 4. every reader, on every fixture -------------------------------------------
minran=4
for f in $FIXTURES; do
  FIX=fixtures/$f.store
  want_n=$(want "$f" n)
  want_root=$(want "$f" root)
  ran=0
  bad=""
  # native: tests/parity.rs has `<fixture>_fixture_reads_to_what_bebop_bin_printed`.
  if grep -q "test ${f}_fixture_reads_to_what_bebop_bin_printed ... ok" "$SCRATCH/native.txt"; then
    ran=$((ran + 1))
  else
    bad="$bad native"
  fi
  out=$(node harness.mjs "$WASM" "$FIX" 2>&1)
  say "$f wasm32: '$out' rc=$?"
  if [ "$out" = "kv status=0 n=$want_n root=$want_root" ]; then ran=$((ran + 1)); else bad="$bad wasm32"; fi
  out=$(python3 oracle.py "$FIX" 2>&1)
  say "$f python: '$out' rc=$?"
  if [ "$out" = "kv status=0 n=$want_n root=$want_root" ]; then ran=$((ran + 1)); else bad="$bad python"; fi
  if [ -n "$KVBIN" ]; then
    # kv.bp opens `kv.store` in the working directory and EXTENDS it to 64 MiB
    # (st_open ftruncates), so it reads a copy, never the fixture.
    mkdir -p "$SCRATCH/$f" && cp "$FIX" "$SCRATCH/$f/kv.store"
    n=$(cd "$SCRATCH/$f" && "$OLDPWD/$SEED" "$KVBIN" n 2>&1)
    r=$(cd "$SCRATCH/$f" && "$OLDPWD/$SEED" "$KVBIN" h 2>&1)
    say "$f bebop.bin: kv.bin n -> '$n', kv.bin h -> '$r'"
    if [ "$n" = "$want_n" ] && [ "$r" = "$want_root" ]; then ran=$((ran + 1)); else bad="$bad bebop.bin"; fi
  fi

  # THE CUT IMAGE: one cell short of the arena. Every reader must REFUSE it;
  # the Rust reader used to answer the previous generation (n=0).
  head -c $(( $(wc -c < "$FIX") - 8 )) "$FIX" > "$SCRATCH/cut.store"
  wout=$(node harness.mjs "$WASM" "$SCRATCH/cut.store" 2>&1)
  pout=$(python3 oracle.py "$SCRATCH/cut.store" 2>&1)
  say "$f cut: wasm32 -> '$wout'; python -> '$pout'"
  case "$wout" in "kv status=0"*) say "$f cut: wasm32 READ a truncated image"; fail=1 ;; esac
  case "$pout" in "kv status=0"*) say "$f cut: python READ a truncated image"; fail=1 ;; esac

  if [ -n "$bad" ]; then
    say "$f: readers agreeing with $f.expected: $ran of 4 -- DISAGREE:$bad"
    fail=1
  else
    say "$f: readers agreeing with $f.expected: $ran of 4 (bebop.bin counts only on aarch64)"
  fi
  [ "$ran" -lt "$minran" ] && minran=$ran
done

# 5. THE NODE KEY (DG2, SPEC-BEBOP-DAG-RUNTIME §2) -----------------------------
# Three frames -- a compile node, a projection with two inputs, a projection
# whose two lists are empty -- each BUILT by each reader from the field values
# in fixtures/key.expected (never read from a shared byte file), hashed to K64
# and K256. Every reader prints `key <name> len=<n> k64=<i64> k256=<hex>`;
# the line must equal the one key.expected implies, and a reader that does not
# is NAMED. The native reader is bebop_store::nodekey (tests/parity.rs); the
# wasm32 reader is src/nodekey.rs, a frame builder of its own, run by node.
keyline() { echo "key $1 len=$(want key "$1_len") k64=$(want key "$1_k64") k256=$(want key "$1_k256")"; }
if [ -n "$NKBIN" ]; then
  k3=$("$SEED" "$NKBIN" c x 2>&1)
  say "key bebop.bin: compile frame length == 1 + 5*8 + 8 + len(fn_source) + 8 + 8 + 8 -> '$k3' (1 = holds)"
  [ "$k3" = 1 ] || { say "key bebop.bin: K-3 does NOT hold"; fail=1; }
fi
minkey=4
for f in compile proj empty; do
  line=$(keyline "$f")
  ran=0
  bad=""
  if grep -q "test key_${f}_frame_agrees_with_key_expected ... ok" "$SCRATCH/native.txt"; then
    ran=$((ran + 1))
  else
    bad="$bad native"
  fi
  out=$(node harness.mjs --key "$KEYWASM" "$f" 2>&1)
  say "key $f wasm32: '$out' rc=$?"
  if [ "$out" = "$line" ]; then ran=$((ran + 1)); else bad="$bad wasm32"; fi
  out=$(python3 oracle.py --key "$f" 2>&1)
  say "key $f python: '$out' rc=$?"
  if [ "$out" = "$line" ]; then ran=$((ran + 1)); else bad="$bad python"; fi
  if [ -n "$NKBIN" ]; then
    c=$(echo "$f" | cut -c1)
    vals=""
    for w in l k 0 1 2 3; do vals="$vals $("$SEED" "$NKBIN" "$c" "$w" 2>&1)"; done
    out=$(python3 -c 'import sys
v = sys.argv[2:]
try:
    hx = "".join("%016x" % (int(x) & (2**64 - 1)) for x in v[2:6])
    print("key %s len=%s k64=%s k256=%s" % (sys.argv[1], v[0], v[1], hx))
except (ValueError, IndexError):
    print("key %s UNREADABLE:%s" % (sys.argv[1], " ".join(v)))' "$f" $vals)
    say "key $f bebop.bin: '$out'"
    if [ "$out" = "$line" ]; then ran=$((ran + 1)); else bad="$bad bebop.bin"; fi
  elif [ "$NKTRY" = 1 ]; then
    bad="$bad bebop.bin"   # measurable here, and it did not compile: a disagreement, not a skip
  fi
  if [ -n "$bad" ]; then
    say "key $f: readers agreeing with key.expected: $ran of 4 -- DISAGREE:$bad"
    fail=1
  else
    say "key $f: readers agreeing with key.expected: $ran of 4 (bebop.bin counts only on aarch64)"
  fi
  [ "$ran" -lt "$minkey" ] && minkey=$ran
done
[ "$minkey" -lt "$minran" ] && minran=$minkey

# 6. THE PROJECTION MEMO (DG5, SPEC-BEBOP-DAG-RUNTIME §6) ---------------------
# fixtures/proj.store: a 9-record log bebop wrote, with its fold memoised in the
# projection table. Each reader prints `proj status=0 n=<records> root=<log fold>
# memo=<memo value>`; proj.expected derives the numbers. RT S-1 rides along: the
# wasm32 and python readers refuse a valid superblock whose cells 13-14 are not 0.
pline="proj status=0 n=$(want proj n) root=$(want proj root) memo=$(want proj memo)"
ran=0
bad=""
if grep -q "test proj_fixture_reads_to_what_bebop_bin_printed ... ok" "$SCRATCH/native.txt"; then
  ran=$((ran + 1))
else
  bad="$bad native"
fi
out=$(node harness.mjs --proj "$PROJWASM" fixtures/proj.store 2>&1)
say "proj wasm32: '$out' rc=$?"
if [ "$out" = "$pline" ]; then ran=$((ran + 1)); else bad="$bad wasm32"; fi
out=$(python3 oracle.py --proj fixtures/proj.store 2>&1)
say "proj python: '$out' rc=$?"
if [ "$out" = "$pline" ]; then ran=$((ran + 1)); else bad="$bad python"; fi
if [ -n "$SPBIN" ]; then
  # sproj.bin reads `proj.store` in the working directory and EXTENDS it (st_open), so a copy.
  mkdir -p "$SCRATCH/proj" && cp fixtures/proj.store "$SCRATCH/proj/proj.store"
  pn=$(cd "$SCRATCH/proj" && "$OLDPWD/$SEED" "$SPBIN" n 2>&1)
  ph=$(cd "$SCRATCH/proj" && "$OLDPWD/$SEED" "$SPBIN" h 2>&1)
  pp=$(cd "$SCRATCH/proj" && "$OLDPWD/$SEED" "$SPBIN" p 2>&1)
  out="proj status=0 n=$pn root=$ph memo=$pp"
  say "proj bebop.bin: '$out'"
  if [ "$out" = "$pline" ]; then ran=$((ran + 1)); else bad="$bad bebop.bin"; fi
elif [ "$NKTRY" = 1 ]; then
  bad="$bad bebop.bin"
fi
head -c $(( $(wc -c < fixtures/proj.store) - 8 )) fixtures/proj.store > "$SCRATCH/pcut.store"
wout=$(node harness.mjs --proj "$PROJWASM" "$SCRATCH/pcut.store" 2>&1)
pout=$(python3 oracle.py --proj "$SCRATCH/pcut.store" 2>&1)
say "proj cut: wasm32 -> '$wout'; python -> '$pout'"
case "$wout" in "proj status=0"*) say "proj cut: wasm32 READ a truncated image"; fail=1 ;; esac
case "$pout" in "proj status=0"*) say "proj cut: python READ a truncated image"; fail=1 ;; esac
if [ -n "$bad" ]; then
  say "proj: readers agreeing with proj.expected: $ran of 4 -- DISAGREE:$bad"
  fail=1
else
  say "proj: readers agreeing with proj.expected: $ran of 4 (bebop.bin counts only on aarch64)"
fi
[ "$ran" -lt "$minran" ] && minran=$ran

# 7. BLOCKS (DG9, SPEC-DATALOG-AND-CODEC §B.6) --------------------------------
# The fixtures DG7's encoder wrote (the 165-dish catalogue, a 120-supply stock_levels, a small
# and an EMPTY block of every type), then BLOCK_COUNT seeded random blocks per type written by
# the same encoder (examples/generate_fixtures.rs; the seed is printed). Every reader decodes
# every block; the gate compares the printed lines (blocks_judge above).
blocks_setup
ls "$BLOCKFIX"/*.dwb > "$SCRATCH/blk.fix.list"
say "blocks fixtures: $(wc -l < "$SCRATCH/blk.fix.list" | tr -d ' ') files: $(cd "$BLOCKFIX" && ls | tr '\n' ' ')"
blocks_run fix "$SCRATCH/blk.fix.list"
blocks_judge fix "$SCRATCH/blk.fix.list" ok || fail=1
view_judge fix "$SCRATCH/blk.fix.list"
if [ -n "$HUBEX" ] && "$HUBEX/generate_fixtures" random "$SCRATCH/rand" "$BLOCKSEED" "$BLOCKCOUNT" > "$SCRATCH/rand.txt" 2>&1; then
  say "blocks $(head -1 "$SCRATCH/rand.txt"); $(tail -1 "$SCRATCH/rand.txt")"
  blocks_run rand "$SCRATCH/rand/list"
  blocks_judge rand "$SCRATCH/rand/list" ok || fail=1
  view_judge rand "$SCRATCH/rand/list"
else
  say "blocks random: the generator did not run -- $(tail -1 "$SCRATCH/rand.txt" 2>/dev/null)"
  fail=1
fi
# B-2: DG7's schema table, oracle.py's and block.bp's print the same four `schema` lines (the
# wasm32 reader's keys are pinned by src/block/tests.rs table_parses_and_keys_are_dg7s).
[ -n "$HUBEX" ] && "$HUBEX/block_read" --schemas > "$SCRATCH/schema.native" 2>&1
python3 oracle.py --schemas > "$SCRATCH/schema.python" 2>&1
sran=0
sbad=""
for r in native python; do
  if [ -s "$SCRATCH/schema.$r" ] && cmp -s "$SCRATCH/schema.$r" "$SCRATCH/schema.python"; then sran=$((sran + 1)); else sbad="$sbad $r"; fi
done
if [ -n "$BRTBIN" ]; then
  if (cd ../../bebop-lang && ./seed/build/seed ./bebop.bin compile bench/vs_rust/std_tests/block_schema.bp "$SCRATCH/block_schema.bin") > "$SCRATCH/bsc.txt" 2>&1; then
    "$SEED" "$SCRATCH/block_schema.bin" 2>&1 | grep '^schema ' > "$SCRATCH/schema.bebop"
    if cmp -s "$SCRATCH/schema.bebop" "$SCRATCH/schema.python"; then sran=$((sran + 1)); else sbad="$sbad bebop.bin"; fi
  else
    sbad="$sbad bebop.bin"
  fi
fi
if [ -n "$sbad" ]; then
  say "blocks schemas: identical schema tables: $sran -- DISAGREE:$sbad"
  fail=1
else
  say "blocks schemas: identical schema tables: $sran ($(awk '{print $2"="$3}' "$SCRATCH/schema.python" | tr '\n' ' '))"
fi
# 8. THE PUBLISHED MENU (BN3): the two blocks a venue publishes (`menu_prices`, `names`), read as the
# hub's `Catalogue::row_of` reads them -- a dish by its K64, CONFIRMED by its bytes in `names`.
# Readers: wasm32 (bw_menu), python (oracle.py --menu), native (src/block_view/tests.rs pins the
# fixture lines). Pairs: the 165-dish catalogue, the small and the empty one, a names block whose
# first string lost its bytes (must resolve one dish fewer), and the blocks swapped (refused).
python3 - "$BLOCKFIX/names.dwb" "$SCRATCH/names_moved.dwb" <<'EOFPY'
import struct, sys, zlib
b = bytearray(open(sys.argv[1], "rb").read())
off = struct.unpack_from("<I", b, 24 + 16 * 1 + 8)[0]
b[off] = ord("y") if b[off] == ord("x") else ord("x")
struct.pack_into("<I", b, len(b) - 4, zlib.crc32(bytes(b[:-4])) & 0xFFFFFFFF)
open(sys.argv[2], "wb").write(b)
EOFPY
mbad=""
mn=0
for pair in "menu_prices.dwb names.dwb" "menu_prices_small.dwb names_small.dwb" "menu_prices_empty.dwb names_empty.dwb" "menu_prices.dwb @moved" "names.dwb menu_prices.dwb"; do
  set -- $pair
  pf="$BLOCKFIX/$1"
  nf="$BLOCKFIX/$2"
  [ "$2" = "@moved" ] && nf="$SCRATCH/names_moved.dwb"
  w=$( [ -f "$BLOCKWASM" ] && node "$SCRATCH/view.mjs" "$BLOCKWASM" menu "$pf" "$nf" 2>&1 || echo "wasm32 ABSENT")
  p=$(python3 oracle.py --menu "$pf" "$nf" 2>&1)
  say "menu $1 + $2: wasm32 -> '$w'; python -> '$p'"
  mn=$((mn + 1))
  [ "$w" = "$p" ] || mbad="$mbad $1+$2"
done
set --
mran=2
if grep -q "test block_view::tests::menu_lines_are_the_oracles ... ok" "$SCRATCH/native.txt"; then mran=3; else mbad="$mbad native"; fi
if [ -n "$mbad" ]; then
  say "menu: DISAGREE on:$mbad"
  fail=1
else
  say "menu: $mn pairs; readers agreeing on every pair: $mran of 3 (wasm32 python native; bebop.bin has no menu reader)"
fi

for t in fix rand; do
  a=$(cat "$SCRATCH/blk.$t.agree" 2>/dev/null || echo 0)
  [ "$a" -lt "$minran" ] && minran=$a
done

if [ "$fail" -ne 0 ]; then
  say "RED"
  exit 1
fi
if [ "$minran" -lt 3 ]; then
  say "RED -- fewer than three readers ran; that is not a parity check"
  exit 1
fi
say "GREEN ($minran readers on each of: $FIXTURES + key compile/proj/empty + proj memo + blocks (fixtures, $BLOCKCOUNT random per type, seed $BLOCKSEED); $bytes bytes)"
exit 0
