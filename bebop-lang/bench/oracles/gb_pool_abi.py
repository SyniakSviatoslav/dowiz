#!/usr/bin/env python3
"""B3 step 4 gate oracle for gb_pool_abi (bench/vs_rust/std_tests/gb_pool_abi.bp): a Kernel
whose compiler_digest ("cmd") field mismatches must be a miss (K == 0), the correct one a hit
(K != 0). The probe folds this as 1000*(miss) + 1*(hit); by construction (a wrong cmd is always
wrong, a right cmd is always right) the correct outcome is deterministic -- 1001. No graph
simulation needed, unlike the other gb_* oracles."""
print(1001)
