# kalman: 1-D fixed-point (2^32) Kalman, F=H=1, Q=0.001, R=0.01, z=5, x0=0, P0=1, 1000 steps; K=floor(P*2^32/(P+R)); fold = (K>>12)*10^8 + (P>>20)*10^4 + trk*10 + fix (trk: |z-x|<0.001, fix: P1000==P999)
ONE = 1 << 32
def fp_mul(a, b):
    p = (abs(a) * abs(b)) >> 32
    return -p if (a < 0) != (b < 0) else p
q, r, z = ONE // 1000, ONE // 100, 5 * ONE
x, p, prevp, k, fix = 0, ONE, ONE, 0, 0
for _ in range(1000):
    x = fp_mul(ONE, x)
    p = fp_mul(ONE, p) + q
    k = (p << 32) // (p + r)
    x = x + fp_mul(k, z - x)
    p = fp_mul(ONE - k, p)
    fix = int(p == prevp)
    prevp = p
trk = int(abs(z - x) < 4294967)
print((k >> 12) * 10**8 + (p >> 20) * 10**4 + trk * 10 + fix)
