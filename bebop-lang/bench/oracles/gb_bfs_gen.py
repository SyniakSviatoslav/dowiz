#!/usr/bin/env python3
"""B3 step 4 design item (f) gate oracle for bench/vs_rust/std_tests/gb_bfs_gen.bp /
selfhost/std/gen_gb.bp's op=5 (bfs) generated kernel. Plain queue BFS over the SAME 1k-node/
4000-edge random_lcg() graph gb_bfs.py/gb_gen already gate against (lag_common.random_lcg,
seed=42), fold = sum over reached vertices of (level+1) -- the SAME formula selfhost/std/
sgraph2.bp's bfs_from/bfs_frontier return (unreached vertices contribute 0, NOT gb_bfs.bp's own
sum(level)-with-1-penalty convention -- a different fold on purpose, see gb_bfs_gen.bp's header)."""
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lag_common import random_lcg


def frontier_fold(n, adj, src=0):
    level = [-1] * n
    level[src] = 0
    frontier = [src]
    acc = 1  # level[src] + 1
    d = 0
    while frontier:
        d += 1
        nxt = []
        for u in frontier:
            for v in adj[u]:
                if level[v] == -1:
                    level[v] = d
                    nxt.append(v)
                    acc += d + 1
        frontier = nxt
    return acc


n, adj = random_lcg()
print(frontier_fold(n, adj, 0))
