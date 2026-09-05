# G8 stage-2 oracle (T117): L1 = the stage-1 CSR (5M LCG-4242 pairs, both directions);
# LOG = 1M directed edges from LCG 777 (100 batches of 10^4; L0 = CSR over the log);
# TOMB = 1M L1 slot positions from LCG 555 over 10M slots (a slot hit twice stays deleted).
# Modes: 'nbr' (10^5 queries, LCG 4711: deg + sum of neighbours over L1 minus tombstones
# plus L0), 'nbr0' (the same before any deletion), '<S>' (BFS fold from S sources, LCG
# 31337, over L1 minus tombstones plus L0), 'nbrlog' (nbr with the log but no deletes).
import sys
from array import array
M = (1 << 64) - 1; A = 6364136223846793005; C = 1442695040888963407
def lcg(x): return (x * A + C) & M
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
N = 1000000; E = 5000000
mode = sys.argv[1] if len(sys.argv) > 1 else 'all3'
with_log = True; with_del = True
src = array('i', [0]) * E; dst = array('i', [0]) * E; deg = array('i', [0]) * (N + 1); x = 4242
for i in range(E):
    x = lcg(x); a = (x >> 20) % N; x = lcg(x); b = (x >> 20) % N
    src[i] = a; dst[i] = b; deg[a + 1] += 1; deg[b + 1] += 1
rp = array('q', [0]) * (N + 1)
for v in range(N): rp[v + 1] = rp[v] + deg[v + 1]
cur = array('q', rp[:N]); ci = array('i', [0]) * (2 * E)
for i in range(E):
    a, b = src[i], dst[i]; ci[cur[a]] = b; cur[a] += 1; ci[cur[b]] = a; cur[b] += 1
tomb = bytearray(2 * E)
if with_del:
    x = 555
    for _ in range(1000000):
        x = lcg(x); tomb[(x >> 20) % (2 * E)] = 1
rp0 = array('q', [0]) * (N + 1); ci0 = array('i', [])
if with_log:
    ls = array('i', [0]) * 1000000; ld = array('i', [0]) * 1000000; d0 = array('i', [0]) * (N + 1); x = 777
    for i in range(1000000):
        x = lcg(x); a = (x >> 20) % N; x = lcg(x); c = (x >> 20) % N; ls[i] = a; ld[i] = c; d0[a + 1] += 1
    for v in range(N): rp0[v + 1] = rp0[v] + d0[v + 1]
    cur0 = array('q', rp0[:N]); ci0 = array('i', [0]) * 1000000
    for i in range(1000000):
        a = ls[i]; ci0[cur0[a]] = ld[i]; cur0[a] += 1
def nbr(v, use_log, use_del):
    acc = 0
    for k in range(rp[v], rp[v + 1]):
        if not (use_del and tomb[k]): acc += 1 + ci[k]
    if use_log:
        for k in range(rp0[v], rp0[v + 1]): acc += 1 + ci0[k]
    return acc
def nbr_fold(use_log, use_del):
    x = 4711; acc = 0
    for _ in range(100000):
        x = lcg(x); acc = (acc + nbr((x >> 20) % N, use_log, use_del)) & M
    return s64(acc)
def bfs_fold(S):
    x = 31337; acc = 0
    for _ in range(S):
        x = lcg(x); s = (x >> 20) % N
        level = array('i', [-1]) * N; level[s] = 0; q = [s]; head = 0; tot = 0
        while head < len(q):
            v = q[head]; head += 1; lv = level[v]; tot += lv + 1
            for k in range(rp[v], rp[v + 1]):
                w = ci[k]
                if not tomb[k] and level[w] < 0: level[w] = lv + 1; q.append(w)
            for k in range(rp0[v], rp0[v + 1]):
                w = ci0[k]
                if level[w] < 0: level[w] = lv + 1; q.append(w)
        acc = (acc + tot) & M
    return s64(acc)
S = int(mode[3:]) if mode.startswith('all') else 3
print('nbr0', nbr_fold(False, False)); print('nbrlog', nbr_fold(True, False)); print('nbr', nbr_fold(True, True)); print('bfs', bfs_fold(S))
