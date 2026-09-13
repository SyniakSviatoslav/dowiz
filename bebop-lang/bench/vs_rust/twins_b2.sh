#!/usr/bin/env bash
# twins_b2.sh — B2 decisive twins driver (docs/blueprints/B2-decisive-twins.md §7). Runs the
# join twin (i, uniform + Zipf) and the specialise-then-run scan twin (ii) across bebop,
# python (oracle), sqlite (ctypes) and Rust, checks folds BEFORE trusting any timing (RED on
# mismatch, per the blueprint's §7/§9), and appends the gate verdicts to
# bench/vs_rust/REPORT-b2.md. honest.sh/sgraph2.sh conventions: PIN auto-detected to a big
# (A78) core, R medians of full-process runs.
#
# THE ONE-COMMAND RUN (register-model bebop.bin promoted, A1 GREEN):
#   BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT R=11 bash bench/vs_rust/twins_b2.sh
# Row (iii), the CSR-build profile, is a separate command — see the tail of this file's report
# section and bench/vs_rust/csr_profile_b2.sh.
# env: BEBOP_BIN (default ./bebop.bin), BEBOP_TMP (default /tmp/opencode), R (default 11),
# N_JOIN (default 1000000), N_SCAN (default 1000000), SCAN_REPS (default 200),
# REPORT (default bench/vs_rust/REPORT-b2.md).
#
# --- two things this driver deliberately does that the blueprint's §3 did not ask for ---
#
# 1. rust_csr is the Rust side the gate is adjudicated against, not rust_hash/rust_merge.
#    The blueprint named `HashMap` and sort-merge, and both are honest GENERIC joins — but
#    neither is what a Rust author writes once told the keys are dense integers in [0, n), and
#    that is exactly what bebop's kernel is told. Gating on them would let an ALGORITHM win
#    (CSR vs hashing) be reported as a language win, which is the one outcome this row must not
#    produce. bench/vs_rust/rust_once/join_csr.rs is the same counting-sort CSR + Gustavson
#    probe bebop runs, in Rust, at two index widths (32 = best Rust, 64 = bebop's i64-only
#    array layout). All four Rust rows are printed; `rust_best` is the minimum over all of them.
#
# 2. the scan twin (ii) scans a MATERIALISED table and reports latency as compile + one scan.
#    See bench/vs_rust/std_tests/gen_scan.py's header for why the rows are no longer generated
#    inside the timed loop. The blueprint's literal gate for this row
#    (`bebop_total(compile+run) <= rust_generic/5`) is unmeetable by construction once the
#    compile is real and the scan is only a few ms — a ~20 ms specialisation cannot be paid off
#    inside a 10 ms query — so this driver reports what the ROADMAP row actually asks for
#    ("scan twin rows first/repeat") plus the CROSSOVER: how many executions, or how many rows,
#    the specialisation has to be amortised over before bebop beats the generic Rust scan. That
#    number, not a pass/fail, is what a specialising compiler should be judged on.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BB=${BEBOP_BIN:-./bebop.bin}
T=${BEBOP_TMP:-/tmp/opencode}/b2
R=${R:-11}
N_JOIN=${N_JOIN:-1000000}
N_SCAN=${N_SCAN:-1000000}
SCAN_REPS=${SCAN_REPS:-200}
REPORT=${REPORT:-bench/vs_rust/REPORT-b2.md}
mkdir -p "$T/rust"
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")

echo "compiling join_twin.bp ($BB) ..."
./seed/build/seed "$BB" compile bench/vs_rust/std_tests/join_twin.bp "$T/join_twin.bin" >/dev/null 2>&1 \
  || { echo "COMPILEFAIL join_twin"; exit 1; }
for r in join_hash join_merge join_csr scan_const scan_generic; do
  rustc -O -o "$T/rust/$r" bench/vs_rust/rust_once/$r.rs 2>/dev/null \
    || { echo "RUSTC FAIL $r"; exit 1; }
done

T="$T" R="$R" PIN="$PIN" BB="$BB" N_JOIN="$N_JOIN" N_SCAN="$N_SCAN" SCAN_REPS="$SCAN_REPS" \
REPORT="$REPORT" python3 - <<'PY'
import os, subprocess, sys, time, hashlib

