#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""bebop-verify (Python half): ONE FILE, STANDARD LIBRARY ONLY, no repo, no wasm, no C.

    python3 bebop-verify.py <image> [--dump]

Prints ONE line, the line every reader of docs/STATUS-CODES.md prints --
    kv status=<s> n=<entries> root=<fold>     or     log status=<s> n=<records> root=<fold>
-- exits with the status (0 = read), and says the status in words on stderr. `--dump` prints
every entry (key order) or every record (oldest first).

WHAT IT CHECKS, in the library's order (docs/FORMAT-BEBOP-IMAGE-v2.md; bebop-image
`pick_checked_in` §3.1 -> 1 and `last_commit_in` §7 -> 10 / 41; then the root's family: bebop-kv
`Kv::load_checked` v1-v3 and `TreeIn::verify` v4 -- every reached object's crc -> 10, any claim or
rule that does not hold -> 2, a version above 4 -> 12; bebop-log `EvLog::chain_crc` -> 10, the
count -> 4). A green line says the bytes are whole and the layout holds; it is not a proof that
the content is true. This is a THIRD implementation beside the Rust library (the wasm half) and
crates/bebop-wasm/oracle.py, sharing code with neither; conformance.sh and tools/verify/gate.sh
hold all of them to one answer on every fixture.
"""
import struct
import sys
import zlib

MAGIC = 3554557610294396226
M32, M64 = 0xFFFFFFFF, 0xFFFFFFFFFFFFFFFF
DIGEST_KV_ROOT, DIGEST_EVLOG_ROOT = 610082063, 0x45564C47
DIGEST_KV4_LEAF, DIGEST_KV4_NODE, DIGEST_KV4_VAL = 284439384, 3702325768, 44497123
FNV_OFFSET, FNV_PRIME = 0xCBF29CE484222325, 0x100000001B3
MAX_VALUE_BYTES = 1 << 30
WORDS = {0: "OK", 1: "NO_SUPERBLOCK: no valid superblock, or the live one refused structurally",
         2: "NOT_A_KV: no root, or a KV claim that does not hold", 4: "TRUNCATED: the log's chain does not deliver the root's count",
         10: "BAD_CRC: an object the read reached fails its crc", 12: "UNKNOWN_VERSION: the KV root names a version above 4",
         41: "LAST_COMMIT: the last commit does not hash to its anchor and every object in it checks"}


class Refuse(Exception):
    """The status, as `args[0]`."""


def fnv(h, data):
    for b in data:
        h = ((h ^ b) * FNV_PRIME) & M64
    return h


def to_i64(u):
    return u - (1 << 64) if u >= 1 << 63 else u


def lossy(kb):
    return kb.decode("utf-8", "replace").encode("utf-8")


class Image:
    """The cells; 0 past the end (FORMAT §1). Every claim read out of a cell is bounded here."""

    def __init__(self, raw):
        self.n = len(raw) // 8
        self.b = raw[: self.n * 8]

    def cell(self, i):
        return struct.unpack_from("<q", self.b, 8 * i)[0] if 0 <= i < self.n else 0

    def crc(self, off, cnt):
        inside = max(0, min(cnt, self.n - off)) if off < self.n else 0
        return zlib.crc32(self.b[8 * off: 8 * (off + inside)] + bytes(8 * (cnt - inside)))

    def obj_len(self, o):
        return self.cell(o) & M32

    def obj_cells(self, o):
        return max(0, min(self.obj_len(o), self.n - (o + 2)))

    def digest(self, o):
        return (self.cell(o) & M64) >> 32

    def crc_ok(self, o):
        ln = self.obj_len(o)
        return o + 2 + ln <= self.n and self.crc(o + 2, ln) == (self.cell(o + 1) & M64) >> 32

    def need_crc(self, o):
        if not self.crc_ok(o):
            raise Refuse(10)

    def get(self, o, i):
        return self.cell(o + 2 + i)

    def follow(self, o, i):
        off = self.get(o, i)
        return None if off == 0 or o + off < 0 or o + off + 2 > self.n else o + off

    def back(self, o, i):
        """A ref that must point strictly backward (v4 rule 1)."""
        off = self.get(o, i)
        if off >= 0 or o + off < 0:
            raise Refuse(2)
        return o + off

    def bytes_at(self, at, ln):
        if at + ln > len(self.b):
            raise Refuse(2)
        return self.b[at: at + ln]


def pick(im):
    """FORMAT §3.1: the live superblock (at, arena_used, pt), or 1. No fallback past the crc."""
    def ok(at):
        return at + 16 <= im.n and im.cell(at) == MAGIC and im.crc(at, 15) == im.cell(at + 15) & M32
    a, b = ok(0), ok(512)
    if not (a or b):
        raise Refuse(1)
    at = 512 if (b and not a) or (a and b and im.cell(514) > im.cell(2)) else 0
    pt, used = im.cell(at + 3), im.cell(at + 4)
    if used < 0 or used > im.n or pt < 0 or (pt and pt + 20 >= im.n) or (pt and pt != at + 16):
        raise Refuse(1)
    if pt:
        ln = im.obj_len(pt)
        if ln < 19 or pt + 2 + ln > im.n or im.crc(pt + 2, ln) != (im.cell(pt + 1) & M64) >> 32:
            raise Refuse(1)
        r = im.cell(pt + 18)
        if r < 0 or (r and r + 2 >= im.n):
            raise Refuse(1)
    return at, used, pt


def last_commit(im, at, used):
    """FORMAT §7, the last-commit rule for bytes handed whole: 10 names a torn object, 41 the anchor."""
    anc = im.cell(at + 9) & M64
    if anc == 0:
        return
    mark, want = anc >> 32, anc & M32
    if mark < 1024 or mark > used or used > im.n:
        raise Refuse(41)
    if im.crc(mark, used - mark) == want:
        return
    o = mark
    while o < used:
        im.need_crc(o)
        o += 2 + im.obj_len(o)
    raise Refuse(41)


def blob_bytes(im, blob, ver, off, ln):
    if ver >= 2:
        return im.b[(blob + 2) * 8 + off: (blob + 2) * 8 + off + ln]
    return bytes(im.get(blob, off + j) & 0xFF for j in range(ln))


def kv_chain(im, root, ver):
    """bebop-kv `delta::chain_in`: the v3 records newest first, every claim held, or 2."""
    if ver < 3:
        return []
    if im.obj_cells(root) < 8 or im.get(root, 7) < 0:
        raise Refuse(2)
    d, budget, out, cur = im.get(root, 7), im.n, [], im.follow(root, 6)
    while cur is not None:
        ln = im.obj_cells(cur)
        if len(out) >= d or ln != im.obj_len(cur) or ln < 4:
            raise Refuse(2)
        budget -= ln + 2
        op, kl, vl = im.get(cur, 1), im.get(cur, 2), im.get(cur, 3)
        if budget < 0 or op not in (1, 2) or kl < 0 or vl < 0 or (op == 2 and vl) or 4 + (kl + 7) // 8 + (vl + 7) // 8 != ln:
            raise Refuse(2)
        out.append((cur, op, kl, vl))
        cur = im.follow(cur, 0)
    if len(out) != d:
        raise Refuse(2)
    return out


def kv_base(im, root, ver):
    """bebop-kv `Kv::decode`: the v1/v2 base entries, every slice bounded by its array, or 2."""
    n = im.get(root, 0)
    arrs = [im.follow(root, i) for i in (1, 2, 3, 4)]
    if n < 0 or None in arrs:
        raise Refuse(2)
    kidx, kblob, vidx, vblob = arrs
    unit = 8 if ver >= 2 else 1
    kcells, vcells = im.obj_cells(kblob) * unit, im.obj_cells(vblob) * unit
    if 2 * n > min(im.obj_cells(kidx), im.obj_cells(vidx)):
        raise Refuse(2)
    budget, entries = kcells + vcells, []
    for i in range(n):
        ko, kl, vo, vl = im.get(kidx, 2 * i), im.get(kidx, 2 * i + 1), im.get(vidx, 2 * i), im.get(vidx, 2 * i + 1)
        if min(ko, kl, vo, vl) < 0 or ko + kl > kcells or vo + vl > vcells:
            raise Refuse(2)
        budget -= kl + vl
        if budget < 0:
            raise Refuse(2)
        entries.append((lossy(blob_bytes(im, kblob, ver, ko, kl)), blob_bytes(im, vblob, ver, vo, vl)))
    return entries


def kv_entries(im, root):
    """`Kv::load_checked` v1-v3: crcs (10), then the base and the chain replayed oldest first."""
    ver = im.get(root, 5) if im.obj_cells(root) >= 6 and im.get(root, 5) > 0 else 1
    if ver > 3:
        raise Refuse(12)
    for i in (1, 2, 3, 4):
        a = im.follow(root, i)
        if a is not None:
            im.need_crc(a)
    chain = kv_chain(im, root, ver)
    for rec in chain:
        im.need_crc(rec[0])
    entries = kv_base(im, root, ver)
    if chain and any(entries[i][0] >= entries[i + 1][0] for i in range(len(entries) - 1)):
        raise Refuse(2)
    m = dict(entries)
    for obj, op, kl, vl in reversed(chain):
        k = lossy(im.b[(obj + 6) * 8: (obj + 6) * 8 + kl])
        if op == 1:
            m[k] = im.b[(obj + 6 + (kl + 7) // 8) * 8: (obj + 6 + (kl + 7) // 8) * 8 + vl]
        else:
            m.pop(k, None)
    return sorted(m.items())


def node_at(im, obj, leaf):
    """A v4 node's slots `(key, ref)`, every length and ref bounded (`node_at` + `items`), or 2."""
    want = DIGEST_KV4_LEAF if leaf else DIGEST_KV4_NODE
    ln, meta = im.obj_len(obj), im.get(obj, 0) & M64
    kind, hc, m = meta >> 56, (meta >> 48) & 0xFF, meta & 0xFFFFFFFFFFFF
    if im.digest(obj) != want or obj + 2 + ln > im.n or ln == 0 or kind != (1 if leaf else 2) or (leaf and hc) or m == 0 or m > 65535:
        raise Refuse(2)
    stride, karea = 2 + hc, 1 + (2 + hc) * m
    if karea > ln:
        raise Refuse(2)
    slots = []
    for i in range(m):
        k = im.get(obj, 1 + stride * i + (0 if leaf else 1)) & M64
        koff, klen = k >> 16, k & 0xFFFF
        if koff + klen > (ln - karea) * 8:
            raise Refuse(2)
        slots.append((im.bytes_at((obj + 2 + karea) * 8 + koff, klen), im.back(obj, 1 + stride * i + (1 if leaf else 0))))
    return slots


