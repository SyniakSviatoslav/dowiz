#!/usr/bin/env bash
# cold_twins.sh -- ROADMAP R4 (research 2026-10-07 §12 L2): the honest "source -> result" row.
#
# For every toolchain PRESENT on the box, compile + run the four kernels K1..K4 (same folds as
# bench/vs_rust/kernels/k{1..4}.bp) and one generated ~300 KB straight-line integer program (BIG),
# a NEW process for every compile and every run, pinned to one core, median of R runs.
#
#   usage:  bench/vs_rust/cold_twins.sh [WORKDIR] [TOOLCHAIN ...]      (run from the bebop-lang root)
#   env:    R=5 runs, CORE=4, PROGS="k1 k2 k3 k4 big"
#   toolchains: bebop tcc gcc-O0 gcc-O2 zig-Debug zig-ReleaseFast go rustc-O  (default: all present)
#
# Honesty rules this script enforces (not just documents):
#   * every cell is a measured median or the words NOT PRESENT / FAIL -- never an estimate;
#   * every output is checked against a Python oracle (64-bit wrapping); a mismatch prints FAIL
#     and the script exits 1;
#   * caches: bebop's <out>.dag and <out>.becache are deleted before every compile; every source
#     carries a fresh nonce comment before every compile, so no content-addressed cache (zig's,
#     go's build cache) can hit the program itself. The toolchains' PREBUILT STANDARD LIBRARIES
#     stay warm (go's std in GOCACHE, zig's compiler_rt/std in its global cache, rustc's std
#     rlibs, libc for C) -- that is the toolchain as installed, warmed once outside the timing
#     by compiling a DIFFERENT program. This is stated in the output.
#   * clang on this box is a Termux (Bionic) binary at /data/data/com.termux/...; it is NOT used:
#     a glibc build with it links Android libs (memory termux-pkgconfig-poisons-glibc-builds).
#   * the CPU frequency of CORE (scaling_cur_freq) is read before and after every run and the
#     min/max are printed, so a DVFS move is visible next to the numbers.
set -u
W=${1:-${TMPDIR:-/tmp}/cold_twins}; shift || true
mkdir -p "$W"
exec python3 - "$W" "$@" <<'PY'
import os, sys, time, glob, shutil, subprocess, statistics, random, platform

W = sys.argv[1]; WANT = sys.argv[2:]
R = int(os.environ.get('R', '5')); CORE = os.environ.get('CORE', '4')
PROGS = os.environ.get('PROGS', 'k1 k2 k3 k4 big').split()
FREQ = f'/sys/devices/system/cpu/cpu{CORE}/cpufreq/scaling_cur_freq'
M = (1 << 64) - 1
def s64(v): v &= M; return v - (1 << 64) if v >> 63 else v

# ---------------------------------------------------------------- the generated BIG program
NF, NL = 600, 10            # 600 functions x 10 lines; ~300 KB in bebop syntax
def big_spec():
    rnd = random.Random(20261007); fns = []
    for f in range(NF):
        lines = []
        for j in range(NL):
            p1 = f'a{j-1}' if j >= 1 else 'x'; p2 = f'a{j-2}' if j >= 2 else 'x'
            c1, c2, c3 = rnd.randrange(3, 99991), rnd.randrange(3, 999983), rnd.randrange(3, 9973)
            lines.append((j, p1, c1, p2, c2, c3))      # a_j = p1*c1 + p2*c3 - c2
        fns.append(lines)
    return fns
SPEC = big_spec()
def big_oracle():
    x = 1
    for lines in SPEC:
        env = {'x': x}
        for j, p1, c1, p2, c2, c3 in lines:
            env[f'a{j}'] = s64(env[p1] * c1 + env[p2] * c3 - c2)
        x = env[f'a{NL-1}']
    return str(x)