T = os.environ['T']; R = int(os.environ['R']); PIN = os.environ['PIN']; BB = os.environ['BB']
N_JOIN = int(os.environ['N_JOIN']); N_SCAN = int(os.environ['N_SCAN'])
SCAN_REPS = int(os.environ['SCAN_REPS']); REPORT = os.environ['REPORT']
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


def rust_join(binname, dist, extra=()):
    """Returns (ms, fold). ms is the twin's own build+probe timer (stderr), matching
    join_twin.bp's mode 't' (t3-t1): generation is outside the timed region on every side."""
    out, err = run([f'{T}/rust/{binname}', str(N_JOIN), dist, *extra])
    return float(err.strip().split('\n')[-1]), int(out.strip().split('\n')[-1].split()[-1])


mismatches = []
join_rows = {}
RUST_JOINS = [('rust_hash', 'join_hash', ()),
              ('rust_merge', 'join_merge', ()),
              ('rust_csr32', 'join_csr', ('32',)),
              ('rust_csr64', 'join_csr', ('64',))]

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

    # sqlite timing: the index is built ONCE, outside the loop, and each rep is one step+reset
    # on an already-prepared statement — i.e. sqlite is charged for neither its index build nor
    # its query planning, while bebop's mode 't' IS charged for its whole CSR build every run.
    # That asymmetry is deliberate and it runs against bebop; the report prints bebop's
    # probe-only row too so the like-for-like (both indexes prebuilt) comparison is available.
    # LANG-DB §8's ctypes floor is per ctypes CALL and this loop makes 2 calls/rep, so at these
    # magnitudes (seconds vs microseconds) it is noise; it is measured and printed, not applied.
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

    bb_ms, bb_build_ms, bb_probe_ms = [], [], []
    for _ in range(R):
        bb_ms.append(float(bb_join(dist, 't')))
        bb_build_ms.append(float(bb_join(dist, 'b')))
        bb_probe_ms.append(float(bb_join(dist, 'p')))
    rust_ms = {}
    for label, binname, extra in RUST_JOINS:
        vals = []
        for _ in range(R):
            ms, fold_r = rust_join(binname, dist, extra)
            vals.append(ms)
            if fold_r != ofold:
                mismatches.append(f'join {dist} {label} fold {fold_r} != oracle {ofold}')
        rust_ms[label] = med(vals)

    join_rows[dist] = dict(bebop_ms=med(bb_ms), bebop_build_ms=med(bb_build_ms),
                           bebop_probe_ms=med(bb_probe_ms),
                           sqlite_idx_ms=med(sql_i_ms), sqlite_noidx_ms=med(sql_n_ms),
                           plan_idx=plan_i, plan_noidx=plan_n, count=ocnt, floor_us=floor_us,
                           **rust_ms)

# --- (ii) specialise-then-run scan over a materialised table ---
# Row A ("first"): a query the engine has never seen — pay the compile, then scan.
# Row B ("repeat"): the same query again — the .bin already exists, so the compile is skipped.
# The scan itself is timed by the program over SCAN_REPS in-process passes (kernel_reps.txt:
# a timed run must last ~100 ms or it measured the box); the reps are a measurement technique
# on a pure function, never part of the latency claimed.
digest, src = gen_scan.render(N_SCAN)
bp_path = f'{T}/scan_{digest}.bp'
bin_path = f'{T}/scan_{digest}.bin'
with open(bp_path, 'w') as f:
    f.write(src)

