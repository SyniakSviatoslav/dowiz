#!/usr/bin/env python3
"""oracles2/nnidx -- bench/tq_sqlite/nnidx.bp window fold (spec: oracle.py docstring
+ RESULT.md + nnidx.bp first comment).  N=1000000 points, Q=1000 queries from the
MMIX LCG x = x*6364136223846793005 + 1442695040888963407 mod 2^64, seed 12345:
u = (x>>33) - 2^30, v = (x'>>33) - 2^30 (two states per point/query, generation order
= id).  Cell = ((u+2^30)>>21)*1024 + ((v+2^30)>>21).  Answer per query = lowest
squared-euclid distance inside the 3x3 cell window around the query's cell (clipped at
the grid border), ties -> lowest id, -1 if the window is empty.
Fold = sum(id_i * 131^i) mod 1e9+7.  The gate returns fold*1e6 + query_ms; the timing
part is not reproducible, so the LAST LINE is the window fold only.
Also prints the exact-nearest fold (ring expansion) for the 2/1000 window-miss finding.
Parameter: ADVANCE_FIRST -- whether the seed itself is a state that yields output
(False) or the LCG is stepped before the first draw (True, chosen)."""
import sys
A, C, M = 6364136223846793005, 1442695040888963407, (1 << 64) - 1
N = int(sys.argv[1]) if len(sys.argv) > 1 else 1000000
Q = int(sys.argv[2]) if len(sys.argv) > 2 else 1000
ADVANCE_FIRST = True
P, B, CELL, G, OFF = 1000000007, 131, 1 << 21, 1024, 1 << 30

x = 12345
def draw():
    global x
    x = (x * A + C) & M
    return (x >> 33) - OFF
if not ADVANCE_FIRST:
    # the seed itself is the first state
    first = [(12345 >> 33) - OFF]
pts_u, pts_v = [], []
for _ in range(N):
    pts_u.append(draw()); pts_v.append(draw())
qs = [(draw(), draw()) for _ in range(Q)]

cells = {}
for i in range(N):
    c = ((pts_u[i] + OFF) >> 21) * G + ((pts_v[i] + OFF) >> 21)
    cells.setdefault(c, []).append(i)

def scan(cu, cv, qu, qv, best):
    lst = cells.get(cu * G + cv)
    if lst:
        for i in lst:
            du, dv = pts_u[i] - qu, pts_v[i] - qv
            d = du * du + dv * dv
            if best is None or d < best[0] or (d == best[0] and i < best[1]):
                best = (d, i)
    return best

wfold = tfold = 0
pw = 1
miss = 0
for qu, qv in qs:
    cu, cv = (qu + OFF) >> 21, (qv + OFF) >> 21
    best = None
    for du in (-1, 0, 1):
        for dv in (-1, 0, 1):
            u, v = cu + du, cv + dv
            if 0 <= u < G and 0 <= v < G:
                best = scan(u, v, qu, qv, best)
    wid = -1 if best is None else best[1]
    # exact nearest: expand rings until ring bound exceeds best
    r = 0
    tb = None
    while True:
        for u in range(cu - r, cu + r + 1):
            for v in range(cv - r, cv + r + 1):
                if max(abs(u - cu), abs(v - cv)) == r and 0 <= u < G and 0 <= v < G:
                    tb = scan(u, v, qu, qv, tb)
        if tb is not None and tb[0] <= (r * CELL) ** 2:
            break
        r += 1
        if r > G:
            break
    tid = -1 if tb is None else tb[1]
    if tid != wid:
        miss += 1
    wfold = (wfold + wid * pw) % P
    tfold = (tfold + tid * pw) % P
    pw = pw * B % P
print("true_nearest_fold", tfold, "window_misses", miss)
print(wfold)
