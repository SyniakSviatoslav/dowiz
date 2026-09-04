# Oracle for gate `modular` (T38): python's % (already the least non-negative
# residue) and pow(a, e, m) / pow(a, -1, m) re-derive every .bp helper.
M62 = (1 << 62) - 1
def mix(h, x): return ((h * 1000003) + x) & M62
def inv(a, m):
    try: return pow(a, -1, m)
    except ValueError: return 0
P = 998244353
h, s = 11, 99
for i in range(40):
    s = (s * 1103515245 + 12345) & 0x7fffffff; a = s - (1 << 30)
    s = (s * 1103515245 + 12345) & 0x7fffffff; b = s - (1 << 30)
    for v in (a % P, b % 97, (a + b) % P, (a * b) % P, (a * b) % 97,
              pow(3, i, P), pow(a, i, 97), inv(i + 1, P), inv(i, 12)):
        h = mix(h, v)
print(h)
