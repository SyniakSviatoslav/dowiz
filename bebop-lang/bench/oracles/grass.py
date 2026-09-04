# Oracle for gate `grass` (T22): Grassmann algebra Lambda_5 as bitmask monomials.
# Computed from the mathematical definition, integers only.
def sgn(i, j):
    if i & j:
        return 0
    t = sum(((i >> x) & 1) * ((j >> y) & 1) for x in range(1, 5) for y in range(x))
    return 1 - (t % 2) * 2

def popc(i):
    return bin(i).count("1")

def gprod(a, b):
    acc = [0] * 32
    for i in range(32):
        for j in range(32):
            acc[i | j] += a[i] * b[j] * sgn(i, j)
    return acc

def mono(k, v=1):
    a = [0] * 32
    a[k] = v
    return a

def add(a, b):
    return [x + y for x, y in zip(a, b)]

zero = [0] * 32
# anticommutation on all 10 generator pairs
na = 0
for i in range(5):
    for j in range(i + 1, 5):
        p1 = gprod(mono(1 << i), mono(1 << j))
        p2 = gprod(mono(1 << j), mono(1 << i))
        na += int(add(p1, p2) == zero and p1[(1 << i) | (1 << j)] == 1)
# nilpotency of all 31 non-scalar monomials
nn = sum(int(gprod(mono(m), mono(m)) == zero) for m in range(1, 32))
# full Cayley table: grading closure, nonzero count, sign sum
par = nz = ssum = 0
for i in range(32):
    for j in range(32):
        s = sgn(i, j)
        z = s * s
        g = (popc(i) + popc(j) - popc(i | j)) % 2
        par += z * (1 - g)
        nz += z
        ssum += s
assert par == nz == 243
# associativity: a = e3 + e12, b = e1 + e4, c = e2 - e5
a = add(mono(4), mono(3))
b = add(mono(1), mono(8))
c = add(mono(2), mono(16, -1))
ab, bc = gprod(a, b), gprod(b, c)
r1, r2 = gprod(ab, c), gprod(a, bc)
assoc = int(r1 == r2)
istern = lambda v: int(all(x * x <= 1 for x in v))
tern = istern(r1) * istern(ab) * istern(bc)
M = (1 << 64) - 1
p = sum((r1[k] + 1) << (2 * k) for k in range(32)) & M
rt = int([((p >> (2 * k)) & 3) - 1 for k in range(32)] == r1)
chk = sum((r1[k] + 1) * (k + 1) * (k + 1) for k in range(32))
print(((((na * 100 + nn) * 1000 + nz) * 10000 + (ssum + 5000)) * 100000 + chk) * 10 + assoc * 4 + rt * 2 + tern)
