# rev: reversible gates (toggle/cnot/toffoli/fredkin) + XOR mix round with recorded deltas and undo; fold = 6 probes chained acc*131 + ok(0..6)
M = (1 << 64) - 1
def bit(v, b): return (v & M) >> b & 1
def toggle(x, i, b): x[i] ^= 1 << b
def cnot(x, i, j): x[j] ^= x[i]
def toffoli(x, a, b, c, k): x[c] ^= (bit(x[a], k) & bit(x[b], k)) << k
def fredkin(x, a, b, c, k):
    if bit(x[a], k):
        bb, cc = bit(x[b], k), bit(x[c], k)
        x[b] = (x[b] & ~(1 << k)) | (cc << k)
        x[c] = (x[c] & ~(1 << k)) | (bb << k)
def rnd(x, n, d):
    for i in range(n):
        d[i] = (x[i] ^ ((x[(i + 1) % n] - 7046029254386353131) & M) ^ (-7723592293110705685 & M)) & M
        x[i] = (x[i] ^ d[i]) & M
def undo(x, n, d):
    for i in reversed(range(n)): x[i] ^= d[i]
x = [0] * 16; d = [0] * 16; n = 5
x[0] = 15; toggle(x, 0, 3); toggle(x, 0, 3)
acc = x[0] == 15
x[1], x[2] = 5, 3; cnot(x, 1, 2); cnot(x, 1, 2)
acc = acc * 131 + (x[2] == 3)
x[1], x[2], x[3] = 3, 1, 0; toffoli(x, 1, 2, 3, 0)
acc = acc * 131 + (x[3] == 1)
toffoli(x, 1, 2, 3, 0)
acc = acc * 131 + (x[3] == 0)
x[1], x[2], x[3] = 1, 1, 0; fredkin(x, 1, 2, 3, 0)
acc = acc * 131 + (x[3] == 1)
fredkin(x, 1, 2, 3, 0)
acc = acc * 131 + (x[3] == 0)
x[:5] = [1000, 0, -5 & M, 42, 7]
rnd(x, n, d); mid = x[2]; undo(x, n, d)
ok = (x[0] == 1000) + (x[1] == 0) + (x[2] == -5 & M) + (x[3] == 42) + (x[4] == 7) + (mid != x[2])
print(acc * 131 + ok)
