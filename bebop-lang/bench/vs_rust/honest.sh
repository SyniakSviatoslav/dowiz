#!/usr/bin/env bash
# honest.sh (D11-C, 2026-09-05): the D1(a) column — in-process pinned ms of the HONEST
# kernels (K1h/K3h: loop-carried nonlinear recurrence, LLVM cannot close-form or
# vectorise; K2h: fib(25) with #[inline(never)]; K4 unchanged) for bebop
# (bench630/k*ht.bp clock_ms) and Rust (rust_once/k*h.rs Instant on stderr).
# Run on a QUIET machine. env: BEBOP_BIN, BEBOP_TMP, R (default 11).
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-11}
T=${BEBOP_TMP:-/tmp/opencode}/honest; mkdir -p "$T/rust"
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
for k in k1h k2h k3h k4; do
  ./seed/build/seed "$BEBOP_BIN" compile bench/vs_rust/bench630/${k}t.bp "$T/${k}t.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL ${k}t"; exit 1; }
  rustc -O -o "$T/rust/$k" bench/vs_rust/rust_once/$k.rs 2>/dev/null || { echo "RUSTC FAIL $k"; exit 1; }
done
T="$T" R="$R" PIN="$PIN" BB="$BEBOP_BIN" python3 - <<'PY'
import os, subprocess, statistics, hashlib
T=os.environ['T']; R=int(os.environ['R']); PIN=os.environ['PIN']; BB=os.environ['BB']
def med(v): v=sorted(v); return v[len(v)//2], v[min(len(v)-1,int(round(0.95*(len(v)-1))))]
rows=[]
for k in ['k1h','k2h','k3h','k4']:
    bb=[]; rs=[]
    for _ in range(R):
        v=subprocess.run(['taskset','-c',PIN,'./seed/build/seed',f'{T}/{k}t.bin'],capture_output=True,text=True).stdout.strip().split('\n')[-1]
        bb.append(int(v)/10.0)
        if k=='k4':
            rs.append(float('nan'))
        else:
            e=subprocess.run(['taskset','-c',PIN,f'{T}/rust/{k}'],capture_output=True,text=True).stderr.strip().split('\n')[-1]
            rs.append(float(e))
    rows.append((k,med(bb),med(rs)))
md5=hashlib.md5(open(BB,'rb').read()).hexdigest()[:8]
print(f'# honest twins (D11-C), in-process pinned core {PIN}, R={R}, bebop.bin {md5}')
print('| kernel | bebop med / p95 ms | Rust honest med / p95 ms | bebop / Rust |')
print('|---|---|---|---|')
for k,(bm,bp),(rm,rp) in rows:
    ratio = bm/rm if rm==rm and rm>0 else float('nan')
    print(f'| {k.upper()} | {bm:.1f} / {bp:.1f} | {rm:.3f} / {rp:.3f} | {ratio:.1f}x |' if rm==rm else f'| {k.upper()} | {bm:.1f} / {bp:.1f} | (K4 twin prints no in-process ms; 2.85 ms measured in SPEEDUP-ANALYSIS) | — |')
PY
