#!/usr/bin/env python3
"""perf.py (D12-A, 2026-09-06): the evals substrate. One CSV (bench/perf.csv), one reader.

  tools/perf.py stamp                                  E7  validity stamp (json)
  tools/perf.py selfcompile [--bin B] [--base A] [--n 5]  E1/E5/E6 wall, utime, stime, maxrss, energy proxy
  tools/perf.py size [--bin B]                         E3  bebop.bin words, stub words, per-fn words (+ budget gate)
  tools/perf.py kernels [--bin B] [--base A] [--r 11]  E2  K1H/K2H/K3H/K4 ms med/p95 interleaved A/B + loop words
  tools/perf.py constructs [--bin B]                   E13 per-construct frozen words, a row only on change
  tools/perf.py fuzz [--bin B]                         E8  seeds / rate / traps per promoted binary (journal reader)
  tools/perf.py record <metric> <value> <unit> [note]  E4/E9/E14 hook for scripts
  tools/perf.py report [--last 12]                     E11 docs/PERF.md; exit 1 on any alert
  tools/perf.py run [--bin B] [--base A]               all of the above (chain.sh calls this)

Rules (docs/EVALS-RESEARCH-2026-09-06.md §3): exact counts gate against the previous row
(growth needs a `<name> <words> <reason>` line in bench/parity_constructs/word_budget.txt);
ms/bytes gate = median of N vs the previous VALID row of another binary, alert when the
increase is > T % AND > 3 MAD of the last 10 valid rows; an invalid row (E7) never alerts.
Energy is a PROXY: A78 core-seconds weighted by freq/fmax on the pinned core (no RAPL, no
power_supply under proot); the column says so.
"""
import csv, hashlib, json, os, re, statistics, subprocess, sys, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
os.chdir(ROOT)
sys.path.insert(0, "tools")
from check_abi import load_bin, fn_starts, entry_stub  # noqa: E402

CSV = "bench/perf.csv"
COLS = ["ts", "commit", "bin", "metric", "value", "unit", "n", "valid", "at_max_pct", "temp_mc", "note"]
PIN = os.environ.get("PIN", "4")           # one A78 core (cpu part 0xd41: 4-7); chain.sh uses 4-6
SEED = "./seed/build/seed"
T = os.environ.get("BEBOP_TMP", "/tmp/opencode") + "/perf"
THRESH = {"selfcompile_wall": 5, "selfcompile_utime": 5, "selfcompile_stime": 15, "selfcompile_maxrss": 10,
          "selfcompile_energy": 5, "k1h_ms": 3, "k2h_ms": 3, "k3h_ms": 3, "k4_ms": 3, "k8h_ms": 3}
EXACT = {"bin_words": "bebop", "stub_words": "stub", "k1h_loopwords": "k1h", "k2h_loopwords": "k2h",
         "k3h_loopwords": "k3h", "k4_loopwords": "k4", "k8h_loopwords": "k8h"}
KERNELS = ["k1h", "k2h", "k3h", "k4", "k8h"]


# ---------- E7: validity ----------
def _read(p, d=""):
    try: return open(p).read().strip()
    except OSError: return d

def temp_max():
    best = 0
    for z in os.listdir("/sys/class/thermal"):
        if not z.startswith("thermal_zone"): continue
        if _read(f"/sys/class/thermal/{z}/type").startswith("cpu"):
            try: best = max(best, int(_read(f"/sys/class/thermal/{z}/temp", "0")))
            except ValueError: pass
    return best

def tis():
    """cpufreq time_in_state of the pinned core -> {khz: ticks}."""
    out = {}
    for line in _read(f"/sys/devices/system/cpu/cpu{PIN}/cpufreq/stats/time_in_state").splitlines():
        f, t = line.split(); out[int(f)] = int(t)
    return out

def fmax():
    return int(_read(f"/sys/devices/system/cpu/cpu{PIN}/cpufreq/cpuinfo_max_freq", "1") or 1)

def _ancestors():
    out, pid = set(), os.getpid()
    while pid > 1:
        out.add(pid)
        try: pid = int(open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()[1])
        except (OSError, IndexError, ValueError): break
    return out

def others_running():
    """battery lanes running that are NOT this process's own chain (perf.py runs at the end of chain.sh)."""
    r = subprocess.run(["pgrep", "-f", "chain[.]sh|battery[.]sh|std_golden[.]sh|invariants[.]sh|std_par[.]sh"], capture_output=True, text=True)
    anc = _ancestors()
    return len([p for p in r.stdout.split() if int(p) not in anc])

