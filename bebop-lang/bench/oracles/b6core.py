#!/usr/bin/env python3
# b6core oracle: compute fold for SCAN and GATHER at n=10^7 i64 = 80 MB
# (blueprint §2.2-2.3: LCG fill, fold_scan = sum of array elements)
# When called with shape=0 (SCAN), output is the sequential fold_scan result.

def lcg(x):
    return (x * 6364136223846793005 + 1442695040888963407) & ((1 << 64) - 1)

def fill_scan(n):
    """Fill array with LCG and return sum (fold_scan)"""
    x = 12345
    fold = 0
    for i in range(n):
        x = lcg(x)
        val = x if x < (1 << 63) else x - (1 << 64)
        fold = fold + val
    fold_unsigned = fold & ((1 << 64) - 1)
    # Convert to signed i64
    return fold_unsigned if fold_unsigned < (1 << 63) else fold_unsigned - (1 << 64)

def fill_gather(n):
    """Fill array and idx with LCG, return sum (fold_gather)"""
    # First fill array like scan
    x = 12345
    a = []
    for i in range(n):
        x = lcg(x)
        val = x if x < (1 << 63) else x - (1 << 64)
        a.append(val)

    # Then fill idx array
    idx = []
    for i in range(n):
        x = lcg(x)
        idx.append(x)

    # Compute fold_gather: sum of a[idx[i] & (n-1)] for all i
    fold = 0
    for i in range(n):
        fold = fold + a[idx[i] & (n - 1)]

    fold_unsigned = fold & ((1 << 64) - 1)
    # Convert to signed i64
    return fold_unsigned if fold_unsigned < (1 << 63) else fold_unsigned - (1 << 64)

n = 8388608  # 1<<23: `idx[i] & (n-1)` is only a MASK when n is a power of two (see b6core.bp)
# Compute both folds (golden is SCAN at W=1 for std_golden registration)
fold_scan = fill_scan(n)
print(fold_scan)
