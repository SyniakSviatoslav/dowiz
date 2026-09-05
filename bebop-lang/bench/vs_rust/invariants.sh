#!/usr/bin/env bash
# T40/T51 structural invariants (no fold involved):
#  (i)  register-zone law            tools/check_abi.py <bins>
#  (ii) branch census, no increase   tools/census.py --check bench/vs_rust/census.txt
#  (iii) fntab zone map + lit trap   tools/check_abi.py --fntab bebop.bp
#  (iv) .bin footer/entry identity   inside check_abi.py for every bin touched
#  (v)  gate-source expansion identity  gen_selfsrc.sh std == bench/vs_rust/std_tests (T38/L9)
# Usage: bench/vs_rust/invariants.sh [--freeze]   (--freeze rewrites census.txt)
set -u
mkdir -p "${BEBOP_TMP:-/tmp/opencode}"
cd "$(dirname "$0")/../.."
OUT=${BEBOP_TMP:-/tmp/opencode}/invariants; mkdir -p "$OUT"
fail=0
tools/guard_artifact.sh "${BEBOP_BIN:-bebop.bin}" || exit 1
[ -x seed/build/seed ] || { echo "GUARD: seed/build/seed missing"; exit 1; }

# fresh compiles of every construct and kernel (census tracks the compiler, not stale frozen kernels)
SRCS=$(ls bench/parity_constructs/*.bp bench/vs_rust/kernels/*.bp)
BINS=""
for f in $SRCS; do
  b=$(basename "$f" .bp)
  if ./seed/build/seed ${BEBOP_BIN:-bebop.bin} compile "$f" "$OUT/$b.bin" >/dev/null 2>&1; then
    BINS="$BINS $OUT/$b.bin"
  else
    echo "COMPILEFAIL $b"; fail=1
  fi
done

echo "== (i)+(iv) register zones, footer/entry identity"
python3 tools/check_abi.py bebop.bin bench/parity_constructs/frozen/*.bin $BINS || fail=1
echo "== (iii) fntab zone map"
python3 tools/check_abi.py --fntab bebop.bp $SRCS || fail=1
echo "== (ii) branch census"
if [ "${1:-}" = "--freeze" ]; then
  if python3 tools/census.py --freeze-check bench/vs_rust/census.txt bench/vs_rust/census_allow.txt bebop.bin $BINS; then
    python3 tools/census.py bebop.bin $BINS > bench/vs_rust/census.txt && echo "census.txt frozen (census_allow.txt lines stay as the record of the increase)"
  else
    echo "census.txt NOT frozen (D11-F: add the allow lines to bench/vs_rust/census_allow.txt in this commit)"; fail=1
  fi
fi
python3 tools/census.py --check bench/vs_rust/census.txt bebop.bin $BINS || fail=1

echo "== (vii) declared types vs use (T48 census, tools/typecheck.py over bpref's AST; morph.bp excluded: \`&&\` is not in bpref, T125)"
TC=$(python3 tools/typecheck.py bebop.bp $(ls bench/vs_rust/std_tests/*.bp | grep -v '/morph.bp') bench/vs_rust/kernels/*.bp bench/parity_constructs/*.bp 2>&1 | tail -1)
echo "$TC"; [ "$TC" = "typecheck census: 0 findings" ] || { echo "TYPECHECK FAIL (see tools/typecheck.py output)"; fail=1; }
echo "== (v) gate-source expansion identity (prelude + selfhost/std == std_tests)"
rm -rf "$OUT/std_expand"; sh tools/gen_selfsrc.sh std "$OUT/std_expand" >/dev/null || fail=1
for t in bench/vs_rust/std_tests/*.bp; do
  cmp -s "$t" "$OUT/std_expand/$(basename "$t")" || { echo "EXPANSION-DRIFT $(basename "$t" .bp): std_tests copy != prelude+selfhost/std (rerun tools/gen_selfsrc.sh std)"; fail=1; }
done
echo "expansion: $(ls bench/vs_rust/std_tests/*.bp | wc -l) gate sources checked"

[ $fail = 0 ] && echo "invariants: GREEN" || echo "invariants: RED"
exit $fail
