#!/usr/bin/env python3
# T47 gate usemod: popcount and lowest-set-bit index over 64 LCG words (prelude bits.bp
# semantics: `>>` logical; tzidx = index of the isolated lowest bit), fold acc*131 + popc*64 + tz.
M = (1 << 64) - 1
def w(x): x &= M; return x - (1 << 64) if x >> 63 else x
x = 12345; acc = 0
for _ in range(64):
    x = (x * 6364136223846793005 + 1442695040888963407) & M
    ww = x >> 3
    lsb = ww & (-ww)
    tz = (lsb.bit_length() - 1) if lsb else 0
    acc = w(acc * 131 + bin(ww).count('1') * 64 + tz)
print(acc)
