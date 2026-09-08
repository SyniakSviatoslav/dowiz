#!/usr/bin/env python3
"""ROADMAP E1 gate oracle for bench/vs_rust/std_tests/gb_mbfs_gen.bp / selfhost/std/gen_gb.bp's
op=6 (mbfs) generated 64-source bit-parallel kernel. 64 SEPARATE plain queue BFS runs over the
SAME 1k-node/4000-edge random_lcg() graph gb_bfs_gen.py already gates against (lag_common.
random_lcg, seed=42), each folded as sum over reached vertices of (level+1) -- the SAME formula
bench/oracles/gb_bfs_gen.py uses -- and summed. That sum IS the gate: a bit-parallel traversal
that drops a source is fast and wrong, and only a fold built from separate runs catches it.
The 64 sources are the LCG(31337) list gmb_src/mbfs_srcv walk: x advanced i+1 times, then
(x >> 20) % n with a LOGICAL shift of the 64-bit pattern (bebop's `>>`; `>>>` is the arithmetic
form). x >> 20 is < 2**44, so the signed `%` bebop runs and python's `%` agree here."""
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lag_common import random_lcg

A = 6364136223846793005
C = 1442695040888963407
M64 = (1 << 64) - 1


def sources(n, count):
    out = []
    x = 31337
    for _ in range(count):
        x = (x * A + C) & M64
        out.append((x >> 20) % n)
    return out


def frontier_fold(n, adj, src):
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
print(sum(frontier_fold(n, adj, s) for s in sources(n, 64)))
