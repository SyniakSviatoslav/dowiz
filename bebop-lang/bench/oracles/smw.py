#!/usr/bin/env python3
"""Oracle for smw (G10): P writer threads, disjoint partitions, cross-partition tx every 100th.
Closed form: fold = P * (N*(N+1)//2)  +  (0 if P==1 else P) * (100 * (M*(M+1)//2))
where M = N//100. Note: at P=1, cross_commit's cp = (p+1) % P equals p, so ta and tb
are the SAME transaction slot; the second st_begin_p restarts it, leaving only one
counter object per cross update instead of two. Cross term exists only for P >= 2.
Usage: python3 smw.py [<P> <N>]  -- defaults to P=3 N=100000 (from std_golden.sh:721)"""

import sys

# Defaults match std_golden.sh:721 where gate runs smw_test.bin 3 100000
DEFAULT_P = 3
DEFAULT_N = 100000

def main():
    if len(sys.argv) == 1:
        # No arguments: use defaults
        P = DEFAULT_P
        N = DEFAULT_N
    elif len(sys.argv) == 3:
        # Both arguments provided
        P = int(sys.argv[1])
        N = int(sys.argv[2])
    else:
        print("Usage: smw.py [<P> <N>]", file=sys.stderr)
        sys.exit(2)

    M = N // 100
    # Regular updates: P * sum(1..N) = P * N*(N+1)//2
    regular_fold = P * (N * (N + 1) // 2)
    # Cross-partition term: only for P >= 2 (at P=1, tx slot is reused, one object per cross update)
    cross_multiplier = 0 if P == 1 else P
    cross_fold = cross_multiplier * (100 * (M * (M + 1) // 2))
    fold = regular_fold + cross_fold
    print(fold)

if __name__ == "__main__":
    main()
