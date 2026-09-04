#!/usr/bin/env bash
# T100 gate (D1(c)): tensor-query latency vs sqlite, both engines on the same
# 1M LCG points and 1000 queries, pinned to one A78 core, folds cross-checked:
#   bebop nn.bp    (brute scan, Q=20)      fold == oracle truth_fold_Q20
#   bebop nnidx.bp (3x3 cell bucket index) fold == oracle window_fold_Q1000 == sqlite C-API fold
# Rows: sqlite scan (python wrapper, ORDER BY ... LIMIT 1), sqlite indexed via the
# python wrapper and via the C API (ctypes prepared statement), bebop scan, bebop
# indexed. env: BEBOP_BIN, BEBOP_TMP, R (runs, default 5). Writes RESULT.md.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-5}
T=${BEBOP_TMP:-/tmp/opencode}/tq_sqlite; mkdir -p "$T"
[ -s "$BEBOP_BIN" ] || { echo "GUARD: BEBOP_BIN=$BEBOP_BIN missing or empty (L12)"; exit 1; }
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
for b in nn nnidx; do ./seed/build/seed "$BEBOP_BIN" compile bench/tq_sqlite/$b.bp "$T/$b.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL $b"; exit 1; }; done
ORA=$(taskset -c "$PIN" python3 bench/tq_sqlite/oracle.py 1000000 1000) || { echo "ORACLE FAIL"; exit 1; }
CAPI=$(taskset -c "$PIN" python3 bench/tq_sqlite/sqlite_capi.py 1000000 1000) || { echo "CAPI FAIL"; exit 1; }
T="$T" R="$R" PIN="$PIN" ORA="$ORA" CAPI="$CAPI" BB="$BEBOP_BIN" python3 - <<'PY'
import os, re, subprocess, statistics, hashlib
T=os.environ['T']; R=int(os.environ['R']); PIN=os.environ['PIN']; ORA=os.environ['ORA']; CAPI=os.environ['CAPI']; BB=os.environ['BB']
g=lambda k,s: re.search(k+r'=([-\d.]+)',s).group(1)
truth20=int(g('truth_fold_Q20',ORA)); win=int(g('window_fold_Q1000',ORA)); wm=re.search(r'window_matches_truth=(\d+)/',ORA).group(1)
sq_scan=float(g('scan_ms_per_query',ORA)); sq_idx=float(g('indexed_us_per_query',ORA)); sq_build=float(g('build_ms',ORA))
capi=float(g('indexed_capi_us_per_query',CAPI)); capi_fold=int(g('window_fold_Q1000',CAPI))
def runs(b):
    ms=[]; folds=set()
    for _ in range(R):
        v=int(subprocess.run(['taskset','-c',PIN,'./seed/build/seed',f'{T}/{b}.bin'],capture_output=True,text=True).stdout.strip().split('\n')[-1])
        folds.add(v//1000000); ms.append(v%1000000)
    return statistics.median(ms), folds
nn_ms,nn_f=runs('nn'); ni_ms,ni_f=runs('nnidx')
nn_per=nn_ms/20.0; ni_us=ni_ms*1000.0/1000
ok_scan = nn_f=={truth20}; ok_idx = ni_f=={win} and capi_fold==win
md5=hashlib.md5(open(BB,'rb').read()).hexdigest()[:8]
L=[f'# T100 — tensor query vs sqlite, 1M points, pinned core {PIN} (A78), R={R} medians, bebop.bin {md5}',
   f'- data: LCG seed 12345, (u,v) in [-2^30,2^30), 1024x1024 cells of 2^21; queries: next 1000 LCG pairs; folds mod 1e9+7 (bench/tq_sqlite/oracle.py)',
   f'- fold checks: bebop scan == truth_fold_Q20 ({"YES" if ok_scan else "NO "+str(nn_f)}); bebop indexed == python window fold == sqlite C-API fold ({"YES" if ok_idx else "NO "+str(ni_f)+" "+str(capi_fold)}); 3x3 window == true nearest on {wm}/1000 queries (both windowed engines share the miss)',
   f'- sqlite 3.46.1 in-memory, build+index {sq_build:.0f} ms (python executemany); bebop build = zeros + LCG fill + counting sort, inside the same process (not timed separately)',
   '', '| engine / query | per query | vs bebop same class |', '|---|---|---|',
   f'| sqlite scan, `ORDER BY d LIMIT 1` (python wrapper, Q=20) | {sq_scan:.1f} ms | {sq_scan/nn_per:.1f}x slower |',
   f'| bebop scan nn.bp (Q=20) | {nn_per:.1f} ms | 1.0x |',
   f'| sqlite indexed 3x3 window, python wrapper (Q=1000) | {sq_idx:.1f} us | {sq_idx/ni_us:.1f}x slower |',
   f'| sqlite indexed 3x3 window, C API prepared statement (Q=1000) | {capi:.1f} us | {capi/ni_us:.1f}x slower |',
   f'| bebop indexed nnidx.bp: cell -> CSR bucket -> 3x3 window (Q=1000) | {ni_us:.1f} us | 1.0x |',
   '', f'- pass rule (docs/SPEEDUP-ANALYSIS.md 4.3): indexed <= 10 us AND >= 3x sqlite C-API: {"PASS" if ni_us<=10 and capi/ni_us>=3 else "FAIL"}; scan >= 10x sqlite scan: {"PASS" if sq_scan/nn_per>=10 else "FAIL ("+f"{sq_scan/nn_per:.1f}x)"}']
open('bench/tq_sqlite/RESULT.md','w').write('\n'.join(L)+'\n'); print('\n'.join(L))
PY
