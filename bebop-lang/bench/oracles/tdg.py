# Oracle for tdg.bp (T16): Einstein contraction + metric lower/raise in fp 2^32.
ONE = 1 << 32; HALF = ONE >> 1; M56 = (1 << 56) - 1
def fp_mul(a, b):
    p = (abs(a) * abs(b)) >> 32
    return -p if (a < 0) != (b < 0) else p
def fp_div(a, b): return (a << 32) // b
def hmix(h, v): return ((h * 31) & M56) + abs(v) * 2 + (1 if v < 0 else 0)
t = [(i // 4 + 1) * ONE + ((i // 2) % 2) * HALF - (i % 2) * (HALF // 2) for i in range(8)]
v = [ONE + HALF, -HALF]
c = [sum(fp_mul(t[i * 4 + j * 2 + k], v[j]) for j in range(2)) for i in range(2) for k in range(2)]
h = 0
for x in c: h = hmix(h, x)
g = [2 * ONE, HALF, HALF, ONE]
det = fp_mul(g[0], g[3]) - fp_mul(g[1], g[2])
idet = fp_div(ONE, det)
gi = [fp_mul(g[3], idet), fp_mul(-g[1], idet), fp_mul(-g[2], idet), fp_mul(g[0], idet)]
for x in [det] + gi: h = hmix(h, x)
mv = lambda m, x: [fp_mul(m[0], x[0]) + fp_mul(m[1], x[1]), fp_mul(m[2], x[0]) + fp_mul(m[3], x[1])]
lo = mv(g, v); up = mv(gi, lo)
rt = 1 if abs(up[0] - v[0]) + abs(up[1] - v[1]) < 8 else 0
s = sum(fp_mul(lo[i], v[i]) for i in range(2))
for x in lo + up + [s]: h = hmix(h, x)
print(h * 2 + rt)