def big_src(lang):
    out = []
    for f, lines in enumerate(SPEC):
        last = f == NF - 1; ret = f'a{NL-1}' if last else f'q{f+1}(a{NL-1})'
        if lang == 'bp':
            out.append(f'fn q{f}(x: i64) -> i64 {{')
            out += [f'  let a{j} = {p1} * {c1} + {p2} * {c3} - {c2};' for j, p1, c1, p2, c2, c3 in lines]
            out.append(f'  {ret}\n}}')
        elif lang == 'c':
            out.append(f'static u q{f}(u x) {{')
            out += [f'  u a{j} = {p1} * {c1}ULL + {p2} * {c3}ULL - {c2}ULL;' for j, p1, c1, p2, c2, c3 in lines]
            out.append(f'  return {ret};\n}}')
        elif lang == 'rs':
            out.append(f'fn q{f}(x: i64) -> i64 {{')
            out += [f'  let a{j} = {p1}.wrapping_mul({c1}).wrapping_add({p2}.wrapping_mul({c3})).wrapping_sub({c2});' for j, p1, c1, p2, c2, c3 in lines]
            out.append(f'  {ret}\n}}')
        elif lang == 'go':
            out.append(f'func q{f}(x int64) int64 {{')
            out += [f'  a{j} := {p1}*{c1} + {p2}*{c3} - {c2}' for j, p1, c1, p2, c2, c3 in lines]
            out.append(f'  return {ret}\n}}')
        elif lang == 'zig':
            out.append(f'fn q{f}(x: i64) i64 {{')
            out += [f'  const a{j} = {p1} *% {c1} +% {p2} *% {c3} -% {c2};' for j, p1, c1, p2, c2, c3 in lines]
            out.append(f'  return {ret};\n}}')
    body = '\n'.join(out)
    if lang == 'bp':  return body + '\nfn main() -> i64 { q0(1) }\n'
    if lang == 'c':   return ('#include <stdio.h>\ntypedef unsigned long long u;\n' + '\n'.join(
        f'static u q{f}(u x);' for f in range(NF)) + '\n' + body +
        '\nvolatile u seed = 1;\nint main(void){ printf("%lld\\n", (long long)q0(seed)); return 0; }\n')
    if lang == 'rs':  return '#![allow(dead_code)]\n' + body + '\nfn main() { println!("{}", q0(std::hint::black_box(1))); }\n'
    if lang == 'go':  return 'package main\nimport "fmt"\nvar seed int64 = 1\n' + body + '\nfunc main() { fmt.Println(q0(seed)) }\n'
    if lang == 'zig': return ('const std = @import("std");\nvar seed: i64 = 1;\n' + body +
        '\npub fn main() !void { try std.io.getStdOut().writer().print("{d}\\n", .{q0(@as(*volatile i64, &seed).*)}); }\n')

# ---------------------------------------------------------------- the four kernel twins
KER = {
 'c': {
  'k1': 'int main(void){ volatile long long n=1000000; volatile long long s=0; for(long long i=n;i>0;i--) s=s+i; printf("%lld\\n",(long long)s); return 0; }',
  'k2': 'static long long fib(long long n){ return n<2?n:fib(n-1)+fib(n-2); } int main(void){ volatile long long n=25; printf("%lld\\n",fib(n)); return 0; }',
  'k3': 'int main(void){ volatile long long n=300; volatile long long a=0; for(long long x=n;x>0;x--) for(long long y=n;y>0;y--) a=a+x*2+y*3; printf("%lld\\n",(long long)a); return 0; }',
  'k4': 'int main(void){ volatile long long n=2000000; volatile unsigned long long v=1; for(unsigned long long i=n;i>0;i--) v=(v+i*7ULL)*3ULL-11ULL; printf("%lld\\n",(long long)v); return 0; }'},
 'go': {
  'k1': 'var n int64 = 1000000\nfunc main(){ s := int64(0); for i := n; i > 0; i-- { s = s + i }; fmt.Println(s) }',
  'k2': 'var n int64 = 25\nfunc fib(n int64) int64 { if n < 2 { return n }; return fib(n-1) + fib(n-2) }\nfunc main(){ fmt.Println(fib(n)) }',
  'k3': 'var n int64 = 300\nfunc main(){ a := int64(0); for x := n; x > 0; x-- { for y := n; y > 0; y-- { a = a + x*2 + y*3 } }; fmt.Println(a) }',
  'k4': 'var n int64 = 2000000\nfunc main(){ v := int64(1); for i := n; i > 0; i-- { v = (v+i*7)*3 - 11 }; fmt.Println(v) }'},
 'zig': {
  'k1': 'var n: i64 = 1000000;\npub fn main() !void { var s: i64 = 0; var i: i64 = @as(*volatile i64, &n).*; while (i > 0) : (i -= 1) { s = s + i; std.mem.doNotOptimizeAway(s); } try std.io.getStdOut().writer().print("{d}\\n", .{s}); }',
  'k2': 'var n: i64 = 25;\nfn fib(k: i64) i64 { return if (k < 2) k else fib(k - 1) + fib(k - 2); }\npub fn main() !void { try std.io.getStdOut().writer().print("{d}\\n", .{fib(@as(*volatile i64, &n).*)}); }',
  'k3': 'var n: i64 = 300;\npub fn main() !void { var a: i64 = 0; var x: i64 = @as(*volatile i64, &n).*; while (x > 0) : (x -= 1) { var y: i64 = n; while (y > 0) : (y -= 1) { a = a + x * 2 + y * 3; std.mem.doNotOptimizeAway(a); } } try std.io.getStdOut().writer().print("{d}\\n", .{a}); }',
  'k4': 'var n: i64 = 2000000;\npub fn main() !void { var v: i64 = 1; var i: i64 = @as(*volatile i64, &n).*; while (i > 0) : (i -= 1) { v = (v +% i *% 7) *% 3 -% 11; std.mem.doNotOptimizeAway(v); } try std.io.getStdOut().writer().print("{d}\\n", .{v}); }'},
}
EXPECT = {'k1': '500000500000', 'k2': '75025', 'k3': '67725000'}
v = 1
for i in range(2000000, 0, -1): v = s64((v + i * 7) * 3 - 11)
EXPECT['k4'] = str(v)
EXPECT['big'] = big_oracle()

