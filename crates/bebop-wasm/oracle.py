#!/usr/bin/env python3
"""The fourth reader: a KV image folded in Python, or key frames hashed.

Nothing here is imported from the Rust side or from bebop; the layout is the
one `crates/bebop-store/src/lib.rs:1-22` and `kv.rs:1-10` document, re-derived
so that a defect in either implementation cannot be inherited. Prints
`kv status=<n> n=<count> root=<fold>` in the harness's shape; a refusal is a
non-zero status and a non-zero exit.

TWO VERSIONS (DG3, 2026-09-28), and the root says which: a root of six or more
cells with a positive cell 5 names its version; anything else is v1. v1 holds
one byte per cell; v2 holds offsets and lengths in BYTES and the bytes packed
eight to a cell, little-endian. A version above 2 is refused (status 3). The
fold is over bytes, so both versions of the same entries print the same root.

`--key <compile|proj|empty>`: build one node-key frame of fixtures/key.expected
(SPEC-BEBOP-DAG-RUNTIME §2) from its field values and print its length, K64 and
K256 -- the fourth of gate.sh's four key readers.
"""
import struct
import sys
import zlib
import hashlib

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


# --- the node key (DG2, SPEC-BEBOP-DAG-RUNTIME §2) --------------------------
# frame := tag(1) || field*;  field := len(8, u64 LE) || bytes(len).
# A number is an 8-byte field; a list is ONE field holding its elements' fields.
# Built here from the field values fixtures/key.expected lists, never from Rust.
KEY_CD = -7046029254386353131
KEY_SRC = b"fn add(a: i64, b: i64) -> i64 { a + b }"


def key_field(b):
    return struct.pack("<Q", len(b)) + b


def key_num(v):
    return key_field(struct.pack("<q", v))


def key64(frame):
    """K64 = (zlib crc32(frame) << 32) | (len & 0xffffffff), as an i64."""
    return to_i64(((zlib.crc32(frame) & 0xFFFFFFFF) << 32) | (len(frame) & 0xFFFFFFFF))


def key_compile():
    return (b"C" + key_num(KEY_CD) + key_field(KEY_SRC) + key_num(81985529216486895)
            + key_num(-2) + key_num(1311768467294899695))


def key_proj(two):
    inputs = params = b""
    if two:
        inputs = (key_num(1234605616436508552) + key_num(7)
                  + key_field(bytes(range(32))) + key_num(1))
        params = key_num(1790000000000) + key_num(3)
    return (b"P" + key_num(KEY_CD) + key_num(key64(key_compile()))
            + key_field(inputs) + key_field(params))


def key_mode(name):
    frames = {"compile": key_compile, "proj": lambda: key_proj(True),
              "empty": lambda: key_proj(False)}
    if name not in frames:
        print("unknown key fixture: %s (compile|proj|empty)" % name, file=sys.stderr)
        return 2
    f = frames[name]()
    if name == "compile":
        # RT K-3, derived: tag + five length prefixes + the five fields' bytes.
        assert len(f) == 1 + 5 * 8 + 8 + len(KEY_SRC) + 8 + 8 + 8, "K-3"
    print("key %s len=%d k64=%d k256=%s" % (name, len(f), key64(f), hashlib.sha256(f).hexdigest()))
    return 0


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

    ver = 1
    if obj_len(root) >= 6 and root + 2 + 5 < n and cells[root + 2 + 5] > 0:
        ver = cells[root + 2 + 5]
    if ver > 2:
        print("kv status=3 n=0 root=0")
        return 3
    per_cell = 8 if ver == 2 else 1

    count = cells[root + 2]
    arrays = [follow(root, i) for i in (1, 2, 3, 4)]
    if count < 0 or any(a is None for a in arrays):
        print("kv status=2 n=0 root=0")
        return 2
    kidx, kblob, vidx, vblob = arrays

    def slice_of(idx, blob, i):
        off, ln = cells[idx + 2 + 2 * i], cells[idx + 2 + 2 * i + 1]
        if off < 0 or ln < 0 or off + ln > obj_len(blob) * per_cell:
            return None
        if blob + 2 + (off + ln + per_cell - 1) // per_cell > n:
            return None
        if per_cell == 1:
            return bytes(cells[blob + 2 + off + j] & 0xFF for j in range(ln))
        return bytes(
            (cells[blob + 2 + (b >> 3)] >> (8 * (b & 7))) & 0xFF
            for b in range(off, off + ln)
        )

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
    if len(sys.argv) == 3 and sys.argv[1] == "--key":
        sys.exit(key_mode(sys.argv[2]))
    elif len(sys.argv) == 2:
        sys.exit(main(sys.argv[1]))
    else:
        print("usage: oracle.py <image> | oracle.py --key compile|proj|empty", file=sys.stderr)
        sys.exit(2)
