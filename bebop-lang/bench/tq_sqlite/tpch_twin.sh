#!/usr/bin/env bash
# tpch_twin.sh -- B7 store-side prep twin harness (docs/blueprints/B7-dsl-planner.md
# section 6/7, docs/RESEARCH-GRAPHBLAS-2026-09-06.md section 1.2). Loads the 600,000-row
# deterministic lineitem CSV (bench/tq_sqlite/gen_lineitem.py, bench/oracles/tpch.py)
# into a store via bench/tq_sqlite/tpch_load.bp, runs the hand-written kernels
# bench/tq_sqlite/tpch_q6.bp / tpch_q1.bp on $BEBOP_BIN and the ctypes sqlite twin
# (bench/tq_sqlite/tpch_sqlite.py, VM_STEP floor per LANG-DB-DESIGN.md section 8),
# both pinned to core 4, R=11 medians (statistics.median, this repo's convention --
# see bench/tq_sqlite/run.sh), and writes bench/tq_sqlite/REPORT-tpch.md with the
# section-7 gate verdicts (Q6 >= 10x, Q1 >= 5x sqlite native).
#
# NOT a clean benchmark yet: bebop's numbers here are whole-process (seed startup +
# mmap + scan) since these hand-written kernels have no in-process timer wired in
# (they are functional-parity templates for B7 step 2's generator, per the prep
# task) -- a fair per-call number needs the tier-0/pool timing infra B7 will add.
# The first-query (tier 0) / repeat (pool) latency rows are placeholders: neither
# exists yet (no DSL/planner/gen_gb -- B7 steps 1-3, docs/blueprints/
# B7-dsl-planner.md section 5), so those rows print "n/a (no planner/pool yet)".
#
# env: BEBOP_BIN, BEBOP_TMP, R (default 11). Box: pinned core 4 (nice -n10 taskset -c4).
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-11}; PIN=4
T=${BEBOP_TMP:-/tmp/opencode}/tq_sqlite; mkdir -p "$T"
[ -s "$BEBOP_BIN" ] || { echo "GUARD: BEBOP_BIN=$BEBOP_BIN missing or empty (L12)"; exit 1; }
run() { nice -n10 taskset -c "$PIN" "$@"; }

run python3 bench/tq_sqlite/gen_lineitem.py "$T/lineitem.csv" || { echo "GENFAIL"; exit 1; }
for b in tpch_load tpch_q6 tpch_q1; do
  run ./seed/build/seed "$BEBOP_BIN" compile bench/tq_sqlite/$b.bp "$T/$b.bin" >/dev/null 2>&1 \
    || { echo "COMPILEFAIL $b"; exit 1; }
done
rm -f "$T/tpch.store"
LOADV=$(run ./seed/build/seed "$T/tpch_load.bin" "$T/lineitem.csv" "$T/tpch.store") \
  || { echo "LOADFAIL"; exit 1; }
LOAD_ROWS=$((LOADV / 1000000000)); LOAD_MS=$(((LOADV / 1000) % 1000000))
[ "$LOAD_ROWS" = 600000 ] || { echo "LOAD ROW COUNT MISMATCH: $LOAD_ROWS"; exit 1; }
rm -f "$T/tpch.sqlite" "$T/tpch.sqlite-wal" "$T/tpch.sqlite-shm"
run python3 bench/tq_sqlite/tpch_sqlite.py load "$T/lineitem.csv" "$T/tpch.sqlite" >/dev/null \
  || { echo "SQLITE LOADFAIL"; exit 1; }

T="$T" R="$R" PIN="$PIN" BB="$BEBOP_BIN" LOAD_MS="$LOAD_MS" python3 - <<'PY'
import os, subprocess, statistics, hashlib
T=os.environ['T']; R=int(os.environ['R']); PIN=os.environ['PIN']; BB=os.environ['BB']; LOAD_MS=os.environ['LOAD_MS']
EXP_Q6, EXP_Q1 = 114672059591, 6105941479581644684

def bebop_run(b):
    t0=__import__('time').perf_counter()
    p=subprocess.run(['nice','-n10','taskset','-c',PIN,'./seed/build/seed',f'{T}/{b}.bin',f'{T}/tpch.store'],
                      capture_output=True, text=True)
    us=(__import__('time').perf_counter()-t0)*1e6
    return int(p.stdout.strip().split('\n')[-1]), us

