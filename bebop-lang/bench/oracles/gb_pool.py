#!/usr/bin/env python3
"""B3 step 4 gate oracle (run_all.sh convention: <gate>.py prints the gate value as its last
line). gb_pool's driver (bench/vs_rust/std_tests/gb_pool.bp) dispatches all 12 kernels through
the pool (mmap + fork + sys_run, selfhost/std/gb_run.bp) and rolling-combines their folds the
same way bench/vs_rust/std_tests/gb_gen.bp's combine12 does over its own tier-0 folds -- by
design (gb.bp's gb_mxv_generic is the single source of truth every tier reads through) pool-hit
== tier-0 == this SAME golden, so this oracle just reuses gb_gen_tier0.py's computation."""
import gb_lagraph as G
print(G.gb_gen_combined()[0])