def kv4_walk(im, obj, level, sep, state, out):
    """`TreeIn::verify` + the entries: crc of every node and value (10), rules 1-5 (2)."""
    im.need_crc(obj)
    slots = node_at(im, obj, level == 1)
    if (level > 1 and len(slots) < 2) or (sep is not None and slots[0][0] != sep):
        raise Refuse(2)
    for k, r in slots:
        if level > 1:
            kv4_walk(im, r, level - 1, k, state, out)
            continue
        if state[0] is not None and state[0] >= k:
            raise Refuse(2)
        im.need_crc(r)
        vlen = im.get(r, 0)
        if im.digest(r) != DIGEST_KV4_VAL or not 0 <= vlen <= MAX_VALUE_BYTES or im.obj_len(r) != 1 + (vlen + 7) // 8:
            raise Refuse(2)
        out.append((k, im.bytes_at((r + 3) * 8, vlen)))
        state[0] = k


def kv4_entries(im, root):
    ln, n, depth = im.obj_len(root), im.get(root, 0), im.get(root, 7)
    if ln != 8 or root + 2 + ln > im.n or any(im.get(root, i) for i in (1, 2, 3, 4)) or n < 0 or not 0 <= depth <= 16:
        raise Refuse(2)
    out = []
    if depth == 0:
        if im.get(root, 6) or n:
            raise Refuse(2)
    else:
        kv4_walk(im, im.back(root, 6), depth, None, [None], out)
    if len(out) != n:
        raise Refuse(2)
    return out


