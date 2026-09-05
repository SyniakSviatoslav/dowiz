# G1 slayout oracle (T112): the store file bytes from the layout rules alone
# (docs/LANG-DB-DESIGN.md §4a/§4b), no bebop involved. Prints crc32 of the 65536-byte file.
import hashlib, struct, zlib
MAGIC = int.from_bytes(b'BEBOPST1', 'little')
def digest32(layout): return int.from_bytes(hashlib.sha256(layout.encode()).digest()[28:32], 'big')
def sb(gen, root, used, live, sup):
    cells = [MAGIC, 1, gen, root, used, 0, 0, live, sup] + [0] * 6
    b = b''.join(struct.pack('<q', c) for c in cells)
    return b + struct.pack('<Q', zlib.crc32(b))
def obj(layout, gen, payload):
    pl = b''.join(struct.pack('<q', c) for c in payload)
    h0 = (digest32(layout) << 32) | len(payload)
    h1 = (zlib.crc32(pl) << 32) | gen
    return struct.pack('<QQ', h0, h1) + pl
P = 1024; R = P + 2 + 3; A = R + 2 + 2; E = A + 2 + 4; USED = E + 2 + 2
arena = obj('P{i64,i64,i64}', 1, [3, 5, 7]) + obj('R{ref P,i64}', 1, [P - R, 11]) + \
        obj('A{arr i64}', 1, [3, 10, 20, 30]) + obj('E{Some(i64)|None}', 1, [0, 42])
f = sb(0, 0, 1024, 0, 0).ljust(4096, b'\0') + sb(1, R, USED, USED - 1024, 0).ljust(4096, b'\0') + arena
f = f.ljust(65536, b'\0')
assert len(f) == 65536
print(zlib.crc32(f))
