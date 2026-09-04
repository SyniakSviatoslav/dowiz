# sheafh0.py - oracle for sheafh0.bp (T28): global sections H^0 as the query.
# Block-Jacobi harmonic iteration on the sheaf Laplacian L_F = delta^T delta
# with node 0 pinned, K = 512 steps, stalks fp 2^32, integer unimodular maps,
# truncating integer division (mirrors bebop `/`). dim H^0 by pins (which
# basis pins at node 0 extend to a residual-free section) vs rank(delta) by
# Bareiss elimination; chi = d*n_v - d*n_e == h0(pins) - h1(rank).
S = 1 << 32
K = 512
THR = 1 << 16
I2 = [1, 0, 0, 1]

def tdiv(a, b):
    q = abs(a) // abs(b)
    return q if (a >= 0) == (b > 0) else -q

def mul(a, b):
    return [a[0]*b[0]+a[1]*b[2], a[0]*b[1]+a[1]*b[3],
            a[2]*b[0]+a[3]*b[2], a[2]*b[1]+a[3]*b[3]]

def mv(a, v):
    return [a[0]*v[0]+a[1]*v[1], a[2]*v[0]+a[3]*v[1]]

def mtv(a, v):
    return [a[0]*v[0]+a[2]*v[1], a[1]*v[0]+a[3]*v[1]]

def tr(a):
    return [a[0], a[2], a[1], a[3]]

def delta(ET, EH, T, H, x):
    return [[a-b for a, b in zip(mv(H[e], x[EH[e]]), mv(T[e], x[ET[e]]))]
            for e in range(len(ET))]

def lap(ET, EH, T, H, x):
    r = delta(ET, EH, T, H, x)
    y = [[0, 0] for _ in x]
    for e in range(len(ET)):
        hh = mtv(H[e], r[e]); tt = mtv(T[e], r[e])
        for k in range(2):
            y[EH[e]][k] += hh[k]
            y[ET[e]][k] -= tt[k]
    return y, r

def res_sum(r):
    return sum(abs(v) for rr in r for v in rr)

def solve(ET, EH, T, H, pin):
    """block-Jacobi with node 0 pinned; returns (residual after K, first it < THR)"""
    n = 5
    D = [[0, 0, 0, 0] for _ in range(n)]
    for e in range(len(ET)):
        for v, m in ((ET[e], T[e]), (EH[e], H[e])):
            q = mul(tr(m), m)
            for k in range(4): D[v][k] += q[k]
    x = [[0, 0] for _ in range(n)]
    x[0] = [pin[0]*S, pin[1]*S]
    it = 0
    for k in range(1, K+1):
        y, r = lap(ET, EH, T, H, x)
        for v in range(1, n):
            a, b, c, d = D[v]
            det = a*d - b*c
            x[v][0] = x[v][0] - tdiv(d*y[v][0] - b*y[v][1], det)
            x[v][1] = x[v][1] - tdiv(a*y[v][1] - c*y[v][0], det)
        _, r = lap(ET, EH, T, H, x)
        rs = res_sum(r)
        if it == 0 and rs < THR: it = k
    return rs, it

def rank(M, rows, cols):
    """fraction-free (Bareiss) elimination with column skipping; returns rank"""
    a = [row[:] for row in M]
    rk = 0; prev = 1
    for col in range(cols):
        if rk >= rows: break
        p = -1
        for i in range(rk, rows):
            if p < 0 and a[i][col] != 0: p = i
        if p < 0: continue
        a[rk], a[p] = a[p], a[rk]
        piv = a[rk][col]
        for i in range(rk+1, rows):
            f = a[i][col]
            for j in range(cols):
                a[i][j] = tdiv(a[i][j]*piv - f*a[rk][j], prev)
        prev = piv
        rk += 1
    return rk

def delta_matrix(ET, EH, T, H):
    m = len(ET); M = [[0]*10 for _ in range(2*m)]
    for e in range(m):
        for i in range(2):
            for j in range(2):
                M[2*e+i][2*EH[e]+j] += H[e][2*i+j]
                M[2*e+i][2*ET[e]+j] -= T[e][2*i+j]
    return M

T0 = [1, 1, 0, 1]; T1 = [1, 0, 1, 1]; T2 = [0, 1, -1, 0]; T3 = [1, -1, 0, 1]
P = mul(T3, mul(T2, mul(T1, T0)))          # path transport 0 -> 4 = [[2,3],[-1,-1]]
# tree: path 0-1-2-3-4
tET = [0, 1, 2, 3]; tEH = [1, 2, 3, 4]; tT = [T0, T1, T2, T3]; tH = [I2]*4
# twisted cycle: + edge (0,4), tail map P*diag(1,-1) (Mobius flip on comp 2), head I
cET = tET + [0]; cEH = tEH + [4]; cT = tT + [mul(P, [1, 0, 0, -1])]; cH = tH + [I2]

def h0_by_pins(ET, EH, T, H):
    found = 0; its = []; floors = []
    for pin in ((1, 0), (0, 1)):
        rs, it = solve(ET, EH, T, H, pin)
        if rs < THR: found += 1; its.append(it)
        else: floors.append(rs)
    return found, its, floors

t_h0, t_its, t_floors = h0_by_pins(tET, tEH, tT, tH)
c_h0, c_its, c_floors = h0_by_pins(cET, cEH, cT, cH)
t_rank = rank(delta_matrix(tET, tEH, tT, tH), 8, 10)
c_rank = rank(delta_matrix(cET, cEH, cT, cH), 10, 10)
t_h1 = 8 - t_rank; c_h1 = 10 - c_rank
chi_t = 10 - 8; chi_c = 10 - 10
chi_ok_t = 1 if chi_t == t_h0 - t_h1 else 0
chi_ok_c = 1 if chi_c == c_h0 - c_h1 else 0
floor_ok = 1 if (len(c_floors) == 1 and c_floors[0] >= (1 << 28) and not t_floors) else 0
it_t = t_its[0] if t_its else 0
it_c = c_its[0] if c_its else 0
fold = (chi_ok_t*10**13 + chi_ok_c*10**12 + floor_ok*10**11 + t_h0*10**10 + c_h0*10**9
        + t_rank*10**8 + c_rank*10**7 + it_t*10**3 + it_c)
print(fold)
