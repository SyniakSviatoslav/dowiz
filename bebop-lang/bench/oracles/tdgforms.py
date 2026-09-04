#!/usr/bin/env python3
# Oracle for tdgforms gate (T19): differential forms on the periodic 3^4 grid
# in R^4, integer-valued. Wedge sign = parity of inversions of the concatenated
# index lists (alternating-tensor definition); d = sum_i dx^i ^ (forward
# difference along i). Checks d(d w)=0, antisymmetry, associativity, Stokes on
# unit squares; fold = ok*10^15 + sum|dw|*10^8 + sum|a^b|. Ints only.
from itertools import combinations

N, G = 4, 3
P = G ** N
MASKS = range(1 << N)

def bits(m):
    return [i for i in range(N) if m >> i & 1]

def wsign(I, J):
    if I & J:
        return 0
    li, lj = bits(I), bits(J)
    inv = sum(1 for x in li for y in lj if x > y)
    return -1 if inv & 1 else 1

def shp(p, i):
    pw = G ** i
    d = (p // pw) % G
    return p + (pw if d != G - 1 else -(G - 1) * pw)

def tform(k, mp, mi, ad, md, sb):
    f = {}
    for I in MASKS:
        for p in range(P):
            f[I, p] = ((mp * p + mi * I + ad) % md) - sb if len(bits(I)) == k else 0
    return f

def wedge(a, b):
    out = {(K, p): 0 for K in MASKS for p in range(P)}
    for I in MASKS:
        for J in MASKS:
            s = wsign(I, J)
            for p in range(P):
                out[I | J, p] += s * a[I, p] * b[J, p]
    return out

def ext_d(a):
    out = {(K, p): 0 for K in MASKS for p in range(P)}
    for i in range(N):
        ei = 1 << i
        for I in MASKS:
            s = wsign(ei, I)
            for p in range(P):
                out[I | ei, p] += s * (a[I, shp(p, i)] - a[I, p])
    return out

def sumabs(f):
    return sum(abs(v) for v in f.values())

def sumabsdiff(a, b, sg):
    return sum(abs(a[k] + sg * b[k]) for k in a)

def stokes(al, da):
    acc = 0
    for i, j in combinations(range(N), 2):
        mi, mj = 1 << i, 1 << j
        for p in range(P):
            pi, pj = shp(p, i), shp(p, j)
            circ = al[mi, p] + al[mj, pi] - al[mi, pj] - al[mj, p]
            acc += abs(da[mi | mj, p] - circ)
    return acc

w = tform(2, 7, 13, 5, 17, 8)
al = tform(1, 11, 3, 1, 13, 6)
be = tform(1, 5, 7, 2, 11, 5)
eta = ext_d(w)
zeta = ext_d(eta)
s_eta = sumabs(eta)
dd_ok = int(sumabs(zeta) == 0)
ab = wedge(al, be)
ba = wedge(be, al)
anti_ok = int(sumabsdiff(ab, ba, 1) == 0)
s_ab = sumabs(ab)
lhs = wedge(ab, w)
rhs = wedge(al, wedge(be, w))
assoc_ok = int(sumabsdiff(lhs, rhs, -1) == 0)
nz_ok = int(sumabs(lhs) > 0)
st_ok = int(stokes(al, ext_d(al)) == 0)
ok = dd_ok * anti_ok * assoc_ok * nz_ok * st_ok
print(ok * 10**15 + s_eta * 10**8 + s_ab)
