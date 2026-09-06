#!/usr/bin/env python3
"""join_twin.py — B2 decisive twin (i) oracle (docs/blueprints/B2-decisive-twins.md §3(i),
docs/RESEARCH-GRAPHBLAS-2026-09-06.md §1.2/§4). Python stdlib only (registered as std_golden
gate join_twin per the blueprint §6).

Generator is bit-for-bit identical to bench/vs_rust/std_tests/join_twin.bp's gen(): one
continuous LCG stream (x' = x*A+C mod 2^64), R's n rows first then S's n rows, exactly 3 LCG
draws per row regardless of distribution (key, coin, payload) so the stream length never
depends on the zipf flag. uniform: k = (x>>20) % n. zipf: heavy = max(1, n//100) keys (top 1%)
take 30% of the rows (coin = (x>>40) % 1000 < 300); this reproduces "1% of keys carry 30% of
the rows" exactly by construction, in place of a literal Zipf(1.1) rank-frequency curve that
would need floats (bebop has none — see bench/vs_rust/B2-PREP.md for why this is the chosen
interpretation). Payload a,b in [0, 65536).

checksum = sum over matching pairs of ((a*b) mod 2^61); fold = count*1000000007 + checksum.
This module is both the CLI oracle (prints count/checksum/fold for one distribution) and a
library (gen(), join_fold()) imported by bench/tq_sqlite/join_sqlite.py so the sqlite twin
loads the SAME generated rows rather than re-deriving them.
"""
import sys

M64 = (1 << 64) - 1
A = 6364136223846793005
C = 1442695040888963407
MOD61 = 1 << 61


def lcg(x):
    return (x * A + C) & M64


def gen(seed, n, zipf):
    """Returns (Rk, Ra, Sk, Sb), each a list of n ints — identical stream/order to the .bp twin."""
    heavy = max(1, n // 100)
    light = n - heavy
    x = seed
    rk = [0] * n
    ra = [0] * n
    sk = [0] * n
    sb = [0] * n
    for i in range(n):
        x = lcg(x)
        k0 = (x >> 20) % n
        coin = (x >> 40) % 1000
        x = lcg(x)
        kz = (x >> 20) % heavy if coin < 300 else heavy + (x >> 20) % light
        k = kz if zipf else k0
        x = lcg(x)
        rk[i] = k
        ra[i] = (x >> 20) % 65536
    for i in range(n):
        x = lcg(x)
        k0 = (x >> 20) % n
        coin = (x >> 40) % 1000
        x = lcg(x)
        kz = (x >> 20) % heavy if coin < 300 else heavy + (x >> 20) % light
        k = kz if zipf else k0
        x = lcg(x)
        sk[i] = k
        sb[i] = (x >> 20) % 65536
    return rk, ra, sk, sb


def join_fold(rk, ra, sk, sb):
    """count of matching (r,s) pairs and checksum = sum((a*b) mod 2^61) over pairs, plus the
    combined fold. Groups S by key first (dict of lists) — same Gustavson-row semantics as the
    CSR probe, just via a python dict instead of rp/ci arrays."""
    n = len(sk)
    buckets = {}
    for s in range(n):
        buckets.setdefault(sk[s], []).append(s)
    cnt = 0
    chk = 0
    for r in range(len(rk)):
        k = rk[r]
        a = ra[r]
        for s in buckets.get(k, ()):
            p = (a * sb[s]) % MOD61
            cnt += 1
            chk += p
    fold = cnt * 1000000007 + chk
    return cnt, chk, fold


def run(n, dist):
    zipf = 1 if dist == 'z' else 0
    seed = 8823 if zipf else 4711
    rk, ra, sk, sb = gen(seed, n, zipf)
    return join_fold(rk, ra, sk, sb)


if __name__ == '__main__':
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 2000
    dist = sys.argv[2] if len(sys.argv) > 2 else 'u'
    dists = ['u', 'z'] if dist == 'both' else [dist]
    for d in dists:
        cnt, chk, fold = run(n, d)
        print(f'{d} count {cnt}')
        print(f'{d} checksum {chk}')
        print(f'{d} fold {fold}')
