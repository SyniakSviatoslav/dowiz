# dpll.py - oracle for dpll.bp (T86): bounded bit-vector DPLL, <=16 vars.
# Clause = (pos, neg) bitmask pair over vars 0..15; partial assignment =
# (A assigned-mask, V value-mask, V subset of A). Unit propagation = passes
# over the clauses in order until A stops changing or a clause is falsified;
# branching = lowest unassigned var, TRUE first, explicit stack (DFS, the
# same order a recursive solver visits). Result per instance: -1 budget
# exhausted, 0 UNSAT, V+1 SAT (V = the model mask). nodes = stack pops.
# fold = fold_{k}: (fold*1000003 + (r+2)*4096 + nodes) % 1000000007.
# `python3 dpll.py --emit` prints the .bp table literals (same encoding).
import sys

def ph(p, h):  # pigeonhole p pigeons -> h holes, var(i,j) = i*h+j+1
    v = lambda i, j: i * h + j + 1
    cs = [[v(i, j) for j in range(h)] for i in range(p)]
    for j in range(h):
        for i in range(p):
            for k in range(i + 1, p):
                cs.append([-v(i, j), -v(k, j)])
    return cs

INST = [
    # ---- SAT ----
    (1, [[1]]),
    (2, [[1, 2], [-1, 2]]),
    (4, [[1], [-1, 2], [-2, 3], [-3, 4]]),
    (3, [[1, 2, 3], [-1, 2], [-2, 3], [-3, -1]]),
    (4, ph(2, 2)),
    (8, [[i + 1, (i + 1) % 8 + 1] for i in range(8)]),
    (16, [[i + 1, -(i + 2)] for i in range(15)] + [[16]]),
    (3, [[1, 2], [-1, -2], [2, 3]]),
    (5, sum([[[i + 1, i + 2], [-(i + 1), -(i + 2)]] for i in range(4)], [])),
    (4, [[-1, -2], [-2, -3], [-3, -4], [1, 3]]),
    # ---- UNSAT ----
    (1, [[1], [-1]]),
    (2, [[1, 2], [-1, 2], [1, -2], [-1, -2]]),
    (6, ph(3, 2)),
    (3, [[s1 * 1, s2 * 2, s3 * 3] for s1 in (1, -1) for s2 in (1, -1) for s3 in (1, -1)]),
    (4, [[1], [-1, 2], [-2, 3], [-3, 4], [-4]]),
    (3, [[1, 2], [-1, -2], [2, 3], [-2, -3], [3, 1], [-3, -1]]),
    (12, ph(4, 3)),
    (3, [[-1, 2], [1, -2], [-2, 3], [2, -3], [1, 3], [-1, -3]]),
    (4, [[1, 2], [1, -2], [-1, 3], [-1, -3], [4, 2]]),
    (16, [[1]] + [[-(i + 1), i + 2] for i in range(15)] + [[-16]]),
]
BUDGET = 2000
M = 1000000007

def masks(cs):
    out = []
    for c in cs:
        p = n = 0
        for l in c:
            if l > 0: p |= 1 << (l - 1)
            else: n |= 1 << (-l - 1)
        out.append((p, n))
    return out

def prop(cls, A, V):
    full = 0xFFFF
    go = 1; conf = 0; ns = 0
    while go:
        a0 = A; conf = 0; ns = 0
        for p, n in cls:
            sat = 1 if ((p & V) | (n & A & (full ^ V))) != 0 else 0
            unl = (p | n) & (full ^ A)
            unit = (1 - sat) * (1 if unl != 0 else 0) * (1 if (unl & (unl - 1)) == 0 else 0)
            A = A | (unl * unit)
            V = V | ((unl & p) * unit)
            conf = conf + (1 - sat) * (1 if unl == 0 else 0)
            ns = ns + sat
        go = (1 if A != a0 else 0) * (1 if conf == 0 else 0)
    return A, V, conf, ns

def solve(nv, cls):
    full = (1 << nv) - 1
    stack = [(0, 0)]
    res = 0; nodes = 0
    while stack and res == 0 and nodes < BUDGET:
        A, V = stack.pop()
        nodes += 1
        A, V, conf, ns = prop(cls, A, V)
        if conf: continue
        if ns == len(cls):
            res = V + 1
            continue
        u = (full ^ A) & -(full ^ A)
        stack.append((A | u, V))       # false branch (tried later)
        stack.append((A | u, V | u))   # true branch (tried first)
    if res == 0 and stack and nodes >= BUDGET:
        res = -1
    return res, nodes

def fold():
    f = 0
    for nv, cs in INST:
        r, nodes = solve(nv, masks(cs))
        f = (f * 1000003 + (r + 2) * 4096 + nodes) % M
    return f

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--emit":
        cl = []; lo = [0]; nvs = []
        for nv, cs in INST:
            for p, n in masks(cs): cl += [p, n]
            lo.append(len(cl) // 2); nvs.append(nv)
        print("  let cl = [" + ", ".join(map(str, cl)) + "];")
        print("  let lo = [" + ", ".join(map(str, lo)) + "];")
        print("  let nv = [" + ", ".join(map(str, nvs)) + "];")
    else:
        print(fold())