def log_records(im, root):
    """`EvLog::chain_crc` then the walk: records OLDEST FIRST as (seq, payload), or 10 / 4."""
    im.need_crc(root)
    cap, budget, cnt, cur, chain = im.n // 14 + 1, im.n, 0, im.follow(root, 1), []
    while cur is not None:
        cnt += 1
        cost = im.obj_cells(cur) + 2
        if cnt > cap or cost > budget:
            raise Refuse(4)
        budget -= cost
        im.need_crc(cur)
        chain.append(cur)
        cur = im.follow(cur, 2)
    if cnt != im.get(root, 0):
        raise Refuse(4)
    ver = im.get(root, 7) if im.obj_len(root) >= 8 and im.get(root, 7) > 0 else 1
    out, budget = [], im.n
    for obj in chain:
        seq, plen = im.get(obj, 0) & M64, im.get(obj, 1) & M64
        if ver >= 2:
            at = 16 if im.get(obj, 11) & 1 and im.obj_cells(obj) >= 16 else 12
            have = min(max(im.obj_cells(obj) - at, 0), budget)
            payload = im.b[(obj + 2 + at) * 8: (obj + 2 + at + have) * 8][: min(plen, have * 8)]
        else:
            have = min(max(im.obj_cells(obj) - 15, 0), budget)
            payload = bytes(im.get(obj, 15 + j) & 0xFF for j in range(min(plen, have)))
        budget -= have
        out.append((seq, payload))
    out.reverse()
    return out


