#!/usr/bin/env python3
"""bench/oracles/lag_common.py -- B3 prep (docs/blueprints/B3-graphblas-kernels-prejit.md
section 6): shared graph generators, LAGraph-style kernels (BFS, PageRank Q32, triangle
count, connected components, SSSP min-plus) and a fold helper for the 5 gb_*.py oracles.
stdlib only. Integer arithmetic only (no floats anywhere, including PageRank's damping
constant, which is built with floor integer division).

Three deterministic graphs (fixed by the B1/B3/B7 prep task, no bebop counterpart yet --
the blueprint's own 1M/10M sgraph LCG graphs are future-work sized, not prep-sized):
  ring_chords()  -- 64 nodes: a ring (i, i+1 mod 64) plus 32 diametrical chords (i, i+32).
  random_lcg()   -- 1000 nodes, 4000 undirected edges from the repo's standard LCG
                    (A=6364136223846793005, C=1442695040888963407, same constants as
                    bench/oracles/scrash.py), seeded 42, self-loops and duplicate edges
                    dropped.
  grid10x10()    -- 100 nodes (r*10+c), 4-connected grid (right + down neighbours).
All three are UNDIRECTED (both directions), matching the push/pull frontier BFS and
csr_spmv conventions already in selfhost/std/sgraph2.bp / csr.bp.
"""
A, C, LCGM = 6364136223846793005, 1442695040888963407, (1 << 64) - 1
M = (1 << 64) - 1


def s64(x):
    x &= M
    return x - (1 << 64) if x >> 63 else x


def _adj(n, edges):
    g = [set() for _ in range(n)]
    for u, v in edges:
        if u == v or not (0 <= u < n and 0 <= v < n):
            continue
        g[u].add(v)
        g[v].add(u)
    return [sorted(s) for s in g]


