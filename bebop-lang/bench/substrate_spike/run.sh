#!/usr/bin/env bash
# run.sh — T55 spike driver: compile spike.bp / spike_fold.bp with BEBOP_BIN,
# build the Rust twin, run everything pinned to the first usable A78 core
# (D3: pinned in-process clock_ms is the primary column), R runs, medians.
# Prints a markdown table. env: BEBOP_BIN, BEBOP_TMP, R (default 5), N (300000).
set -u
cd "$(dirname "$0")/../.."
ulimit -s 65536 2>/dev/null
BEBOP_BIN=${BEBOP_BIN:-./bebop.bin}; R=${R:-5}; N=${N:-300000}
T=${BEBOP_TMP:-/tmp/opencode}/substrate_spike; mkdir -p "$T"
SEED=./seed/build/seed
(cd bench/substrate_spike && python3 lower.py "$N" >/dev/null) || exit 1
$SEED "$BEBOP_BIN" compile bench/substrate_spike/spike.bp "$T/spike.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL spike"; exit 1; }
$SEED "$BEBOP_BIN" compile bench/substrate_spike/spike_fold.bp "$T/spike_fold.bin" >/dev/null 2>&1 || { echo "COMPILEFAIL spike_fold"; exit 1; }
rustc -O -o "$T/twin" bench/substrate_spike/spike_twin.rs 2>/dev/null || { echo "RUSTC FAIL"; exit 1; }
rustc -O -o "$T/subtwin" bench/substrate_spike/spike_sub_twin.rs 2>/dev/null || { echo "RUSTC FAIL subtwin"; exit 1; }
BIG=$(awk '/^processor/{p=$3} /CPU part/ && $NF=="0xd41"{print p}' /proc/cpuinfo | tr '\n' ' ')
PIN=$(python3 -c "import os;u=sorted(os.sched_getaffinity(0));b=[int(x) for x in '$BIG'.split()];print(next((c for c in b if c in u),u[0]))")
T="$T" R="$R" N="$N" PIN="$PIN" SEED="$SEED" python3 - <<'PY'
import os, re, subprocess, statistics
T=os.environ['T']; R=int(os.environ['R']); N=int(os.environ['N']); PIN=os.environ['PIN']; SEED=os.environ['SEED']
def run(argv):
    return subprocess.run(['taskset','-c',PIN]+argv,capture_output=True,text=True).stdout.strip().split('\n')[-1]
lin=[];call=[];sub=[];oks=set();sws=set()
for _ in range(R):
    v=int(run([SEED,f'{T}/spike.bin']))
    oks.add(v//10**13); sws.add((v//10**12)%10)
    lin.append((v//10**9)%1000); call.append((v//10**6)%1000); sub.append(v%10**6)
fold=int(run([SEED,f'{T}/spike_fold.bin']))
rs=[];rf=set()
for _ in range(R):
    f,ms=run([f'{T}/twin']).split(); rf.add(int(f)); rs.append(float(ms))
rss=[];rsf=set();rsw=set()
for _ in range(R):
    f,ms,sw=run([f'{T}/subtwin']).split(); rsf.add(int(f)); rss.append(float(ms)); rsw.add(int(sw))
def med(x): return statistics.median(x)
d=subprocess.run(['objdump','-D','-b','binary','-m','aarch64',f'{T}/spike_fold.bin'],capture_output=True,text=True).stdout
loops=sorted({(int(m.group(1),16)-int(m.group(3),16))//4+1 for m in re.finditer(r'^\s*([0-9a-f]+):\s+[0-9a-f]{8}\s+(b|b\.\w+|cbz|cbnz|tbz|tbnz)\s+(?:\w+,\s*(?:#\d+,\s*)?)?0x([0-9a-f]+)\s*$',d,re.M) if int(m.group(3),16)<=int(m.group(1),16)})
ops=12; depth=7
L=[]
L.append(f'# T55 spike — one straight-line fn (12 ops, DAG depth {depth}) : linear vs cell substrate')
L.append(f'- N={N} evaluations, pinned core {PIN} (A78), R={R} medians, in-process clock_ms; bebop.bin md5 {subprocess.run(["md5sum","bebop.bin"],capture_output=True,text=True).stdout.split()[0]}')
L.append(f'- fold identical across all three bebop modes: {"YES" if oks=={1} else "NO"}; sweeps == {depth}*N: {"YES" if sws=={1} else "NO"}; bebop fold {fold} == Rust twin fold {"YES" if rf=={fold} else "NO "+str(rf)}')
L.append(f'- linear inlined loop: backward-branch spans in spike_fold.bin (words/iteration incl. driver): {loops}')
L.append('')
L.append('| mode | median ms | ns per op | vs linear-inlined | vs Rust |')
L.append('|---|---|---|---|---|')
ml,mc,ms_,mr,mrs=med(lin),med(call),med(sub),med(rs),med(rss)
tot=N*ops
for nm,m in (('bebop linear, inlined',ml),('bebop linear, fn call per eval',mc),('bebop substrate (sweeps to quiescence)',ms_),('Rust -O twin (inlined, black_box)',mr),('Rust -O twin of the SAME substrate engine (model floor)',mrs)):
    L.append(f'| {nm} | {m:.1f} | {m*1e6/tot:.1f} | {m/ml if ml else float("nan"):.2f}x | {m/mr:.2f}x |')
L.append('')
L.append(f'- substrate per sweep: {ms_*1e6/(N*depth):.0f} ns; per fired cell: {ms_*1e6/tot:.0f} ns (each sweep = tzcnt drain + branch-free 6-way op select + candidate/readiness scan)')
L.append(f'- Rust substrate twin: fold == {"YES" if rsf=={fold} else "NO"}, sweeps {rsw}; per sweep {mrs*1e6/(N*depth):.1f} ns, per cell {mrs*1e6/tot:.1f} ns -> the MODEL alone costs {mrs/mr:.0f}x over linear Rust on this ISA; bebop codegen adds another {ms_/mrs:.0f}x on top')
L.append(f'- linear inlined per op: {ml*1e6/tot:.1f} ns; Rust per op: {mr*1e6/tot:.2f} ns')
open('bench/substrate_spike/RESULT.md','w').write('\n'.join(L)+'\n')
print('\n'.join(L))
PY
