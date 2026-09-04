# dispatcher: .bt rank-4 tensor pack/unpack round-trip (rt) + accumulate w8 over the set bits of activity word 94;
# fold = sum*1e6 + rt*1e3*(popcount == fired count) + n
import struct
d = [1, 1, 8, 1]
w8 = [i * i + 3 for i in range(8)]
blob = b"BT4R" + struct.pack("<II", 1, 4) + struct.pack("<4I", *d) + struct.pack("<8q", *w8)
hdr, dims, data = blob[:12], struct.unpack("<4I", blob[12:28]), list(struct.unpack("<8q", blob[28:]))
rt = int(hdr == b"BT4R" + struct.pack("<II", 1, 4) and list(dims) == d and data == w8 and len(blob) == 92)
act = 94
fired = [i for i in range(64) if act >> i & 1]
acc = sum(w8[i] for i in fired)
n = len(fired)
nok = int(n == bin(act).count("1"))
print(acc * 10**6 + rt * 1000 * nok + n)
