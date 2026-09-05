# G2 sround oracle (T112): the fold from the LCG spec alone; when sround.store exists
# it is also parsed by the layout rules (superblock pick, object-relative refs) and
# must give the same fold. Prints the fold as a signed i64.
import os, struct, zlib
M = (1 << 64) - 1
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
v = 42; vals = []
for k in range(100000):
    v = (v * 6364136223846793005 + 1442695040888963407) & M; vals.append(v)
acc = 0
for x in reversed(vals): acc = (acc * 31 + x) & M
if os.path.exists('sround.store'):
    d = open('sround.store', 'rb').read()
    def cells(off, n): return list(struct.unpack('<%dq' % n, d[off*8:(off+n)*8]))
    MAGIC = int.from_bytes(b'BEBOPST1', 'little')
    def valid(sb):
        c = cells(sb, 16); return c[0] == MAGIC and (c[15] & M) == zlib.crc32(d[sb*8:(sb+15)*8])
    sbs = [sb for sb in (0, 512) if valid(sb)]
    sb = max(sbs, key=lambda s: cells(s, 16)[2])
    root = cells(sb, 16)[3]; cur = root; acc2 = 0; n = 0
    while cur:
        h0, h1, val, ref = cells(cur, 4)
        assert (h0 & 0xFFFFFFFF) == 2
        acc2 = (acc2 * 31 + val) & M; n += 1
        cur = cur + ref if ref else 0
    assert n == 100000 and acc2 == acc, (n, acc2, acc)
print(s64(acc))
