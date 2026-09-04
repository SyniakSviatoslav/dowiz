# deltasync: FNV-1a (bytes LSB-first) over 8 hypervector cells; XOR delta applied round-trips (good), one flipped bit is detected; fold = d1 mod 1e11 + good*1e11 + detected*1e12
M64 = (1 << 64) - 1
def fnv8(cells):
    h = 14695981039346656037
    for v in cells:
        v &= M64
        for sh in range(8):
            h = ((h ^ ((v >> (sh * 8)) & 255)) * 1099511628211) & M64
    return h
cb1 = [(1229782938247303441 * (i + 1)) & M64 for i in range(8)]
mask = 1311768467294899696
cb2 = [c ^ mask for c in cb1]
delta = [a ^ b for a, b in zip(cb1, cb2)]
applied = [c ^ d for c, d in zip(cb2, delta)]
d1 = fnv8(cb1)
good = int(fnv8(applied) == d1)
bad = delta[:]
bad[3] ^= 1
detected = int(fnv8([c ^ d for c, d in zip(cb2, bad)]) != d1)
print(d1 % 10**11 + good * 10**11 + detected * 10**12)
