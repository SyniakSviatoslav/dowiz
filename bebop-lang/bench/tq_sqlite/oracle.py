#!/usr/bin/env python3
"""T100 (D1(c)) — the tensor-query-vs-sqlite gate, data + truth + sqlite timings.

Data: N points (u,v) as i64 Q32-range values from the 64-bit LCG
  x = x*6364136223846793005 + 1442695040888963407 (mod 2^64), seed 12345,
  u = (x >> 33) - 2^30, v = (x' >> 33) - 2^30  (x' = next state), so |u|,|v| < 2^30.
Queries: Q further (qu,qv) from the same LCG. Answer = nearest id (0-based) by
squared euclid, ties -> lowest id. Fold = sum(id_i * 131^i) mod 1e9+7.
Cell = ((u + 2^30) >> 21) * 1024 + ((v + 2^30) >> 21)  (1024 x 1024 grid).

Usage: oracle.py [N] [Q]   (default 1000000 1000). Prints the fold (last line)
and, before it, the sqlite3 timings (scan via MIN(), indexed 3x3 window) so the
same file is both the truth oracle (python brute force, cached) and the SQL row.
The brute force is cached in bench/tq_sqlite/truth_N_Q.txt (30-60 s once).
"""
import os, sqlite3, sys, time
N = int(sys.argv[1]) if len(sys.argv) > 1 else 1000000
Q = int(sys.argv[2]) if len(sys.argv) > 2 else 1000
M = (1 << 64) - 1; A = 6364136223846793005; C = 1442695040888963407
def gen(n, x=12345):
    out = []
    for _ in range(n):
        x = (x * A + C) & M; u = (x >> 33) - (1 << 30)
        x = (x * A + C) & M; v = (x >> 33) - (1 << 30)
        out.append((u, v))
    return out, x
def cell(u, v): return ((u + (1 << 30)) >> 21) * 1024 + ((v + (1 << 30)) >> 21)
pts, x = gen(N)
qs, _ = gen(Q, x)
here = os.path.dirname(os.path.abspath(__file__))
cache = os.path.join(here, f'truth_{N}_{Q}.txt')
if os.path.exists(cache):
    ans = [int(t) for t in open(cache).read().split()]
else:
    # exact nearest by expanding cell rings: after scanning Chebyshev ring r, every
    # unscanned point is at least (r*CS - CS) away on one axis, so stop once
    # best_d <= ((r-1)*CS)^2. Average 1 point per cell -> r <= 2 typically.
    from collections import defaultdict
    CS = 1 << 21
    byc0 = defaultdict(list)
    for i, (u, v) in enumerate(pts): byc0[cell(u, v)].append(i)
    ans = []
    for qu, qv in qs:
        c = cell(qu, qv); cx, cy = c // 1024, c % 1024; best = None; bi = -1; r = 0
        while True:
            for dx in range(-r, r + 1):
                for dy in range(-r, r + 1):
                    if max(abs(dx), abs(dy)) != r: continue
                    x, y = cx + dx, cy + dy
                    if not (0 <= x < 1024 and 0 <= y < 1024): continue
                    for i in byc0.get(x * 1024 + y, ()):
                        u, v = pts[i]; d = (u - qu) * (u - qu) + (v - qv) * (v - qv)
                        if best is None or d < best or (d == best and i < bi): best, bi = d, i
            if best is not None and r >= 1 and best <= ((r - 1) * CS) ** 2: break
            r += 1
            if r > 1024: break
        ans.append(bi)
    open(cache, 'w').write(' '.join(map(str, ans)))
def fold_of(seq):
    f = 0; p = 1
    for a in seq:
        f = (f + (a + 1000000007) * p) % 1000000007; p = (p * 131) % 1000000007
    return f
QS = min(Q, 20)
# windowed (3x3 cell) answer with the SAME rule bebop nnidx.bp uses: lowest d, then lowest id, -1 if empty
from collections import defaultdict
byc = defaultdict(list)
for i, (u, v) in enumerate(pts): byc[cell(u, v)].append(i)
wans = []
for qu, qv in qs:
    c = cell(qu, qv); cx, cy = c // 1024, c % 1024; best = None; bi = -1
    for dx in (-1, 0, 1):
        for dy in (-1, 0, 1):
            if 0 <= cx + dx < 1024 and 0 <= cy + dy < 1024:
                for i in byc.get((cx + dx) * 1024 + (cy + dy), ()):
                    u, v = pts[i]; d = (u - qu) * (u - qu) + (v - qv) * (v - qv)
                    if best is None or d < best or (d == best and i < bi): best, bi = d, i
    wans.append(bi)
print(f'truth_fold_Q{QS}={fold_of(ans[:QS])} truth_fold_Q{Q}={fold_of(ans)} window_fold_Q{Q}={fold_of(wans)} window_matches_truth={sum(a == b for a, b in zip(ans, wans))}/{Q}')
fold = fold_of(ans)
# ---- sqlite rows (in-memory db, same core the caller pinned us to) ----
db = sqlite3.connect(':memory:')
db.execute('CREATE TABLE p(id INTEGER PRIMARY KEY, u INTEGER, v INTEGER, cell INTEGER)')
t0 = time.perf_counter()
db.executemany('INSERT INTO p VALUES(?,?,?,?)', ((i, u, v, cell(u, v)) for i, (u, v) in enumerate(pts)))
db.execute('CREATE INDEX ic ON p(cell)'); db.commit()
build_ms = (time.perf_counter() - t0) * 1000
t0 = time.perf_counter()
scan_ok = 0
for k in range(QS):
    qu, qv = qs[k]
    r = db.execute('SELECT id FROM p ORDER BY (u-?)*(u-?)+(v-?)*(v-?), id LIMIT 1', (qu, qu, qv, qv)).fetchone()[0]
    scan_ok += (r == ans[k])
scan_ms = (time.perf_counter() - t0) * 1000 / QS
t0 = time.perf_counter(); idx_ok = 0
for k in range(Q):
    qu, qv = qs[k]; c = cell(qu, qv); cx, cy = c // 1024, c % 1024
    cells = [(cx + dx) * 1024 + (cy + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if 0 <= cx + dx < 1024 and 0 <= cy + dy < 1024]
    r = db.execute(f'SELECT id FROM p WHERE cell IN ({",".join("?" * len(cells))}) ORDER BY (u-?)*(u-?)+(v-?)*(v-?), id LIMIT 1', (*cells, qu, qu, qv, qv)).fetchone()
    idx_ok += (r is not None and r[0] == ans[k])
idx_us = (time.perf_counter() - t0) * 1e6 / Q
print(f'sqlite build_ms={build_ms:.0f} scan_ms_per_query={scan_ms:.1f} (ok {scan_ok}/{QS}) indexed_us_per_query={idx_us:.1f} (ok {idx_ok}/{Q}; a 3x3 window can miss the true nearest when the query sits near a cell edge and the nearest is 2 cells away) N={N} Q={Q}')
print(fold)
