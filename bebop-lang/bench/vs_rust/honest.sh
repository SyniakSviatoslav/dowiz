#!/usr/bin/env bash
# honest.sh (D11-C, 2026-09-05): the D1(a) column — in-process pinned ms of the HONEST
# kernels (K1h/K3h: loop-carried nonlinear recurrence, LLVM cannot close-form or
# vectorise; K2h: fib(25) with #[inline(never)]; K4 unchanged) for bebop
# (bench630/k*ht.bp clock_ms) and Rust (rust_once/k*h.rs Instant on stderr).
# Run on a QUIET machine. env: BEBOP_BIN, BEBOP_TMP, R (default 11).
# 2026-09-06: clock_ms is 1 ms coarse, so every kernel runs REPS=100 reps in-process and
# returns the TOTAL ms; the table divides by REPS (0.01 ms resolution). K4 has its own
# honest twin now (rust_once/k4h.rs, black_box only on input/output).
# 2026-10-07 (R6, research 2026-10-07 §12 L2 STEP 4): two honesty columns. (1) the ratio's
# p05..p95 range = [bebop p05 / Rust p95, bebop p95 / Rust p05] over the R runs, so a ratio is
# read with its spread; (2) the pinned core's scaling_cur_freq, read before and after each
# kernel's R runs (both languages). A kernel whose frequency moved > 10 % prints `?` after its
# ratio: the two sides may have run at different clocks. An unreadable frequency prints
# `NOT MEASURED` and `?` too -- never a clean ratio. The governor is printed in the header.
# Governor state when this was proved (2026-10-07, core 4): `walt`, cur_freq moving
# 691200..2304000 kHz between consecutive reads on a busy box.
# Mutation proof (HONEST_CPUFREQ=<dir> replaces the sysfs dir): a dir whose scaling_cur_freq is
# rewritten 2054400 -> 1190400 mid-run prints `?` on the straddling row only; a missing dir prints
# NOT MEASURED + `?` on every row; the real sysfs at a steady 2054 MHz prints no `?`.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-11}
T=${BEBOP_TMP:-/tmp/opencode}/honest; mkdir -p "$T/rust"
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
for k in k1h k2h k3h k4 k8h; do
  ./seed/build/seed "$BEBOP_BIN" compile bench/vs_rust/bench630/${k}t.bp "$T/${k}t.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL ${k}t"; exit 1; }
  rustc -O -o "$T/rust/$k" bench/vs_rust/rust_once/$([ $k = k4 ] && echo k4h || echo $k).rs 2>/dev/null || { echo "RUSTC FAIL $k"; exit 1; }
done
# R-CF (lane rfast-lsr, 2026-10-08): the compiler now eliminates K1H/K3H/K4's affine loops (closed
# form, compiler/affine.bp), so those rows are "loop eliminated" and NOT a codegen comparison; the
# codegen claim is carried by the "kept loop" twins, compiled with BEBOP_NO_CF=1 (pass off).
for k in k1h k3h k4; do
  BEBOP_NO_CF=1 ./seed/build/seed "$BEBOP_BIN" compile bench/vs_rust/bench630/${k}t.bp "$T/${k}t_kept.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL ${k}t kept"; exit 1; }
done
T="$T" R="$R" PIN="$PIN" BB="$BEBOP_BIN" python3 - <<'PY'
import os, subprocess, statistics, hashlib
T=os.environ['T']; R=int(os.environ['R']); PIN=os.environ['PIN']; BB=os.environ['BB']
# 2026-09-08: the rep count is per kernel now (bench/vs_rust/kernel_reps.txt is the single
# source of truth, baked into both bench630/<k>t.bp and rust_once/<k>.rs). A kernel whose run
# is only a few ms measures the box, not the code -- see the table's header for the numbers.
REPS={l.split()[0]: int(l.split()[1]) for l in open('bench/vs_rust/kernel_reps.txt')
      if l.strip() and not l.startswith('#')}
