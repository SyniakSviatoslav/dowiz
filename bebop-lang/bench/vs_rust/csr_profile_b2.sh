#!/usr/bin/env bash
# csr_profile_b2.sh — B2 decisive twin (iii): CSR-build profile driver
# (docs/blueprints/B2-decisive-twins.md §3(iii)/§7 row iii, and the B8 profile row the ROADMAP
# B2 gate names). Compiles and runs BOTH arms of the profile at the sgraph2 scale
# (n=1,000,000 nodes, e=5,000,000 undirected edges = m=10,000,000 CSR slots):
#
#   store arm  bench/vs_rust/std_tests/csr_build_profile.bp — a LOCAL, instrumented copy of
#              selfhost/std/sgraph2.bp's phase_build/csr_build (see that file's header for why
#              it is a copy, not an edit), with rp/ci as STORE OBJECTS in a 512 MB file-backed
#              store, printing a clock_ms delta per sub-phase.
#   plain arm  bench/vs_rust/std_tests/csr_build_plain.bp — the IDENTICAL counting sort at the
#              identical scale with rp/ci as plain `zeros()` arrays: no store, no file.
#
# Both arms print the same FOLD (sum of the filled ci range + the final row pointer), so the
# difference between their phase tables is the store and nothing else. Running only the store
# arm — which is what this script did before 2026-09-08 — names the expensive phase but not its
# cause, and the two candidate causes (a random scatter over an 80 MB working set, which any
# engine pays, versus the store's file-backed page path, which is a persistence-layer property)
# lead to completely different conclusions for B4's "L0 rebuild <= 2 ms for 2^18 edges" budget.
#
#   BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT bash bench/vs_rust/csr_profile_b2.sh
# env: BEBOP_BIN (default ./bebop.bin), BEBOP_TMP (default /tmp/opencode), N (nodes, default
# 1000000), E (undirected edges, default 5000000).
#
# Run it under tools/slot.sh like any other heavy job: the store arm writes ~88 MB into a
# 512 MB mapping and takes tens of seconds.
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

for f in csr_build_profile csr_build_plain; do
  echo "compiling $f.bp ($BB) ..."
  ./seed/build/seed "$BB" compile "bench/vs_rust/std_tests/$f.bp" "$T/$f.bin" >/dev/null 2>&1 \
    || { echo "COMPILEFAIL $f"; exit 1; }
done

# the store arm opens ./b2_csrprofile.store relative to its CWD — run both arms from an
# absolute $T so nothing lands in the repo root, and start the store arm from a clean file each
# run (a fresh build, not a reopen: a warm store file would hide exactly the cost being hunted).
REPO=$(pwd)
( cd "$T" && rm -f b2_csrprofile.store
  taskset -c "$PIN" "$REPO/seed/build/seed" "$T/csr_build_profile.bin" "$N" "$E" > store_out.txt
  taskset -c "$PIN" "$REPO/seed/build/seed" "$T/csr_build_plain.bin"   "$N" "$E" > plain_out.txt
  cat store_out.txt plain_out.txt )

TOTAL_SLOTS=$((2 * E))
python3 - "$T/store_out.txt" "$T/plain_out.txt" "$TOTAL_SLOTS" <<'PY'
import sys
store_path, plain_path, slots = sys.argv[1], sys.argv[2], int(sys.argv[3])


def parse(path):
    rows, fold = [], None
    for line in open(path):
        line = line.strip()
        # skip blanks and the trailing bare integer the runner prints for main()'s return value
        # (seed prints a compiled program's i64 return on its own line after any of its output)
        if not line or not any(c.isalpha() for c in line):
            continue
        label, val = line.rsplit(' ', 1)
        label = label.strip()
        if label == 'fold':
            fold = int(val)
        else:
            rows.append((label, int(val)))
    return rows, fold


srows, sfold = parse(store_path)
prows, pfold = parse(plain_path)
pmap = dict(prows)
print()
print(f'fold: store {sfold}  plain {pfold}  -> {"EQUAL (arms comparable)" if sfold == pfold else "MISMATCH — STOP, the arms are not doing the same work"}')
print()
print('| phase | store ms | store ns/slot | plain ms | plain ns/slot | store/plain |')
print('|---|---|---|---|---|---|')
top = None
for label, ms in srows:
    if label == 'total build ms':
        continue
    pms = pmap.get(label)
    r = f'{ms / pms:.1f}x' if pms else '—'
    pstr = f'{pms}' if pms is not None else '— (store only)'
    pns = f'{pms * 1e6 / slots:.1f}' if pms is not None else '—'
    print(f'| {label} | {ms} | {ms * 1e6 / slots:.1f} | {pstr} | {pns} | {r} |')
    if top is None or ms > top[1]:
        top = (label, ms)
stotal = next((ms for l, ms in srows if l == 'total build ms'), 0)
ptotal = next((ms for l, ms in prows if l == 'total build ms'), 0)
print(f'| **total build ms** | **{stotal}** | {stotal * 1e6 / slots:.1f} | **{ptotal}** | '
      f'{ptotal * 1e6 / slots:.1f} | **{stotal / ptotal:.1f}x** |' if ptotal else '')
print()
print(f'top phase (store arm): {top[0] if top else "?"} = {top[1] if top else 0} ms, '
      f'{(top[1] if top else 0) * 100.0 / stotal:.1f}% of the build')
print(f'slots = 2*E = {slots}')
PY
