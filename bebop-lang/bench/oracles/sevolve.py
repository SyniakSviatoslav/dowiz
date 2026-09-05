# G3 sevolve oracle (T114): the four folds from the spec (v1 writes 1000 P{a,b}; v2 appends
# 1000 P{a,b,c}; v1 reads v2's store ignoring c; v3 migrates every P to Q{a+b,c} and
# compacts). When sevolve.store exists after the run: the migration record must survive
# compaction (superblock cell 6 -> M{from,to,sha}) and superseded must be 0.
import hashlib, os, struct, zlib
M = (1 << 64) - 1
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
v = 42; P = []
for i in range(2000):
    v = (v * 6364136223846793005 + 1442695040888963407) & M
    a = v; b = v ^ i; c = (a + b) & M if i >= 1000 else None
    P.append((a, b, c))
def f1(objs): 
    acc = 0
    for a, b, c in objs: acc = (acc * 31 + a * 7 + b) & M
    return acc
def f2(objs):
    acc = 0
    for a, b, c in objs: acc = (acc * 31 + a * 7 + b + (c or 0) * 3) & M
    return acc
def fq(objs):
    acc = 0
    for a, b, c in objs: acc = (acc * 31 + ((a + b) & M) * 5 + (c or 0)) & M
    return acc
F1 = f1(P[:1000]); F2 = f2(P); F3 = f1(P); F4 = fq(P)
res = (((F1 * 31 + F2) * 31 + F3) * 31 + F4) & M
if os.path.exists('sevolve.store'):
    d = open('sevolve.store', 'rb').read()
    def cells(off, n): return list(struct.unpack('<%dq' % n, d[off*8:(off+n)*8]))
    MAGIC = int.from_bytes(b'BEBOPST1', 'little')
    def valid(sb):
        c = cells(sb, 16); return c[0] == MAGIC and (c[15] & M) == zlib.crc32(d[sb*8:(sb+15)*8])
    sb = max([s for s in (0, 512) if valid(s)], key=lambda s: cells(s, 16)[2]); c = cells(sb, 16)
    assert c[8] == 0 and c[6] != 0, ('compaction record', c[:9])
    m = cells(c[6], 5); dq = int.from_bytes(hashlib.sha256(b'Q{i64,i64}').digest()[28:32], 'big')
    assert (m[0] & 0xFFFFFFFF) == 3 and m[3] == dq, ('migration table', m)
    assert len(d) <= (c[7] + 1024) * 8 + 2 * 4096, len(d)
print(s64(res))
