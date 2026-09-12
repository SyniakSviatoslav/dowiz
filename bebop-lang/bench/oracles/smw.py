#!/usr/bin/env python3
"""Oracle for smw (G10): P writer threads, disjoint partitions, cross-partition tx every 100th.
Closed form: fold = P * (N*(N+1)//2 + 100 * (M*(M+1)//2)) where M = N//100
Usage: python3 smw.py <P> <N>  -- prints fold as last line."""

import sys

def main():
    if len(sys.argv) < 3:
        print("Usage: smw.py <P> <N>", file=sys.stderr)
        sys.exit(2)
    P = int(sys.argv[1])
    N = int(sys.argv[2])
    M = N // 100
    fold = P * (N * (N + 1) // 2 + 100 * (M * (M + 1) // 2))
    print(fold)

if __name__ == "__main__":
    main()
