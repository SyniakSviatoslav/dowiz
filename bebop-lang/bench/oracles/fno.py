# fno: NTT(998244353, root 3) circular convolution of v=[3,1,4,1,5,9,2,6] with kernel k=[7,-3,2,0..];
# fold = conv_ok*10^14 + modes_ok*10^13 + spec_ok*10^12 + fwht_ok*10^11 + gapq*10^6 + mask*10^3 + mchk
# gapq = |centered(Y0)-centered(Y1)|/4096, mchk = sum centered(y)[i]*(i+1), mask = kernel nonzero bitmask
P = 998244353
def ntt(a, inv=False):
    n = len(a); a = [x % P for x in a]
    # bit reversal
    for i in range(1, n):
        r = int(bin(i)[2:].zfill(n.bit_length() - 1)[::-1], 2)
        if i < r: a[i], a[r] = a[r], a[i]
    ln = 2
    while ln <= n:
        w = pow(3, (P - 1) // ln, P)
        if inv: w = pow(w, P - 2, P)
        for i in range(0, n, ln):
            wk = 1
            for j in range(ln // 2):
                u = a[i + j]; v = a[i + j + ln // 2] * wk % P
                a[i + j] = (u + v) % P; a[i + j + ln // 2] = (u - v) % P
                wk = wk * w % P
        ln *= 2
    if inv:
        ni = pow(n, P - 2, P); a = [x * ni % P for x in a]
    return a
def centered(r):
    r %= P
    return r - P if r > P // 2 else r
v = [3, 1, 4, 1, 5, 9, 2, 6]
k = [7, -3, 2, 0, 0, 0, 0, 0]
A, B = ntt(v), ntt(k)
Y = [a * b % P for a, b in zip(A, B)]
gapq = abs(centered(Y[0]) - centered(Y[1])) // 4096
y = ntt(Y, inv=True)
mchk = sum(centered(y[i]) * (i + 1) for i in range(8))
yd = [sum(k[j] * v[(i - j) % 8] for j in range(8)) % P for i in range(8)]
conv_ok = 1 if y == yd else 0
spec_ok = 1 if ntt(y) == Y else 0
# FWHT roundtrip (unnormalized Hadamard applied twice = 8*identity)
def fwht(x):
    x = x[:]; ln = 1
    while ln < len(x):
        for b in range(0, len(x), 2 * ln):
            for i in range(ln):
                u, w = x[b + i], x[b + i + ln]
                x[b + i], x[b + i + ln] = u + w, u - w
        ln *= 2
    return x
fwht_ok = 1 if [t // 8 for t in fwht(fwht(v))] == v else 0
nz = [1 if t else 0 for t in k]
modes_ok = 1 if sum(nz) == 3 else 0
mask = sum(b << i for i, b in enumerate(nz))
print(conv_ok * 10**14 + modes_ok * 10**13 + spec_ok * 10**12 + fwht_ok * 10**11 + gapq * 10**6 + mask * 1000 + mchk)