def procs():
    return len(os.listdir("/proc")) and sum(1 for p in os.listdir("/proc") if p.isdigit())

class Window:
    """E7: freq/thermal/load stamp around a measurement. valid = no throttling, no concurrent battery."""
    def __enter__(self):
        self.t0, self.temp0, self.busy0, self.procs0 = tis(), temp_max(), others_running(), procs()
        return self
    def __exit__(self, *a):
        t1 = tis(); fm = fmax()
        d = {f: t1.get(f, 0) - self.t0.get(f, 0) for f in t1}
        tot = sum(d.values()) or 1
        self.at_max_pct = round(100.0 * sum(t for f, t in d.items() if f >= 0.9 * fm) / tot, 1)
        self.energy = sum(t * f / fm for f, t in d.items()) / 100.0   # core-seconds at fmax-equivalent (ticks = 10 ms)
        self.temp = max(self.temp0, temp_max())
        self.busy = max(self.busy0, others_running())
        self.valid = int(self.temp < 60000 and self.at_max_pct >= 80 and self.busy == 0)
    def stamp(self):
        return {"valid": self.valid, "at_max_pct": self.at_max_pct, "temp_mc": self.temp, "busy": self.busy, "procs": self.procs0}


# ---------- measurement ----------
def run1(argv):
    """-> (wall_ms, utime_s, stime_s, maxrss_kb, last stdout line)."""
    t0 = time.perf_counter()
    p = subprocess.Popen(["taskset", "-c", PIN] + argv, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    out = p.stdout.read(); _, st, ru = os.wait4(p.pid, 0)
    wall = (time.perf_counter() - t0) * 1000.0
    last = out.decode(errors="replace").strip().split("\n")[-1] if out else ""
    return wall, ru.ru_utime, ru.ru_stime, ru.ru_maxrss, last

def med(v):
    v = sorted(v); return v[len(v) // 2], v[min(len(v) - 1, int(round(0.95 * (len(v) - 1))))]

def md5(path):
    return hashlib.md5(open(path, "rb").read()).hexdigest()[:8]

def commit():
    return subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()

def rows():
    if not os.path.exists(CSV): return []
    return list(csv.DictReader(open(CSV)))

def record(binpath, metric, value, unit, n, st, note=""):
    new = not os.path.exists(CSV)
    with open(CSV, "a", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        if new: w.writerow(COLS)
        w.writerow([int(time.time()), commit(), md5(binpath), metric, value, unit, n, st.get("valid", 1),
                    st.get("at_max_pct", ""), st.get("temp_mc", ""), note])

def budget_ok(name, words):
    return any(l.split()[:2] == [name, str(words)] for l in open("bench/parity_constructs/word_budget.txt") if l.strip() and not l.startswith("#"))

def check(metric, value, binm, hist):
    """-> (alert:bool, text). hist = rows before this run."""
    if metric == "fuzz_trap82":  # D12-C: 0 tolerated, any count on the current binary alerts
        v = int(float(value))
        return v > 0, f"{v} TRAP-82 (SIGSEGV/SIGBUS) on {binm}, 0 tolerated"
    prev = [r for r in hist if r["metric"] == metric and r["valid"] == "1"]
    if metric in EXACT:
        last = [r for r in prev if r["bin"] != binm] or prev
        if not last: return False, "first"
        try: old = int(float(last[-1]["value"])); new = int(float(value))
        except ValueError: return False, "non-numeric"
        if new > old and not budget_ok(EXACT[metric], new):
            return True, f"{old} -> {new} (+{new - old}) without a `{EXACT[metric]} {new} <reason>` line in word_budget.txt"
        return False, f"{old} -> {new}"
    base = [r for r in prev if r["bin"] != binm]
    if not base: return False, "first"
    try: old = float(base[-1]["value"]); v = float(value); tail = [float(r["value"]) for r in prev[-10:]]
    except ValueError: return False, "non-numeric"
    mad = statistics.median([abs(x - statistics.median(tail)) for x in tail]) if len(tail) >= 3 else 0.0
    pct = 100.0 * (v - old) / old if old else 0.0
    t = THRESH.get(metric, 5)
    alert = pct > t and (v - old) > 3 * mad
    return alert, f"{old:.4g} -> {v:.4g} ({pct:+.1f} %, MAD {mad:.3g})"


# ---------- E1/E5/E6 ----------
def selfcompile(binpath, base=None, n=5):
    os.makedirs(T, exist_ok=True)
    def one(b):
        for f in (f"{T}/x.bin", f"{T}/x.bin.becache", f"{T}/x.bin.use"):
            try: os.remove(f)
            except FileNotFoundError: pass
        return run1([SEED, b, "compile", "bebop.bp", f"{T}/x.bin"])
    bins = [binpath] + ([base] if base and md5(base) != md5(binpath) else [])
    res = {b: [] for b in bins}
    with Window() as w:
        for b in bins: one(b)                       # warmup
        for _ in range(n):
            for b in bins: res[b].append(one(b))    # interleaved A/B
        e_total = None
    st = w.stamp()
    out = {}
    for b in bins:
        r = res[b]
        out[b] = {"selfcompile_wall": med([x[0] for x in r])[0], "selfcompile_utime": med([x[1] for x in r])[0],
                  "selfcompile_stime": med([x[2] for x in r])[0], "selfcompile_maxrss": max(x[3] for x in r)}
    # energy proxy: the window's fmax-equivalent core-seconds split by each bin's share of cpu time
    cpu = {b: sum(x[1] + x[2] for x in res[b]) for b in bins}; tot = sum(cpu.values()) or 1
    for b in bins: out[b]["selfcompile_energy"] = w.energy * cpu[b] / tot / n
    units = {"selfcompile_wall": "ms", "selfcompile_utime": "s", "selfcompile_stime": "s", "selfcompile_maxrss": "kB", "selfcompile_energy": "core-s@fmax (proxy)"}
    for k, v in out[binpath].items():
        note = f"base {md5(base)} {out[base][k]:.4g}" if base in out else ""
        record(binpath, k, round(v, 4), units[k], n, st, note)
    return out, st


# ---------- E3 ----------
def size(binpath):
    W, entry, end = load_bin(binpath)
    starts = fn_starts(W, end)
    stub = len(entry_stub("bebop.bp"))   # T118b/T90: the stub words of THIS source (131 since T90 2c)
    names = re.findall(r"^fn (\w+)", open("bebop.bp").read(), re.M)
    if any(l.startswith('use "') for l in open("bebop.bp")):  # prelude fns come first in the stream
        for u in re.findall(r'^use "([^"]+)"', open("bebop.bp").read(), re.M):
            try: names = re.findall(r"^fn (\w+)", open(u).read(), re.M) + names
            except OSError: pass
    fnw = {}
    for i, s in enumerate(starts):
        nxt = starts[i + 1] if i + 1 < len(starts) else (entry if entry > s else end)
        fnw[names[i] if i < len(names) and len(names) == len(starts) else f"fn{i}"] = nxt - s
    st = {"valid": 1}
    record(binpath, "bin_words", end, "words", 1, st, f"bytes {os.path.getsize(binpath)}")
    record(binpath, "stub_words", stub, "words", 1, st)
    record(binpath, "bin_fns", len(starts), "fns", 1, st)
    os.makedirs("bench/perf_fn", exist_ok=True)
    with open("bench/perf_fn/latest.txt", "w") as f:
        for k, v in fnw.items(): f.write(f"{k} {v}\n")
    return {"bin_words": end, "stub_words": stub, "bin_fns": len(starts)}, fnw


# ---------- E2 ----------
def loop_words(binpath):
    d = subprocess.run(["objdump", "-D", "-b", "binary", "-m", "aarch64", binpath], capture_output=True, text=True).stdout
    loops = []
    for m in re.finditer(r"^\s*([0-9a-f]+):\s+[0-9a-f]{8}\s+(b|b\.\w+|cbz|cbnz|tbz|tbnz)\s+.*?0x([0-9a-f]+)\s*$", d, re.M):
        a, t = int(m.group(1), 16), int(m.group(3), 16)
        if t <= a: loops.append((a - t) // 4 + 1)
    return min(loops) if loops else 0   # the innermost loop

def kernels(binpath, base=None, r=11):
    os.makedirs(T, exist_ok=True)
    bins = [binpath] + ([base] if base and md5(base) != md5(binpath) else [])
    for b in bins:
        for k in KERNELS:
            subprocess.run([SEED, b, "compile", f"bench/vs_rust/bench630/{k}t.bp", f"{T}/{md5(b)}_{k}.bin"], capture_output=True)
            subprocess.run([SEED, b, "compile", f"bench/vs_rust/kernels/{k}.bp", f"{T}/{md5(b)}_{k}_plain.bin"], capture_output=True)
    res = {(b, k): [] for b in bins for k in KERNELS}
    with Window() as w:
        for b in bins:
            for k in KERNELS: run1([SEED, f"{T}/{md5(b)}_{k}.bin"])
        for _ in range(r):
            for k in KERNELS:
                for b in bins:
                    v = run1([SEED, f"{T}/{md5(b)}_{k}.bin"])[4]
                    res[(b, k)].append(int(v) / 100.0)   # TOTAL ms over REPS=100
    st = w.stamp()
    out = {}
    for k in KERNELS:
        m, p95 = med(res[(binpath, k)])
        lw = loop_words(f"{T}/{md5(binpath)}_{k}_plain.bin")
        note = ""
        if base in bins:
            bm, bp = med(res[(base, k)]); note = f"base {md5(base)} {bm:.2f}/{bp:.2f} loop {loop_words(f'{T}/{md5(base)}_{k}_plain.bin')}"
        record(binpath, f"{k}_ms", round(m, 3), "ms/rep", r, st, f"p95 {p95:.3f} {note}")
        record(binpath, f"{k}_loopwords", lw, "words", 1, {"valid": 1}, note)
        out[k] = (m, p95, lw)
    return out, st


# ---------- E13: per-construct / per-kernel words, rows only on change ----------
def constructs(binpath):
    hist = rows(); changed = 0
    for f in sorted(os.listdir("bench/parity_constructs/frozen")):
        if not f.endswith(".bin"): continue
        name = f[:-4]
        try: _, _, end = load_bin(f"bench/parity_constructs/frozen/{f}")
        except ValueError: continue
        prev = [r for r in hist if r["metric"] == f"cw:{name}"]
        if not prev or int(float(prev[-1]["value"])) != end:
            record(binpath, f"cw:{name}", end, "words", 1, {"valid": 1}, f"was {prev[-1]['value']}" if prev else "first"); changed += 1
    return changed

# ---------- E8: fuzz throughput / coverage per promoted binary (docs/exp.journal reader) ----------
def fuzz(binpath):
    per = {}
    for line in open(os.environ.get("EXP_JOURNAL", "docs/exp.journal")):
        if "H:fuzzd batch" not in line and "H:fuzz batch" not in line: continue
        # TRAP-81=/TRAP-82= are D12-C fields; older journal lines lack them (treated as 0, never alerted on).
        m = re.search(r"GOT:N=(\d+) .*?DIVERGE=(\d+) COMPILEFAIL=(\d+) CRASH=(\d+).*?TRAP-UNPREDICTED=(\d+)(?: TRAP-81=(\d+) TRAP-82=(\d+))?.*?rate=([0-9.]+)/s bin=(\w+)", line)
        if not m: continue
        n, dv, cf, cr, tu, t81, t82, rate, b = m.groups()
        d = per.setdefault(b, {"seeds": 0, "bad": 0, "trap": 0, "trap82": 0, "rates": []})
        d["seeds"] += int(n); d["bad"] += int(dv) + int(cf) + int(cr); d["trap"] += int(tu)
        d["trap82"] += int(t82) if t82 is not None else 0
        d["rates"].append(float(rate))
    cur = md5(binpath); d = per.get(cur, {"seeds": 0, "bad": 0, "trap": 0, "trap82": 0, "rates": [0.0]})
    st = {"valid": 1}
    record(binpath, "fuzz_seeds_on_bin", d["seeds"], "seeds", len(d["rates"]), st, f"TG-DONE 8: {d['seeds']} on {cur}; total {sum(x['seeds'] for x in per.values())} over {len(per)} bins")
    record(binpath, "fuzz_rate", round(statistics.median(d["rates"]), 2), "prog/s", len(d["rates"]), st, "median over the bin's batches (LITTLE cores, nice)")
    record(binpath, "fuzz_trap_unpredicted", d["trap"], "count", 1, st, "TRAP-80/81/82 total; 81 by design, 82 is fuzz_trap82 (ALERT)")
    record(binpath, "fuzz_trap82", d["trap82"], "count", 1, st, "D12-C: SIGSEGV/SIGBUS, 0 tolerated, repros in $REPROS")
    return per

# ---------- E11 ----------
def report(last=12):
    R = rows()
    if not R: print("perf.csv empty"); return 0
    metrics = []
    for r in R:
        if r["metric"] not in metrics: metrics.append(r["metric"])
    runs = []   # distinct (commit, bin) in order
    for r in R:
        key = (r["commit"], r["bin"])
        if key not in runs: runs.append(key)
    runs = runs[-last:]
    alerts = []
    L = ["# PERF — per-commit evals (tools/perf.py, D12-A; generated, do not edit)", "",
         f"Status: {time.strftime('%Y-%m-%d')} CURRENT (last {len(runs)} runs; `!` = alert: > T % and > 3 MAD vs the previous valid row of another binary; `?` = invalid window: throttled / busy box; exact counts gate with word_budget.txt)", "",
         "| metric | unit | " + " | ".join(f"{c}/{b}" for c, b in runs) + " | last delta |", "|---|---|" + "---|" * len(runs) + "---|"]
    for m in metrics:
        cells = []; unit = ""
        for c, b in runs:
            rr = [r for r in R if r["metric"] == m and (r["commit"], r["bin"]) == (c, b)]
            if not rr: cells.append(""); continue
            r = rr[-1]; unit = r["unit"]
            v = r["value"]
            try: v = f"{float(v):.4g}" if "." in v else v
            except ValueError: pass
            cells.append(v + ("" if r["valid"] == "1" else " ?"))
        latest = [r for r in R if r["metric"] == m][-1]
        hist = [r for r in R if r["metric"] == m and int(r["ts"]) < int(latest["ts"])]
        alert, txt = check(m, latest["value"], latest["bin"], hist) if latest["valid"] == "1" else (False, "invalid window")
        if alert: alerts.append(f"{m}: {txt}")
        L.append(f"| {m} | {unit} | " + " | ".join(cells) + f" | {'! ' if alert else ''}{txt} |")
    L += ["", "Energy is a proxy (A78 core-seconds weighted by freq/fmax on the pinned core, no RAPL/power_supply under proot).",
          "Per-fn words of the latest binary: bench/perf_fn/latest.txt (diff it against git for the growth per fn).", ""]
    if alerts: L += ["## ALERTS", ""] + [f"- {a}" for a in alerts] + [""]
    open("docs/PERF.md", "w").write("\n".join(L))
    for a in alerts: print("PERF ALERT", a)
    print(f"perf: {len(metrics)} metrics, {len(runs)} runs, {len(alerts)} alerts -> docs/PERF.md")
    return 1 if alerts else 0


def main(a):
    cmd = a[0] if a else "run"
    def opt(k, d=None):
        return a[a.index(k) + 1] if k in a else d
    b = opt("--bin", "./bebop.bin"); base = opt("--base")
    for k in ("--bin", "--base", "--n", "--r", "--last"):
        if k in a: i = a.index(k); del a[i:i + 2]
    if cmd == "stamp":
        with Window() as w: time.sleep(0.2)
        print(json.dumps(w.stamp())); return 0
    if cmd == "selfcompile":
        out, st = selfcompile(b, base, int(opt("--n", 5)))
        for k, v in out[b].items(): print(f"{k} {v:.4g}" + (f"  (base {out[base][k]:.4g})" if base in out else ""))
        print("stamp", json.dumps(st)); return 0
    if cmd == "size":
        s, fnw = size(b); print(s); print("top fns:", sorted(fnw.items(), key=lambda x: -x[1])[:5]); return 0
    if cmd == "kernels":
        out, st = kernels(b, base, int(opt("--r", 11)))
        for k, (m, p, lw) in out.items(): print(f"{k} {m:.2f} / {p:.2f} ms  loop {lw} words")
        print("stamp", json.dumps(st)); return 0
    if cmd == "report":
        return report(int(opt("--last", 12)))
    if cmd == "constructs":
        print("construct rows changed:", constructs(b)); return 0
    if cmd == "fuzz":
        per = fuzz(b)
        for k, v in sorted(per.items(), key=lambda x: -x[1]["seeds"])[:6]: print(k, v["seeds"], "seeds", v["bad"], "bad", v["trap"], "trap-unpredicted", v.get("trap82", 0), "trap82")
        return 0
    if cmd == "record":   # scripts (E4/E9/E14): perf.py record <metric> <value> <unit> [note]
        record(b, a[1], a[2], a[3], 1, {"valid": 1}, a[4] if len(a) > 4 else ""); return 0
    if cmd == "run":
        size(b); constructs(b); fuzz(b); selfcompile(b, base, int(opt("--n", 5))); kernels(b, base, int(opt("--r", 11)))
        return report(int(opt("--last", 12)))
    print(__doc__); return 2

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
