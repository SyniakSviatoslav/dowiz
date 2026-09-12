#!/usr/bin/env bash
# T40/T51 structural invariants (no fold involved):
#  (i)  register-zone law            tools/check_abi.py <bins>
#  (ii) branch census, no increase   tools/census.py --check bench/vs_rust/census.txt
#  (iii) fntab zone map + lit trap   tools/check_abi.py --fntab bebop.bp
#  (iv) .bin footer/entry identity   inside check_abi.py for every bin touched
#  (v)  gate-source expansion identity  gen_selfsrc.sh std == bench/vs_rust/std_tests (T38/L9)
# Usage: bench/vs_rust/invariants.sh [--freeze]   (--freeze rewrites census.txt)
# env (2026-09-06, battery lane): BEBOP_BIN = the candidate compiler (default ./bebop.bin;
# census rows are named by basename, so a candidate is copied to $OUT/bebop.bin first),
# BEBOP_SRC = its source (default bebop.bp), BEBOP_TMP. Compiles run J at a time.
# --- one-compile guard (operator 2026-09-12) -----------------------------------
# Not inside a slot? Re-exec through it. tools/slot.sh runs SLOTS=1 against ONE global flock
# in /root/.cache/bebop/slots, shared by every lane worktree, so at most one heavy job -- one
# compilation -- exists on the box at any instant. NO_SLOT=1 opts out (main-session triage only).
if [ "${BEBOP_SLOT_HELD:-0}" != 1 ] && [ "${NO_SLOT:-0}" != 1 ]; then
  _slot="$(dirname "$0")/../../tools/slot.sh"
  [ -f "$_slot" ] || _slot=/root/dowiz/bebop-lang/tools/slot.sh
  [ -f "$_slot" ] && exec bash "$_slot" "auto:$(basename "$0")" bash "$0" "$@"
fi
set -u
mkdir -p "${BEBOP_TMP:-/tmp/opencode}"
cd "$(dirname "$0")/../.."
OUT=${BEBOP_TMP:-/tmp/opencode}/invariants; mkdir -p "$OUT"
fail=0
tools/guard_artifact.sh "${BEBOP_BIN:-bebop.bin}" || exit 1
[ -x seed/build/seed ] || { echo "GUARD: seed/build/seed missing"; exit 1; }
BIN=bebop.bin; SRC=${BEBOP_SRC:-bebop.bp}
if [ -n "${BEBOP_BIN:-}" ] && [ "$(realpath "$BEBOP_BIN")" != "$(realpath bebop.bin)" ]; then cp "$BEBOP_BIN" "$OUT/bebop.bin"; BIN=$OUT/bebop.bin; fi

