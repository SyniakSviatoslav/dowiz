#!/usr/bin/env python3
"""B3 prep gate oracle (pending -- no bebop gb_bfs counterpart yet, see
docs/blueprints/B3-graphblas-kernels-prejit.md section 6 and bench/vs_rust/PREP-b1-b3-b7.md).
BFS levels sum over the 3 standard graphs (bench/oracles/lag_common.py), combined into one
fold (run_all.sh's `tail -1` convention). No gate line exists yet in std_golden.sh -- this
file is unregistered on purpose (see PREP note) and cannot turn any lane RED."""
import lag_common as L
print(L.run_over_graphs(L.bfs_levels))
