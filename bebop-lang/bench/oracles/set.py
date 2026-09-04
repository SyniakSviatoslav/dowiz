# Oracle for gate `set` (T38): python set algebra over the same three sets.
M62 = (1 << 62) - 1
def mix(h, x): return ((h * 1000003) + x) & M62
a = set(range(0, 60, 3)); b = set(range(0, 80, 4)); c = set(range(0, 60, 12)); e = set()
h = 29
for x in range(81): h = mix(h, x in a); h = mix(h, x in b)
for v in (len(a | b), len(a | c), len(c | b), len(a | e),
          len(a & b), len(a & c), len(c & b), len(e & b),
          c <= a, a <= c, c <= b, a <= b, e <= b, a <= e):
    h = mix(h, int(v))
print(h)
