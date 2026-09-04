#!/usr/bin/env python3
"""oracles2/crc -- gate `crc` (old oracle name crc32): CRC-32/ISO-HDLC (reflected
0xEDB88320, init/xorout 0xFFFFFFFF) of the ASCII bytes "123456789"; the standard
check value 0xCBF43926 (std_golden.sh header comment names this input).
Own table-driven implementation, cross-checked against zlib.crc32 (stdlib)."""
import zlib
tab = []
for n in range(256):
    c = n
    for _ in range(8):
        c = (c >> 1) ^ 0xEDB88320 if c & 1 else c >> 1
    tab.append(c)
def crc32(bs):
    c = 0xFFFFFFFF
    for b in bs:
        c = tab[(c ^ b) & 0xFF] ^ (c >> 8)
    return c ^ 0xFFFFFFFF
v = crc32(b"123456789")
assert v == zlib.crc32(b"123456789") == 0xCBF43926
print(v)
