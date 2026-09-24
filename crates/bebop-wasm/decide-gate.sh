#!/bin/sh
# THE D7 GATE IN wasm32: the deciders, built with `--features decide` for
# wasm32-unknown-unknown and run by node, write the COMMITTED bytes for the
# committed images and inputs (`fixtures/decide/`). The same files are what
# `tests/decide_agrees.rs` (this crate, native) and the Worker's
# `command/amend/agrees.rs` are held to, so green here + green there is one
# set of bytes from three builds.
#
# THIS SCRIPT DOES NOT BUILD. On this box cargo runs only through the slot:
#   RUSTUP_TOOLCHAIN=1.96.1-aarch64-unknown-linux-gnu \
#     bash ../../bebop-lang/tools/slot.sh <label> $HOME/.cargo/bin/cargo build \
#     --offline --features decide --target wasm32-unknown-unknown --release
# and then `sh decide-gate.sh [module.wasm]`. `--prove` feeds the module a
# stale edit where the delta is expected and demands RED: a gate that cannot
# fail measures nothing.
set -u
cd "$(dirname "$0")"
WASM="${1:-target/wasm32-unknown-unknown/release/bebop_wasm.wasm}"
[ "${1:-}" = "--prove" ] && WASM=target/wasm32-unknown-unknown/release/bebop_wasm.wasm
F=fixtures/decide
SCRATCH="${BEBOP_WASM_SCRATCH:-${TMPDIR:-/tmp}/bebop-wasm-decide.$$}"
mkdir -p "$SCRATCH"
trap 'rm -rf "$SCRATCH"' EXIT
fail=0
say() { echo "decide-gate: $*"; }

# check <name> <want-file> <want-rc> <what> <a> <b> <c>
check() {
  name=$1; want=$2; want_rc=$3; shift 3
  node decide.mjs "$WASM" "$@" > "$SCRATCH/$name.out" 2> "$SCRATCH/$name.err"
  rc=$?
  if [ "$rc" -eq "$want_rc" ] && cmp -s "$SCRATCH/$name.out" "$want"; then
    say "$name: rc=$rc, $(wc -c < "$want" | tr -d ' ') bytes identical to $want ($(cat "$SCRATCH/$name.err"))"
  else
    say "$name: DISAGREES -- rc=$rc (want $want_rc), $(cat "$SCRATCH/$name.err"); got: $(head -c 160 "$SCRATCH/$name.out")"
    fail=1
  fi
}

[ -f "$WASM" ] || { say "no module at $WASM -- build it first (header)"; exit 1; }
say "module: $WASM, $(wc -c < "$WASM" | tr -d ' ') bytes"
if [ "${1:-}" = "--prove" ]; then
  check prove "$F/amend.delta" 0 amend "$F/amend.log" "$F/amend.stock" "$F/amend_stale.in.json"
  [ "$fail" -eq 1 ] && { say "prove: RED as it must be -- the gate reads the bytes"; exit 0; }
  say "prove: FAILED -- a stale edit matched the delta"; exit 1
fi
check amend "$F/amend.delta" 0 amend "$F/amend.log" "$F/amend.stock" "$F/amend.in.json"
check pay "$F/pay.delta" 0 pay "$F/pay.log" "$F/pay.room.json" "$F/pay.in.json"
# The refusal file is "<status> <words>"; the module answers the words, and the status on stderr.
tail -c +4 "$F/amend_stale.refusal" > "$SCRATCH/stale.words"
check stale "$SCRATCH/stale.words" 1 amend "$F/amend.log" "$F/amend.stock" "$F/amend_stale.in.json"
grep -q "status=$(cut -d' ' -f1 "$F/amend_stale.refusal") " "$SCRATCH/stale.err" || { say "stale: wrong status: $(cat "$SCRATCH/stale.err")"; fail=1; }
[ "$fail" -eq 0 ] && { say "GREEN"; exit 0; }
say "RED"; exit 1
