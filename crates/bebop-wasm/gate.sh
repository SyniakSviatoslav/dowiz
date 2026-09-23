#!/bin/sh
# THE FOUR-WAY FOLD, RUN. One image, `fixtures/kv.store`; four readers; one
# number each; the gate is green only when every reader that COULD run agrees
# with `fixtures/kv.expected`, and it says which readers ran.
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
FIX=fixtures/kv.store
SCRATCH="${BEBOP_WASM_SCRATCH:-${TMPDIR:-/tmp}/bebop-wasm-gate.$$}"
mkdir -p "$SCRATCH"
trap 'rm -rf "$SCRATCH"' EXIT

want_n=$(awk -F= '/^n=/{print $2}' fixtures/kv.expected)
want_root=$(awk -F= '/^root=/{print $2}' fixtures/kv.expected)
fail=0
ran=0

say() { echo "bebop-wasm: $*"; }

# 1. native ---------------------------------------------------------------
if "$CARGO" test --offline --quiet > "$SCRATCH/native.txt" 2>&1; then
  say "native: cargo test rc=0 (n=$want_n root=$want_root asserted in tests/parity.rs)"
  ran=$((ran + 1))
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
if [ "${1:-}" = "--prove" ]; then
  # One bit, in a copy: cell 2 of the first key's index entry (its length).
  python3 - "$FIX" "$SCRATCH/broken.store" <<'EOF'
import sys
b = bytearray(open(sys.argv[1], 'rb').read())
# The live PartTab names the root; the root's first ref names KIDX; flip a
# bit in the LAST payload byte of the image, which is the newest value blob.
b[-8] ^= 1
open(sys.argv[2], 'wb').write(b)
EOF
  out=$(node harness.mjs "$WASM" "$SCRATCH/broken.store" 2>&1)
  rc=$?
  say "prove: one flipped payload bit -> '$out' rc=$rc"
  case "$out" in
    *"root=$want_root"*) say "prove: FAILED -- the root did not move"; exit 1 ;;
  esac
  say "prove: the number moved; the gate measures the bytes"
  exit 0
fi
out=$(node harness.mjs "$WASM" "$FIX" 2>&1)
rc=$?
say "wasm32: node -> '$out' rc=$rc"
case "$out" in
  "kv status=0 n=$want_n root=$want_root") ran=$((ran + 1)) ;;
  *) say "wasm32: DISAGREES with kv.expected"; fail=1 ;;
esac

# 3. python ---------------------------------------------------------------
out=$(python3 oracle.py "$FIX" 2>&1)
rc=$?
say "python: '$out' rc=$rc"
case "$out" in
  "kv status=0 n=$want_n root=$want_root") ran=$((ran + 1)) ;;
  *) say "python: DISAGREES with kv.expected"; fail=1 ;;
esac

# 3b. THE CUT IMAGE: one cell short of the arena. Every reader must REFUSE
# it; the Rust reader used to answer the previous generation (n=0), which is
# the disagreement this crate's second reader found.
head -c $(( $(wc -c < "$FIX") - 8 )) "$FIX" > "$SCRATCH/cut.store"
wout=$(node harness.mjs "$WASM" "$SCRATCH/cut.store" 2>&1)
pout=$(python3 oracle.py "$SCRATCH/cut.store" 2>&1)
say "cut: wasm32 -> '$wout'; python -> '$pout'"
case "$wout" in "kv status=0"*) say "cut: wasm32 READ a truncated image"; fail=1 ;; esac
case "$pout" in "kv status=0"*) say "cut: python READ a truncated image"; fail=1 ;; esac

# 4. bebop.bin ------------------------------------------------------------
SEED=../../bebop-lang/seed/build/seed
BIN=../../bebop-lang/bebop.bin
if [ "$(uname -m)" = "aarch64" ] && [ -x "$SEED" ] && [ -f "$BIN" ]; then
  if (cd ../../bebop-lang && ./seed/build/seed ./bebop.bin compile selfhost/std/kv.bp "$SCRATCH/kv.bin") > "$SCRATCH/kvc.txt" 2>&1; then
    # kv.bp opens `kv.store` in the working directory and EXTENDS it to 64 MiB
    # (st_open ftruncates), so it reads a copy, never the fixture.
    cp "$FIX" "$SCRATCH/kv.store"
    n=$(cd "$SCRATCH" && "$OLDPWD/$SEED" ./kv.bin n 2>&1)
    r=$(cd "$SCRATCH" && "$OLDPWD/$SEED" ./kv.bin h 2>&1)
    say "bebop.bin: kv.bin n -> '$n', kv.bin h -> '$r'"
    if [ "$n" = "$want_n" ] && [ "$r" = "$want_root" ]; then
      ran=$((ran + 1))
    else
      say "bebop.bin: DISAGREES with kv.expected"; fail=1
    fi
  else
    say "bebop.bin: kv.bp did not compile rc=$? -- $(tail -1 "$SCRATCH/kvc.txt")"; fail=1
  fi
else
  say "bebop.bin: NOT MEASURED (uname -m = $(uname -m); seed present: $([ -x "$SEED" ] && echo yes || echo no)) -- a skip is not an agreement"
fi

say "readers agreeing with kv.expected: $ran of 4 (bebop.bin counts only on aarch64)"
if [ "$fail" -ne 0 ]; then
  say "RED"
  exit 1
fi
if [ "$ran" -lt 3 ]; then
  say "RED -- fewer than three readers ran; that is not a parity check"
  exit 1
fi
say "GREEN ($ran readers, $bytes bytes)"
exit 0
