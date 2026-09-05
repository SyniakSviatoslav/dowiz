#!/usr/bin/env bash
# pcprof.sh (2026-09-06): poor man's profiler for a bebop.bin run — samples $pc with gdb
# while the command runs and maps every sample to the fn of <compiler.bp> (fn order in the
# source == prologue order in the .bin; T118b's entry stub is "stub"). No symbols needed.
# Usage: tools/pcprof.sh <bin> <src.bp> <n-samples> <interval-s> -- <seed args...>
#   e.g. tools/pcprof.sh bebop.bin bebop.bp 40 0.5 -- bebop.bin compile x.bp /tmp/x.bin
cd "$(dirname "$0")/.." || exit 1
BIN=$1; SRC=$2; N=$3; DT=$4; shift 5
./seed/build/seed "$@" >/dev/null 2>&1 & pid=$!
sleep 0.3
pcs=()
for _ in $(seq "$N"); do
  kill -0 $pid 2>/dev/null || break
  pc=$(gdb -q -p $pid -batch -ex 'p/x $pc' 2>/dev/null | awk '/^\$1/{print $3}')
  [ -n "$pc" ] && pcs+=("$pc")
  sleep "$DT"
done
wait $pid
printf '%s\n' "${pcs[@]}" | BIN="$BIN" SRC="$SRC" python3 -c '
import sys, os, re, struct, collections
sys.path.insert(0, "tools"); import check_abi
W, entry, code_end = check_abi.load_bin(os.environ["BIN"])
starts = check_abi.fn_starts(W, code_end)
names = re.findall(r"^fn (\w+)", open(os.environ["SRC"]).read(), re.M)
assert len(names) == len(starts), (len(names), len(starts))
pcs = [int(x, 16) for x in sys.stdin.read().split()]
# the .bin is mmapped at one base: every sample lies in [base, base + 4*len(W)); take the
# smallest sample as an anchor and fit the base from the code size
if not pcs: sys.exit("no samples")
base = min(pcs) & ~0xFFF
while max(pcs) - base >= 4 * len(W): base -= 0x1000
c = collections.Counter()
for pc in pcs:
    w = (pc - base) // 4
    k = max((i for i, s in enumerate(starts) if s <= w), default=None)
    c[names[k] if k is not None else "?"] += 1
for n, v in c.most_common(12): print("%5.1f%%  %s" % (100.0 * v / len(pcs), n))
print("samples", len(pcs))
'
