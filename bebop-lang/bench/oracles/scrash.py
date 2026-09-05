# G5 scrash oracle (T113): fold of the chain for generation g (argv[1], default 10^4):
# 100*g nodes, val = LCG chain from 42, fold = acc*31 + val walking from the newest node.
# With --parse it reads scrash.store, checks the superblock crcs, picks the higher valid
# generation, walks the chain by the layout rules and prints "g fold" for that store.
import os, struct, sys, zlib
M = (1 << 64) - 1
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
def fold_for(g):
    v = 42; vals = []
    for _ in range(100 * g):
        v = (v * 6364136223846793005 + 1442695040888963407) & M; vals.append(v)
    acc = 0
    for x in reversed(vals): acc = (acc * 31 + x) & M
    return s64(acc)
if len(sys.argv) > 1 and sys.argv[1] == '--parse':
    d = open('scrash.store', 'rb').read() if os.path.exists('scrash.store') else b''
    if len(d) < 2 * 4096:  # killed before the superblock existed: no generation was ever published
        print(0, 0); sys.exit(0)
    def cells(off, n): return list(struct.unpack('<%dq' % n, d[off*8:(off+n)*8]))
    MAGIC = int.from_bytes(b'BEBOPST1', 'little')
    def valid(sb):
        c = cells(sb, 16); return c[0] == MAGIC and (c[15] & M) == zlib.crc32(d[sb*8:(sb+15)*8])
    sbs = [sb for sb in (0, 512) if valid(sb)]
    assert sbs, 'no valid superblock'
    sb = max(sbs, key=lambda s: cells(s, 16)[2]); c = cells(sb, 16); g = c[2]; cur = c[3]
    acc = 0; n = 0
    while cur:
        h0, h1, val, ref = cells(cur, 4)
        assert (h0 & 0xFFFFFFFF) == 2 and (h1 >> 32) & 0xFFFFFFFF == zlib.crc32(d[(cur+2)*8:(cur+4)*8]), ('torn object', cur)
        acc = (acc * 31 + val) & M; n += 1; cur = cur + ref if ref else 0
    assert n == 100 * g, (n, g)
    other = [s for s in (0, 512) if s != sb][0]
    og = cells(other, 16)[2] if valid(other) else None
    assert og is None or og == g - 1 or (og == 0 and g <= 1), ('other superblock generation', og, g)
    print(g, s64(acc))
else:
    print(fold_for(int(sys.argv[1]) if len(sys.argv) > 1 else 10000))
