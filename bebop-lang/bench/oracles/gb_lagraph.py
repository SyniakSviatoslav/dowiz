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
from lag_common import ring_chords, random_lcg, random_lcg_edges, ring2, bfs_levels, triangle_count, sssp_minplus, pagerank_q32, combine, s64, M


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



# ---- B3 step 2 (docs/blueprints/B3-graphblas-kernels-prejit.md section 5 step 2, section 9):
# oracle for bench/vs_rust/std_tests/gb_gen.bp's gb_gen gate (mxv/vxm x semirings 1-4 on the
# 1k/4000-edge random_lcg() graph) and gb_bfs.bp's gb_bfs gate (BFS level-sum, same graph).
# mxv(A,x)[i] = fold over A's row i of (x[j] oplus/otimes edge weight) -- gb.bp's own
# gb_mxv_generic (selfhost/prelude/gb.bp:440-465), mirrored here arm for arm. vxm(x,A) ==
# mxv(A^T,x) exactly (gen_gb.bp's own header note); random_lcg() is UNDIRECTED (both directions
# added, lag_common._adj) with a symmetric edge weight (edge_weight below == gb.bp's
# gb_edge_weight), so A^T == A structurally and by weight on this graph -- vxm and mxv fold to
# the SAME value per semiring, which is exactly what both the bebop tier-0 (gb_mxv_generic
# called on A then on gb_transpose(A)) and the generated kernels (root field 0 vs field 1)
# independently confirmed (2026-09-07: all 8 (op,sr) folds pairwise equal, verified by running
# each of the 8 compiled kernels against the driver's own store file).
FP32 = 1 << 32
BIG = 1099511627776  # gb.bp's own min-plus identity ("big")


def gb_gen_x(n):
    # gb_gen.bp's build_x: x[i] = 0 when i%3==0 (~third of vertices inactive), else
    # ((i%5)+1) * 2**32 (Q32-scaled small positive integer -- plus-times' schoolbook
    # `(x[j]*w)/2**32` divides out EXACTLY since x[j] is a multiple of 2**32).
    return [0 if i % 3 == 0 else ((i % 5) + 1) * FP32 for i in range(n)]


def _row_fold(i, adj, x, sr):
    # gb_mxv_generic's per-row loop (gb.bp:446-460), one semiring arm at a time -- same
    # arithmetic the generated kernels run (gen_gb.bp's wr_init/wr_row_body).
    acc = BIG if sr == 4 else 0
    for v in adj[i]:
        w = edge_weight(i, v)
        if sr == 3:
            acc = acc + (x[v] * w) // FP32
        elif sr == 4:
            mp = x[v] + w
            acc = mp if mp < acc else acc
        else:  # sr 1 or-and / 2 any-pair -- both reduce to "any live neighbour" (gb.bp:24-31)
            xb = 1 if x[v] != 0 else 0
            acc = 1 if acc == 1 else xb
    return acc


def gb_fold(n, adj, x, sr):
    return s64(sum(_row_fold(i, adj, x, sr) for i in range(n)))


def directed_lt_adj():
    """B3 step 2 coordinator review item 2 (2026-09-07): the symmetric random_lcg() graph has
    AT == A (structurally, and by weight -- edge_weight is symmetric), so mxv and vxm fold to
    the SAME value per semiring and the gate can't tell the two ops apart. This keeps only
    (u, v) with u < v from the SAME raw LCG pair stream (random_lcg_edges(), no reverse edge
    added -- gen_gb.bp's build_directed_lt mirrors this exactly: a bitmap dedupe keyed by
    u*n+v, no symmetrise step) so the resulting matrix is genuinely directed/asymmetric."""
    n, edges = random_lcg_edges()
    s = set(uv for uv in edges if uv[0] < uv[1])
    adj = [[] for _ in range(n)]
    for u, v in s:
        adj[u].append(v)
    for row in adj:
        row.sort()
    return n, adj


def transpose_adj(n, adj):
    tadj = [[] for _ in range(n)]
    for u in range(n):
        for v in adj[u]:
            tadj[v].append(u)
    for row in tadj:
        row.sort()
    return tadj


def gb_directed_folds():
    """dmxv/dvxm sr1 (or-and) and sr4 (min-plus) on directed_lt_adj() -- mxv(DA,x) and
    vxm(x,DA) == mxv(DA^T,x) must (and do) differ, unlike the symmetric graph's pairs."""
    n, adj = directed_lt_adj()
    tadj = transpose_adj(n, adj)
    x = gb_gen_x(n)
    dmxv1 = gb_fold(n, adj, x, 1)
    dmxv4 = gb_fold(n, adj, x, 4)
    dvxm1 = gb_fold(n, tadj, x, 1)
    dvxm4 = gb_fold(n, tadj, x, 4)
    return dmxv1, dmxv4, dvxm1, dvxm4


