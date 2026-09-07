#!/usr/bin/env python3
"""bench/oracles/gb_lagraph.py -- B3 step 1 oracle (docs/blueprints/B3-graphblas-kernels-
prejit.md section 6/9) for bench/vs_rust/std_tests/gb_roundtrip.bp (the G9a round-trip gate).
Builds the SAME ring_chords() graph as selfhost/prelude/gb.bp's gb_build_ring_chords (via
lag_common.py's shared generator -- same LCG-free ring+chord edge list, same undirected-set
dedupe dropping self-loops and duplicate (u,v) pairs, same sorted-adjacency row order), then
independently reimplements gb.bp's CSR build / transpose (csr_build over (dst,src), stable
counting sort, gb_csr_fill_w's exact traversal order) / row-range extract (gb_extract_rows'
contiguous-slice semantics), and folds n/m/nnz/rp/ci/vv of each the same way the .bp gate
does: n*1000003 + m*7919 + nnz*104729 + sum(rp)*31 + sum(ci)*17 + sum(vv)*13, combined as
original*3 + transpose*5 + extract*7. stdlib only, i64 wrap throughout (s64/M from
lag_common.py).
"""
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lag_common import ring_chords, s64, M


def edge_weight(u, v):
    # gb.bp gb_edge_weight / lag_common.sssp_minplus's w(): symmetric, order-independent.
    a, b = (u, v) if u < v else (v, u)
    return 1 + ((a * 1000003 + b * 7919) % 9)


def build_csr(n, adj):
    # adj[u] is already sorted ascending (lag_common._adj: sorted(set)) -- matches
    # gb_build_pairs' dedupe-bitmap drain order (ascending key = u*n+v).
    rp = [0] * (n + 1)
    ci = []
    vv = []
    for u in range(n):
        rp[u + 1] = rp[u] + len(adj[u])
        for v in adj[u]:
            ci.append(v)
            vv.append(edge_weight(u, v))
    return rp, ci, vv


def transpose(n, rp, ci, vv):
    # gb_transpose: csr_build over (dst, src) -- traverse A row-major (u ascending, k
    # ascending within row), stable counting sort into new rows keyed by ci[k].
    nnz = len(ci)
    deg = [0] * n
    for u in range(n):
        for k in range(rp[u], rp[u + 1]):
            deg[ci[k]] += 1
    trp = [0] * (n + 1)
    for i in range(n):
        trp[i + 1] = trp[i] + deg[i]
    cur = trp[:]
    tci = [0] * nnz
    tvv = [0] * nnz
    for u in range(n):
        for k in range(rp[u], rp[u + 1]):
            v = ci[k]
            s = cur[v]
            tci[s] = u
            tvv[s] = vv[k]
            cur[v] = s + 1
    return trp, tci, tvv


def extract_rows(rp, ci, vv, lo, hi):
    # gb_extract_rows: rows lo..hi-1 are a contiguous slice of A's own ci/vv.
    off = rp[lo]
    end = rp[hi]
    erp = [rp[lo + i] - off for i in range(hi - lo + 1)]
    return erp, ci[off:end], vv[off:end]


def fold_component(n, m, rp, ci, vv):
    return s64(n * 1000003 + m * 7919 + len(ci) * 104729 + sum(rp) * 31 + sum(ci) * 17 + sum(vv) * 13)


def roundtrip_fold():
    n, adj = ring_chords()
    rp, ci, vv = build_csr(n, adj)
    trp, tci, tvv = transpose(n, rp, ci, vv)
    lo, hi = 10, 20
    erp, eci, evv = extract_rows(rp, ci, vv, lo, hi)
    fa = fold_component(n, n, rp, ci, vv)
    ft = fold_component(n, n, trp, tci, tvv)
    fe = fold_component(hi - lo, n, erp, eci, evv)
    return s64(fa * 3 + ft * 5 + fe * 7)


if __name__ == '__main__':
    print(roundtrip_fold())
