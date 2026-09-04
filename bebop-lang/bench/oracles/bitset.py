# Oracle for gate `bitset` (T38): a python int bitmap; words are its 64-bit
# slices, masked to 62 bits (the .bp masks the sign-carrying top bits too).
M62 = (1 << 62) - 1
def mix(h, x): return ((h * 1000003) + x) & M62
bits = 0
for k in range(100): bits |= 1 << ((5 * k) % 120)
h = 23
for i in range(128): h = mix(h, (bits >> i) & 1)
for wi in range(2): h = mix(h, ((bits >> (64 * wi)) & ((1 << 64) - 1)) & M62)
print(h)
