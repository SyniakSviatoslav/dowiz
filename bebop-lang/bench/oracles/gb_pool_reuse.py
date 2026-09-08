#!/usr/bin/env python3
"""ROADMAP B3 defect gate oracle (run_all.sh convention: <gate>.py prints the gate value as its
last line). gb_pool_reuse re-runs bench/vs_rust/std_tests/gb_pool.bp against a pool file that is
ALREADY populated -- once with the same compiler_digest and once with a foreign one -- and both
must return the same value a fresh pool returns. A pool is a CACHE: a hit must return exactly
what a rebuild would have returned, and a miss must be rebuilt rather than dispatched. So the
golden is not a new number, it is gb_pool's own -- which is gb_gen's tier-0 fold, since
gb.bp's gb_mxv_generic is the single source of truth every tier reads through."""
import gb_lagraph as G
print(G.gb_gen_combined()[0])
