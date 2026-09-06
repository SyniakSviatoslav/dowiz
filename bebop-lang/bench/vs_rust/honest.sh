#!/usr/bin/env bash
# honest.sh (D11-C, 2026-09-05): the D1(a) column — in-process pinned ms of the HONEST
# kernels (K1h/K3h: loop-carried nonlinear recurrence, LLVM cannot close-form or
# vectorise; K2h: fib(25) with #[inline(never)]; K4 unchanged) for bebop
# (bench630/k*ht.bp clock_ms) and Rust (rust_once/k*h.rs Instant on stderr).
# Run on a QUIET machine. env: BEBOP_BIN, BEBOP_TMP, R (default 11).
# 2026-09-06: clock_ms is 1 ms coarse, so every kernel runs REPS=100 reps in-process and
# returns the TOTAL ms; the table divides by REPS (0.01 ms resolution). K4 has its own
# honest twin now (rust_once/k4h.rs, black_box only on input/output).
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-11}
T=${BEBOP_TMP:-/tmp/opencode}/honest; mkdir -p "$T/rust"
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
for k in k1h k2h k3h k4; do
  ./seed/build/seed "$BEBOP_BIN" compile bench/vs_rust/bench630/${k}t.bp "$T/${k}t.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL ${k}t"; exit 1; }
  rustc -O -o "$T/rust/$k" bench/vs_rust/rust_once/$([ $k = k4 ] && echo k4h || echo $k).rs 2>/dev/null || { echo "RUSTC FAIL $k"; exit 1; }
done
T="$T" R="$R" PIN="$PIN" BB="$BEBOP_BIN" python3 - <<'PY'
import os, subprocess, statistics, hashlib
T=os.environ['T']; R=int(os.environ['R']); PIN=os.environ['PIN']; BB=os.environ['BB']
def med(v): v=sorted(v); return v[len(v)//2], v[min(len(v)-1,int(round(0.95*(len(v)-1))))]
rows=[]; rss={}
for k in ['k1h','k2h','k3h','k4']:
    bb=[]; rs=[]
    for _ in range(R):
        p=subprocess.Popen(['taskset','-c',PIN,'./seed/build/seed',f'{T}/{k}t.bin'],stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
        v=p.stdout.read().decode().strip().split('\n')[-1]; _,_,ru=os.wait4(p.pid,0); rss[k]=max(rss.get(k,0),ru.ru_maxrss)  # D12-D: RSS column (T97)
        bb.append(int(v)/100.0)  # TOTAL ms over REPS=100 reps
        if True:
            e=subprocess.run(['taskset','-c',PIN,f'{T}/rust/{k}'],capture_output=True,text=True).stderr.strip().split('\n')[-1]
            rs.append(float(e))
    rows.append((k,med(bb),med(rs)))
md5=hashlib.md5(open(BB,'rb').read()).hexdigest()[:8]
print(f'# honest twins (D11-C), in-process pinned core {PIN}, R={R}, REPS=100 per run, bebop.bin {md5}')
print('| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |')
print('|---|---|---|---|---|---|---|')
for k,(bm,bp),(rm,rp) in rows:
    ratio = bm/rm if rm==rm and rm>0 else float('nan')
    print(f'| {k.upper()} | {bm:.2f} / {bp:.2f} | {rm:.3f} / {rp:.3f} | {ratio:.1f}x | {"MET" if ratio <= 2.0 else "UNMET"} | {ratio:.1f}x | {rss.get(k,0)/1024:.1f} |')
import re
try:
    k6=re.search(r'\| bebop scan nn\.bp \(Q=20\) \| ([0-9.]+) ms', open('bench/tq_sqlite/RESULT.md').read()).group(1)
except Exception: k6='?'
# K5 (2026-09-06): measured here, COLD (the .becache replay is removed before every run), 3 runs, median
import time
k5v=[]
for _ in range(3):
    for f in (f'{T}/k5.bin', f'{T}/k5.bin.becache', f'{T}/k5.bin.use'):
        try: os.remove(f)
        except FileNotFoundError: pass
    t=time.time(); subprocess.run(['taskset','-c',PIN,'./seed/build/seed',BB,'compile','bebop.bp',f'{T}/k5.bin'],capture_output=True); k5v.append(time.time()-t)
k5=sorted(k5v)[1]
print(f'| K5 self-compile of bebop.bp (cold, pinned, median of 3) | {k5:.2f} s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |')
print(f'| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | {k6} ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |')
PY
