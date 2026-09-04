# tq oracle: tensor query engine fold from the mathematical definition
# (fp Q32, fp_mul = trunc toward zero, fp_sqrt = isqrt(x<<26)<<3).
import math
ONE = 1 << 32
M = 1000000007

def fp_mul(a, b):
    p = (abs(a) * abs(b)) >> 32
    return -p if (a < 0) != (b < 0) else p

def fp_sqrt(x):
    return math.isqrt(x << 26) << 3

def sqdist(du, dv):
    return fp_mul(du, du) + fp_mul(dv, dv)

def grid_cell(u, v):
    ui = min(max(((u + ONE) * 4) >> 32, 0), 7)
    vi = min(max(((v + ONE) * 4) >> 32, 0), 7)
    return vi * 8 + ui

def geodesic(u1, v1, u2, v2, cu, cv):
    tenth = 429496729
    dist, pu, pv = 0, u1, v1
    for _ in range(3):
        bd, bi = 1 << 62, 0
        for i in range(len(cu)):
            d = sqdist(cu[i] - u2, cv[i] - v2)
            if d < bd:
                bd, bi = d, i
        nu, nv = cu[bi], cv[bi]
        dist += fp_sqrt(sqdist(nu - pu, nv - pv))
        pu, pv = nu, nv
        if bd < tenth:
            break
    return dist + fp_sqrt(sqdist(u2 - pu, v2 - pv))

ev0 = [3 * ONE // 8, ONE // 8, -(2 * ONE // 8), 3 * ONE // 8]
ev1 = [-(ONE // 8), 3 * ONE // 8, 3 * ONE // 8, -(2 * ONE // 8)]
pu, pv, cell = [], [], []
for i in range(12):
    u = v = 0
    for j in range(4):
        x = ((i * 7 + j * 3) % 11 - 5) * ONE // 8
        u += fp_mul(ev0[j], x)
        v += fp_mul(ev1[j], x)
    pu.append(u); pv.append(v); cell.append(grid_cell(u, v))
h = ONE // 2
cu = [h, -h, -h, h]
cv = [h, h, -h, -h]
s = ONE // 16
fold = 0
for qu, qv in [(2 * s, 3 * s), (-5 * s, 10 * s), (11 * s, -6 * s), (-10 * s, -10 * s)]:
    qc = grid_cell(qu, qv)
    qx, qy = qc % 8, qc // 8
    bd, bi, cnt = 1 << 62, 0, 0
    for i in range(12):
        d = geodesic(qu, qv, pu[i], pv[i], cu, cv)
        if d < bd:
            bd, bi = d, i
        if abs(cell[i] % 8 - qx) <= 1 and abs(cell[i] // 8 - qy) <= 1:
            cnt += 1
    fold = (fold * 131 + bi + 1) % M
    fold = (fold * 131 + bd % M) % M
    fold = (fold * 131 + cnt) % M
fold = (fold * 131 + sum(cell)) % M
print(fold + 1)
