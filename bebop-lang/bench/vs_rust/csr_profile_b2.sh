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
#   ram arm    bench/vs_rust/std_tests/csr_build_ram.bp — D1
#              (docs/blueprints/D1-build-in-ram-publish-sequentially.md): the plain arm's three
#              passes, then the result copied into the SAME st_alloc'd store objects in one
#              ascending pass, sealed and committed exactly as the store arm. It is the arm the
#              D1 gate is stated against (ram total <= 1.3x plain total), and its extra
#              `publish copy ms` row is the blueprint's own open question §3(1) -- whether a
#              SEQUENTIAL write into the MAP_SHARED mapping pays the tax the scatter pays.
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

for f in csr_build_profile csr_build_plain csr_build_ram; do
  echo "compiling $f.bp ($BB) ..."
  ./seed/build/seed "$BB" compile "bench/vs_rust/std_tests/$f.bp" "$T/$f.bin" >/dev/null 2>&1 \
    || { echo "COMPILEFAIL $f"; exit 1; }
done

# the store arm opens ./b2_csrprofile.store relative to its CWD — run both arms from an
# absolute $T so nothing lands in the repo root, and start the store arm from a clean file each
# run (a fresh build, not a reopen: a warm store file would hide exactly the cost being hunted).
REPO=$(pwd)
( cd "$T" && rm -f b2_csrprofile.store b2_csrram.store
  taskset -c "$PIN" "$REPO/seed/build/seed" "$T/csr_build_profile.bin" "$N" "$E" > store_out.txt
  taskset -c "$PIN" "$REPO/seed/build/seed" "$T/csr_build_plain.bin"   "$N" "$E" > plain_out.txt
  taskset -c "$PIN" "$REPO/seed/build/seed" "$T/csr_build_ram.bin"     "$N" "$E" > ram_out.txt
  cat store_out.txt plain_out.txt ram_out.txt )

TOTAL_SLOTS=$((2 * E))
python3 - "$T/store_out.txt" "$T/plain_out.txt" "$T/ram_out.txt" "$TOTAL_SLOTS" <<'PY'
import sys
store_path, plain_path, ram_path, slots = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])


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
rrows, rfold = parse(ram_path)
pmap, rmap = dict(prows), dict(rrows)
folds_ok = sfold == pfold == rfold
print()
print(f'fold: store {sfold}  plain {pfold}  ram {rfold}  -> '
      f'{"EQUAL (arms comparable)" if folds_ok else "MISMATCH — STOP, the arms are not doing the same work"}')
print()
# Every phase label either arm printed, in the store arm's order, then the ram-only rows
# (`publish copy ms`) that the store arm has no counterpart for.
labels = [l for l, _ in srows] + [l for l, _ in rrows if l not in dict(srows)]
print('| phase | store ms | plain ms | ram ms | ram ns/slot | store/plain | ram/plain |')
print('|---|---|---|---|---|---|---|')
top = None
smap = dict(srows)
for label in labels:
    if label == 'total build ms':
        continue
    sms, pms, rms = smap.get(label), pmap.get(label), rmap.get(label)
    def cell(v):
        return '—' if v is None else str(v)
    sr = f'{sms / pms:.1f}x' if sms is not None and pms else '—'
    rr = f'{rms / pms:.1f}x' if rms is not None and pms else '—'
    rns = f'{rms * 1e6 / slots:.1f}' if rms is not None else '—'
    print(f'| {label} | {cell(sms)} | {cell(pms)} | {cell(rms)} | {rns} | {sr} | {rr} |')
    if sms is not None and (top is None or sms > top[1]):
        top = (label, sms)
stotal = next((ms for l, ms in srows if l == 'total build ms'), 0)
ptotal = next((ms for l, ms in prows if l == 'total build ms'), 0)
rtotal = next((ms for l, ms in rrows if l == 'total build ms'), 0)
if ptotal:
    print(f'| **total build ms** | **{stotal}** | **{ptotal}** | **{rtotal}** | '
          f'{rtotal * 1e6 / slots:.1f} | **{stotal / ptotal:.1f}x** | **{rtotal / ptotal:.1f}x** |')
print()
print(f'top phase (store arm): {top[0] if top else "?"} = {top[1] if top else 0} ms, '
      f'{(top[1] if top else 0) * 100.0 / stotal:.1f}% of the build')
if ptotal and rtotal:
    print(f'D1 gate (ram total <= 1.3x plain total): {rtotal}/{ptotal} = {rtotal / ptotal:.2f}x '
          f'-> {"PASS" if rtotal <= 1.3 * ptotal else "FAIL"};  speedup over the store arm '
          f'{stotal / rtotal:.1f}x')
print(f'slots = 2*E = {slots}')
PY