def med(v): v=sorted(v); return v[len(v)//2], v[min(len(v)-1,int(round(0.95*(len(v)-1))))]
def p05(v): v=sorted(v); return v[int(round(0.05*(len(v)-1)))]
CPUF=os.environ.get('HONEST_CPUFREQ') or f'/sys/devices/system/cpu/cpu{PIN}/cpufreq/'  # override = the mutation proof's input
def rd(f):
    try: return open(CPUF+f).read().strip()
    except Exception: return None
def freq():
    v=rd('scaling_cur_freq'); return int(v) if v and v.isdigit() else -1
rows=[]; rss={}; fq={}; lo05={}
for k in ['k1h','k2h','k3h','k4','k8h']:
    bb=[]; rs=[]; f0=freq()
    for _ in range(R):
        p=subprocess.Popen(['taskset','-c',PIN,'./seed/build/seed',f'{T}/{k}t.bin'],stdout=subprocess.PIPE,stderr=subprocess.DEVNULL)
        v=p.stdout.read().decode().strip().split('\n')[-1]; _,_,ru=os.wait4(p.pid,0); rss[k]=max(rss.get(k,0),ru.ru_maxrss)  # D12-D: RSS column (T97)
        bb.append(int(v)/float(REPS[k]))  # the kernel returns its TOTAL ms; divide by ITS rep count reps
        if True:
            e=subprocess.run(['taskset','-c',PIN,f'{T}/rust/{k}'],capture_output=True,text=True).stderr.strip().split('\n')[-1]
            rs.append(float(e))
    f1=freq(); fq[k]=(f0,f1); lo05[k]=(p05(bb),p05(rs))
    rows.append((k,med(bb),med(rs)))
md5=hashlib.md5(open(BB,'rb').read()).hexdigest()[:8]
print(f'# honest twins (D11-C), in-process pinned core {PIN}, R={R}, reps per run ' + ' '.join(f'{k}={REPS[k]}' for k in ['k1h','k2h','k3h','k4','k8h']) + f', bebop.bin {md5}')
print(f'# core {PIN} governor {rd("scaling_governor")}; a row whose scaling_cur_freq moved > 10 % between its before/after reads prints ? after the ratio')
print('| kernel | bebop med / p95 ms per rep | Rust honest med / p95 ms per rep | bebop / Rust | ratio p05..p95 range | core MHz before -> after | gate <= 2.0x (TG-DONE 1) | 1.0x (D1(a) long target) | bebop RSS MB |')
print('|---|---|---|---|---|---|---|---|---|')
ELIM={'k1h','k3h','k4'}
for k,(bm,bp),(rm,rp) in rows:
    ratio = bm/rm if rm==rm and rm>0 else float('nan')
    (bl,rl),(f0,f1) = lo05[k], fq[k]
    rng = f'{bl/rp:.2f}..{bp/rl:.2f}x' if rp>0 and rl>0 else 'n/a'
    moved = f0<=0 or f1<=0 or abs(f1-f0) > 0.10*f0
    fcol = 'NOT MEASURED' if f0<=0 or f1<=0 else f'{f0//1000} -> {f1//1000}'
    q = ' ?' if moved else ''
    lab = k.upper() + (' loop eliminated (R-CF; not codegen)' if k in ELIM else '')
    print(f'| {lab} | {bm:.4f} / {bp:.3f} | {rm:.3f} / {rp:.3f} | {ratio:.1f}x{q} | {rng} | {fcol} | {"MET" if ratio <= 2.0 else "UNMET"} | {ratio:.1f}x | {rss.get(k,0)/1024:.1f} |')
# the kept-loop twins: same kernels, BEBOP_NO_CF=1, same pin/reps, interleaved with a fresh Rust run
for k in ['k1h','k3h','k4']:
    bb=[]; rs=[]
    for _ in range(R):
        v=subprocess.run(['taskset','-c',PIN,'./seed/build/seed',f'{T}/{k}t_kept.bin'],capture_output=True,text=True).stdout.strip().split('\n')[-1]
        bb.append(int(v)/float(REPS[k]))
        rs.append(float(subprocess.run(['taskset','-c',PIN,f'{T}/rust/{k}'],capture_output=True,text=True).stderr.strip().split('\n')[-1]))
    (bm,bp),(rm,rp)=med(bb),med(rs); ratio=bm/rm
    print(f'| {k.upper()} kept loop (BEBOP_NO_CF=1) | {bm:.3f} / {bp:.3f} | {rm:.3f} / {rp:.3f} | {ratio:.1f}x | | | {"MET" if ratio <= 2.0 else "UNMET"} | {ratio:.1f}x | |')
# --- K2H2 row (lane rfast-lsr, 2026-10-07, operator Q3: K2H shown BOTH ways). K2H above lets
# both compilers eliminate the second recursive call (LLVM's TRE; bebop's when the compiler has
# it); K2H2 forbids it on both sides: bench630/k2h2t.bp binds both calls with `let` (no
# `f(..) + f(..)` tail) and rust_once/k2h_2call.rs puts black_box on the second call (objdump:
# two `bl fib` per node). Same reps as k2h, same pin, same interleaving. A compile failure
# REFUSES the row rather than printing a number for nothing.
if subprocess.run(['./seed/build/seed',BB,'compile','bench/vs_rust/bench630/k2h2t.bp',f'{T}/k2h2t.bin'],capture_output=True).returncode!=0 or \
   subprocess.run(['rustc','-O','-o',f'{T}/rust/k2h2','bench/vs_rust/rust_once/k2h_2call.rs'],capture_output=True).returncode!=0:
    raise SystemExit('K2H2 COMPILE FAILED')
bb=[]; rs=[]
for _ in range(R):
    v=subprocess.run(['taskset','-c',PIN,'./seed/build/seed',f'{T}/k2h2t.bin'],capture_output=True,text=True).stdout.strip().split('\n')[-1]
    bb.append(int(v)/float(REPS['k2h']))
    rs.append(float(subprocess.run(['taskset','-c',PIN,f'{T}/rust/k2h2'],capture_output=True,text=True).stderr.strip().split('\n')[-1]))
(bm,bp),(rm,rp)=med(bb),med(rs); ratio=bm/rm
print(f'| K2H2 (no TRE either side) | {bm:.3f} / {bp:.3f} | {rm:.3f} / {rp:.3f} | {ratio:.1f}x | | | {"MET" if ratio <= 2.0 else "UNMET"} | {ratio:.1f}x | |')
import re
try:
    k6=re.search(r'\| bebop scan nn\.bp \(Q=20\) \| ([0-9.]+) ms', open('bench/tq_sqlite/RESULT.md').read()).group(1)
except Exception: k6='?'
# K5 (2026-09-06): measured here, COLD (every k5.bin* sidecar is removed before every run), 3 runs, median
import time
k5v=[]
for _ in range(3):
    # 2026-10-07: every k5.bin* sidecar goes (the .dag memo added later made runs 2-3 a 0.04 s
    # replay and the row read 0.09 s), and a failed compile refuses instead of timing an error.
    import glob
    for f in glob.glob(f'{T}/k5.bin*'): os.remove(f)
    t=time.time(); r=subprocess.run(['taskset','-c',PIN,'./seed/build/seed',BB,'compile','bebop.bp',f'{T}/k5.bin'],capture_output=True); k5v.append(time.time()-t)
    if r.returncode != 0: raise SystemExit(f'K5 COMPILE FAILED rc={r.returncode}')
k5=sorted(k5v)[1]
print(f'| K5 self-compile of bebop.bp (cold, pinned, median of 3) | {k5:.2f} s | (no twin: rustc is not a fair twin of a 200 KB one-pass compiler) | |')
print(f'| K6 nnidx scan 1M (bench/tq_sqlite/RESULT.md, Q=20) | {k6} ms | sqlite scan 183 ms python / ~158 ms native (T100) | store faster |')
PY
