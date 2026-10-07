#!/usr/bin/env python3 -I
# Oracle for the R-SPIKE probes: the loop semantics in plain Python over Z/2^64, carried across reps
# exactly as the C probes carry them. Prints "ORACLE <name> <u64>".
import sys
M = (1 << 64) - 1
def k1h(reps=20, n=1000000):
    s = 0
    for _ in range(reps):
        for i in range(n, 0, -1): s = (s * 3 + i) & M
    return s
def k4(reps=10, n=2000000):
    v = 1
    for _ in range(reps):
        for i in range(n, 0, -1): v = ((v + i * 7) * 3 - 11) & M
    return v
def k3h(reps=200, n=300):
    a = 0
    for _ in range(reps):
        for x in range(n, 0, -1):
            for y in range(n, 0, -1): a = (a * 3 + x * 2 + y * 3) & M
    return a
def k8h_one(n=20000):
    acc = 0; x = 1
    for i in range(n, 0, -1):
        x = (x * 6364136223846793005 + 1442695040888963407) & M
        bit = (x >> 60) & 1
        acc = (acc + x) & M if bit else (acc - i) & M
    return acc
def k1h_arr_full(n=1000000):
    # the 1%-changed array of k1h.c: same LCG, same index/value rule
    a = [n - j for j in range(n)]
    k = n // 100; rng = 12345
    for t in range(k):
        rng = (rng * 6364136223846793005 + 1442695040888963407) & M
        idx = (rng >> 33) % n; dnew = (a[idx] + 1 + (rng >> 40) % 7) & M
        # k1h.c computes dold/dnew in one pass from the UNCHANGED array, then applies sequentially;
        # a duplicate index therefore ends with the LAST dnew computed from the ORIGINAL value. Mirror that:
        pass
    a = [n - j for j in range(n)]; rng = 12345; idxs = []; news = []
    for t in range(k):
        rng = (rng * 6364136223846793005 + 1442695040888963407) & M
        idx = (rng >> 33) % n; idxs.append(idx); news.append((a[idx] + 1 + (rng >> 40) % 7) & M)
    for idx, nv in zip(idxs, news): a[idx] = nv
    s = 0
    for j in range(n): s = (s * 3 + a[j]) & M
    return s
def sum_base(log=20):
    n = 1 << log; rng = 99; s = 0
    for _ in range(n):
        rng = (rng * 6364136223846793005 + 1442695040888963407) & M; s += rng >> 20
    return s & M
if __name__ == "__main__":
    which = sys.argv[1:] or ["k1h", "k4", "k3h", "k8h_one", "fib", "k1h_arr_full", "sum_base"]
    for w in which:
        if w == "fib": print("ORACLE k2h 75025"); continue
        print("ORACLE", w, globals()[w]())
        sys.stdout.flush()
