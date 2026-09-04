# bt: .bt rank-4 codec — pack 2x3x2x2 tensor (magic BT4R, u32 ver=1, rank=4, dims, i64 LE data),
# FNV-1a 64 vs golden bt_fnv, unpack round-trip, stride offset(1,2,1,0)==22; fold = base-131 flag chain (i64 wrap)
import os, struct
M = (1 << 64) - 1
def i64(x):
    x &= M
    return x - (1 << 64) if x >> 63 else x
gold = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "vs_rust", "spectral_golden", "golden.txt")).read()
sec = gold.split(".bt RANK-4 GOLDEN")[1]
gfnv = int([l for l in sec.splitlines() if l.startswith("bt_fnv:")][0].split()[1])
gdata = [int(v) for v in [l for l in sec.splitlines() if l.startswith("bt_data:")][0].split()[1:]]
dims = (2, 3, 2, 2)
data = [((k * 2654435761 + 7) & ((1 << 44) - 1)) - (1 << 43) for k in range(24)]
buf = b"BT4R" + struct.pack("<II4I", 1, 4, *dims) + struct.pack("<24q", *data)
h = 0xcbf29ce484222325
for byte in buf:
    h = ((h ^ byte) * 0x100000001b3) & M
# unpack
magic, ver, rank = buf[:4], *struct.unpack("<II", buf[4:12])
d2 = struct.unpack("<4I", buf[12:28])
data2 = list(struct.unpack("<24q", buf[28:]))
rc = 0 if (magic == b"BT4R" and ver == 1 and rank == 4) else -1
off = lambda i, j, k, l: ((i * dims[1] + j) * dims[2] + k) * dims[3] + l
flags = [len(buf) == 220, h == gfnv and i64(h) == i64(gfnv) and data == gdata, rc == 0]
flags += [data2[k] != data[k] for k in range(24)]
flags += [off(1, 2, 1, 0) == 22]
acc = 0
for fl in flags:
    acc = i64(acc * 131 + int(fl))
acc = i64(acc * 131 + int(d2[0] == 2) + int(d2[3] == 2))
print(acc)
