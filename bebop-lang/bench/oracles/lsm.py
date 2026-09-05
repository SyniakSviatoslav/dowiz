# lsm: 8-node {-1,0,1} CSR reservoir from xorshift64(1234567) (2-pass), leaky step liq=floor((liq+floor(pre/16))/2); fold = base-131 chain of echo bits, degree pin, 8 FWHT sign words, energy sum (i64 wrap)
M = (1 << 64) - 1
def w(x): x &= M; return x - (1 << 64) if x >> 63 else x
def gen():
    x = 1234567
    while True:
        x = (x ^ (x << 13)) & M
        x ^= x >> 7
        x = (x ^ (x << 17)) & M
        yield w(x)
g = gen()
adj = [[(j, (z & 7) % 3 - 1) for j in range(8) if ((z := next(g)) & 7) % 3 != 1] for _ in range(8)]
win = [1 - (i & 1) for i in range(8)]
def step(liq, u):
    pre = [sum(wt * liq[j] for j, wt in adj[i]) + u * win[i] * 8 for i in range(8)]
    return [(liq[i] + pre[i] // 16) // 2 for i in range(8)]
def fwht(a):
    a = a[:]; h = 1
    while h < 8:
        for i in range(0, 8, 2 * h):
            for j in range(i, i + h):
                a[j], a[j + h] = a[j] + a[j + h], a[j] - a[j + h]
        h *= 2
    return a
energy = lambda v: sum(map(abs, v))
liq = [256] + [0] * 7
m0 = energy(liq); liq = step(liq, 0); m1 = energy(liq); liq = step(liq, 0); m2 = energy(liq)
acc = (m0 > m1) + 2 * (m1 > m2) + 4 * (m0 == 256) + 8 * (m0 > m2)
deg = [len(r) for r in adj]
acc = w(acc * 131 + sum(d * (i + 1) for i, d in enumerate(deg)) * 97 + sum(9 * (d == 0) + 3 * (d > 6) for d in deg) * 13)
pv = fwht([1, 2, 3, 4, 5, 6, 7, 8]); acc = w(acc * 131 + sum(pv[i] * (i + 1) for i in range(8)))  # T123 dense probe
liq = [0] * 8; ew = 0
for _ in range(8):
    liq = step(liq, next(g) & 1)
    tm = fwht(liq)
    acc = w(acc * 131 + sum(1 << i for i in range(8) if tm[i] > 0))
    acc = w(acc * 131 + sum(tm[i] * (i + 1) for i in range(8)))  # T123
    ew += energy(liq)
print(w(acc * 131 + ew))
