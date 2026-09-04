# ptrless: FNV-64 digest of 4 i64 state cells as the only pointer; fold = (d1 mod 1e11, C-style) + verify*1e11 + corrupt_detected*1e12
M = (1 << 64) - 1
def w(x): x &= M; return x - (1 << 64) if x >> 63 else x
def fnv4(st):
    h = 14695981039346656037
    for v in st:
        for sh in range(8):
            h = w((h ^ ((v & M) >> (sh * 8) & 255)) * 1099511628211)
    return h
s = [[1, 2, 3, 4], [2, 3, 4, 5], [3, 4, 5, 6]]
d = [fnv4(x) for x in s]
tgt = [sum(int(d[j] == d[1]) * s[j][k] for j in range(3)) for k in range(4)]
verify = int(fnv4(tgt) == d[1])
bad = int(w(d[1] + 1) not in d)
r = d[1] - (abs(d[1]) // 10**11) * (10**11 if d[1] >= 0 else -10**11)
dl = r + (10**11 if r < 0 else 0)
print(dl + verify * 10**11 + bad * 10**12)