# fresh compiles of every construct and kernel (census tracks the compiler, not stale frozen kernels)
SRCS=$(ls bench/parity_constructs/*.bp bench/vs_rust/kernels/*.bp)
comp() { ./seed/build/seed "$BIN" compile "$1" "$OUT/$(basename "$1" .bp).bin" >/dev/null 2>&1 || echo "COMPILEFAIL $(basename "$1" .bp)"; }
export -f comp; export BIN OUT
CF=$(echo "$SRCS" | tr ' ' '\n' | xargs -P "${J:-4}" -n 1 bash -c 'comp "$@"' _)
[ -z "$CF" ] || { echo "$CF"; fail=1; }
BINS=$(for f in $SRCS; do b=$OUT/$(basename "$f" .bp).bin; [ -s "$b" ] && printf ' %s' "$b"; done)

echo "== (i)+(iv) register zones, footer/entry identity"
python3 tools/check_abi.py "$BIN" bench/parity_constructs/frozen/*.bin $BINS || fail=1
echo "== (iii) fntab zone map"
python3 tools/check_abi.py --fntab "$SRC" $SRCS || fail=1
echo "== (ii) branch census"
if [ "${1:-}" = "--freeze" ]; then
  if python3 tools/census.py --freeze-check bench/vs_rust/census.txt bench/vs_rust/census_allow.txt "$BIN" $BINS; then
    python3 tools/census.py "$BIN" $BINS > bench/vs_rust/census.txt && echo "census.txt frozen (census_allow.txt lines stay as the record of the increase)"
  else
    echo "census.txt NOT frozen (D11-F: add the allow lines to bench/vs_rust/census_allow.txt in this commit)"; fail=1
  fi
fi
python3 tools/census.py --check bench/vs_rust/census.txt "$BIN" $BINS || fail=1

echo "== (vii) declared types vs use (T48 census, tools/typecheck.py over bpref's AST; every std gate since T125 gave bpref \`&&\`/\`||\`)"
TC=$(python3 tools/typecheck.py "$SRC" bench/vs_rust/std_tests/*.bp bench/vs_rust/kernels/*.bp bench/parity_constructs/*.bp 2>&1 | tail -1)
echo "$TC"; [ "$TC" = "typecheck census: 0 findings" ] || { echo "TYPECHECK FAIL (see tools/typecheck.py output)"; fail=1; }
NEG=$(python3 tools/typecheck.py bench/typecheck_neg/*.bp 2>&1 | tail -1)
echo "negative sample: $NEG (T48b: must NOT be 0 findings)"; [ "$NEG" != "typecheck census: 0 findings" ] || { echo "TYPECHECK NEG FAIL: bench/typecheck_neg/*.bp type-checked clean"; fail=1; }
echo "== (v) gate-source expansion identity + declared authority (bench/vs_rust/std_tests/GENERATED.txt)"
# 2026-09-09 (lane C5): this rung used to regenerate into a temp dir and cmp, which
# detected a byte difference and nothing else. Four lanes lost a gate to a drift that
# was COMMITTED by a fifth, and the message named the generated copy rather than the
# source to edit. The comparison now lives in ONE place -- gen_selfsrc.sh std --check --
# so the checker and the fixer cannot disagree, and it additionally rejects an
# UNREGISTERED gate source, a `generated` entry whose twin has been deleted (which used
# to promote the copy to authoritative in silence), and a `local` entry that has grown
# a twin. A byte comparison alone can catch none of those three.
rm -rf "$OUT/std_expand_check"
BEBOP_TMP="$OUT" sh tools/gen_selfsrc.sh std --check || fail=1

echo "== (viii) seed loader rebuild (D12-D): as seed/seed.S | ld -static | .text == .text of the committed seed/build/seed"
mkdir -p "$OUT/seed"; if as seed/seed.S -o "$OUT/seed/seed.o" 2>/dev/null && ld -static -o "$OUT/seed/seed" "$OUT/seed/seed.o" 2>/dev/null \
   && objcopy -O binary -j .text "$OUT/seed/seed" "$OUT/seed/rebuilt.text" && objcopy -O binary -j .text seed/build/seed "$OUT/seed/committed.text" \
   && cmp -s "$OUT/seed/rebuilt.text" "$OUT/seed/committed.text"; then echo "seed: .text identical ($(stat -c %s "$OUT/seed/committed.text") B; the ELF wrapper may differ by linker version)"
else echo "SEED DRIFT: seed/seed.S no longer rebuilds seed/build/seed's .text (or as/ld missing)"; fail=1; fi

echo "== (ix) push_words == 0 (REGISTER-MODEL-BLUEPRINT §7: the stack machine is retired)"
PW=$(python3 tools/perf.py size --bin "$BIN" 2>/dev/null | python3 -c "import ast,sys; print(ast.literal_eval(sys.stdin.readline())['push_words'])")
echo "push_words: $PW"
[ "$PW" = 0 ] || { echo "PUSH_WORDS NONZERO: $PW stack-machine words remain in $BIN (want 0)"; fail=1; }

# --- mechanical architecture and process invariants (tools/arch_check.py) ----------
# Every check there exists because a specific defect cost this project real time: a
# promoted binary that was not the source's compiler, stale .store files that read as
# miscompiles, a memo replay that skipped writing a file other gates consume, files that
# grew past the point where a change to them can be reviewed.
# --- platform identity, FIRST: every assumption below is about this machine -----------
# ldaddal and crc32x are OPTIONAL ARMv8 extensions assembled under a plain .arch armv8-a;
# a 16 KiB-page kernel makes msync return EINVAL for these ranges; every latency number on
# this box is inflated 10-100x because TracerPid is non-zero; and the mount is nobarrier,
# so "durable" writes are not. A change in any of these invalidates results silently.
echo "== (0) platform identity"
if [ -f tools/platform.txt ]; then
  diff <(bash tools/platform.sh) <(grep -v '^#' tools/platform.txt | grep -v '^$') \
    || { echo "PLATFORM CHANGED: the frozen assumptions above no longer hold"; fail=1; }
fi

echo "== (x) arch_check: architecture and process invariants"
python3 tools/arch_check.py || fail=1

[ $fail = 0 ] && echo "invariants: GREEN" || echo "invariants: RED"
exit $fail
