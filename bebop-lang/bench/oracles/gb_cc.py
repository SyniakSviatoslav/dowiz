#!/usr/bin/env python3
"""B3 prep gate oracle (pending, see gb_bfs.py header). Connected-components label sum
(label = min node id per component) over the 3 standard graphs, combined fold."""
import lag_common as L
print(L.run_over_graphs(L.connected_components))
