#!/usr/bin/env bash
# csr_profile_b2.sh — B2 decisive twin (iii): CSR-build profile driver
# (docs/blueprints/B2-decisive-twins.md §3(iii)/§7 row iii). Compiles and runs
# bench/vs_rust/std_tests/csr_build_profile.bp (a LOCAL, instrumented copy of
# selfhost/std/sgraph2.bp's phase_build/csr_build — see that file's header for why it is a
# copy, not an edit) at the sgraph2 scale (n=1,000,000 nodes, e=5,000,000 undirected edges),
# and prints the phase table plus the top phase (ns per edge slot, 2*e slots).
#
# NOT RUN as part of B2 prep (deliverable 5: "as a script, not run now" — a 5M-edge build is
# the 45-90s row from HISTORY STORE PULL; compile-checked and smoke-tested at n=1000/e=2000
# only, see bench/vs_rust/B2-PREP.md). This script is what makes the real run later a single
# command:
#   BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT bash bench/vs_rust/csr_profile_b2.sh
# env: BEBOP_BIN (default ./bebop.bin), BEBOP_TMP (default /tmp/opencode), N (nodes, default
# 1000000), E (undirected edges, default 5000000).
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BB=${BEBOP_BIN:-./bebop.bin}
T=${BEBOP_TMP:-/tmp/opencode}/b2
N=${N:-1000000}
E=${E:-5000000}
mkdir -p "$T"
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")

echo "compiling csr_build_profile.bp ($BB) ..."
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/csr_build_profile.bp "$T/csr_build_profile.bin" >/dev/null 2>&1 \
  || { echo "COMPILEFAIL csr_build_profile"; exit 1; }

# the profiled program opens ./b2_csrprofile.store relative to its CWD — run it from an
# absolute $T so it never lands in the repo root, and start from a clean file each run (fresh
# build, not reopen). $T/csr_build_profile.bin is itself already absolute (BEBOP_TMP is).
REPO=$(pwd)
mkdir -p "$T"
( cd "$T" && rm -f b2_csrprofile.store
  taskset -c "$PIN" "$REPO/seed/build/seed" "$T/csr_build_profile.bin" "$N" "$E" | tee csr_profile_out.txt )

TOTAL_SLOTS=$((2 * E))
python3 - "$T/csr_profile_out.txt" "$TOTAL_SLOTS" <<'PY'
import sys
path, slots = sys.argv[1], int(sys.argv[2])
rows = []
for line in open(path):
    line = line.strip()
    # skip blanks and the trailing bare integer the runner prints for main()'s return value
    # (seed prints a compiled program's i64 return on its own line after any of its own output)
    if not line or not any(c.isalpha() for c in line):
        continue
    label, ms = line.rsplit(' ', 1)
    rows.append((label.strip(), int(ms)))
print()
print('| phase | ms | ns/edge-slot |')
print('|---|---|---|')
top = None
for label, ms in rows:
    if label == 'total build ms':
        continue
    nsps = ms * 1e6 / slots
    print(f'| {label} | {ms} | {nsps:.2f} |')
    if top is None or ms > top[1]:
        top = (label, ms)
total = next((ms for l, ms in rows if l == 'total build ms'), None)
print(f'\ntop phase: {top[0] if top else "?"}  |  total build ms: {total}  |  {(total or 0) * 1e6 / slots:.2f} ns/edge-slot overall')
PY
