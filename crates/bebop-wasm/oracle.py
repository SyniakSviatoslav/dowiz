#!/usr/bin/env python3
"""The fourth reader: a KV image folded in Python from the bytes alone.

Nothing here is imported from the Rust side or from bebop; the layout is the
one `crates/bebop-store/src/lib.rs:1-22` and `kv.rs:1-10` document, re-derived
so that a defect in either implementation cannot be inherited. Prints
`kv status=<n> n=<count> root=<fold>` in the harness's shape; a refusal is a
non-zero status and a non-zero exit.
"""
import struct
import sys
import zlib

MAGIC = 3554557610294396226  # "BEBOPST1" as a little-endian i64
FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
M64 = (1 << 64) - 1


def fnv(h, data):
    for b in data:
        h = ((h ^ b) * FNV_PRIME) & M64
    return h


def to_i64(u):
    return u - (1 << 64) if u >= (1 << 63) else u


def main(path):
    raw = open(path, "rb").read()
    n = len(raw) // 8
    cells = struct.unpack("<%dq" % n, raw[: n * 8])

    def sb_ok(at):
        if at + 16 > n or cells[at] != MAGIC:
            return False
        crc = zlib.crc32(struct.pack("<15q", *cells[at : at + 15]))
        return crc == cells[at + 15] & 0xFFFFFFFF

    live = None
    for at in (0, 512):
        if sb_ok(at) and (live is None or cells[at + 2] > cells[live + 2]):
            live = at
    if live is None or cells[live + 4] > n:
        print("kv status=1 n=0 root=0")
        return 1
    pt = cells[live + 3]
    root = cells[pt + 18] if pt > 0 and pt + 18 < n else 0
    if root <= 0 or root + 2 > n:
        print("kv status=2 n=0 root=0")
        return 2

    def obj_len(o):
        return cells[o] & 0xFFFFFFFF

    def follow(o, i):
        off = cells[o + 2 + i]
        t = o + off
        return t if off != 0 and 0 <= t and t + 2 <= n else None

    count = cells[root + 2]
    arrays = [follow(root, i) for i in (1, 2, 3, 4)]
    if count < 0 or any(a is None for a in arrays):
        print("kv status=2 n=0 root=0")
        return 2
    kidx, kblob, vidx, vblob = arrays

    def slice_of(idx, blob, i):
        off, ln = cells[idx + 2 + 2 * i], cells[idx + 2 + 2 * i + 1]
        if off < 0 or ln < 0 or off + ln > obj_len(blob) or blob + 2 + off + ln > n:
            return None
        return bytes(cells[blob + 2 + off + j] & 0xFF for j in range(ln))

    if 2 * count > min(obj_len(kidx), obj_len(vidx)):
        print("kv status=2 n=0 root=0")
        return 2
    h = FNV_OFFSET
    for i in range(count):
        k, v = slice_of(kidx, kblob, i), slice_of(vidx, vblob, i)
        if k is None or v is None:
            print("kv status=2 n=0 root=0")
            return 2
        h = fnv(h, struct.pack("<Q", len(k)))
        h = fnv(h, k)
        h = fnv(h, struct.pack("<Q", len(v)))
        h = fnv(h, v)
    print(f"kv status=0 n={count} root={to_i64(h)}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: oracle.py <image>", file=sys.stderr)
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
