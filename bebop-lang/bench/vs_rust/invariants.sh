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
echo "== (v) gate-source expansion identity (prelude + selfhost/std == std_tests)"
rm -rf "$OUT/std_expand"; sh tools/gen_selfsrc.sh std "$OUT/std_expand" >/dev/null || fail=1
for t in bench/vs_rust/std_tests/*.bp; do
  cmp -s "$t" "$OUT/std_expand/$(basename "$t")" || { echo "EXPANSION-DRIFT $(basename "$t" .bp): std_tests copy != prelude+selfhost/std (rerun tools/gen_selfsrc.sh std)"; fail=1; }
done
echo "expansion: $(ls bench/vs_rust/std_tests/*.bp | wc -l) gate sources checked"

[ $fail = 0 ] && echo "invariants: GREEN" || echo "invariants: RED"
exit $fail
