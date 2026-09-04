#!/usr/bin/env python3
"""oracles2/tq -- gate `tq` (T20).  PROSE (ROADMAP T20 + tq.bp first comment): data =
vectors in R^4 as fp Q32; "INDEX" = (u,v) = contraction with the top-2 eigenvectors
ev0/ev1, grid_cell quantize with GRID_RES=8 (port of parametric_spectral.rs
grid_cell: ((u+1)/2*RES) clamped to [0,RES-1], cell = vi*RES+ui); "SELECT nearest" =
geodesic distance with K anchor control points (memory_search.rs geodesic_distance:
3-segment walk through the anchor nearest the TARGET, break when best_d < 0.1) over
the 3x3-cell candidate window (search_spins); fp_mul = trunc(a*b/2^32),
fp_sqrt(x) = isqrt(x<<26)<<3; fold = polynomial hash mod 1e9+7 of (nearest index,
geodesic distance fp, window count) per query.
NOT IN ANY PROSE: N, the point/query generator, ev0/ev1 (a top-2 eigen solve of WHAT
covariance, in which fp iteration), K and which points are anchors, the hash base,
tie rule.  All are parameters here; the geometry/fp part is exact to the prose."""
import os
M64 = (1 << 64) - 1
def s64(x): x &= M64; return x - (1 << 64) if x >> 63 else x
ONE = 1 << 32; RES = 8; P = 1000000007
N = int(os.environ.get("N", "64")); K = int(os.environ.get("K", "4"))
QN = int(os.environ.get("Q", "16")); BASE = int(os.environ.get("BASE", "131"))
SEED = int(os.environ.get("SEED", "0x9E3779B97F4A7C15"), 0)
def fp_mul(a, b):
    p = abs(a) * abs(b) >> 32
    return s64(-p if (a < 0) != (b < 0) else p)
import math
def isqrt(s): return math.isqrt(s)   # prelude fp.bp digit-by-digit == floor sqrt for 0 <= s < 2^62
def fp_sqrt(x): return isqrt(x << 26) << 3
def lcg(s): return (s * 6364136223846793005 + 1442695040888963407) & M64
def frac_fp(s): return ((s >> 11) << 32 >> 52) - (ONE >> 1)   # in [-0.5, 0.5) fp
# data: N points in R^4, coordinates in (-0.5, 0.5) fp
s = SEED; pts = []
for _ in range(N):
    p = []
    for _ in range(4): s = lcg(s); p.append(frac_fp(s))
    pts.append(p)
# ev0/ev1 parameter: axis vectors e0, e1 (a real top-2 eigen solve is unspecified)
EV0 = [ONE, 0, 0, 0]; EV1 = [0, ONE, 0, 0]
def project(p):
    return (sum(fp_mul(a, b) for a, b in zip(EV0, p)), sum(fp_mul(a, b) for a, b in zip(EV1, p)))
def cell_of(u, v):
    ui = max(0, min(RES - 1, ((u + ONE) * RES) >> 33))
    vi = max(0, min(RES - 1, ((v + ONE) * RES) >> 33))
    return vi * RES + ui
uv = [project(p) for p in pts]
grid = {}
for i, (u, v) in enumerate(uv): grid.setdefault(cell_of(u, v), []).append(i)
anchors = uv[:K]
def sq(du, dv): return fp_mul(du, du) + fp_mul(dv, dv)
def geodesic(u1, v1, u2, v2):
    if K < 2: return fp_sqrt(sq(u2 - u1, v2 - v1))
    dist, pu, pv = 0, u1, v1
    for _ in range(3):
        best, bd = 0, None
        for i, (au, av) in enumerate(anchors):
            d = sq(au - u2, av - v2)
            if bd is None or d < bd: bd, best = d, i
        nu, nv = anchors[best]
        dist += fp_sqrt(sq(nu - pu, nv - pv)); pu, pv = nu, nv
        if bd < ONE // 10: break
    return dist + fp_sqrt(sq(u2 - pu, v2 - pv))
h = 0
for _ in range(QN):
    s = lcg(s); qu = frac_fp(s); s = lcg(s); qv = frac_fp(s)
    c = cell_of(qu, qv); ci, ri = c % RES, c // RES
    cands = []
    for dc in (-1, 0, 1):
        for dr in (-1, 0, 1):
            x, y = ci + dc, ri + dr
            if 0 <= x < RES and 0 <= y < RES: cands += grid.get(y * RES + x, [])
    best, bd = -1, None
    for i in cands:
        d = geodesic(qu, qv, uv[i][0], uv[i][1])
        if bd is None or d < bd or (d == bd and i < best): bd, best = d, i
    for x in (best, bd if bd is not None else 0, len(cands)):
        h = (h * BASE + x) % P
print("N", N, "K", K, "Q", QN, "base", BASE)
print(h)
