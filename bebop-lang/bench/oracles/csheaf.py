# csheaf.py - oracle for csheaf.bp (T29): content-addressable sheaf nodes.
# Sheaf of sheaf.bp (5 nodes, 6 edges, unimodular maps, consistent section).
# Record = (v, x0, x1); address = FNV-1a-64 over the 3 words (little-endian
# bytes); store = 16-slot open-addressing table, slot = digest & 15, linear
# probing; addr[v] = digest of the stalk stored at v (ptrless reference).
# insert checks delta == 0 against every resolvable neighbour; verify(v)
# = resolve + re-hash == key + delta check. Phase address: angle = digest >> 32
# (fp 2^32 fraction of a turn), bucket = angle >> 29 (8 rotor sectors).
M64 = (1 << 64) - 1
S = 1 << 32
ET = [0, 1, 2, 3, 0, 1]
EH = [1, 2, 3, 4, 2, 4]
I2 = [1, 0, 0, 1]

def mul(a, b):
    return [a[0]*b[0]+a[1]*b[2], a[0]*b[1]+a[1]*b[3],
            a[2]*b[0]+a[3]*b[2], a[2]*b[1]+a[3]*b[3]]

def mv(a, v):
    return [a[0]*v[0]+a[1]*v[1], a[2]*v[0]+a[3]*v[1]]

T0 = [1, 1, 0, 1]; T1 = [1, 0, 1, 1]; T2 = [0, 1, -1, 0]; T3 = [1, -1, 0, 1]
B4 = [1, 1, 0, 1]; B5 = [1, 0, -1, 1]
T = [T0, T1, T2, T3, mul(B4, mul(T1, T0)), mul(B5, mul(T3, mul(T2, T1)))]
H = [I2, I2, I2, I2, B4, B5]
X = [[3*S, 2*S]]
for e in range(4):
    X.append(mv(T[e], X[e]))

def fnv3(v, a, b):
    h = 0xcbf29ce484222325
    for w in (v, a, b):
        w &= M64
        for sh in range(8):
            h = ((h ^ ((w >> (sh*8)) & 255)) * 0x100000001b3) & M64
    return h

keys = [0]*16; vals = [[0, 0, 0] for _ in range(16)]; addr = [0]*5

def probe(d):
    s = d & 15
    while keys[s] != 0 and keys[s] != d:
        s = (s + 1) & 15
    return s

def resolve(d):
    if d == 0: return -1
    s = probe(d)
    return s if keys[s] == d else -1

def check(v, x0, x1):
    ok = 1
    for e in range(6):
        if v not in (ET[e], EH[e]): continue
        u = EH[e] if ET[e] == v else ET[e]
        s = resolve(addr[u])
        if s < 0: continue
        xu = vals[s][1:]
        mine, other = (T[e], H[e]) if ET[e] == v else (H[e], T[e])
        r = [a-b for a, b in zip(mv(mine, [x0, x1]), mv(other, xu))]
        if r != [0, 0]: ok = 0
    return ok

def insert(v, x0, x1):
    ok = check(v, x0, x1)
    if ok:
        d = fnv3(v, x0, x1); s = probe(d)
        keys[s] = d; vals[s] = [v, x0, x1]; addr[v] = d
    return ok

def verify(v):
    s = resolve(addr[v])
    if s < 0: return 0
    rv, a, b = vals[s]
    if fnv3(rv, a, b) != keys[s]: return 0
    return check(rv, a, b)

ins = sum(insert(v, X[v][0], X[v][1]) for v in range(5))
rej = 1 - insert(2, X[2][0] + S, X[2][1])          # inconsistent stalk -> rejected
res_ok = sum(1 for v in range(5) if resolve(addr[v]) >= 0)
ver_ok = sum(verify(v) for v in range(5))
hist = [0]*8
for v in range(5):
    hist[((addr[v] >> 32) >> 29) & 7] += 1
hpack = sum(hist[b] << (3*b) for b in range(8))
# corrupt ONE key (flip bit 7 of the slot holding v = 3)
keys[resolve(addr[3])] ^= 128
after = sum(verify(v) for v in range(5))
bad_v = sum(v for v in range(5) if verify(v) == 0)
fold = (ins*10**15 + rej*10**14 + res_ok*10**13 + ver_ok*10**12
        + after*10**11 + bad_v*10**10 + hpack)
print(fold)
