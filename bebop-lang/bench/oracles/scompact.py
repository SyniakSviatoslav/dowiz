# G4 scompact oracle (T113): live fold from the spec; when scompact.store exists after a
# run, its size must be <= live*8 + 3 pages and its superblock must say superseded == 0.
import os, struct, zlib
M = (1 << 64) - 1
def s64(x): x &= M; return x - (1 << 64) if x >> 63 else x
n = 1000000; v = 42; acc = 0
for i in range(n):
    v = (v * 6364136223846793005 + 1442695040888963407) & M
    val = (v + 1) & M if i < 600000 else v
    acc = (acc + val * (i + 1)) & M
live = n * 3 + n + 3
if os.path.exists('scompact.store'):
    sz = os.path.getsize('scompact.store')
    assert sz <= live * 8 + 3 * 4096, (sz, live * 8)
    d = open('scompact.store', 'rb').read(16 * 8)
    c = struct.unpack('<16q', d)
    assert c[0] == int.from_bytes(b'BEBOPST1', 'little') and c[8] == 0 and c[7] == live, c[:9]
print(s64(acc))
