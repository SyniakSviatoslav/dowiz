#!/usr/bin/env python3
# Oracle for tdgcurv gate (T18): Riemann/Ricci/scalar/sectional curvature of
# the unit 2-sphere in i64 fixed-point 2^32, from the mathematical definition
# (MTW convention R^i_jkl = d_k G^i_lj - d_l G^i_kj + G^i_km G^m_lj - G^i_lm G^m_kj).
# Integer-only; fp_mul = sign * floor(|a||b| / 2^32).
from math import isqrt

ONE = 1 << 32
N = 2

def fp_mul(a, b):
    p = (abs(a) * abs(b)) >> 32
    return -p if (a < 0) != (b < 0) else p

# (sin, cos, csc) at theta in {pi/6, pi/4, pi/3, pi/2}
TAB = [
    (ONE // 2, isqrt(3 << 64) // 2, 2 * ONE),
    (isqrt(2 << 64) // 2, isqrt(2 << 64) // 2, isqrt(2 << 64)),
    (isqrt(3 << 64) // 2, ONE // 2, isqrt((4 << 64) // 3)),
    (ONE, 0, ONE),
]

def point(s, c, cs):
    g = [[ONE, 0], [0, fp_mul(s, s)]]
    ginv = [[ONE, 0], [0, fp_mul(cs, cs)]]
    gam = [[[0] * N for _ in range(N)] for _ in range(N)]        # gam[i][j][k]
    dgam = [[[[0] * N for _ in range(N)] for _ in range(N)] for _ in range(N)]  # dgam[m][i][j][k]
    cot = fp_mul(c, cs)
    gam[0][1][1] = -fp_mul(s, c)
    gam[1][0][1] = cot
    gam[1][1][0] = cot
    dgam[0][0][1][1] = fp_mul(s, s) - fp_mul(c, c)
    dgam[0][1][0][1] = -fp_mul(cs, cs)
    dgam[0][1][1][0] = -fp_mul(cs, cs)
    R = [[[[0] * N for _ in range(N)] for _ in range(N)] for _ in range(N)]
    for i in range(N):
        for j in range(N):
            for k in range(N):
                for l in range(N):
                    r = dgam[k][i][l][j] - dgam[l][i][k][j]
                    for m in range(N):
                        r += fp_mul(gam[i][k][m], gam[m][l][j]) - fp_mul(gam[i][l][m], gam[m][k][j])
                    R[i][j][k][l] = r
    ric = [[sum(R[i][j][i][l] for i in range(N)) for l in range(N)] for j in range(N)]
    sc = sum(fp_mul(ginv[j][l], ric[j][l]) for j in range(N) for l in range(N))
    x, y = [ONE, 0], [0, ONE]
    num = 0
    for i in range(N):
        for j in range(N):
            for k in range(N):
                for l in range(N):
                    low = sum(fp_mul(g[i][m], R[m][j][k][l]) for m in range(N))
                    w = fp_mul(fp_mul(x[i], y[j]), fp_mul(x[k], y[l]))
                    num += fp_mul(low, w)
    def gram(u, v):
        return sum(fp_mul(g[i][j], fp_mul(u[i], v[j])) for i in range(N) for j in range(N))
    xx, yy, xy = gram(x, x), gram(y, y), gram(x, y)
    den = fp_mul(xx, yy) - fp_mul(xy, xy)
    eps = 1 << 16
    ok = int(abs(sc - 2 * ONE) < eps and abs(num - den) < eps)
    return ok * 10**15 + (abs(num) >> 12) * 10**8 + (abs(sc) >> 12)

print(sum(point(*t) for t in TAB))
