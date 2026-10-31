#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""One changed byte, as a forger makes it (tools/gates/maingoal.sh's kills). Stdlib only.

    python3 -I maingoal.forge.py <mode> <in> <out>

Modes marked "resealed" also fix the object's crc and the last commit's anchor (FORMAT §7: cell
at+9 = (mark << 32) | crc32(cells[mark..used)), then the superblock's own crc), so the bytes are
whole and only the check that mode aims at can catch it. The cell arithmetic is FORMAT §5/§9.4, the
same as tools/verify/gate.sh's: an object is h0 (digest << 32 | payload cells), h1 (crc << 32 | ..),
payload; payload cell i of object o is cell o + 2 + i; a ref is relative to its object.

  rec-bit     newest record, a payload bit; crc NOT fixed             -> the crc refuses (10)
  kept-bit    oldest kept record, a payload bit; resealed             -> its id no longer hashes (broken)
  seal-old    a signature byte of the second-newest seal; resealed    -> seals_bad, ML-DSA only
  seal-new    a signature byte of the newest seal; resealed           -> the TRUNCATE it covers is not effective
  proof-byte  a signature byte of the newest anchor's proof; resealed -> that anchor is BAD
  seg-range   the segment's `last` moved by one; resealed             -> no TRUNCATE names it (75)
  zero-revoked  the first KEY with a revoked_at: zeroed; resealed      -> the records and the table disagree
  unsealed-tail the first HEADS row moved back one seal; resealed       -> the newest record is under no seal
  wasm-byte   byte 100 of an archive's embedded reader                -> read.mjs refuses before running it
"""
import struct
import sys
import zlib

SB_MAGIC = 3554557610294396226
PROOF_SIG_BYTE = 3000  # past the 1 952 B public key of a TSA mark: inside the signature


class Image:
    def __init__(self, path):
        self.b = bytearray(open(path, "rb").read())
        self.sb = max((at for at in (0, 512) if self.i(at) == SB_MAGIC), key=lambda at: self.i(at + 2))
        self.doc = self.i(self.i(self.sb + 3) + 18)

    def i(self, cell):
        return struct.unpack_from("<q", self.b, 8 * cell)[0]

    def u(self, cell):
        return struct.unpack_from("<Q", self.b, 8 * cell)[0]

    def put(self, cell, v):
        struct.pack_into("<Q", self.b, 8 * cell, v & (2**64 - 1))

    def ref(self, obj, i):
        r = self.i(obj + 2 + i)
        if r == 0:
            raise SystemExit("maingoal.forge: object %d payload cell %d is an empty ref" % (obj, i))
        return obj + r

    def reseal(self, obj):
        """The object's crc, then the last commit's anchor, then the superblock's crc."""
        n = self.u(obj) & 0xFFFFFFFF
        crc = zlib.crc32(self.b[8 * (obj + 2): 8 * (obj + 2 + n)])
        self.put(obj + 1, (crc << 32) | (self.u(obj + 1) & 0xFFFFFFFF))
        at = self.sb
        anc = self.u(at + 9)
        if anc:
            mark, used = anc >> 32, self.u(at + 4)
            self.put(at + 9, (mark << 32) | zlib.crc32(self.b[8 * mark: 8 * used]))
            self.put(at + 15, (self.u(at + 15) & ~0xFFFFFFFF) | zlib.crc32(self.b[8 * at: 8 * (at + 15)]))

    def newest_record(self):
        return self.ref(self.ref(self.doc, 1), 1)

    def oldest_record(self):
        rec = self.newest_record()
        while self.i(rec + 2 + 2):
            rec = rec + self.i(rec + 2 + 2)
        return rec

    def seals(self):
        """The first head's seal chain, newest first."""
        out = [self.ref(self.ref(self.doc, 3), 1 + 9)]
        while self.i(out[-1] + 2 + 11):
            out.append(self.ref(out[-1], 11))
        return out


def main(mode, src, dst):
    im = Image(src)
    if mode == "rec-bit":
        im.b[8 * (im.newest_record() + 2 + 16)] ^= 1
    elif mode == "kept-bit":
        rec = im.oldest_record()
        im.b[8 * (rec + 2 + 16)] ^= 1
        im.reseal(rec)
    elif mode in ("seal-old", "seal-new"):
        s = im.seals()[0 if mode == "seal-new" else 1]
        im.b[8 * (s + 2 + 15) + 100] ^= 1
        im.reseal(s)
    elif mode == "proof-byte":
        a = im.ref(im.doc, 8)
        im.b[8 * (a + 2 + 9) + PROOF_SIG_BYTE] ^= 1
        im.reseal(a)
    elif mode == "seg-range":
        s = im.ref(im.doc, 9)
        im.put(s + 2 + 2, im.u(s + 2 + 2) + 1)
        im.reseal(s)
    elif mode == "zero-revoked":
        t = im.ref(im.doc, 4)
        for i in range(im.i(t + 2)):
            k = im.ref(t, 1 + 5 * i + 4)
            if im.i(k + 2 + 6) != 0:
                im.put(k + 2 + 6, 0)
                im.reseal(k)
                break
        else:
            raise SystemExit("maingoal.forge: no KEY in this file has a revoked_at")
    elif mode == "unsealed-tail":
        t, s = im.ref(im.doc, 3), im.seals()[1]
        im.put(t + 2 + 1 + 4, im.u(s + 2 + 5))
        for j in range(4):
            im.put(t + 2 + 1 + 5 + j, im.u(s + 2 + 6 + j))
        im.put(t + 2 + 1 + 9, s - t)
        im.reseal(t)
    elif mode == "wasm-byte":
        im.b[(1024 + im.i(1027) + 3) * 8 + 100] ^= 1
    else:
        raise SystemExit("maingoal.forge: unknown mode " + mode)
    open(dst, "wb").write(im.b)


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    main(*sys.argv[1:])
