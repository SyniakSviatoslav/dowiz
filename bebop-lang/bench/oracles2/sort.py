#!/usr/bin/env python3
"""oracles2/sort -- gate `sort`.  PROSE AVAILABLE: sort.bp first comment only --
"in-place integer sorting over [i64]; is_sorted: 1 if a[0..n) ascending else 0;
find_min_idx: index of the smallest element in a[start..n)" (=> selection sort).
NOT IN ANY PROSE: the driver's input array, n, and the fold.  Every one of those is
a parameter below (env N, SEED, FOLD); the algorithm part (selection sort with
find_min_idx = FIRST minimal index, is_sorted) is exact.
Default guess: n=16 cells from the prelude MMIX LCG (seed 1), values = state>>48,
fold = poly31 over the sorted cells then *10 + is_sorted."""
import os
M64 = (1 << 64) - 1
def s64(x): x &= M64; return x - (1 << 64) if x >> 63 else x
N = int(os.environ.get("N", "16")); SEED = int(os.environ.get("SEED", "1"))
def lcg(s): return (s * 6364136223846793005 + 1442695040888963407) & M64
def find_min_idx(a, start, n):
    m = start
    for i in range(start + 1, n):
        if a[i] < a[m]: m = i
    return m
def selection_sort(a, n):
    for i in range(n):
        j = find_min_idx(a, i, n)
        a[i], a[j] = a[j], a[i]
def is_sorted(a, n): return int(all(a[i] <= a[i + 1] for i in range(n - 1)))
s = SEED; a = []
for _ in range(N):
    s = lcg(s); a.append(s >> 48)
orig = list(a)
selection_sort(a, N)
assert a == sorted(orig)
h = 0
for x in a: h = s64(h * 31 + x)
print("input", orig); print("sorted", a)
print(s64(h * 10 + is_sorted(a, N)))
