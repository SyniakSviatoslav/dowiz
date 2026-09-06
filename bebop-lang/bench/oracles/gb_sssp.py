#!/usr/bin/env python3
"""B3 prep gate oracle (pending, see gb_bfs.py header). SSSP (min-plus, Dijkstra) distance
sum over the 3 standard graphs with the deterministic per-edge weight in lag_common.py,
combined fold."""
import lag_common as L
print(L.run_over_graphs(L.sssp_minplus))