def gb_gen_combined():
    """The gb_gen gate's golden value: rolling-combine (bebop's own `c*1000003+v`, gen_gb.bp's
    combine12) of the 8 symmetric (op, sr) folds (order mxv sr1..4 then vxm sr1..4 -- mxv and
    vxm folds are IDENTICAL per semiring on this undirected graph, see module docstring above)
    PLUS the 4 directed folds (dmxv sr1, dmxv sr4, dvxm sr1, dvxm sr4 -- these DIFFER, the
    asymmetric case review item 2 asked for)."""
    n, adj = random_lcg()
    x = gb_gen_x(n)
    mxv = [gb_fold(n, adj, x, sr) for sr in (1, 2, 3, 4)]
    vxm = list(mxv)  # vxm(x,A) == mxv(A^T,x) == mxv(A,x) here (A^T == A, symmetric weights)
    directed = list(gb_directed_folds())
    vals = mxv + vxm + directed
    c = vals[0]
    for v in vals[1:]:
        c = s64(c * 1000003 + v)
    return c, mxv, directed


def gb_bfs_fold():
    n, adj = random_lcg()
    return bfs_levels(n, adj, src=0)


# ---- B3 step 3 (docs/blueprints/B3-graphblas-kernels-prejit.md section 5 step 3, section 6):
# G9b oracles for gb_tc.bp/gb_cc.bp/gb_sssp.bp/gb_pr.bp. Connectivity fact discovered during
# this step (2026-09-07, verified independently by BFS and by brute-force Dijkstra): the 1k
# random_lcg() graph has 750 of its 1000 nodes UNREACHABLE from node 0 (average degree 7.76
# sits close to the random-graph connectivity threshold ln(1000)=6.9) -- this is already why
# gb_bfs's own golden fold is a small negative number (-3), not a large positive level sum, and
# it is why gb_cc's algorithm below must not assume a single connected component, and why
# gb_sssp's -1-unreachable sentinel convention (already lag_common.sssp_minplus()'s own
# convention) actually matters here rather than being dead code.


def gb_tc_fold():
    """gb_tc.bp's golden value: triangle_count() combined over [ring_chords, random_lcg, ring2]
    the same rolling-fold way as combine(). ring_chords and random_lcg have EXACTLY ZERO
    triangles (independently re-verified by brute force, not a bug in triangle_count()) --
    coordinator review 2026-09-07 added ring2() (a ring with chords of length 1 AND 2, exactly
    64 triangles) as the third term specifically so this fold has real distinguishing power: a
    broken mxm/select/reduce pipeline can no longer coincidentally return 0 and pass."""
    nR, adjR = ring_chords()
    nL, adjL = random_lcg()
    n2, adj2 = ring2()
    return combine([triangle_count(nR, adjR), triangle_count(nL, adjL), triangle_count(n2, adj2)])


def cc_label_sum(n, adj):
    """gb_cc.bp's algorithm, mirrored bit-for-bit: synchronous ("Jacobi") min-label
    propagation -- label[i]=i initially; each round every node's next label is the min of its
    OWN current label and its neighbours' CURRENT (pre-round) labels, applied simultaneously;
    repeat until a round changes nothing. Correct regardless of how many components the graph
    has (each converges to its own min node id independently)."""
    label = list(range(n))
    while True:
        nx = list(label)
        changed = False
        for i in range(n):
            m = label[i]
            for j in adj[i]:
                if label[j] < m:
                    m = label[j]
            nx[i] = m
            if m < label[i]:
                changed = True
        label = nx
        if not changed:
            break
    return sum(label)


def gb_cc_fold():
    n, adj = random_lcg()
    return cc_label_sum(n, adj)


def gb_sssp_fold():
    """gb_sssp.bp's golden value: reuses lag_common.sssp_minplus() UNCHANGED -- SSSP has a
    unique correct distance vector regardless of algorithm (Dijkstra here vs the .bp side's
    from-scratch min-plus relaxation to fixpoint), including the -1 sentinel for the graph's
    750 unreachable nodes, which sssp_minplus() already applies."""
    n, adj = random_lcg()
    return sssp_minplus(n, adj, src=0)


def gb_pr_fold():
    """gb_pr.bp's golden value: reuses lag_common.pagerank_q32() UNCHANGED (10 iterations,
    damping 0.85, Q32 schoolbook multiply-then-shift) -- overflow-checked 2026-09-07: the
    largest wgt*r[u] / d_fp*incoming product seen across the 10 iterations on this graph is
    ~3.6e16, far under i64's 2**63 ceiling, so bebop's native i64 multiply and python's
    unbounded-int arithmetic agree exactly, no wraparound divergence to account for."""
    n, adj = random_lcg()
    return pagerank_q32(n, adj)


if __name__ == '__main__':
    print(roundtrip_fold())
    combined, per_sr, directed = gb_gen_combined()
    print('gb_gen mxv/vxm sr1..4', per_sr, 'directed dmxv1/4,dvxm1/4', directed, 'combined', combined)
    print('gb_bfs', gb_bfs_fold())
    print('gb_tc', gb_tc_fold())
    print('gb_cc', gb_cc_fold())
    print('gb_sssp', gb_sssp_fold())
    print('gb_pr', gb_pr_fold())
