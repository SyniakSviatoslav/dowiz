# oracle for sdiag gate: string diagrams as open hypergraphs with Z2 wires.
# Independent model: wires = (parity), boxes = (label, ins, outs), interfaces.
P = 1000000007
def hm(h, x): return (h * 131 + x + 1) % P

class D:
    def __init__(s): s.par = []; s.boxes = []; s.ii = []; s.io = []
    def wire(s, p): s.par.append(p); return len(s.par) - 1
    def box(s, label, ins, outs): s.boxes.append((label, list(ins), list(outs))); return len(s.boxes) - 1

def one(label, pin, pout):
    d = D(); a = d.wire(pin); b = d.wire(pout); d.box(label, [a], [b]); d.ii = [a]; d.io = [b]; return d

def seq(a, b):
    if len(a.io) != len(b.ii): return -1, None
    if any(a.par[a.io[k]] != b.par[b.ii[k]] for k in range(len(a.io))): return -1, None
    c = D(); c.par = list(a.par)
    m = {}
    for k in range(len(b.ii)): m[b.ii[k]] = a.io[k]
    for w in range(len(b.par)):
        if w not in m: m[w] = c.wire(b.par[w])
    c.boxes = [(l, list(i), list(o)) for l, i, o in a.boxes]
    c.boxes += [(l, [m[x] for x in i], [m[x] for x in o]) for l, i, o in b.boxes]
    c.ii = list(a.ii); c.io = [m[x] for x in b.io]
    return len(c.boxes), c

def par(a, b):
    c = D(); c.par = a.par + b.par; n = len(a.par)
    c.boxes = [(l, list(i), list(o)) for l, i, o in a.boxes]
    c.boxes += [(l, [x + n for x in i], [x + n for x in o]) for l, i, o in b.boxes]
    c.ii = a.ii + [x + n for x in b.ii]; c.io = a.io + [x + n for x in b.io]
    return len(c.boxes), c

def canon(d):
    wd = {}
    for k, w in enumerate(d.ii): wd[w] = hm(hm(1, k), d.par[w])
    alive = [b for b in range(len(d.boxes)) if d.boxes[b][0] != 0]
    acc = hm(hm(hm(0, len(alive)), len(d.ii)), len(d.io))
    done = set()
    for _ in range(len(d.boxes)):
        best = None
        for b in alive:
            if b in done: continue
            l, i, o = d.boxes[b]
            if not all(x in wd for x in i): continue
            cd = hm(hm(hm(7, l), len(i)), len(o))
            for x in i: cd = hm(cd, wd[x])
            if best is None or cd < best[0]: best = (cd, b)
        if best is None: break
        cd, b = best; done.add(b); acc = hm(acc, cd)
        for j, x in enumerate(d.boxes[b][2]): wd[x] = hm(hm(cd, 2 + j), d.par[x])
    for w in d.io: acc = hm(acc, wd.get(w, -1))
    return acc

def diamond():
    d = D(); w = [d.wire(p) for p in (0, 1, 0, 1, 0, 1)]
    d.box(5, [w[0]], [w[1], w[2]]); d.box(6, [w[1]], [w[3]]); d.box(7, [w[2]], [w[4]]); d.box(8, [w[3], w[4]], [w[5]])
    d.ii = [w[0]]; d.io = [w[5]]; return d

def diamond_perm():
    d = D(); r = [d.wire(p) for p in (1, 0, 1, 0, 1, 0)]; w = r[::-1]
    d.box(8, [w[3], w[4]], [w[5]]); d.box(7, [w[2]], [w[4]]); d.box(6, [w[1]], [w[3]]); d.box(5, [w[0]], [w[1], w[2]])
    d.ii = [w[0]]; d.io = [w[5]]; return d

f, g, h, k = one(1, 0, 1), one(2, 1, 0), one(3, 1, 0), one(4, 0, 1)
_, fg = par(f, g); _, hk = par(h, k); nl, lhs = seq(fg, hk)
n1, fh = seq(f, h); n2, gk = seq(g, k); _, rhs = par(fh, gk)
cl, cr = canon(lhs), canon(rhs)
acc = hm(hm(0, cl), 1 if cl == cr else 0)
acc = hm(acc, nl * 100 + n1 * 10 + n2)
t1, _ = seq(f, f); t2, _ = seq(f, fg); t3, gf = seq(g, f)
acc = hm(acc, (t1 == -1) + 2 * (t2 == -1) + 4 * (t3 == 2))
acc = hm(acc, canon(gf))  # slot reused by the successful g;f
d0, d1 = diamond(), diamond_perm()
c0, c1 = canon(d0), canon(d1)
acc = hm(hm(acc, c0), 1 if c0 == c1 else 0)
hp = one(2, 1, 1)
n3, e0 = seq(d0, hp); n4, e1 = seq(d1, hp)
c2, c3 = canon(e0), canon(e1)
acc = hm(hm(acc, c2), (c2 == c3) + n3 * 2 + n4 * 16)
print(acc)
