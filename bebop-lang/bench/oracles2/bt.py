#!/usr/bin/env python3
"""oracles2/bt -- gate `bt`: .bt rank-4 word-tensor codec v1 (spec: bt.bp first
comment + spectral_golden/README.md).  Byte stream = "BT4R" | u32 1 | u32 4 |
u32 dims[4] | i64 LE data, dims 2x3x2x2, data[k] = ((k*2654435761+7) mod 2^44) - 2^43.
Checks reproduced: byte length 220, FNV-1a 64 over the byte stream ==
12242088766677946451 (signed -6204655307031605165, golden.txt), unpack round-trip
(dims + all 24 cells), stride view offset ((i*d1+j)*d2+k)*d3+l enumerates 0..23.
UNDERSPECIFIED: how "pack/FNV/unpack/stride roundtrip flags" are folded into ONE
number.  FOLD env selects a candidate; default = the FNV itself (flags asserted)."""
import os, struct
M64 = (1 << 64) - 1
def s64(x): x &= M64; return x - (1 << 64) if x >> 63 else x
D = [2, 3, 2, 2]
n = D[0] * D[1] * D[2] * D[3]
data = [(((k * 2654435761 + 7) & ((1 << 44) - 1)) - (1 << 43)) for k in range(n)]
bs = b"BT4R" + struct.pack("<II", 1, 4) + struct.pack("<4I", *D) + b"".join(struct.pack("<q", v) for v in data)
assert len(bs) == 220
h = 0xcbf29ce484222325
for b in bs: h = ((h ^ b) * 0x100000001b3) & M64
assert h == 12242088766677946451
fnv = s64(h)
# unpack
magic, ver, rank = bs[:4], struct.unpack_from("<I", bs, 4)[0], struct.unpack_from("<I", bs, 8)[0]
dims = list(struct.unpack_from("<4I", bs, 12))
vals = [struct.unpack_from("<q", bs, 28 + 8 * k)[0] for k in range(n)]
unpack_ok = int(magic == b"BT4R" and ver == 1 and rank == 4 and dims == D and vals == data)
def off(i, j, k, l): return ((i * D[1] + j) * D[2] + k) * D[3] + l
stride_ok = int([off(i, j, k, l) for i in range(D[0]) for j in range(D[1]) for k in range(D[2]) for l in range(D[3])] == list(range(n)))
len_ok = int(len(bs) == 220)
FOLDS = {
    "fnv": fnv,
    "fnv_flags3": s64(fnv * 8 + len_ok * 4 + unpack_ok * 2 + stride_ok),
    "fnv_flags1000": s64(fnv * 1000 + len_ok * 100 + unpack_ok * 10 + stride_ok),
    "fnv_plus_flags": s64(fnv + len_ok * 4 + unpack_ok * 2 + stride_ok),
    "fnv_x31_chain": s64(s64(s64(fnv * 31 + len_ok) * 31 + unpack_ok) * 31 + stride_ok),
}
if __name__ == "__main__":
    name = os.environ.get("FOLD", "fnv")
    print("len_ok", len_ok, "unpack_ok", unpack_ok, "stride_ok", stride_ok, "fnv", fnv, "fold", name)
    print(FOLDS[name])
