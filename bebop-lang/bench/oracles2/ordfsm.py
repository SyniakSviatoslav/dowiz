#!/usr/bin/env python3
"""oracles2/ordfsm -- gate `ordfsm`: independent Python port of PRODUCTION
crates/dowiz-core/src/order_machine.rs (allowed_next, assert_transition,
fold_transitions, reachable BFS, Kahn topological_order, has_cycle, cyclomatic,
fsm_graph_report, verify_fsm_signature) driven by the T66 harness (12x12 decide
matrix, 16 packed-nibble hand sequences, 64 LCG sequences seed 1234, structural
signature) as documented in bench/oracles/rust/src/bin/ordfsm.rs and the ordfsm.bp
first comment.  Fold = (h % 1e9)*1e6 + oks*1000 + errs."""
M62 = (1 << 62) - 1
def lcg(s): return (s * 1103515245 + 12345) & 2147483647
def mix(h, x): return (h * 1000003 + x) & M62
# idx_of order: 0 Pending 1 Confirmed 2 Preparing 3 Ready 4 InDelivery 5 Delivered
# 6 Rejected 7 Cancelled 8 Scheduled 9 PickedUp 10 Refunding 11 CompensatedRefund
N = 12
SUCC = {0: [1, 6, 7], 1: [2, 4, 10], 2: [3, 10], 3: [4, 9, 10], 4: [5, 10], 10: [11]}
ADJ = [sum(1 << t for t in SUCC.get(i, [])) for i in range(N)]
SCAFFOLD = {8}
def assert_transition(f, t):  # 0 ok | 1 SameStatus | 2 ScaffoldDisabled | 3 Illegal
    if f == t: return 1
    if t in SCAFFOLD or f in SCAFFOLD: return 2
    return 0 if ADJ[f] >> t & 1 else 3
def fold_transitions(start, steps):
    cur = start
    for nxt in steps:
        c = assert_transition(cur, nxt)
        if c: return ("err", c, cur)
        cur = nxt
    return ("ok", cur)
def reachable(f):
    seen, frontier = 0, 1 << f
    while frontier:
        nxt = 0
        for i in range(N):
            if frontier >> i & 1 and not seen >> i & 1:
                seen |= 1 << i; nxt |= ADJ[i]
        frontier = nxt & ~seen
    return seen
def topological_order():
    indeg = [0] * N
    for i in range(N):
        for t in range(N):
            if ADJ[i] >> t & 1: indeg[t] += 1
    queue = [i for i in range(N) if indeg[i] == 0]
    order = []
    while queue:
        u = queue.pop(0); order.append(u)
        for v in range(N):
            if ADJ[u] >> v & 1:
                indeg[v] -= 1
                if indeg[v] == 0: queue.append(v)
        queue.sort()   # Kahn with the lowest-index source first
    return order if len(order) == N else None
def has_cycle():
    visited, stack = [False] * N, [False] * N
    def dfs(i):
        visited[i] = stack[i] = True
        for j in range(N):
            if ADJ[i] >> j & 1:
                if not visited[j]:
                    if dfs(j): return True
                elif stack[j]: return True
        stack[i] = False; return False
    return any(not visited[i] and dfs(i) for i in range(N))
EDGES = sum(bin(a).count("1") for a in ADJ)
def cyclomatic():
    parent = list(range(N))
    def find(x):
        while parent[x] != x: parent[x] = parent[parent[x]]; x = parent[x]
        return x
    for i in range(N):
        for j in range(N):
            if ADJ[i] >> j & 1:
                a, b = find(i), find(j)
                if a != b: parent[a] = b
    comps = sum(1 for k in range(N) if find(k) == k)
    return EDGES - N + comps

h = oks = errs = 0
# A: exhaustive decide matrix
for f in range(N):
    for t in range(N):
        c = assert_transition(f, t)
        h = mix(h, c)
        if c: errs += 1
        else: oks += 1
def fold(start, steps):
    global h, oks, errs
    r = fold_transitions(start, steps)
    if r[0] == "ok": h = mix(mix(h, 1), r[1]); oks += 1
    else: h = mix(mix(h, 0), r[1] * 16 + r[2]); errs += 1
# B1: 16 hand sequences (packed nibbles: start, steps.., 15 sentinel)
HAND = [257176080, 16331280, 1004560, 1030672, 3936, 3952, 4221841936, 3840,
        3968, 3848, 987664, 3845, 4199821840, 3872, 4011, 1026576]
for v in HAND:
    start = v & 15; v >>= 4; steps = []
    while v & 15 != 15: steps.append(v & 15); v >>= 4
    fold(start, steps)
# B2: 64 LCG sequences, seed 1234
s = 1234
def d():
    global s
    s = lcg(s); return s
for _ in range(64):
    start = d() % 12
    ln = 1 + d() % 4
    prev, steps = start, []
    for _ in range(ln):
        r = d()
        nxt = (r >> 4) % 12 if r % 4 == 0 else (prev + 1 + (r >> 4) % 2) % 12
        steps.append(nxt); prev = nxt
    fold(start, steps)
# C: structural signature
reach0 = reachable(0)
topo = topological_order()
acyclic = not has_cycle()
mu = cyclomatic()
rstates = bin(reach0).count("1")
tlen = len(topo) if topo is not None else -1
for v in [N, EDGES, int(acyclic), mu, 0, reach0, rstates, tlen]:
    h = mix(h, v)
for st in range(N): h = mix(h, reachable(st))
if topo is not None:
    for st in topo: h = mix(h, st)
golden_ok = (N, EDGES, acyclic, mu, reach0, rstates, tlen) == (12, 14, True, 4, 3839, 11, 12)
h = mix(h, int(golden_ok))
print("V", N, "E", EDGES, "acyclic", acyclic, "mu", mu, "reach", reach0, "states", rstates,
      "topo", topo, "golden_ok", golden_ok, "oks", oks, "errs", errs)
print((h % 1000000000) * 1000000 + oks * 1000 + errs)
