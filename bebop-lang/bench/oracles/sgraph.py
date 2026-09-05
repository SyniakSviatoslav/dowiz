# G8 sgraph oracle (T117): array BFS (no networkx) over the LCG graph: 1M nodes, 5M pairs
# both directions; prints the bfs fold for S sources (argv[1], default 3) or, with 'nbr',
# the neighbour-query fold. Pure python: ~40 s for the adjacency build + ~12 s per source.
import sys
from array import array
M = (1 << 64) - 1; A = 6364136223846793005; C = 1442695040888963407
def lcg(x): return (x * A + C) & M
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
N = 1000000; E = 5000000
src = array('i', [0]) * E; dst = array('i', [0]) * E; deg = array('i', [0]) * (N + 1)
x = 4242
for i in range(E):
    x = lcg(x); a = (x >> 20) % N; x = lcg(x); b = (x >> 20) % N
    src[i] = a; dst[i] = b; deg[a + 1] += 1; deg[b + 1] += 1
rp = array('q', [0]) * (N + 1)
for v in range(N): rp[v + 1] = rp[v] + deg[v + 1]
cur = array('q', rp[:N]); ci = array('i', [0]) * (2 * E)
for i in range(E):
    a, b = src[i], dst[i]; ci[cur[a]] = b; cur[a] += 1; ci[cur[b]] = a; cur[b] += 1
mode = sys.argv[1] if len(sys.argv) > 1 else '3'
if mode == 'nbr':
    x = 4711; acc = 0
    for _ in range(100000):
        x = lcg(x); v = (x >> 20) % N; lo, hi = rp[v], rp[v + 1]
        acc = (acc + (hi - lo) + sum(ci[lo:hi])) & M
    print(s64(acc)); sys.exit(0)
S = int(mode); x = 31337; acc = 0
for _ in range(S):
    x = lcg(x); s = (x >> 20) % N
    level = array('i', [-1]) * N; level[s] = 0; q = [s]; head = 0; tot = 0
    while head < len(q):
        v = q[head]; head += 1; lv = level[v]; tot += lv + 1
        for w in ci[rp[v]:rp[v + 1]]:
            if level[w] < 0: level[w] = lv + 1; q.append(w)
    acc = (acc + tot) & M
print(s64(acc))
