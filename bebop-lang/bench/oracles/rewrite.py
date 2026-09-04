# Oracle for gate `rewrite` (T31): rewriting string-diagram terms to normal
# form. Independent tree model: m(children) = n-ary monoid product (m() =
# unit), tok = Petri token, t(children) = transition (one output), var k =
# free variable (interface input k), hole k = query pattern variable.
# Rules (all strictly decrease the box count):
#   flatten  m(.., m(cs), ..) -> m(.., cs.., ..)      (assoc; unit = empty cs)
#   erase    m(c) -> c
#   fire     t(tok..tok) -> tok
#   badfire  t(tok..tok) -> m()   (BAD rule set only; conflicts with fire)
# Loader = exhaustive one-step-reduct pairs on small terms, each reduct
# normalized by the first-redex strategy and canonised with the T30 hash;
# accepted iff every pair joins. Canon reimplemented on the tree -> diagram.
P = 1000000007
M62 = (1 << 62) - 1
def hm(h, x): return (h * 131 + x + 1) % P
def mix(h, x): return ((h * 1000003) + x) & M62

def m(*cs): return ('m', tuple(cs))
def t(*cs): return ('t', tuple(cs))
tok = ('tok',)
def var(k): return ('var', k)
def hole(k): return ('hole', k)

def size(x):                      # alive boxes
    if x[0] == 'var': return 0
    return 1 + sum(size(c) for c in x[1]) if x[0] in ('m', 't') else 1

def reducts(x, bad):
    """one-step reducts in redex order: this node (flatten per input, erase,
    fire, badfire) then inside children; the first entry is the leftmost redex."""
    out = []
    if x[0] == 'm':
        cs = x[1]
        for i, c in enumerate(cs):
            if c[0] == 'm' and len(cs) + len(c[1]) - 1 <= 6:
                out.append(m(*(cs[:i] + c[1] + cs[i + 1:])))
        if len(cs) == 1: out.append(cs[0])
    if x[0] == 't' and len(x[1]) >= 1 and all(c == tok for c in x[1]):
        out.append(tok)
        if bad: out.append(m())
    if x[0] in ('m', 't'):
        for i, c in enumerate(x[1]):
            for r in reducts(c, bad):
                out.append((x[0], x[1][:i] + (r,) + x[1][i + 1:]))
    return out

def nf(x, bad, pick):
    steps = 0
    while True:
        rs = reducts(x, bad)
        if not rs: return x, steps
        nxt = rs[0] if pick == 0 else rs[-1]
        assert size(nxt) < size(x)          # termination measure
        x = nxt; steps += 1

class D:
    def __init__(s): s.par = []; s.boxes = []; s.ii = {}; s.io = []
    def wire(s): s.par.append(0); return len(s.par) - 1

def build(d, x):
    w = d.wire()
    if x[0] == 'var': d.ii[x[1]] = w; return w
    if x[0] == 'tok': d.boxes.append((30, [], [w])); return w
    if x[0] == 'hole': d.boxes.append((40 + x[1], [], [w])); return w
    ins = [build(d, c) for c in x[1]]
    d.boxes.append((20 if x[0] == 'm' else 50, ins, [w]))
    return w

def canon(x):
    d = D(); d.io = [build(d, x)]
    ii = [d.ii[k] for k in range(len(d.ii))]
    wd = {}
    for k, w in enumerate(ii): wd[w] = hm(hm(1, k), d.par[w])
    acc = hm(hm(hm(0, len(d.boxes)), len(ii)), len(d.io))
    done = set()
    for _ in range(len(d.boxes)):
        best = None
        for b, (l, i, o) in enumerate(d.boxes):
            if b in done or not all(y in wd for y in i): continue
            cd = hm(hm(hm(7, l), len(i)), len(o))
            for y in i: cd = hm(cd, wd[y])
            if best is None or cd < best[0]: best = (cd, b)
        if best is None: break
        cd, b = best; done.add(b); acc = hm(acc, cd)
        for j, y in enumerate(d.boxes[b][2]): wd[y] = hm(hm(cd, 2 + j), d.par[y])
    for w in d.io: acc = hm(acc, wd.get(w, -1))
    return acc

def cpairs(terms, bad):
    total = joined = 0
    for x in terms:
        cs = [canon(nf(r, bad, 0)[0]) for r in reducts(x, bad)]
        for k in range(len(cs)):
            for l in range(k + 1, len(cs)):
                total += 1; joined += cs[k] == cs[l]
    return total, joined

def unify(p, s, env):
    if p[0] == 'hole':
        if p[1] in env: return env[p[1]] == s
        env[p[1]] = s; return True
    if p[0] != s[0]: return False
    if p[0] in ('m', 't'):
        return len(p[1]) == len(s[1]) and all(unify(a, b, env) for a, b in zip(p[1], s[1]))
    return p == s

x, y, z = var(0), var(1), var(2)
M = m(m(t(tok, tok), m()), m(m(x), m(y, t(tok))))
F = [m(m(x, m()), y), m(m(), m()), t(tok), m(m(x), m(t(tok, tok))), m(m(m()), t(tok))]
S = [m(m(x, y), z), m(m(tok, tok), x), m(tok, tok), t(tok), m(x, m(y, z))]
Q = [m(m(hole(0), hole(1)), hole(2)), m(hole(0), hole(0)), hole(0), m(hole(0), tok, hole(1))]

ptg, pjg = cpairs(F, False)
ptb, pjb = cpairs(F, True)
accg = ptg == pjg; rejb = ptb != pjb
na, _ = nf(M, False, 0); nb, _ = nf(M, False, 1)
ca, cb = canon(na), canon(nb); same = ca == cb
redn = size(M) - size(na)
na2, s2 = nf(na, False, 0); fix = s2 == 0 and canon(na2) == ca
dec = 1                                    # asserted inside nf on every step
h = mix(mix(0, ca), cb)
h = mix(mix(mix(mix(h, ptg), pjg), ptb), pjb)
SN = [nf(s, False, 0)[0] for s in S]
for s in SN: h = mix(h, canon(s))
QN = [nf(q, False, 0)[0] for q in Q]
for q in QN: h = mix(h, canon(q))
nmatch = sum(unify(q, s, {}) for q in QN for s in SN)
h = mix(h, redn)
print((h % 1000000000) * 100000 + nmatch * 100 + accg * 16 + rejb * 8 + same * 4 + dec * 2 + fix)
