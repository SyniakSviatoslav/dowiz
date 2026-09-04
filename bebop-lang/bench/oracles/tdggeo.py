# Oracle for tdggeo.bp (T17): Christoffel symbols + covariant derivative in fp 2^32.
ONE = 1 << 32; HALF = ONE >> 1; H = ONE >> 2; M56 = (1 << 56) - 1
def fp_mul(a, b):
    p = (abs(a) * abs(b)) >> 32
    return -p if (a < 0) != (b < 0) else p
def fp_div(a, b): return (a << 32) // b
def tdiv(a, b): return -((-a) // b) if a < 0 else a // b
def hmix(h, v): return ((h * 31) & M56) + abs(v) * 2 + (1 if v < 0 else 0)
def metric(x, y, flat): return [ONE, 0, 0, ONE if flat else fp_mul(x, x)]
def field(x, y): return [ONE + fp_mul(HALF, x) + fp_mul(HALF // 2, y), -HALF + fp_mul(ONE, x) - fp_mul(ONE, y)]
def run(flat, x, y):
    g = metric(x, y, flat)
    dg = []
    for k in range(2):
        dx, dy = H * (1 - k), H * k
        gp, gm = metric(x + dx, y + dy, flat), metric(x - dx, y - dy, flat)
        dg += [(gp[m] - gm[m]) * 2 for m in range(4)]
    det = fp_mul(g[0], g[3]) - fp_mul(g[1], g[2]); idet = fp_div(ONE, det)
    gi = [fp_mul(g[3], idet), fp_mul(-g[1], idet), fp_mul(-g[2], idet), fp_mul(g[0], idet)]
    g1 = [tdiv(dg[i*4+j*2+k] + dg[j*4+i*2+k] - dg[k*4+i*2+j], 2) for k in range(2) for i in range(2) for j in range(2)]
    g2 = [fp_mul(gi[k*2], g1[ij]) + fp_mul(gi[k*2+1], g1[4+ij]) for k in range(2) for ij in range(4)]
    v = field(x, y)
    dv = []
    for i in range(2):
        dx, dy = H * (1 - i), H * i
        vp, vm = field(x + dx, y + dy), field(x - dx, y - dy)
        dv += [(vp[0] - vm[0]) * 2, (vp[1] - vm[1]) * 2]
    out = [dv[i*2+j] + fp_mul(g2[j*4+i*2], v[0]) + fp_mul(g2[j*4+i*2+1], v[1]) for i in range(2) for j in range(2)]
    d = sum(abs(out[n] - dv[n]) for n in range(4))
    return g2, out, d
x, y = ONE + HALF, HALF
g2, nab, dcur = run(0, x, y)
h = 0
for val in g2 + nab: h = hmix(h, val)
chk = 1 if abs(g2[3] + x) < 8 and dcur > 0 else 0
_, nabf, dflat = run(1, x, y)
for val in nabf: h = hmix(h, val)
deg = 1 if dflat == 0 else 0
print(h * 4 + chk * 2 + deg)
