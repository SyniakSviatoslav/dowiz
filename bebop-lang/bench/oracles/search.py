# Oracle for gate `search` (T38): list.index / bisect over the same array.
import bisect
M62 = (1 << 62) - 1
def mix(h, x): return ((h * 1000003) + x) & M62
a = [i * 3 for i in range(32)]
def lin(x): return a.index(x) if x in a else -1
def bs(x):
    i = bisect.bisect_left(a, x); return i if i < len(a) and a[i] == x else -1
h = 31
for x in range(-1, 97): h = mix(h, lin(x)); h = mix(h, bs(x))
h = mix(h, -1); h = mix(h, -1)
print(h)
