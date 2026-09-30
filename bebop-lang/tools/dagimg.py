#!/usr/bin/env python3
"""dagimg.py -- an independent reader of the compile-node memo image `<out>.dag` (DG4;
docs/design/SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md §4, layout in selfhost/prelude/dagc.bp's header).
It reads the image from BYTES (nothing imported from bebop) and checks what the writer claims:
the store superblocks and PartTab crcs, every object's crc, ROOT magic/version, IDX sorted by key,
each ENTRY's key == (zlib crc32 of its SRC cells << 32) | src_len, and -- given the source the
image was compiled from -- that every ENTRY's SRC bytes are a real fn span of that source.

  python3 tools/dagimg.py <out>.dag [--src <expanded source, e.g. <out>.use or the .bp>]
Prints `dagimg: <n> entries, gen <g>, ok` (exit 0) or the first violation (exit 1).
"""
import struct, sys, zlib

MAGIC_ST = 3554557610294396226
MAGIC_DAG = sum(c << (8 * i) for i, c in enumerate(b"\0\0DAGMO1"))


def s64(u):
    u &= (1 << 64) - 1
    return u - (1 << 64) if u >> 63 else u


class Img:
    def __init__(self, data):
        self.d = data
        self.n = len(data) // 8

    def c(self, i):
        return s64(struct.unpack_from("<Q", self.d, i * 8)[0])

    def crc(self, off, n):
        return zlib.crc32(self.d[off * 8:(off + n) * 8]) & 0xFFFFFFFF

    def obj_ok(self, o):
        ln = self.c(o) & 0xFFFFFFFF
        return 1 <= ln and o + 2 + ln <= self.n and (self.c(o + 1) >> 32) & 0xFFFFFFFF == self.crc(o + 2, ln)

    def ln(self, o):
        return self.c(o) & 0xFFFFFFFF

    def get(self, o, i):
        return self.c(o + 2 + i)

    def ref(self, o, i):
        return o + self.get(o, i)


def fail(msg):
    print("dagimg: FAIL " + msg)
    sys.exit(1)


def main():
    a = sys.argv[1:]
    if not a:
        sys.exit(__doc__)
    img = Img(open(a[0], "rb").read())
    src = open(a[a.index("--src") + 1], "rb").read() if "--src" in a else None
    sbs = [sb for sb in (0, 512) if img.c(sb) == MAGIC_ST and img.c(sb + 15) & 0xFFFFFFFF == img.crc(sb, 15)]
    if not sbs:
        fail("no valid superblock")
    sb = max(sbs, key=lambda x: img.c(x + 2))
    pt = img.c(sb + 3)
    if not img.obj_ok(pt):
        fail("PartTab crc")
    root = img.get(pt, 16)
    if not img.obj_ok(root) or img.ln(root) != 9:
        fail("root object")
    if img.get(root, 0) != MAGIC_DAG or img.get(root, 1) != 1:
        fail("root magic/version")
    n, gen = img.get(root, 4), img.get(root, 6)
    idx = img.ref(root, 5)
    if n and not img.obj_ok(idx):
        fail("IDX crc")
    keys = [img.c(idx + 2 + 3 * p) for p in range(n)]
    if keys != sorted(keys):
        fail("IDX not sorted by key")
    for p in range(n):
        e = idx + img.c(idx + 3 + 3 * p)
        if not img.obj_ok(e) or img.ln(e) != 13:
            fail("ENTRY %d crc/len" % p)
        srco = img.ref(e, 7)
        if not img.obj_ok(srco) or not img.obj_ok(img.ref(e, 8)) or not img.obj_ok(img.ref(e, 11)):
            fail("ENTRY %d object crc" % p)
        if img.get(e, 9) and not img.obj_ok(img.ref(e, 9)):
            fail("ENTRY %d RELOCS crc" % p)
        slen = img.get(e, 2)
        want = s64((img.crc(srco + 2, (slen + 7) // 8) << 32) | (slen & 0xFFFFFFFF))
        if img.get(e, 0) != want or keys[p] != want:
            fail("ENTRY %d key %d != crc/len %d" % (p, img.get(e, 0), want))
        if img.get(e, 3) < 1 or img.get(e, 4) * 2 != (img.ln(img.ref(e, 9)) if img.get(e, 9) else 0):
            fail("ENTRY %d nwords/nrelocs" % p)
        if src is not None:
            b = img.d[(srco + 2) * 8:(srco + 2) * 8 + slen]
            if src.find(b) < 0:
                fail("ENTRY %d SRC is not a span of the source" % p)
    print("dagimg: %d entries, gen %d, ok" % (n, gen))


if __name__ == "__main__":
    main()