def source(lang, prog):
    if prog == 'big': return big_src(lang)
    if lang == 'bp': return open(f'bench/vs_rust/kernels/{prog}.bp').read()
    if lang == 'rs': return open(f'bench/vs_rust/rust_once/{prog}.rs').read()
    if lang == 'c':  return '#include <stdio.h>\n' + KER['c'][prog] + '\n'
    if lang == 'go': return 'package main\nimport "fmt"\n' + KER['go'][prog] + '\n'
    if lang == 'zig': return 'const std = @import("std");\n' + KER['zig'][prog] + '\n'

# ---------------------------------------------------------------- toolchains
def ver(cmd):
    try: r = subprocess.run(cmd, capture_output=True, text=True); return (r.stdout + r.stderr).strip().splitlines()[0]
    except FileNotFoundError: return None
ENV = dict(os.environ, GOCACHE=f'{W}/gocache', GOTELEMETRY='off', GOTOOLCHAIN='local', GOFLAGS='',
           GO111MODULE='off', ZIG_GLOBAL_CACHE_DIR=f'{W}/zigglobal', ZIG_LOCAL_CACHE_DIR=f'{W}/zigglobal')
TC = {  # name: (lang, ext, version cmd, compile argv builder(src, out, d))
 'bebop':  ('bp', 'bp', None, lambda s, o, d: ['./seed/build/seed', './bebop.bin', 'compile', s, o]),
 'tcc':    ('c', 'c', ['tcc', '-v'], lambda s, o, d: ['tcc', '-o', o, s]),
 'gcc-O0': ('c', 'c', ['gcc', '--version'], lambda s, o, d: ['gcc', '-O0', '-o', o, s]),
 'gcc-O2': ('c', 'c', ['gcc', '--version'], lambda s, o, d: ['gcc', '-O2', '-o', o, s]),
 'zig-Debug':       ('zig', 'zig', ['zig', 'version'], lambda s, o, d: ['zig', 'build-exe', '-ODebug', f'-femit-bin={o}', '--cache-dir', f'{d}/zc', s]),
 'zig-ReleaseFast': ('zig', 'zig', ['zig', 'version'], lambda s, o, d: ['zig', 'build-exe', '-OReleaseFast', f'-femit-bin={o}', '--cache-dir', f'{d}/zc', s]),
 'go':      ('go', 'go', ['go', 'version'], lambda s, o, d: ['go', 'build', '-o', o, s]),
 'rustc-O': ('rs', 'rs', ['rustc', '--version'], lambda s, o, d: ['rustc', '-O', '-o', o, s]),
}
order = WANT or list(TC)
md5 = subprocess.run(['md5sum', 'bebop.bin'], capture_output=True, text=True).stdout[:8]
print(f'# cold_twins.sh  R={R} core={CORE} host={platform.machine()} bebop.bin={md5}  workdir={W}')
print(f'# BIG = generated straight-line integer program (fns q0..q599; zig forbids f80/f128 as names), {NF} fns x {NL} lines; bebop source {len(big_src("bp"))} B, '
      f'C {len(big_src("c"))} B, Rust {len(big_src("rs"))} B, Go {len(big_src("go"))} B, Zig {len(big_src("zig"))} B; oracle {EXPECT["big"]}')
clang = shutil.which('clang')
print(f'# clang: {clang or "absent"} -- {"Termux (Bionic) binary, NOT used for glibc builds" if clang and "com.termux" in os.path.realpath(clang) else "not used"}')
present = []
for t in order:
    lang, ext, vc, _ = TC[t]
    v = 'bebop.bin ' + md5 if t == 'bebop' else (ver(vc) if vc else None)
    if v is None: print(f'# {t}: NOT PRESENT'); continue
    print(f'# {t}: {v}'); present.append(t)

