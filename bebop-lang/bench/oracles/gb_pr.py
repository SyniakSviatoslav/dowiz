#!/usr/bin/env python3
"""B3 prep gate oracle (pending, see gb_bfs.py header). PageRank in Q32 fixed point (10
iterations, d=0.85, integer arithmetic only -- lag_common.pagerank_q32) over the 3 standard
graphs, combined fold."""
import lag_common as L
print(L.run_over_graphs(L.pagerank_q32))
