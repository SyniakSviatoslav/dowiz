#!/usr/bin/env bash
# T40/T51 structural invariants (no fold involved):
#  (i)  register-zone law            tools/check_abi.py <bins>
#  (ii) branch census, no increase   tools/census.py --check bench/vs_rust/census.txt
#  (iii) fntab zone map + lit trap   tools/check_abi.py --fntab bebop.bp
#  (iv) .bin footer/entry identity   inside check_abi.py for every bin touched
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
  python3 tools/census.py bebop.bin $BINS > bench/vs_rust/census.txt && echo "census.txt frozen"
fi
python3 tools/census.py --check bench/vs_rust/census.txt bebop.bin $BINS || fail=1

[ $fail = 0 ] && echo "invariants: GREEN" || echo "invariants: RED"
exit $fail
