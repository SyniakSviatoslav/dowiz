#!/usr/bin/env python3
"""oracles2/store -- gate `store`: .bt atomic-publish store -- the SAME 220-byte
"BT4R" stream bt.bp packs is written to a tmp file, published by rename, read back
and unpacked against the golden (std_golden.sh header comment).  The fold is a
function of that byte stream + round-trip flags; the function itself is NOT in any
prose (only the frozen number, which this oracle does not use).
This oracle performs the file round trip for real (tempfile + os.replace) and
prints, per FOLD env (default fnv), the same candidates as bt.py over the bytes read
back from the published file; flags (byte-identical read-back, unpack ok) asserted."""
import os, struct, tempfile
M64 = (1 << 64) - 1
def s64(x): x &= M64; return x - (1 << 64) if x >> 63 else x
D = [2, 3, 2, 2]
n = 24
data = [(((k * 2654435761 + 7) & ((1 << 44) - 1)) - (1 << 43)) for k in range(n)]
bs = b"BT4R" + struct.pack("<II", 1, 4) + struct.pack("<4I", *D) + b"".join(struct.pack("<q", v) for v in data)
d = tempfile.mkdtemp(prefix="oracles2_store_")
tmp, final = os.path.join(d, "t.bt.tmp"), os.path.join(d, "t.bt")
with open(tmp, "wb") as f: f.write(bs)
os.replace(tmp, final)                       # renameat(AT_FDCWD) publish
with open(final, "rb") as f: rb = f.read()
os.remove(final); os.rmdir(d)
rt_ok = int(rb == bs and len(rb) == 220)
dims = list(struct.unpack_from("<4I", rb, 12))
vals = [struct.unpack_from("<q", rb, 28 + 8 * k)[0] for k in range(n)]
unpack_ok = int(rb[:4] == b"BT4R" and dims == D and vals == data)
assert rt_ok and unpack_ok
h = 0xcbf29ce484222325
for b in rb: h = ((h ^ b) * 0x100000001b3) & M64
fnv = s64(h)
FOLDS = {
    "fnv": fnv,
    "fnv_flags3": s64(fnv * 4 + rt_ok * 2 + unpack_ok),
    "fnv_flags1000": s64(fnv * 100 + rt_ok * 10 + unpack_ok),
    "fnv_plus_flags": s64(fnv + rt_ok * 2 + unpack_ok),
}
if __name__ == "__main__":
    name = os.environ.get("FOLD", "fnv")
    print("rt_ok", rt_ok, "unpack_ok", unpack_ok, "fnv", fnv, "fold", name)
    print(FOLDS[name])