def ring_chords(n=64):
    edges = [(i, (i + 1) % n) for i in range(n)] + [(i, (i + n // 2) % n) for i in range(n // 2)]
    return n, _adj(n, edges)


def random_lcg(n=1000, m=4000, seed=42):
    x = seed
    edges = []
    while len(edges) < m:
        x = (x * A + C) & LCGM
        u = x % n
        x = (x * A + C) & LCGM
        v = x % n
        edges.append((u, v))
    return n, _adj(n, edges)


def grid10x10(rows=10, cols=10):
    n = rows * cols
    edges = []
    for r in range(rows):
        for c in range(cols):
            i = r * cols + c
            if c + 1 < cols: edges.append((i, i + 1))
            if r + 1 < rows: edges.append((i, i + cols))
    return n, _adj(n, edges)


GRAPHS = [('ring_chords', ring_chords), ('random_lcg', random_lcg), ('grid10x10', grid10x10)]


def combine(vals):  # acc*1000003+v rolling fold across per-graph results -> one printable value
    acc = 0
    for v in vals:
        acc = (acc * 1000003 + v) & M
    return s64(acc)


# ---- kernels (fold definitions match docs/blueprints/B3-graphblas-kernels-prejit.md section 6) ----

def bfs_levels(n, adj, src=0):
    """BFS levels sum: unreached nodes get level -1 (never happens on these 3 connected
    graphs, but handled so the oracle stays correct if that ever changes)."""
    level = [-1] * n
    level[src] = 0
    frontier = [src]
    d = 0
    while frontier:
        d += 1
        nxt = []
        for u in frontier:
            for v in adj[u]:
                if level[v] == -1:
                    level[v] = d
                    nxt.append(v)
        frontier = nxt
    return sum(level)


def connected_components(n, adj):
    """CC label sum: union-find, label = min node id in the component."""
    parent = list(range(n))

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for u in range(n):
        for v in adj[u]:
            ru, rv = find(u), find(v)
            if ru != rv:
                if ru < rv: parent[rv] = ru
                else: parent[ru] = rv
    label = [find(u) for u in range(n)]
    return sum(label)


def triangle_count(n, adj):
    """Exact count via forward adjacency (neighbours with id > u): for each edge (u,v)
    with u<v, count common forward neighbours w>v with edges to both -- no triangle is
    counted more than once."""
    fwd = [sorted(x for x in adj[u] if x > u) for u in range(n)]
    fwdset = [set(f) for f in fwd]
    tri = 0
    for u in range(n):
        fu = fwd[u]
        for i in range(len(fu)):
            v = fu[i]
            for w in fu[i + 1:]:
                if w in fwdset[v]:
                    tri += 1
    return tri


def sssp_minplus(n, adj, src=0):
    """SSSP min-plus: edge weight w(u,v) = 1 + ((min*1000003 + max*7919) % 9) in [1,9],
    a deterministic pure function of the (unordered) edge -- distinguishes this from
    plain BFS. Dijkstra (heapq) computes the exact min-plus fixed point; unreached = -1.
    Fold = sum of distances (blueprint section 6: 'SSSP min-plus distance sum')."""
    import heapq
    def w(u, v):
        a, b = (u, v) if u < v else (v, u)
        return 1 + ((a * 1000003 + b * 7919) % 9)
    dist = [-1] * n
    dist[src] = 0
    pq = [(0, src)]
    seen = [False] * n
    while pq:
        d, u = heapq.heappop(pq)
        if seen[u]: continue
        seen[u] = True
        for v in adj[u]:
            nd = d + w(u, v)
            if dist[v] == -1 or nd < dist[v]:
                dist[v] = nd
                heapq.heappush(pq, (nd, v))
    return sum(dist)


FP_ONE = 1 << 32  # Q32 fixed point, fp(1.0) = 2**32 (matches selfhost/std/csr.bp comment)


def fp_mul(a, b):
    return (a * b) >> 32  # exact for a,b >= 0 (PageRank never carries a negative fp value)


def pagerank_q32(n, adj, iters=10, d_num=85, d_den=100):
    """PageRank in Q32 fixed point, integer arithmetic only. d = d_num/d_den (default
    0.85) built by floor integer division -- no float anywhere. Dangling nodes (outdeg
    0, possible on the sparse random graph) simply do not disperse mass (documented
    simplification: PageRank mass is not conserved across a dangling node here, only
    the fold needs to be exact and reproducible -- ponytail: full teleport-redistribution
    is unneeded for a prep oracle with no bebop counterpart yet).
    Fold = rolling fold of the final rank vector (blueprint section 6: 'PageRank Q32
    after 10 iterations, exact integer arithmetic mirrored')."""
    d_fp = (d_num * FP_ONE) // d_den
    base_fp = (FP_ONE - d_fp) // n
    outdeg = [len(adj[u]) for u in range(n)]
    r = [FP_ONE // n] * n
    for _ in range(iters):
        incoming = [0] * n
        for u in range(n):
            if outdeg[u] == 0:
                continue
            wgt = FP_ONE // outdeg[u]
            contrib = fp_mul(wgt, r[u])
            for v in adj[u]:
                incoming[v] += contrib
        r = [base_fp + fp_mul(d_fp, incoming[v]) for v in range(n)]
    acc = 0
    for x in r:
        acc = (acc * 1000003 + x) & M
    return s64(acc)


def run_over_graphs(kernel):
    """kernel(n, adj) -> int. Runs over the 3 standard graphs, prints one diagnostic
    line per graph, returns the combined fold (the caller prints it last)."""
    vals = []
    for name, gen in GRAPHS:
        n, adj = gen()
        v = kernel(n, adj)
        print(name, n, v)
        vals.append(v)
    return combine(vals)


if __name__ == '__main__':
    # sanity print: graph sizes and degree stats (not a gate; run_all.sh never calls this file)
    for name, gen in GRAPHS:
        n, adj = gen()
        m = sum(len(a) for a in adj) // 2
        iso = sum(1 for a in adj if not a)
        print(name, 'n', n, 'm', m, 'isolated', iso)
