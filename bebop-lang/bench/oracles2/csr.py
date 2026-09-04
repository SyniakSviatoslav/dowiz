#!/usr/bin/env python3
"""oracles2/csr -- gate `csr`: structural twin of dowiz-core csr.rs Csr::from_edges
on the five golden graphs of bench/vs_rust/spectral_golden (P4, C3, K4W, B6, D2DUP;
edge lists from generator/src/main.rs, each edge inserted in BOTH directions in input
order), weights as Q32 fixed point (1.0 = 2^32).  from_edges: per-row buckets in
input order, stable sort by column, adjacent duplicate columns merged by wrapping i64
sum, out-of-range endpoints ignored.  The rp/ci/vv arrays are checked against the
CSR GOLDENS section of golden.txt (assert).
UNDERSPECIFIED: the gate fold "over rp+ci+vv of the five golden graphs" -- the fold
function is not in any prose.  FOLD env selects a candidate; default = fnv (prelude
hash.bp fnv_cells chained over every cell of rp, ci, vv, graph after graph)."""
import os, sys
M64 = (1 << 64) - 1
def s64(x): x &= M64; return x - (1 << 64) if x >> 63 else x
ONE = 1 << 32
GRAPHS = [
    ("P4", 4, [(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0)]),
    ("C3", 3, [(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0)]),
    ("K4W", 4, [(0, 1, 2.0), (0, 2, 3.0), (0, 3, 1.0), (1, 2, 1.0), (1, 3, 4.0), (2, 3, 2.0)]),
    ("B6", 6, [(0, 1, 1.0), (0, 2, 1.0), (1, 2, 1.0), (3, 4, 1.0), (3, 5, 1.0), (4, 5, 1.0), (2, 3, 0.5)]),
    ("D2DUP", 2, [(0, 1, 1.0), (0, 1, 1.0), (1, 0, 0.5)]),
]
GOLDEN = {  # golden.txt CSR GOLDENS: row_ptr, col_idx, val_fp32
    "P4": ([0, 1, 3, 5, 6], [1, 0, 2, 1, 3, 2], [ONE] * 6),
    "C3": ([0, 2, 4, 6], [1, 2, 0, 2, 0, 1], [ONE] * 6),
    "K4W": ([0, 3, 6, 9, 12], [1, 2, 3, 0, 2, 3, 0, 1, 3, 0, 1, 2],
            [2*ONE, 3*ONE, ONE, 2*ONE, ONE, 4*ONE, 3*ONE, ONE, 2*ONE, ONE, 4*ONE, 2*ONE]),
    "B6": ([0, 2, 4, 7, 10, 12, 14], [1, 2, 0, 2, 0, 1, 3, 2, 4, 5, 3, 5, 3, 4],
           [ONE] * 6 + [ONE // 2, ONE // 2] + [ONE] * 6),
    "D2DUP": ([0, 1, 2], [1, 0], [ONE * 5 // 2, ONE * 5 // 2]),
}
def from_edges(n, edges):
    rows = [[] for _ in range(n)]
    for s, d, w in edges:
        if 0 <= s < n and 0 <= d < n: rows[s].append((d, w))
    rp, ci, vv = [0], [], []
    for r in rows:
        r.sort(key=lambda t: t[0])          # stable
        merged = []
        for c, w in r:
            if merged and merged[-1][0] == c: merged[-1][1] = s64(merged[-1][1] + w)
            else: merged.append([c, w])
        for c, w in merged: ci.append(c); vv.append(w)
        rp.append(len(ci))
    return rp, ci, vv
def sym(edges):
    out = []
    for s, d, w in edges: out.append((s, d, int(w * ONE))); out.append((d, s, int(w * ONE)))
    return out
FOLDS = {
    "fnv":   (s64(0xcbf29ce484222325), lambda h, x: s64((h ^ x) * 0x100000001b3)),
    "poly31": (0, lambda h, x: s64(h * 31 + x)),
    "mmix":  (0, lambda h, x: s64((h + x) * 6364136223846793005 + 1442695040888963407)),
    "mix1000003": (0, lambda h, x: (h * 1000003 + x) & ((1 << 62) - 1)),
}
def fold_all(name, with_meta=False):
    h, step = FOLDS[name]
    for g, n, edges in GRAPHS:
        rp, ci, vv = from_edges(n, sym(edges))
        assert (rp, ci, vv) == GOLDEN[g], g
        cells = ([n, len(ci)] if with_meta else []) + rp + ci + vv
        for x in cells: h = step(h, x)
    return h
if __name__ == "__main__":
    name = os.environ.get("FOLD", "fnv")
    meta = os.environ.get("META", "0") == "1"
    print("fold", name, "meta", meta)
    print(fold_all(name, meta))
