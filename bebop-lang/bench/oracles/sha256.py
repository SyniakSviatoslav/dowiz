# sha256: fold of the 8 u32 words of SHA-256("abc") as acc = acc*31 + word (i64 wrap)
import hashlib
d = hashlib.sha256(b"abc").digest()
acc = 0
for i in range(8):
    acc = (acc * 31 + int.from_bytes(d[4 * i:4 * i + 4], "big")) & 0xFFFFFFFFFFFFFFFF
print(acc - (1 << 64) if acc >> 63 else acc)