def sqlite_run(phase):
    p=subprocess.run(['nice','-n10','taskset','-c',PIN,'python3','bench/tq_sqlite/tpch_sqlite.py',phase,f'{T}/tpch.sqlite'],
                      capture_output=True, text=True)
    line=p.stdout.strip().split('\n')[-1].split()
    # "<phase> <us> <fold> vm_steps <n>"
    return float(line[1]), int(line[2]), int(line[4])

bq6=[]; bq1=[]; b6f=set(); b1f=set()
for _ in range(R):
    f,us = bebop_run('tpch_q6'); bq6.append(us); b6f.add(f)
    f,us = bebop_run('tpch_q1'); bq1.append(us); b1f.add(f)
sq6=[]; sq1=[]; s6f=set(); s1f=set(); vm6=vm1=0
for _ in range(R):
    us,f,vm = sqlite_run('q6'); sq6.append(us); s6f.add(f); vm6=vm
    us,f,vm = sqlite_run('q1'); sq1.append(us); s1f.add(f); vm1=vm

m = statistics.median
bq6m, bq1m, sq6m, sq1m = m(bq6), m(bq1), m(sq6), m(sq1)
ok6 = b6f == {EXP_Q6} == s6f
ok1 = b1f == {EXP_Q1} == s1f
x6 = sq6m/bq6m if bq6m>0 else float('nan')
x1 = sq1m/bq1m if bq1m>0 else float('nan')
gate6 = x6 >= 10.0
gate1 = x1 >= 5.0
md5=hashlib.md5(open(BB,'rb').read()).hexdigest()[:8]
L=[f'# REPORT-tpch -- B7 store-side prep twin, 600,000-row lineitem, pinned core {PIN} (A55), R={R} medians, bebop.bin {md5}',
   '',
   f'- load: bench/tq_sqlite/tpch_load.bp, {LOAD_MS} ms in-process (informational, not gated), 600000 rows, 7 columns as store `arr i64` objects + root, one commit.',
   f'- fold check Q6: bebop {"=={"+str(EXP_Q6)+"}==oracle" if ok6 else "MISMATCH "+str(b6f)+" vs "+str(s6f)} (sqlite twin folds: {sorted(s6f)})',
   f'- fold check Q1: bebop {"=={"+str(EXP_Q1)+"}==oracle" if ok1 else "MISMATCH "+str(b1f)+" vs "+str(s1f)} (sqlite twin folds: {sorted(s1f)})',
   '',
   '| query | bebop (whole-process, us) | sqlite native (ctypes prepared, us, VM_STEP floor) | ratio | gate | first-query (tier 0) | repeat (pool) |',
   '|---|---|---|---|---|---|---|',
   f'| Q6 | {bq6m:.1f} | {sq6m:.1f} (vm_steps {vm6}) | {x6:.1f}x | {"PASS >=10x" if gate6 else "FAIL <10x"} | n/a (no planner/pool yet) | n/a (no planner/pool yet) |',
   f'| Q1 | {bq1m:.1f} | {sq1m:.1f} (vm_steps {vm1}) | {x1:.1f}x | {"PASS >=5x" if gate1 else "FAIL <5x"} | n/a (no planner/pool yet) | n/a (no planner/pool yet) |',
   '',
   'Note: bebop numbers above are whole-`seed`-process wall time (fork+exec+mmap+scan),',
   'NOT an in-process kernel timer -- these hand-written kernels are functional-parity',
   'templates for the B7 generator (docs/blueprints/B7-dsl-planner.md section 5 step 2),',
   'not yet wired to the register-model / pool timing infra the real gate needs. The',
   'ratio column is therefore a lower bound on bebop\'s advantage (process overhead is',
   'shared by both sides only for sqlite\'s own python/ctypes startup, not for `seed`\'s',
   'own fixed costs), reported honestly rather than gated as a final number.',
   f'- VERDICT: GREEN if fold checks OK and gate6/gate1 both PASS, else RED. This run: {"GREEN" if (ok6 and ok1 and gate6 and gate1) else "RED"}',
]
open('bench/tq_sqlite/REPORT-tpch.md','w').write('\n'.join(L)+'\n')
print('\n'.join(L))
PY