def freq():
    try: return int(open(FREQ).read())
    except Exception: return -1
def timed(argv, cwd=None):
    a = time.perf_counter(); r = subprocess.run(['taskset', '-c', CORE] + argv, capture_output=True, text=True, env=ENV, cwd=cwd)
    return (time.perf_counter() - a) * 1000, r

# warm the prebuilt-std caches with a DIFFERENT program (outside timing)
for t in present:
    lang, ext, _, cc = TC[t]
    if lang not in ('go', 'zig'): continue
    d = f'{W}/warm-{t}'; os.makedirs(d, exist_ok=True)
    src = f'{d}/w.{ext}'
    open(src, 'w').write('package main\nimport "fmt"\nfunc main(){ fmt.Println(7) }\n' if lang == 'go' else
                         'const std = @import("std");\npub fn main() !void { try std.io.getStdOut().writer().print("{d}\\n", .{7}); }\n')
    ms, r = timed(cc(src, f'{d}/w', d))
    print(f'# {time.strftime("%H:%M:%S")} warm {t}: {ms:.0f} ms rc={r.returncode} (prebuilt std cache, not timed below)')
    if r.returncode: print(r.stderr[-2000:])

res = {}; bad = 0; fmin, fmax = 10**12, -1
for rep in range(R):
    for prog in PROGS:
        for t in present:
            lang, ext, _, cc = TC[t]
            d = f'{W}/{t}/{prog}'; os.makedirs(d, exist_ok=True)
            src, out = f'{d}/{prog}.{ext}', f'{d}/{prog}.bin'
            for f in glob.glob(out + '*'): os.remove(f)
            shutil.rmtree(f'{d}/zc', ignore_errors=True)
            nonce = f'{rep}-{random.getrandbits(48):x}'
            open(src, 'w').write(f'// nonce {nonce}\n' + source(lang, prog))
            f0 = freq(); cms, r = timed(cc(src, out, d)); f1 = freq()
            if r.returncode != 0:
                res.setdefault((t, prog), []).append(None); bad += 1
                print(f'# FAIL compile {t} {prog} rc={r.returncode}: {(r.stdout + r.stderr)[-600:]}'); continue
            run = ['./seed/build/seed', out] if t == 'bebop' else [out]
            f2 = freq(); rms, r = timed(run); f3 = freq()
            got = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else f'rc={r.returncode}'
            for f in (f0, f1, f2, f3):
                if f > 0: fmin, fmax = min(fmin, f), max(fmax, f)
            if got != EXPECT[prog]:
                res.setdefault((t, prog), []).append(None); bad += 1
                print(f'# FAIL output {t} {prog}: got {got!r} want {EXPECT[prog]!r} (run rc={r.returncode})'); continue
            res.setdefault((t, prog), []).append((cms, rms))
            print(f'#   {time.strftime("%H:%M:%S")} rep {rep} {t:15s} {prog:3s} compile {cms:8.1f} ms  run {rms:7.1f} ms  rc=0  out ok  freq {f0}/{f1}/{f2}/{f3}', flush=True)

med = lambda v: statistics.median(v)
print(f'\n# core {CORE} scaling_cur_freq over all runs: min {fmin} max {fmax} kHz')
print('| toolchain | ' + ' | '.join(f'{p.upper()} compile / run / TOTAL ms' for p in PROGS) + ' |')
print('|---|' + '---|' * len(PROGS))
tot = {}
for t in present:
    cells = []
    for p in PROGS:
        v = res.get((t, p), [])
        if not v or any(x is None for x in v): cells.append('FAIL'); continue
        c, r_ = med([x[0] for x in v]), med([x[1] for x in v]); tt = med([x[0] + x[1] for x in v]); tot[(t, p)] = tt
        cells.append(f'{c:.0f} / {r_:.0f} / **{tt:.0f}**')
    print(f'| {t} | ' + ' | '.join(cells) + ' |')
if 'bebop' in present:
    print('\n| bebop TOTAL / toolchain TOTAL | ' + ' | '.join(p.upper() for p in PROGS) + ' |')
    print('|---|' + '---|' * len(PROGS))
    for t in present:
        if t == 'bebop': continue
        print(f'| vs {t} | ' + ' | '.join(f'{tot[("bebop", p)] / tot[(t, p)]:.2f}x' if ("bebop", p) in tot and (t, p) in tot else 'n/a' for p in PROGS) + ' |')
print(f'\n# rc: {1 if bad else 0} ({bad} failed cells)')
sys.exit(1 if bad else 0)
PY
