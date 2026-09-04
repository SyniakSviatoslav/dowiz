# Oracle for gate `zgrade` (T24): supercommutator on the graded pair
# Cl^0(4,1) (+) Lambda_5 (graded tensor basis (I,J), k = I*32+J), nilpotent
# trigger, select-equivalence. Computed from the definitions, integers only.
def inv(i, j):
    return sum(((i >> x) & 1) * ((j >> y) & 1) for x in range(1, 5) for y in range(x))

def sgn(i, j):            # exterior product sign on Lambda_5
    return 0 if i & j else 1 - (inv(i, j) % 2) * 2

def csgn(i, j):           # Clifford sign, signature (+,+,+,+,-)
    return (1 - (inv(i, j) % 2) * 2) * (1 - (((i & j) >> 4) & 1) * 2)

popc = lambda i: bin(i).count("1")
par = lambda k: popc(k % 32) % 2
pc = lambda kx, ky: csgn(kx // 32, ky // 32) * sgn(kx % 32, ky % 32)
idx = lambda kx, ky: ((kx // 32) ^ (ky // 32)) * 32 + ((kx % 32) | (ky % 32))
ks = lambda kx, ky: 1 - 2 * par(kx) * par(ky)
bc = lambda kx, ky: pc(kx, ky) - ks(kx, ky) * pc(ky, kx)

basis = [k for k in range(1024) if popc(k // 32) % 2 == 0]   # 512 elements
assert len(basis) == 512

# all pairs: bracket vs if/else reference, even-even vs commutator, zero count
mmall = mmee = zc = 0
for kx in basis:
    px = par(kx)
    for ky in basis:
        py = par(ky)
        s1, s2 = pc(kx, ky), pc(ky, kx)
        br = bc(kx, ky)
        ref = s1 + s2 if px and py else s1 - s2
        mmall += int(br != ref)
        if not px and not py:
            mmee += int(br != s1 - s2)
        zc += int(br == 0)
okall, okee = int(mmall == 0), int(mmee == 0)

# odd generators (0, e_i): anticommutator == 0 on all 25 pairs
gz = sum(int(bc(1 << i, 1 << j) == 0) for i in range(5) for j in range(5))

# generators: 5 odd e_i, 10 even bivectors
gens = [1 << i for i in range(5)] + [((1 << a) | (1 << b)) * 32 for a in range(5) for b in range(a + 1, 5)]
assert len(gens) == 15

def jacobi(kx, ky, kz):
    t1 = ks(kx, kz) * bc(ky, kz) * bc(kx, idx(ky, kz))
    t2 = ks(ky, kx) * bc(kz, kx) * bc(ky, idx(kz, kx))
    t3 = ks(kz, ky) * bc(kx, ky) * bc(kz, idx(kx, ky))
    return t1 + t2 + t3
jz = sum(int(jacobi(kx, ky, kz) == 0) for kx in basis for ky in gens for kz in gens)

# trigger applied twice: t(t v) == 0 for all 31 t, all 32 v
trig = sum(int(sgn(t, v) * sgn(t, t | v) == 0) for t in range(1, 32) for v in range(32))

# select-equivalence over all 256 (px, py, a, b) patterns
mis = tot = 0
for w in range(256):
    px, py = w & 1, (w >> 1) & 1
    a, b = ((w >> 2) & 7) - 4, ((w >> 5) & 7) - 4
    k = 1 - 2 * px * py
    out = (b * (1 + k) + a * (1 - k)) // 2      # numerator is even: exact
    ref = a if px * py == 1 else b
    mis += int(out != ref)
    tot += out
okmis = int(mis == 0)

print("DBG", zc, okall, okee, gz, jz, trig, tot, okmis)
fold = ((((((zc * 2 + okall) * 2 + okee) * 26 + gz) * 57601 + jz) * 993 + trig) * 2048 + tot + 1024) * 2 + okmis
assert 0 < fold < (1 << 63)
print(fold)
