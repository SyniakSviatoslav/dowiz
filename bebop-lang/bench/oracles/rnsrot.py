# rnsrot: Cl(3) rotor sandwich R e_k R~ (R = 1+e12) on e1,e2 via 2-bit-coefficient multivector packing; every coefficient
# round-trips through a 4-modulus RNS (Garner decode). fold = digest(sum (acc[b]+8)*10^b) + eq*10^12
def tdiv(a, b): return -(-a // b) if (a < 0) != (b < 0) else a // b
def unpack(p): return [((p >> (2 * i)) & 3) - 1 for i in range(8)]
def pack(acc): return sum((a + 1) << (2 * k) for k, a in enumerate(acc))
def gprod(pa, pb, acc):
    for i in range(8):
        for j in range(8):
            t = sum(1 for x in range(3) for y in range(3) if x > y and (i >> x) & 1 and (j >> y) & 1)
            acc[i ^ j] += unpack(pa)[i] * unpack(pb)[j] * (-1) ** t
base = 21845
rp, rt, e1p, e2p = base + 1 + 64, base + 1 - 64, base + 4, base + 16
t1 = [0] * 8; gprod(rp, e1p, t1); s1 = [0] * 8; gprod(pack(t1), rt, s1)
t2 = [0] * 8; gprod(rp, e2p, t2); s2 = [0] * 8; gprod(pack(t2), rt, s2)
acc = [s1[b] + s2[b] for b in range(8)]
def modq(a, m):
    r = a - tdiv(a, m) * m
    return r + m if r < 0 else r
m0, m1, m2, m3 = 16383, 16381, 16379, 16375
c1, c2, c3 = 8191, 10237, 15778
koff = 35989228557189187
acc2 = []
for v in acc:
    r0, r1, r2, r3 = (modq(v + koff, m) for m in (m0, m1, m2, m3))
    v0 = r0
    v1 = modq(modq(r1 - v0, m1) * c1, m1)
    v2 = modq(modq(r2 - v0 - v1 * m0, m2) * c2, m2)
    v3 = modq(modq(r3 - v0 - v1 * m0 - v2 * m0 * m1, m3) * c3, m3)
    acc2.append(v0 + v1 * m0 + v2 * m0 * m1 + v3 * m0 * m1 * m2 - koff)
dig1 = sum((acc[b] + 8) * 10 ** b for b in range(8))
dig2 = sum((acc2[b] + 8) * 10 ** b for b in range(8))
print(dig1 + (dig1 == dig2) * 10 ** 12)
