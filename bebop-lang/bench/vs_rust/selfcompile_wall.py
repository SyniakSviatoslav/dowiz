#!/usr/bin/env python3
"""selfcompile_wall.py (W-R7, R-GRAPH B3, 2026-10-11): the cold self-compile wall at W=0 vs W=<w>.

  python3 bench/vs_rust/selfcompile_wall.py [--bin B] [--src bebop.bp] [--r 5] [--w 3] [--ratio 0.5]

What it measures: `seed <bin> compile <src> <tmp>/x.bin` from COLD (x.bin, .becache, .use and .dag
removed before every run, so no memo replay and no cache hit), R runs per W, INTERLEAVED (W=0, W=w,
W=0, ...) after one warmup each, inside whatever cores the caller has (run it under tools/slot.sh:
cores 4,5,6). Unlike tools/perf.py selfcompile this does NOT narrow to one core -- a parallel compile
needs the slot's three A78s. W reaches the compiler as BEBOP_CC_W=<w> in its environment
(compiler/dagpar.bp reads /proc/self/environ); W=0 is the serial emit.

Prints, per W: every wall, median, min, max, spread (max-min) in ms; the md5 of every output; the
pinned cores' scaling_cur_freq before and after. Verdict line:

  selfcompile_wall: W=0 <m0> ms  W=<w> <mw> ms  ratio <mw/m0>  identical <k>/<n>  PASS|FAIL

PASS needs (a) ratio <= --ratio AND (b) every output of every run at every W byte-identical (T-1) and
(c) every compile rc=0. A failed compile is FAIL with its rc named -- never a wall.
"""
import hashlib, os, statistics, subprocess, sys, threading, time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.chdir(ROOT)


def opt(a, k, d):
    return a[a.index(k) + 1] if k in a else d


def freqs():
    out = []
    for c in sorted(os.sched_getaffinity(0)):
        try:
            out.append(f"cpu{c}={int(open(f'/sys/devices/system/cpu/cpu{c}/cpufreq/scaling_cur_freq').read()) // 1000}MHz")
        except (OSError, ValueError):
            out.append(f"cpu{c}=?")
    return " ".join(out)


class Sampler(threading.Thread):
    """scaling_cur_freq of every allowed core every 50 ms while the runs go (an idle read after the
    runs says only what the governor dropped to -- measured 960 MHz idle vs 1651 MHz busy, 2026-10-11)."""
    def __init__(self):
        super().__init__(daemon=True); self.stop = False; self.s = {c: [] for c in sorted(os.sched_getaffinity(0))}
    def run(self):
        while not self.stop:
            for c in self.s:
                try: self.s[c].append(int(open(f"/sys/devices/system/cpu/cpu{c}/cpufreq/scaling_cur_freq").read()) // 1000)
                except (OSError, ValueError): pass
            time.sleep(0.05)
    def line(self):
        return " ".join(f"cpu{c}={statistics.median(v) if v else '?'}MHz(min {min(v) if v else '?'})" for c, v in self.s.items())


def one(seed, b, src, tmp, w):
    xb = f"{tmp}/x.bin"
    for f in (xb, xb + ".becache", xb + ".use", xb + ".dag"):
        try:
            os.remove(f)
        except FileNotFoundError:
            pass
    env = dict(os.environ, BEBOP_CC_W=str(w))
    t0 = time.perf_counter()
    p = subprocess.run([seed, b, "compile", src, xb], env=env, capture_output=True)
    wall = (time.perf_counter() - t0) * 1000.0
    if p.returncode != 0 or not os.path.exists(xb):
        return wall, p.returncode, "rc=%d:%s" % (p.returncode, p.stderr.decode(errors="replace").strip()[-200:])
    return wall, 0, hashlib.md5(open(xb, "rb").read()).hexdigest()[:8]


def main(a):
    b = opt(a, "--bin", "./bebop.bin")
    src = opt(a, "--src", "bebop.bp")
    r = int(opt(a, "--r", "5"))
    w = int(opt(a, "--w", "3"))
    ratio = float(opt(a, "--ratio", "0.5"))
    seed = "./seed/build/seed"
    tmp = os.environ.get("BEBOP_TMP", "/tmp/opencode") + "/scwall"
    os.makedirs(tmp, exist_ok=True)
    print(f"selfcompile_wall: bin {b} ({hashlib.md5(open(b, 'rb').read()).hexdigest()[:8]}) src {src} R={r} W=0 vs W={w} cpus {sorted(os.sched_getaffinity(0))}")
    print(f"  freq before: {freqs()}")
    ws = [0, w]
    res = {x: [] for x in ws}
    bad = []
    for _ in range(2):
        for x in ws:
            one(seed, b, src, tmp, x)  # warmup x2: page cache + the walt governor's ramp (2086 -> 1081 -> 716 ms measured)
    smp = Sampler(); smp.start()
    for _ in range(r):
        for x in ws:
            res[x].append(one(seed, b, src, tmp, x))
    smp.stop = True; smp.join()
    print(f"  freq during (median of 50 ms samples): {smp.line()}")
    med = {}
    for x in ws:
        walls = [round(v[0]) for v in res[x]]
        med[x] = statistics.median(walls)
        bad += [f"W={x} {v[2]}" for v in res[x] if v[1] != 0]
        print(f"  W={x}: walls {walls} median {med[x]:.0f} min {min(walls)} max {max(walls)} spread {max(walls) - min(walls)} ms"
              f"  md5 {sorted(set(v[2] for v in res[x]))}")
    digests = [v[2] for x in ws for v in res[x]]
    n = len(digests)
    k = digests.count(digests[0]) if all(v[1] == 0 for x in ws for v in res[x]) else 0
    rt = med[w] / med[0] if med[0] else 0.0
    ok = (not bad) and k == n and rt <= ratio
    for m in bad:
        print(f"  COMPILEFAIL {m}")
    print(f"selfcompile_wall: W=0 {med[0]:.0f} ms  W={w} {med[w]:.0f} ms  ratio {rt:.3f}  identical {k}/{n}  {'PASS' if ok else 'FAIL'}"
          f" (mark ratio <= {ratio})")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
