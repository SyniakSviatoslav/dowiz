#!/usr/bin/env bash
# twins_b2.sh — B2 decisive twins driver (docs/blueprints/B2-decisive-twins.md §7). Runs the
# join twin (i, uniform + Zipf) and the specialise-then-run scan twin (ii) across bebop,
# python (oracle), sqlite (ctypes) and Rust, checks folds BEFORE trusting any timing (RED on
# mismatch, per the blueprint's §7/§9), and appends the gate verdicts to
# bench/vs_rust/REPORT-b2.md. honest.sh/sgraph2.sh conventions: PIN auto-detected to a big
# (A78) core, R medians of full-process runs — this kernel (1M rows / 1M scan rows) is well
# above clock resolution per run, unlike the sub-ms k*h kernels that needed an in-process REPS
# loop to get above it; REPS still exists inside the bebop/Rust binaries (default 1) for a
# possible future finer-grained row, just unused by this driver.
#
# THIS SCRIPT IS THE ONE-COMMAND RUN FOR LATER (register-model bebop.bin promoted, A1 GREEN):
#   BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT R=11 bash bench/vs_rust/twins_b2.sh
# env: BEBOP_BIN (default ./bebop.bin), BEBOP_TMP (default /tmp/opencode), R (default 11),
# N_JOIN (default 1000000), N_SCAN (default 1000000), REPORT (default bench/vs_rust/REPORT-b2.md).
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BB=${BEBOP_BIN:-./bebop.bin}
T=${BEBOP_TMP:-/tmp/opencode}/b2
R=${R:-11}
N_JOIN=${N_JOIN:-1000000}
N_SCAN=${N_SCAN:-1000000}
REPORT=${REPORT:-bench/vs_rust/REPORT-b2.md}
mkdir -p "$T/rust"
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")

echo "compiling join_twin.bp ($BB) ..."
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/join_twin.bp "$T/join_twin.bin" >/dev/null 2>&1 \
  || { echo "COMPILEFAIL join_twin"; exit 1; }
for r in join_hash join_merge scan_const scan_generic; do
  rustc -O -o "$T/rust/$r" bench/vs_rust/rust_once/$r.rs 2>/dev/null \
    || { echo "RUSTC FAIL $r"; exit 1; }
done

T="$T" R="$R" PIN="$PIN" BB="$BB" N_JOIN="$N_JOIN" N_SCAN="$N_SCAN" REPORT="$REPORT" python3 - <<'PY'
import os, subprocess, sys, time, hashlib

T = os.environ['T']; R = int(os.environ['R']); PIN = os.environ['PIN']; BB = os.environ['BB']
N_JOIN = int(os.environ['N_JOIN']); N_SCAN = int(os.environ['N_SCAN']); REPORT = os.environ['REPORT']
sys.path.insert(0, 'bench/oracles')
sys.path.insert(0, 'bench/tq_sqlite')
sys.path.insert(0, 'bench/vs_rust/std_tests')
import join_twin as jt          # bench/oracles/join_twin.py
import scan_twin as stw         # bench/oracles/scan_twin.py
import join_sqlite as js        # bench/tq_sqlite/join_sqlite.py
import gen_scan                 # bench/vs_rust/std_tests/gen_scan.py

L = js.L


