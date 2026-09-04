# ringvsa: 16-cell HV ring, bind = XOR-index convolution; axioms assoc, WHT convolution theorem, delta identity.
# fold = assoc*10^12 + conv*10^11 + ident*10^10 + chk, chk = sum |a*b|[k]*(k+1)
N = 16
def bind(a, b):
    r = [0] * N
    for i in range(N):
        for j in range(N):
            r[i ^ j] += a[i] * b[j]
    return r
def wht(x):
    x = x[:]
    h = 1
    while h < N:
        for base in range(0, N, 2 * h):
            for i in range(base, base + h):
                x[i], x[i + h] = x[i] + x[i + h], x[i] - x[i + h]
        h *= 2
    return x
pc = lambda i: bin(i).count("1")
a = [1 if pc(i) % 2 == 0 else -1 for i in range(N)]
b = [1 if (pc(i) // 2) % 2 == 0 else -1 for i in range(N)]
c = [1 if i % 3 == 0 else -1 for i in range(N)]
e = [1] + [0] * (N - 1)
ab = bind(a, b)
chk = sum(abs(v) * (k + 1) for k, v in enumerate(ab))
assoc = int(bind(ab, c) == bind(a, bind(b, c)))
wa, wb, wab = wht(a), wht(b), wht(ab)
conv = int(all(wab[k] == wa[k] * wb[k] for k in range(N)))
ident = int(bind(a, e) == a)
print(assoc * 10**12 + conv * 10**11 + ident * 10**10 + chk)
