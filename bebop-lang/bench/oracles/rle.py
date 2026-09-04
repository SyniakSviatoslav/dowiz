# Oracle for gate `rle` (T38): itertools.groupby re-derives the (value,count) pairs.
from itertools import groupby
M62 = (1 << 62) - 1
def mix(h, x): return ((h * 1000003) + x) & M62
src = []; s = 41
for _ in range(96):
    s = (s * 1103515245 + 12345) & 0x7fffffff; src.append((s >> 16) % 4)
pairs = [(v, len(list(g))) for v, g in groupby(src)]
enc = [x for p in pairs for x in p]
dec = [v for v, c in pairs for _ in range(c)]
assert dec == src
h = mix(43, len(pairs)); h = mix(h, len(enc)); h = mix(h, len(dec))
for v in enc: h = mix(h, v)
for v in dec: h = mix(h, v)
h = mix(h, 0); h = mix(h, 0); h = mix(h, 0)
print(h)
