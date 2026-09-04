#!/usr/bin/env python3
"""oracles2/sha256 -- gate `sha256`: FIPS 180-4 SHA-256 of the ASCII bytes "abc"
(input per bench/vs_rust/std_golden.sh header comment), folded over the 8 output
u32 words as acc = acc*31 + word (mod 2^64, acc0 = 0), printed as signed i64 (Bebop cell).
Digest from hashlib (stdlib), cross-checked against the FIPS 180-4 appendix vector."""
import hashlib
M64 = (1 << 64) - 1
d = hashlib.sha256(b"abc").digest()
assert d.hex() == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
acc = 0
for i in range(8):
    acc = (acc * 31 + int.from_bytes(d[4*i:4*i+4], "big")) & M64
print("unsigned", acc)
print(acc - (1 << 64) if acc >> 63 else acc)
