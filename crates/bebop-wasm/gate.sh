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
  exit 0
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

if [ "$fail" -ne 0 ]; then
  say "RED"
  exit 1
fi
if [ "$minran" -lt 3 ]; then
  say "RED -- fewer than three readers ran; that is not a parity check"
  exit 1
fi
say "GREEN ($minran readers on each of: $FIXTURES + key compile/proj/empty + proj memo; $bytes bytes)"
exit 0
