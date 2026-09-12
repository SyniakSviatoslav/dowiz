# G2 sround oracle (T112): the fold from the LCG spec alone; when sround.store exists
# it is also parsed by the layout rules (superblock pick, object-relative refs) and
# must give the same fold. Prints the fold as a signed i64.
# B5 step 1 FIX: now dereferences the PartTab to get the root (was accessing cell 3 directly).
import os, struct, zlib
from storelib import cells, mask64, pick_live, parttab

M = mask64()
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
v = 42; vals = []
for k in range(100000):
    v = (v * 6364136223846793005 + 1442695040888963407) & M; vals.append(v)
acc = 0
for x in reversed(vals): acc = (acc * 31 + x) & M
if os.path.exists('sround.store'):
    d = open('sround.store', 'rb').read()
    sb = pick_live(d)
    assert sb >= 0, 'no valid superblock'
    
    # B5 step 1: dereference PartTab to get the root
    pt = parttab(d, sb)
    root = pt['payload'][0] if pt else None
    assert root is not None, 'no PartTab'
    
    cur = root; acc2 = 0; n = 0
    while cur:
        h0, h1, val, ref = cells(d, cur, 4)
        assert (h0 & 0xFFFFFFFF) == 2
        acc2 = (acc2 * 31 + val) & M; n += 1
        cur = cur + ref if ref else 0
    assert n == 100000 and acc2 == acc, (n, acc2, acc)
print(s64(acc))