compile_ms = []
for i in range(R):
    # every rep compiles a DISTINCT source (N_SCAN + i => distinct digest) so no memo, cache or
    # warm page can turn a cold specialisation into a repeat.
    d_i, src_i = gen_scan.render(N_SCAN + i)
    p_i = f'{T}/scanc_{d_i}.bp'
    b_i = f'{T}/scanc_{d_i}.bin'
    with open(p_i, 'w') as f:
        f.write(src_i)
    if os.path.exists(b_i):
        os.remove(b_i)
    t0 = time.perf_counter()
    cp = subprocess.run(['taskset', '-c', PIN, './seed/build/seed', BB, 'compile', p_i, b_i],
                        capture_output=True)
    compile_ms.append((time.perf_counter() - t0) * 1000)
    # LOUD: this result went unchecked until 2026-09-13, and that is why this gate had not run
    # since 2026-09-09. gen_scan.py emitted `fn scan`, `scan` became a RESERVED WORD, every
    # compile died at `error[E99]: reserved word used as a fn name` rc=99 -- and because nothing
    # looked at rc, the next line's os.remove(b_i) raised FileNotFoundError instead. The harness
    # reported `rc=1` after 475 s with NO diagnostic at all, which reads as a flaky benchmark
    # rather than a one-word compile error.
    if cp.returncode != 0 or not os.path.exists(b_i):
        sys.stderr.write('COMPILEFAIL scan rep %d rc=%d: %s\n'
                         % (i, cp.returncode, cp.stderr.decode('utf-8', 'replace').strip()[-400:]))
        raise SystemExit(1)
    os.remove(b_i)
    os.remove(p_i)

try:
    os.remove(bin_path)
except FileNotFoundError:
    pass
cp2 = subprocess.run(['taskset', '-c', PIN, './seed/build/seed', BB, 'compile', bp_path, bin_path],
                     capture_output=True)
if cp2.returncode != 0 or not os.path.exists(bin_path):
    sys.stderr.write('COMPILEFAIL scan rc=%d: %s\n'
                     % (cp2.returncode, cp2.stderr.decode('utf-8', 'replace').strip()[-400:]))
    raise SystemExit(1)

oracle_sum = stw.scan(N_SCAN)
out, _ = run(['./seed/build/seed', bin_path, 's', '1'])
bb_sum = int(out.strip().split('\n')[-1])
if bb_sum != oracle_sum:
    mismatches.append(f'scan bebop {bb_sum} != oracle {oracle_sum}')
bb_scan_ms = []
for _ in range(R):
    out, _ = run(['./seed/build/seed', bin_path, 't', str(SCAN_REPS)])
    bb_scan_ms.append(float(out.strip().split('\n')[-1]) / SCAN_REPS)

rc_ms, rg_ms = [], []
for _ in range(R):
    out, err = run([f'{T}/rust/scan_const', str(N_SCAN), str(SCAN_REPS)])
    rc_ms.append(float(err.strip().split('\n')[-1]))
    s = int(out.strip().split()[-1])
    if s != oracle_sum:
        mismatches.append(f'scan rust_const {s} != oracle {oracle_sum}')
for _ in range(R):
    out, err = run([f'{T}/rust/scan_generic', str(N_SCAN), str(SCAN_REPS)])
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
             f'core {PIN}, R={R}, N_JOIN={N_JOIN}, N_SCAN={N_SCAN}, SCAN_REPS={SCAN_REPS}) — '
             f'folds {"equal" if folds_ok else "MISMATCH"}')
if not folds_ok:
    lines.append('\nMISMATCHES (RED — a bug, not a number):')
    for m in mismatches:
        lines.append(f'- {m}')

lines.append('\n### (i) join as SpGEMM — bebop CSR build + Gustavson probe, generation untimed everywhere\n')
lines.append('| dist | bebop total ms | (build) | (probe) | rust_csr32 ms | rust_csr64 ms | '
             'rust_hash ms | rust_merge ms | sqlite_idx ms | sqlite_noidx ms | pairs | '
             'sqlite_best/bebop | rust_best/bebop | gate |')
