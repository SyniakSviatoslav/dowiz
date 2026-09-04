# ntt: DFT over Z/998244353 (root 3^((p-1)/8)); word = sign bits of centered spectrum of ramp [1..8];
# ok = 1 (inverse round-trips) + 2 (circular conv ramp * reverse(ramp) via pointwise product == expected); fold = word*1000 + ok
P = 998244353
def dft(a, inv=False):
    n = len(a)
    g = pow(3, (P - 1) // n, P)
    if inv:
        g = pow(g, P - 2, P)
    out = [sum(a[j] * pow(g, j * k, P) for j in range(n)) % P for k in range(n)]
    if inv:
        ni = pow(n, P - 2, P)
        out = [x * ni % P for x in out]
    return out
centered = lambda r: r - P if r > P // 2 else r
ramp = list(range(1, 9))
A = dft(ramp)
word = sum(1 << i for i, v in enumerate(A) if centered(v) > 0)
rt = dft(A, inv=True) == ramp
B = dft(ramp[::-1])
conv = [centered(x) for x in dft([x * y % P for x, y in zip(A, B)], inv=True)]
cv = conv == [176, 156, 144, 140, 144, 156, 176, 204]
print(word * 1000 + int(rt) + 2 * int(cv))
