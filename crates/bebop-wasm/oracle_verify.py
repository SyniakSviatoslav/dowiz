#!/usr/bin/env python3
"""The price-proof oracle (LV-PCD-C, AC-LV-PCD-03): the SECOND reader of a receipt certificate,
written from docs/design/SPEC-PCD-2026-10-27.md and RFC 9162 §2.1, sharing no code with the Rust
(crates/bebop-store/src/merkle.rs), the wasm32 (src/verify.rs) or the bebop (prelude/merkle.bp)
readers. hashlib's sha256 is the only borrowed piece.

    python3 oracle_verify.py <name> <cert>   -> `verify <name> status=<s> root=<hex>`
    python3 oracle_verify.py --make <dir>    -> writes the four gate fixtures (verify.sh)

Statuses: 0 the path climbs to the named root; 50 malformed; 51 mismatch (the root printed is the
one the path climbs to, so a moved byte shows WHERE it went).
"""
import hashlib
import struct
import sys

OK, MALFORMED, MISMATCH = 0, 50, 51
MAX_ID, MAX_DEPTH = 256, 64


def sha(b):
    return hashlib.sha256(b).digest()


def leaf(data):
    return sha(b"\x00" + data)


def node(left, right):
    return sha(b"\x01" + left + right)


def field(b):
    return struct.pack("<Q", len(b)) + b


def price_leaf(pid, price):
    return field(pid) + field(struct.pack("<q", price))


def mth(hs):
    n = len(hs)
    if n == 0:
        return sha(b"")
    if n == 1:
        return hs[0]
    k = 1
    while k * 2 < n:
        k *= 2
    return node(mth(hs[:k]), mth(hs[k:]))


def path(hs, m):
    n = len(hs)
    if n == 1:
        return []
    k = 1
    while k * 2 < n:
        k *= 2
    if m < k:
        return path(hs[:k], m) + [mth(hs[k:])]
    return path(hs[k:], m - k) + [mth(hs[:k])]


def climb(h, index, n, sibs):
    """RFC 9162 §2.1.3.2; None when the shape is impossible."""
    if index >= n:
        return None
    f, s = index, n - 1
    for p in sibs:
        if s == 0:
            return None
        if f & 1 or f == s:
            h = node(p, h)
            while not f & 1 and f != 0:
                f >>= 1
                s >>= 1
        else:
            h = node(h, p)
        f >>= 1
        s >>= 1
    return h if s == 0 else None


def parse(b):
    if not b or b[0] != ord("V"):
        return None
    at, out = 1, []
    for mx in (MAX_ID, 8, 8, 8, 32, 32 * MAX_DEPTH):
        if at + 8 > len(b):
            return None
        ln = struct.unpack_from("<Q", b, at)[0]
        if ln > mx or at + 8 + ln > len(b):
            return None
        out.append(b[at + 8:at + 8 + ln])
        at += 8 + ln
    if at != len(b) or any(len(out[i]) != 8 for i in (1, 2, 3)) or len(out[4]) != 32 or len(out[5]) % 32:
        return None
    pid, price, index, n, root, raw = out
    return (pid, struct.unpack("<q", price)[0], struct.unpack("<Q", index)[0], struct.unpack("<Q", n)[0],
            root, [raw[i:i + 32] for i in range(0, len(raw), 32)])


def check(b):
    c = parse(b)
    if c is None:
        return MALFORMED, bytes(32)
    pid, price, index, n, root, sibs = c
    got = climb(leaf(price_leaf(pid, price)), index, n, sibs)
    if got is None:
        return MALFORMED, bytes(32)
    return (OK if got == root else MISMATCH), got


def encode(pid, price, index, n, root, sibs):
    return (b"V" + field(pid) + field(struct.pack("<q", price)) + field(struct.pack("<Q", index))
            + field(struct.pack("<Q", n)) + field(root) + field(b"".join(sibs)))


def catalog(n=165):
    rows = sorted((("p_%03d" % i).encode(), 100 + 7 * i) for i in range(n))
    return rows, [leaf(price_leaf(p, v)) for p, v in rows]


def make(d):
    rows, hs = catalog()
    i = 7
    good = encode(rows[i][0], rows[i][1], i, len(hs), mth(hs), path(hs, i))
    at = len(good) - 32 * len(path(hs, i))  # byte 0 of the FIRST sibling: the named byte
    moved = bytearray(good)
    moved[at] ^= 0x01
    price = encode(rows[i][0], rows[i][1] + 1, i, len(hs), mth(hs), path(hs, i))
    for name, b in (("good", good), ("path", bytes(moved)), ("price", price), ("short", good[:-1])):
        open("%s/%s.cert" % (d, name), "wb").write(b)


def main(argv):
    if len(argv) == 3 and argv[1] == "--make":
        make(argv[2])
        return 0
    if len(argv) != 3:
        print(__doc__.strip().splitlines()[3], file=sys.stderr)
        return 2
    s, r = check(open(argv[2], "rb").read())
    print("verify %s status=%d root=%s" % (argv[1], s, r.hex()))
    return 0 if s == OK else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