lines.append('|---|---|---|---|---|---|---|---|---|---|---|---|---|---|')
for dist, row in join_rows.items():
    sqlite_best = min(row['sqlite_idx_ms'], row['sqlite_noidx_ms'])
    rust_best = min(row['rust_csr32'], row['rust_csr64'], row['rust_hash'], row['rust_merge'])
    r_sql = ratio(sqlite_best, row['bebop_ms'])
    r_rust = ratio(rust_best, row['bebop_ms'])   # gate wants bebop >= 0.7x best Rust
    gate = 'MET' if (r_sql >= 10 and r_rust >= 0.7) else 'UNMET'
    lines.append(f"| {dist} | {row['bebop_ms']:.1f} | {row['bebop_build_ms']:.1f} | "
                 f"{row['bebop_probe_ms']:.1f} | {row['rust_csr32']:.1f} | {row['rust_csr64']:.1f} | "
                 f"{row['rust_hash']:.1f} | {row['rust_merge']:.1f} | "
                 f"{row['sqlite_idx_ms']:.1f} | {row['sqlite_noidx_ms']:.1f} | {row['count']} | "
                 f"{r_sql:.1f}x | {r_rust:.2f}x | {gate} |")
    lines.append(f"  - plan idx: {row['plan_idx']}; plan no-idx: {row['plan_noidx']}; "
                 f"ctypes floor: {row['floor_us']:.2f} us/call (2 calls/rep, not subtracted)")
lines.append('\n`rust_best` is the minimum over ALL four Rust twins — including rust_csr, which runs '
             "bebop's own algorithm. Reading the gate against rust_hash/rust_merge alone would "
             'credit bebop with an algorithm choice; see this script\'s header.')

lines.append('\n### (ii) specialise-then-run scan over a materialised 3-column table\n')
lines.append('| row | ms |')
lines.append('|---|---|')
lines.append(f'| bebop cold specialisation (compile a never-seen query), median of {R} | {med(compile_ms):.1f} |')
lines.append(f'| bebop scan, per pass (median of {R} runs of {SCAN_REPS} in-process passes) | {med(bb_scan_ms):.3f} |')
lines.append(f'| **bebop FIRST: compile + one scan** | **{med(compile_ms) + med(bb_scan_ms):.1f}** |')
lines.append(f'| **bebop REPEAT: one scan, compile memoised away** | **{med(bb_scan_ms):.3f}** |')
lines.append(f'| rust_generic (predicate interpreted at run time) | {med(rg_ms):.3f} |')
lines.append(f'| rust_const (query known to rustc — the unattainable floor) | {med(rc_ms):.3f} |')
gain = med(rg_ms) - med(bb_scan_ms)
if gain > 0:
    n_exec = med(compile_ms) / gain
    rows_even = N_SCAN * med(compile_ms) / gain
    lines.append(f'\ncrossover: bebop saves {gain:.3f} ms per {N_SCAN}-row scan against rust_generic, so '
                 f'the {med(compile_ms):.1f} ms specialisation pays for itself after **{n_exec:.1f} executions** '
                 f'of this query, or in a single query over **{rows_even/1e6:.1f}M rows**.')
else:
    lines.append(f'\ncrossover: NONE — bebop\'s specialised scan ({med(bb_scan_ms):.3f} ms) is not faster than '
                 f'the generic Rust scan ({med(rg_ms):.3f} ms), so the compile can never be amortised.')
lines.append(f'specialisation is worth {ratio(med(rg_ms), med(rc_ms)):.2f}x inside Rust itself '
             f'(rust_generic / rust_const); bebop\'s specialised scan is '
             f'{ratio(med(bb_scan_ms), med(rc_ms)):.2f}x rust_const and '
             f'{ratio(med(rg_ms), med(bb_scan_ms)):.2f}x faster than rust_generic.')

lines.append('\n### (iii) CSR-build profile\n')
lines.append('not run by this script — two commands, both needed, and they must be read together:\n'
             '```\n'
             'BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT bash bench/vs_rust/csr_profile_b2.sh   # store arm\n'
             './seed/build/seed $OUT/b2/csr_build_plain.bin 1000000 5000000                # plain arm\n'
             '```\n'
             'The store arm (bench/vs_rust/std_tests/csr_build_profile.bp) is sgraph2.bp\'s phase_build '
             'with rp/ci as store objects; the plain arm (csr_build_plain.bp) is the identical counting '
             'sort over `zeros()` arrays. Both print the same fold, so the difference between their '
             'phase tables is the store and nothing else.')

with open(REPORT, 'a') as f:
    f.write('\n'.join(lines) + '\n')
print('\n'.join(lines))
if not folds_ok:
    print('B2 FAIL: fold mismatch (see above)')
    sys.exit(1)
PY
