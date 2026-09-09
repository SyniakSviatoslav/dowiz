#!/usr/bin/env python3
"""B3 step 4 design item (f) gate oracle for the _addr (fmt=2) variant of gen_gb.bp's op=5
(bfs) generated kernel -- the shape selfhost/std/sgraph2.bp's bfs_frontier dispatches via
fork+sys_run with an already-mmap'd store base + GbMatrix cell offset as decimal argv
(bench/vs_rust/std_tests/gb_bfs_gen_addr.bp), as opposed to gb_bfs_gen.py's _path (fmt=1)
shape that maps the store itself.

The two variants share one core text (gen_bfs_core) and differ ONLY in main's argv
unpacking, so the fold is IDENTICAL: sum over reached vertices of (level+1) over the SAME
1k-node/4000-edge random_lcg() graph, unreached contribute 0. A broken _addr main (wrong
argv index, st_cells vs st_map_ro confusion) changes which graph is traversed and moves
this fold, which is exactly what the gate checks. Usage: `gb_bfs_gen_addr.py [src]`,
default source 0 -> 997."""
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
src = int(sys.argv[1]) if len(sys.argv) > 1 else 0
print(frontier_fold(n, adj, src))
