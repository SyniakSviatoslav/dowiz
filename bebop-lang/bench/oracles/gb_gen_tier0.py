#!/usr/bin/env python3
"""B3 gate oracle (run_all.sh convention: <gate>.py prints the gate value as its last line).
The value is computed independently in gb_lagraph.py (python mirrors of gb.bp's generators and ops).
2026-09-07: replaces the unregistered B3-prep placeholder of the same name."""
import gb_lagraph as G
print(G.gb_gen_combined()[0])