def read(raw):
    """(family, entries-or-records); raises Refuse."""
    im = Image(raw)
    at, used, pt = pick(im)
    last_commit(im, at, used)
    root = im.cell(pt + 18) if pt else 0
    if root <= 0:
        raise Refuse(2)
    if im.digest(root) == DIGEST_EVLOG_ROOT:
        return "log", log_records(im, root)
    im.need_crc(root)
    ver = im.get(root, 5) if im.obj_cells(root) >= 6 and im.get(root, 5) > 0 else 1
    return "kv", (kv4_entries(im, root) if ver == 4 else kv_entries(im, root))


def main(argv):
    if len(argv) < 2 or argv[1] == "--help":
        print("usage: python3 bebop-verify.py <image> [--dump]", file=sys.stderr)
        return 2
    try:
        family, items = read(open(argv[1], "rb").read())
    except Refuse as r:
        print("kv status=%d n=0 root=0" % r.args[0])
        print(WORDS.get(r.args[0], "status %d" % r.args[0]), file=sys.stderr)
        return r.args[0]
    # the two folds (FORMAT §9.1; lib.rs): LOG = seq8 || len8 || payload oldest first,
    # KV = len8 || key || len8 || value in key order; FNV-1a 64 over the bytes.
    h = FNV_OFFSET
    for a, b in items:
        head = struct.pack("<Q", a) if family == "log" else struct.pack("<Q", len(a)) + a
        h = fnv(h, head + struct.pack("<Q", len(b)) + b)
    print("%s status=0 n=%d root=%d" % (family, len(items), to_i64(h)))
    print("OK; %s; every reached object whole, the last commit anchored" % ("EvLog" if family == "log" else "KV"), file=sys.stderr)
    if "--dump" in argv[2:]:
        for i, (a, b) in enumerate(items):
            print("#%d seq=%d payload=%r" % (i, a, b) if family == "log" else "%r = %r" % (a, b))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
