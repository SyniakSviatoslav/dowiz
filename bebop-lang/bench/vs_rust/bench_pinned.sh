#!/usr/bin/env bash
# bench_pinned.sh — T63/T72/T97 (decision D3): K1-K4 process wall-clock pinned
# to one A78 core vs unpinned, Bebop vs the Rust once-twins (rust_once/*.rs,
# rustc -O), 31 runs each, median + p95, peak RSS (ru_maxrss == VmHWM, read via
# wait4 so no /proc race), in-process clock_ms (bench630/k*t.bp) pinned, and
# words/iteration of every backward-branch loop in the compiled .bin.
# env: BEBOP_BIN (default ./bebop.bin), BEBOP_TMP (scratch), R (runs, 31).
# Honest: taskset is verified through Cpus_allowed_list and the result is
# printed; if proot ignored the mask the table says so instead of pretending.
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}
T=${BEBOP_TMP:-/tmp/opencode}/bench_pinned; mkdir -p "$T"
R=${R:-31}
SEED=./seed/build/seed
[ -s "$BEBOP_BIN" ] || { echo "empty/missing BEBOP_BIN=$BEBOP_BIN (L12)"; exit 90; }

BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
USABLE=$(python3 -c 'import os;print(*sorted(os.sched_getaffinity(0)))')
PIN=""; for c in $BIG; do for u in $USABLE; do [ "$c" = "$u" ] && [ -z "$PIN" ] && PIN=$c; done; done
[ -n "$PIN" ] || { echo "no usable A78 core (big=[$BIG] usable=[$USABLE])"; exit 1; }
ALLOWED=$(taskset -c "$PIN" cat /proc/self/status | awk '/Cpus_allowed_list/{print $2}')
PINOK=no; [ "$ALLOWED" = "$PIN" ] && PINOK=yes

for k in 1 2 3 4; do
  $SEED "$BEBOP_BIN" compile bench/vs_rust/kernels/k$k.bp "$T/k$k.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL k$k"; exit 1; }
  $SEED "$BEBOP_BIN" compile bench/vs_rust/bench630/k${k}t.bp "$T/k${k}t.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL k${k}t"; exit 1; }
done
printf 'fn main() -> i64 { 0 }\n' > "$T/k0.bp"
$SEED "$BEBOP_BIN" compile "$T/k0.bp" "$T/k0.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL k0"; exit 1; }
mkdir -p "$T/rust"
for k in 0 1 2 3 4; do
  [ -x "$T/rust/k$k" ] || rustc -O -o "$T/rust/k$k" bench/vs_rust/rust_once/k$k.rs || { echo "RUSTC FAIL k$k"; exit 1; }
done

T="$T" R="$R" PIN="$PIN" PINOK="$PINOK" BIG="$BIG" USABLE="$USABLE" ALLOWED="$ALLOWED" BEBOP_BIN="$BEBOP_BIN" SEED="$SEED" python3 - <<'PY'
import os, re, subprocess, time, statistics, hashlib
T=os.environ['T']; R=int(os.environ['R']); PIN=os.environ['PIN']; SEED=os.environ['SEED']; BB=os.environ['BEBOP_BIN']
EXPECT={'k1':'500000500000','k2':'75025','k3':'40635000','k4':None,'k0':'0'}