def med(vals):
    v = sorted(vals)
    return v[len(v) // 2]


def run(cmd):
    p = subprocess.run(['taskset', '-c', PIN] + cmd, capture_output=True, text=True)
    return p.stdout, p.stderr


def bb_join(dist, mode):
    out, _ = run(['./seed/build/seed', f'{T}/join_twin.bin', str(N_JOIN), dist, mode])
    return out.strip().split('\n')[-1]


mismatches = []
join_rows = {}
for dist in ('u', 'z'):
    zipf = 1 if dist == 'z' else 0
    seed = 8823 if zipf else 4711
    rk, ra, sk, sb = jt.gen(seed, N_JOIN, zipf)
    ocnt, ochk, ofold = jt.join_fold(rk, ra, sk, sb)

    bcnt = int(bb_join(dist, 'c'))
    bchk = int(bb_join(dist, 's'))
    if (bcnt, bchk) != (ocnt, ochk):
        mismatches.append(f'join {dist} bebop {bcnt}/{bchk} != oracle {ocnt}/{ochk}')

    db = js.opendb()
    js.load_tables(db, rk, ra, sk, sb)
    cnt_i, chk_i, plan_i, _, _ = js.run_join(db, True)   # creates + drops idx_s_k internally
    cnt_n, chk_n, plan_n, _, _ = js.run_join(db, False)
    if (cnt_i, chk_i) != (ocnt, ochk) or (cnt_n, chk_n) != (ocnt, ochk):
        mismatches.append(f'join {dist} sqlite {cnt_i}/{chk_i},{cnt_n}/{chk_n} != oracle {ocnt}/{ochk}')

    # timing: index built once, R reps of step+reset on one prepared statement (no DDL inside
    # the loop), same for the no-index plan — LANG-DB §8's ctypes floor is per ctypes CALL, and
    # this loop makes exactly 2 calls/rep (step, reset), so js.ctypes_floor_us() below is the
    # per-call number to subtract once a real timing claim is made; not applied here.
    js.exe(db, 'CREATE INDEX idx_s_k ON S(k)')
    st_i = js.prep(db, js.JOIN_SQL)
    sql_i_ms = []
    for _ in range(R):
        t0 = time.perf_counter()
        L.sqlite3_step(st_i)
        sql_i_ms.append((time.perf_counter() - t0) * 1000)
        L.sqlite3_reset(st_i)
    L.sqlite3_finalize(st_i)
    js.exe(db, 'DROP INDEX idx_s_k')
    st_n = js.prep(db, js.JOIN_SQL)
    sql_n_ms = []
    for _ in range(R):
        t0 = time.perf_counter()
        L.sqlite3_step(st_n)
        sql_n_ms.append((time.perf_counter() - t0) * 1000)
        L.sqlite3_reset(st_n)
    L.sqlite3_finalize(st_n)
    L.sqlite3_close(db)
    floor_us = js.ctypes_floor_us()

    bb_ms = []
    for _ in range(R):
        out, _ = run(['./seed/build/seed', f'{T}/join_twin.bin', str(N_JOIN), dist, 't'])
        bb_ms.append(float(out.strip().split('\n')[-1]))
    rh_ms = []
    rm_ms = []
    for _ in range(R):
        out, err = run([f'{T}/rust/join_hash', str(N_JOIN), dist])
        rh_ms.append(float(err.strip().split('\n')[-1]))
        fold_r = int(out.strip().split('\n')[-1].split()[-1])
        if fold_r != ofold:
            mismatches.append(f'join {dist} rust_hash fold {fold_r} != oracle {ofold}')
    for _ in range(R):
        out, err = run([f'{T}/rust/join_merge', str(N_JOIN), dist])
        rm_ms.append(float(err.strip().split('\n')[-1]))
        fold_r = int(out.strip().split('\n')[-1].split()[-1])
        if fold_r != ofold:
            mismatches.append(f'join {dist} rust_merge fold {fold_r} != oracle {ofold}')

    join_rows[dist] = dict(bebop_ms=med(bb_ms), rust_hash_ms=med(rh_ms), rust_merge_ms=med(rm_ms),
                            sqlite_idx_ms=med(sql_i_ms), sqlite_noidx_ms=med(sql_n_ms),
                            plan_idx=plan_i, plan_noidx=plan_n, count=ocnt, floor_us=floor_us)

# --- scan (ii) ---
digest, src = gen_scan.render(N_SCAN)
bp_path = f'{T}/scan_{digest}.bp'
bin_path = f'{T}/scan_{digest}.bin'
with open(bp_path, 'w') as f:
    f.write(src)
try:
    os.remove(bin_path)
except FileNotFoundError:
    pass
t0 = time.perf_counter()
subprocess.run(['taskset', '-c', PIN, './seed/build/seed', BB, 'compile', bp_path, bin_path], capture_output=True)
compile_ms = (time.perf_counter() - t0) * 1000
t0 = time.perf_counter()
out, _ = run(['./seed/build/seed', bin_path])
first_run_ms = (time.perf_counter() - t0) * 1000
oracle_sum = stw.scan(N_SCAN)
bb_sum_a = int(out.strip().split('\n')[-1])
run_b_ms = []
bb_sum_b = bb_sum_a
for _ in range(R):
    t0 = time.perf_counter()
    out, _ = run(['./seed/build/seed', bin_path])
    run_b_ms.append((time.perf_counter() - t0) * 1000)
    bb_sum_b = int(out.strip().split('\n')[-1])
if bb_sum_a != oracle_sum or bb_sum_b != oracle_sum:
    mismatches.append(f'scan bebop {bb_sum_a}/{bb_sum_b} != oracle {oracle_sum}')

rc_ms = []
rg_ms = []
for _ in range(R):
    out, err = run([f'{T}/rust/scan_const', str(N_SCAN)])
    rc_ms.append(float(err.strip().split('\n')[-1]))
    s = int(out.strip().split()[-1])
    if s != oracle_sum:
        mismatches.append(f'scan rust_const {s} != oracle {oracle_sum}')
for _ in range(R):
    out, err = run([f'{T}/rust/scan_generic', str(N_SCAN)])
    rg_ms.append(float(err.strip().split('\n')[-1]))
    s = int(out.strip().split()[-1])
    if s != oracle_sum:
        mismatches.append(f'scan rust_generic {s} != oracle {oracle_sum}')

folds_ok = len(mismatches) == 0


def ratio(a, b):
    return a / b if b else float('nan')


lines = []
md5 = hashlib.md5(open(BB, 'rb').read()).hexdigest()[:8]
lines.append(f'\n## B2 decisive twins ({time.strftime("%Y-%m-%d", time.gmtime())}, bebop.bin {md5}, '
             f'core {PIN}, R={R}, N_JOIN={N_JOIN}, N_SCAN={N_SCAN}) — folds {"equal" if folds_ok else "MISMATCH"}')
if not folds_ok:
    lines.append('\nMISMATCHES (RED — a bug, not a number):')
    for m in mismatches:
        lines.append(f'- {m}')

lines.append('\n### (i) join as SpGEMM\n')
lines.append('| dist | bebop ms | rust_hash ms | rust_merge ms | sqlite_idx ms | sqlite_noidx ms | '
             'pairs | bebop/sqlite_best | bebop/rust_best | gate |')
lines.append('|---|---|---|---|---|---|---|---|---|---|')
for dist, row in join_rows.items():
    sqlite_best = min(row['sqlite_idx_ms'], row['sqlite_noidx_ms'])
    rust_best = min(row['rust_hash_ms'], row['rust_merge_ms'])
    r_sql = ratio(sqlite_best, row['bebop_ms'])
    r_rust = ratio(row['bebop_ms'], rust_best)     # bebop/rust; gate wants bebop >= 0.7x rust i.e. r_rust <= 1/0.7
    gate = 'MET' if (r_sql >= 10 and r_rust <= 1 / 0.7) else 'UNMET'
    lines.append(f"| {dist} | {row['bebop_ms']:.3f} | {row['rust_hash_ms']:.3f} | {row['rust_merge_ms']:.3f} | "
                 f"{row['sqlite_idx_ms']:.3f} | {row['sqlite_noidx_ms']:.3f} | {row['count']} | "
                 f"{r_sql:.1f}x | {(rust_best / row['bebop_ms']):.2f}x | {gate} |")
    lines.append(f"  - plan idx: {row['plan_idx']}; plan no-idx: {row['plan_noidx']}; "
                 f"ctypes floor: {row['floor_us']:.2f} us/call (not yet subtracted above)")

lines.append('\n### (ii) specialise-then-run scan\n')
lines.append('| row | ms |')
lines.append('|---|---|')
lines.append(f'| bebop compile (row A) | {compile_ms:.3f} |')
lines.append(f'| bebop first run after compile | {first_run_ms:.3f} |')
lines.append(f'| bebop run-only median, R={R} (row B, memo hit: same .bin, no recompile) | {med(run_b_ms):.3f} |')
lines.append(f'| rust_generic median | {med(rg_ms):.3f} |')
lines.append(f'| rust_const median | {med(rc_ms):.3f} |')
total_a = compile_ms + first_run_ms
gate_total = total_a <= med(rg_ms) / 5
gate_run = med(run_b_ms) <= 1.5 * med(rc_ms)
lines.append(f'\ngate: total(compile+run) <= rust_generic/5: {"MET" if gate_total else "UNMET"} '
             f'({total_a:.3f} vs {med(rg_ms) / 5:.3f}); run <= 1.5x rust_const: {"MET" if gate_run else "UNMET"} '
             f'({med(run_b_ms):.3f} vs {1.5 * med(rc_ms):.3f})')

lines.append('\n### (iii) CSR-build profile\n')
lines.append('not run by this script — see bench/vs_rust/csr_profile_b2.sh (differential '
             'clock_ms timing of a local instrumented copy of sgraph2.bp phase_build).')

with open(REPORT, 'a') as f:
    f.write('\n'.join(lines) + '\n')
print('\n'.join(lines))
if not folds_ok:
    print('B2 FAIL: fold mismatch (see above)')
    sys.exit(1)
PY
