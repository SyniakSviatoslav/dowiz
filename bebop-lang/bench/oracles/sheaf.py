# sheaf.py - oracle for sheaf.bp (T27): cellular sheaf on a 5-node graph,
# stalk dim 2, stalks fp 2^32, integer (unimodular) restriction maps.
# Orientation as incidence.rs: edge = (tail, head), tail < head;
# (delta x)_e = rho_{head<e} x_head - rho_{tail<e} x_tail ; L_F = delta^T delta.
S = 1 << 32
N = 5
ET = [0, 1, 2, 3, 0, 1]
EH = [1, 2, 3, 4, 2, 4]
I2 = [1, 0, 0, 1]

def mul(a, b):  # 2x2 row-major
    return [a[0]*b[0]+a[1]*b[2], a[0]*b[1]+a[1]*b[3],
            a[2]*b[0]+a[3]*b[2], a[2]*b[1]+a[3]*b[3]]

def mv(a, v):
    return [a[0]*v[0]+a[1]*v[1], a[2]*v[0]+a[3]*v[1]]

def mtv(a, v):  # transpose * v
    return [a[0]*v[0]+a[2]*v[1], a[1]*v[0]+a[3]*v[1]]

def delta(T, H, x):
    return [[a-b for a, b in zip(mv(H[e], x[EH[e]]), mv(T[e], x[ET[e]]))]
            for e in range(6)]

def lap(T, H, x):
    r = delta(T, H, x)
    y = [[0, 0] for _ in range(N)]
    for e in range(6):
        hh = mtv(H[e], r[e]); tt = mtv(T[e], r[e])
        for k in range(2):
            y[EH[e]][k] += hh[k]
            y[ET[e]][k] -= tt[k]
    return y

# --- 1. identity restrictions: L_F == D - A componentwise
xg = [[((v+1)*(c+2)) * S for c in range(2)] for v in range(N)]
TI = [I2]*6; HI = [I2]*6
y = lap(TI, HI, xg)
deg = [0]*N; A = [[0]*N for _ in range(N)]
for e in range(6):
    deg[ET[e]] += 1; deg[EH[e]] += 1
    A[ET[e]][EH[e]] += 1; A[EH[e]][ET[e]] += 1
ident_ok = 1; lapsum = 0
for v in range(N):
    for c in range(2):
        g = deg[v]*xg[v][c] - sum(A[v][u]*xg[u][c] for u in range(N))
        if g != y[v][c]: ident_ok = 0
        lapsum += abs(y[v][c]) >> 32

# --- 2. non-identity sheaf, consistent section
T0 = [1, 1, 0, 1]; T1 = [1, 0, 1, 1]; T2 = [0, 1, -1, 0]; T3 = [1, -1, 0, 1]
B4 = [1, 1, 0, 1]; B5 = [1, 0, -1, 1]
T = [T0, T1, T2, T3, mul(B4, mul(T1, T0)), mul(B5, mul(T3, mul(T2, T1)))]
H = [I2, I2, I2, I2, B4, B5]
x = [[3*S, 2*S]]
for e in range(4):
    x.append(mv(T[e], x[e]))
r = delta(T, H, x)
cons_ok = 1 if all(v == 0 for rr in r for v in rr) else 0

# --- 3. breaker: perturb one entry of rho_{tail<e4}
Tp = [list(m) for m in T]
Tp[4][1] += 1
r = delta(Tp, H, x)
mask = 0
for e in range(6):
    if r[e][0] != 0 or r[e][1] != 0: mask |= 1 << e
n_leaky = bin(mask).count("1")
leak_edge = 4 if mask == 16 else 0
leak_mag = (abs(r[4][0]) + abs(r[4][1])) >> 32

fold = (ident_ok*10**9 + cons_ok*10**8 + (1 if n_leaky == 1 else 0)*10**7
        + leak_edge*10**6 + leak_mag*10**4 + lapsum)
print(fold)
