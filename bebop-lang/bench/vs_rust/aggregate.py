#!/usr/bin/env python3
"""Aggregate run_bench.sh results into a comparison table."""
import glob, os, statistics

def pct(xs, p):
    xs = sorted(xs)
    k = max(0, min(len(xs)-1, int(round(p/100.0*(len(xs)-1)))))
    return xs[k]

def parse_ns(path):
    for line in open(path):
        if line.startswith("ns"):
            return [int(x) for x in line.split()[1:]]
    return []

rows = []
for k in ["1","2","3","4"]:
    r = {"k":k}
    for impl in ["bebop","c","rust"]:
        ns = parse_ns(f"results/k{k}.{impl}.txt")
        if not ns: continue
        med = statistics.median(ns)
        r[impl] = dict(
            med=med, p95=pct(ns,95), p99=pct(ns,99), mn=min(ns), mx=max(ns),
            sd=statistics.stdev(ns) if len(ns)>1 else 0)
    rows.append(r)

names = {"1":"K1 sum-loop 1M","2":"K2 fib(25) rec","3":"K3 nested 300x300","4":"K4 arith-chain 2M"}
print("%-20s %10s | %11s %11s %9s" % ("kernel","impl","median","p95","stdev"))
for r in rows:
    for impl in ["bebop","c","rust"]:
        if impl in r:
            d=r[impl]
            print("%-20s %10s | %8.1f us %8.1f us %7.1f" % (
                names[r["k"]], impl, d["med"]/1000, d["p95"]/1000, d["sd"]))
    if "bebop" in r and "c" in r and r["c"]["med"]:
        print("%-20s %10s   bebop/C = %.1fx  bebop/rust = %.1fx" % (
            "", "", r["bebop"]["med"]/r["c"]["med"], r["bebop"]["med"]/r["rust"]["med"]))
    print()

print("== startup (per process spawn+run+exit, 50 runs) ==")
for line in open("results/startup.txt"):
    n,v = line.split(); print("  %-18s %6.1f ms" % (n, int(v)/1e6))

print("\n== peak RSS ==")
for tag in ["bebop","c","rust"]:
    line = open(f"results/rss_{tag}.txt").read().strip()
    kv = dict(x.split("=") for x in line.split())
    print("  %-6s %6s KB  wall %.1f ms" % (tag, kv.get("RSS_KB"), float(kv.get("WALL_MS",0))))

print("\n== binary/artifact sizes ==")
for line in open("results/sizes.txt"):
    n,s = line.split(); print("  %-24s %8d B" % (n, int(s)))

print("\n== compile throughput (source KB/s) ==")
for line in open("results/compile_throughput.txt"):
    parts = line.split()
    if len(parts) >= 4:
        tool, kern, ns, nbytes = parts[0], parts[1], int(parts[2]), int(parts[3])
        kbps = nbytes/1024.0/(ns/1e9)
        print("  %-8s %-12s %6.1f KB/s (%4.0f ms)" % (tool, kern, kbps, ns/1e6))
