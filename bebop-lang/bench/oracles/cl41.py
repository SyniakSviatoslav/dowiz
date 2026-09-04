# Oracle for gate `cl41` (T23): Cl(4,1) ternary CGA basis, computed from the
# mathematical definition (signature +,+,+,+,-), integers only.
def csgn(i, j):
    t = sum(((i >> x) & 1) * ((j >> y) & 1) for x in range(1, 5) for y in range(x))
    m = ((i & j) >> 4) & 1
    return (1 - (t % 2) * 2) * (1 - m * 2)

def popc(i):
    return bin(i).count("1")

def cprod(a, b):
    acc = [0] * 32
    for i in range(32):
        for j in range(32):
            acc[i ^ j] += a[i] * b[j] * csgn(i, j)
    return acc

def mv(**kw):
    a = [0] * 32
    for k, v in kw.items():
        a[int(k[1:])] = v
    return a

zero = [0] * 32
iseven = lambda a: all(a[k] == 0 for k in range(32) if popc(k) % 2)
istern = lambda a: all(x * x <= 1 for x in a)
chk = lambda a: sum(a[k] * (k + 1) for k in range(32))
M32 = (1 << 32) - 1

# exhaustive blade table
ev = ssum = 0
for i in range(32):
    for j in range(32):
        if popc(i) % 2 == 0 and popc(j) % 2 == 0:
            ev += int(popc(i ^ j) % 2 == 0)
        ssum += csgn(i, j)
assert ev == 256

# two even multivectors A = 1 + e12, B = 1 + e13 packed in one i64
evens = [k for k in range(32) if popc(k) % 2 == 0]
packe = lambda a: sum((a[k] + 1) << (2 * s) for s, k in enumerate(evens))
def unpacke(p):
    a = [0] * 32
    for s, k in enumerate(evens):
        a[k] = ((p >> (2 * s)) & 3) - 1
    return a
A, B = mv(b0=1, b3=1), mv(b0=1, b5=1)
reg = (packe(A) + (packe(B) << 32)) & ((1 << 64) - 1)
U, V = unpacke(reg & M32), unpacke(reg >> 32)
rt = int(U == A and V == B)
AB = cprod(U, V)
re, tern = int(iseven(AB)), int(istern(AB))
chkAB = chk(AB) + 528

# null vectors n_inf = e4 + e5, 2 n_o = e5 - e4
ninf, no = mv(b8=1, b16=1), mv(b8=-1, b16=1)
z1 = int(cprod(ninf, ninf) == zero)
z2 = int(cprod(no, no) == zero)
sp = int(cprod(ninf, no)[0] == -2)

# 27 ternary CGA points, rotor sandwich vs direct table
def point(x1, x2, x3, s):
    q = x1 * x1 + x2 * x2 + x3 * x3
    return mv(b1=s * 2 * x1, b2=s * 2 * x2, b4=s * 2 * x3, b8=s * (q - 1), b16=s * (q + 1))
R, Rt = mv(b0=1, b3=1), mv(b0=1, b3=-1)
nullc = sw = schk = 0
for x1 in (-1, 0, 1):
    for x2 in (-1, 0, 1):
        for x3 in (-1, 0, 1):
            P = point(x1, x2, x3, 1)
            nullc += int(cprod(P, P) == zero)
            S = cprod(cprod(R, P), Rt)
            sw += int(S == point(x2, -x1, x3, 2))
            schk += chk(S)
assert 0 <= schk < 10000 and 0 <= chkAB < 1100

bits6 = z1 * 32 + z2 * 16 + sp * 8 + re * 4 + tern * 2 + rt
fold = ((((ev * 1000 + nullc * 28 + sw) * 64 + bits6) * 10000 + schk) * 10000 + ssum + 1024) * 1100 + chkAB
assert 0 < fold < (1 << 63)
print(fold)
