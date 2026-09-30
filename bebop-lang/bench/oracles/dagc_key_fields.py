# Oracle for gate dagc_key_fields (DG4, SPEC-BEBOP-DAG-RUNTIME §2.3 as built in selfhost/prelude/dagc.bp):
# the compile-node K64 = (zlib crc32 of the span bytes zero-padded to a multiple of 8) << 32 | length.
# dagc_key_fields.bp slices the same 29-byte span out of a text at alignments 0..7 (a leading run of
# '#' bytes) and folds acc = acc*31 + K64 (wrapping i64); alignment must not change the key.
import zlib
M = (1 << 64) - 1
def s64(u): u &= M; return u - (1 << 64) if u >> 63 else u
span = b"fn f(x: i64) -> i64 { x * 3 }"
n = len(span)
acc = 0
for a in range(8):
    padded = span + bytes((-n) % 8)
    k = s64(((zlib.crc32(padded) & 0xFFFFFFFF) << 32) | n)
    acc = (acc * 31 + k) & M
print(s64(acc))