def run1(argv, pin):
    if pin: argv=['taskset','-c',pin]+argv
    t0=time.perf_counter()
    p=subprocess.Popen(argv, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    out=p.stdout.read()
    _,st,ru=os.wait4(p.pid,0)
    dt=(time.perf_counter()-t0)*1000.0
    p.returncode=os.waitstatus_to_exitcode(st) if hasattr(os,'waitstatus_to_exitcode') else 0
    return dt, ru.ru_maxrss, out.decode(errors='replace').strip().split('\n')[-1] if out else ''

def series(argv, pin, n):
    run1(argv, pin)  # warmup
    ts=[]; rss=0; last=''
    for _ in range(n):
        dt,r,last=run1(argv,pin); ts.append(dt); rss=max(rss,r)
    ts.sort()
    return statistics.median(ts), ts[min(len(ts)-1,int(round(0.95*(len(ts)-1))))], rss, last

def words_per_iter(binpath):
    d=subprocess.run(['objdump','-D','-b','binary','-m','aarch64',binpath],capture_output=True,text=True).stdout
    loops=[]
    for m in re.finditer(r'^\s*([0-9a-f]+):\s+[0-9a-f]{8}\s+(b|b\.\w+|cbz|cbnz|tbz|tbnz)\s+(?:\w+,\s*(?:#\d+,\s*)?)?0x([0-9a-f]+)\s*$', d, re.M):
        a=int(m.group(1),16); t=int(m.group(3),16)
        if t<=a: loops.append((a-t)//4+1)
    return sorted(set(loops))

rows=[]
res={}
for k in ['k0','k1','k2','k3','k4']:
    bp=[SEED, f'{T}/{k}.bin']; ru=[f'{T}/rust/{k}']
    bpin=series(bp,PIN,R); bun=series(bp,'',R); rpin=series(ru,PIN,R); run_=series(ru,'',R)
    e=EXPECT[k]
    ok = (e is None) or (bpin[3]==e and rpin[3]==e)
    same = bpin[3]==rpin[3]
    res[k]=(bpin,bun,rpin,run_,ok,same)
# in-process clock_ms (bench630 twins, 0.1 ms units), pinned & unpinned
inproc={}
for k in ['k1','k2','k3','k4']:
    for pin,label in ((PIN,'pin'),('','unpin')):
        vals=[]
        run1([SEED,f'{T}/{k}t.bin'],pin)
        for _ in range(R):
            _,_,v=run1([SEED,f'{T}/{k}t.bin'],pin)
            try: vals.append(int(v)/10.0)
            except ValueError: vals.append(float('nan'))
        vals.sort(); inproc[(k,label)]=(statistics.median(vals), vals[min(len(vals)-1,int(round(0.95*(len(vals)-1))))])
# self-compile RSS + wall (3 runs)
sc=[]
for _ in range(3):
    dt,r,_=run1([SEED,BB,'compile','bebop.bp',f'{T}/self.bin'],PIN); sc.append((dt,r))
scdt=statistics.median([x[0] for x in sc]); scrss=max(x[1] for x in sc)
md5=hashlib.md5(open(BB,'rb').read()).hexdigest()

L=[]
L.append('# REPORT-pinned — K1-K4 pinned vs unpinned, Bebop vs Rust once-twins (T63/T72/T97, D3)\n')
L.append(f'- date: {time.strftime("%Y-%m-%d %H:%M")}; bebop.bin md5 {md5}; runs per cell R={R} (+1 warmup); rustc -O (no lto), twins bench/vs_rust/rust_once/k*.rs')
L.append(f'- big cores (part 0xd41): [{os.environ["BIG"].strip()}], sched_getaffinity usable: [{os.environ["USABLE"]}], pinned core: {PIN}; taskset honoured under proot: {os.environ["PINOK"]} (Cpus_allowed_list={os.environ["ALLOWED"]})')
L.append('- wall = whole process (seed load+mmap+run / rust start+run), perf_counter around spawn..wait4; RSS = ru_maxrss from wait4 (= VmHWM hiwater_rss, no /proc race); K0 = empty program = startup floor')
L.append('- ratio = bebop pinned median / rust pinned median (process wall, includes both startup floors)\n')
L.append('| kernel | bebop pinned med/p95 ms | bebop unpinned med/p95 ms | rust pinned med/p95 ms | rust unpinned med ms | ratio | RSS bebop KB | RSS rust KB | fold ok |')
L.append('|---|---|---|---|---|---|---|---|---|')
for k in ['k0','k1','k2','k3','k4']:
    bpin,bun,rpin,run_,ok,same=res[k]
    L.append(f'| {k.upper()} | {bpin[0]:.2f} / {bpin[1]:.2f} | {bun[0]:.2f} / {bun[1]:.2f} | {rpin[0]:.2f} / {rpin[1]:.2f} | {run_[0]:.2f} | {bpin[0]/rpin[0]:.2f}x | {bpin[2]} | {rpin[2]} | {"ok" if ok and same else ("SAME-but-unexpected" if same else "MISMATCH "+bpin[3]+" vs "+rpin[3])} |')
L.append('\n## In-process clock_ms (bench630/k*t.bp, D3 primary column), ms\n')
L.append('| kernel | pinned med / p95 | unpinned med / p95 | spread unpinned/pinned |')
L.append('|---|---|---|---|')
for k in ['k1','k2','k3','k4']:
    a=inproc[(k,'pin')]; b=inproc[(k,'unpin')]
    L.append(f'| {k.upper()} | {a[0]:.1f} / {a[1]:.1f} | {b[0]:.1f} / {b[1]:.1f} | {b[0]/a[0] if a[0] else float("nan"):.2f}x |')
L.append('\n## Words per iteration (backward-branch spans in the compiled .bin, head..back-branch inclusive; smallest = innermost loop)\n')
L.append('| kernel | loop spans (words) |'); L.append('|---|---|')
for k in ['k1','k2','k3','k4']:
    L.append(f'| {k.upper()} | {words_per_iter(f"{T}/{k}.bin")} |')
L.append(f'\n## Self-compile (`seed bebop.bin compile bebop.bp`), pinned, 3 runs\n\n- wall median {scdt:.0f} ms, peak RSS {scrss} KB ({scrss/1024:.1f} MB)\n')
open(f'{T}/REPORT-pinned.md','w').write('\n'.join(L)+'\n')
print('\n'.join(L))
PY
