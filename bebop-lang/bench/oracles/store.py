# store: pack rank-4 tensor dims (2,3,2,2), data[k]=((k*2654435761+7)&(2^44-1))-2^43 into the 220-byte "BT4R" v1 LE stream; FNV-1a 64 == golden bt_fnv; atomic tmp->rename publish, read back, FNV again, unpack round-trip. fold = i64-wrapped Horner base 131 over the check bits (7 ok bits, 24 mismatch bits, offset(1,2,1,0)==22, d2[0]==2 + d2[3]==2)
import os, tempfile, struct
dims = [2, 3, 2, 2]
data = [((k * 2654435761 + 7) & ((1 << 44) - 1)) - (1 << 43) for k in range(24)]
stream = b"BT4R" + struct.pack("<II4I", 1, 4, *dims) + struct.pack("<24q", *data)
def fnv(b):
    h = 0xcbf29ce484222325
    for c in b: h = ((h ^ c) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return h
gold = 12242088766677946451
checks = [len(stream) == 220, fnv(stream) == gold]
# atomic publish: write tmp, rename onto out, read back
fd, tmp = tempfile.mkstemp(); out = tmp + ".out"
os.write(fd, stream); os.close(fd); os.rename(tmp, out); checks.append(True)   # export+close+rename all 0
rfd = os.open(out, os.O_RDONLY); checks.append(rfd >= 0)
rb = os.read(rfd, 220); os.close(rfd); os.remove(out)
checks.append(len(rb) == 220); checks.append(fnv(rb) == gold)
checks.append(rb[:12] == b"BT4R" + struct.pack("<II", 1, 4))   # unpack header rc==0
d2 = struct.unpack("<4I", rb[12:28]); data2 = struct.unpack("<24q", rb[28:])
checks += [data2[k] != data[k] for k in range(24)]
checks.append(((1 * dims[1] + 2) * dims[2] + 1) * dims[3] + 0 == 22)
acc = 0
for c in checks: acc = acc * 131 + int(c)
acc = (acc * 131 + int(d2[0] == 2) + int(d2[3] == 2)) & 0xFFFFFFFFFFFFFFFF   # i64 wrap
print(acc - (1 << 64) if acc >> 63 else acc)
