#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""W-MG-ATK3: a bebop image written cell by cell, so one named cell can lie. Stdlib only. NOT a
fuzzer: tools/gates/atk3_cases*.py name every image by hand. The layouts are FORMAT §1-§9.5 and the
writer's (`crates/bebop-log/src/lib.rs` stage_append_in, `doc/*.rs` stage_*, `bebop-kv` tree)."""
import hashlib
import struct
import zlib

M64 = (1 << 64) - 1
MAGIC = 3554557610294396226
ARENA = 1024
D_KV_ROOT, D_LOG, D_EV = 610082063, 0x45564C47, 0x45565F52
D_LEAF, D_NODE, D_VAL = 284439384, 3702325768, 44497123
D_DOC, D_SEAL, D_HEADS, D_KEY, D_KEYS = 2619226223, 1951851249, 4237402659, 2540354000, 1448248784
D_SEG, D_ANCH, D_SPEC, D_BLOB = 2192499951, 180762881, 849619745, 799774958
FLAG_ACTOR, FLAG_SHA256 = 1, 2
ZERO = bytes(32)


def cells_of(b):
    """Little-endian cells, eight bytes to a cell, the last one zero-padded."""
    b = bytes(b) + bytes(-len(b) % 8)
    return list(struct.unpack("<%dQ" % (len(b) // 8), b))


def id4(b):
    return list(struct.unpack("<4Q", b))


class Img:
    def __init__(self):
        self.c = [0] * ARENA

    def obj(self, digest, payload, ln=None):
        """An object at the arena's end; `ln` lets the header lie about the payload's length."""
        at = len(self.c)
        self.c += [0, 0] + [v & M64 for v in payload]
        self.c[at] = (digest << 32) | ((len(payload) if ln is None else ln) & 0xFFFFFFFF)
        self.reseal(at, len(payload) if ln is None else min(ln, len(payload)))
        return at

    def crc(self, off, n):
        return zlib.crc32(struct.pack("<%dQ" % n, *self.c[off:off + n]))

    def reseal(self, at, n=None):
        n = self.c[at] & 0xFFFFFFFF if n is None else n
        self.c[at + 1] = self.crc(at + 2, n) << 32

    def put(self, at, i, v):
        self.c[at + 2 + i] = v & M64
        self.reseal(at)

    def link(self, at, i, target):
        self.put(at, i, target - at)

    def commit(self, root, capacity=None, anchor=None, used=None, cut=None):
        """Superblock A + PartTab. `capacity` default = exactly the arena (nothing to pad);
        `anchor` = mark of a true anchor, `cut` = cells of the file kept."""
        c = self.c
        used = len(c) if used is None else used
        cap = used - ARENA if capacity is None else capacity
        anc = 0
        if anchor is not None:
            anc = (anchor << 32) | self.crc(anchor, used - anchor)
        sb = [MAGIC, 2, 1, 16, used, 0, 0, used - ARENA, 0, anc, 0, 0, cap & M64, 0, 0]
        c[0:15] = [v & M64 for v in sb]
        c[15] = self.crc(0, 15)
        c[16] = 19
        c[18:34] = c[0:16]
        c[34], c[35], c[36] = root, used, 1
        c[17] = self.crc(18, 19) << 32
        return struct.pack("<%dQ" % len(c), *c)[: 8 * (len(c) if cut is None else cut)]


# ---- KV -------------------------------------------------------------------------------------
def kv2(im, pairs, ver=2):
    """Base v2 (v1 with ver=1): four arrays and a root. `pairs` are written as given, in order."""
    kb, vb, ki, vi = b"", b"", [], []
    for k, v in pairs:
        ki += [len(kb), len(k)]
        vi += [len(vb), len(v)]
        kb += k
        vb += v
    if ver == 1:
        kc, vc = list(kb), list(vb)
    else:
        kc, vc = cells_of(kb), cells_of(vb)
    kidx, kblob, vidx, vblob = (im.obj(9, x) for x in (ki, kc, vi, vc))
    root = im.obj(D_KV_ROOT, [len(pairs), kidx, kblob, vidx, vblob, ver])
    for i, a in enumerate((kidx, kblob, vidx, vblob)):
        im.link(root, 1 + i, a)
    return root


def kv4(im, pairs, val=None, digest=D_KV_ROOT):
    """One leaf under a v4 root. `val(i)` overrides the value object a slot names (a shared one)."""
    vals = [val(i) if val else im.obj(D_VAL, [len(v)] + cells_of(v)) for i, (_, v) in enumerate(pairs)]
    m, karea = len(pairs), 1 + 2 * len(pairs)
    kb, slots = b"", []
    for (k, _), v in zip(pairs, vals):
        slots += [(len(kb) << 16) | len(k), v]
        kb += k
    leaf = len(im.c)
    im.obj(D_LEAF, [(1 << 56) | m] + [0] * (2 * m) + cells_of(kb))
    for i in range(m):
        im.c[leaf + 2 + 1 + 2 * i] = slots[2 * i]
        im.c[leaf + 2 + 1 + 2 * i + 1] = (vals[i] - leaf) & M64
    im.reseal(leaf)
    root = im.obj(digest, [m, 0, 0, 0, 0, 4, 0, 1])
    im.link(root, 6, leaf)
    return root


# ---- LOG ------------------------------------------------------------------------------------
class Rec:
    """A record to lay out. Default: chained under the v2 id, actor named."""

    def __init__(self, actor, seq, prev, payload, flags=FLAG_SHA256, rid=None, ln=None):
        self.actor, self.seq, self.prev, self.payload, self.flags, self.ln = actor, seq, prev, payload, flags, ln
        self.id = chained_id(flags, actor, seq, prev, payload) if rid is None else rid


def chained_id(flags, actor, seq, prev, payload):
    return hashlib.sha256(b"ein-record/1" + struct.pack("<q", flags) + actor + struct.pack("<Q", seq) + prev + payload).digest()


def nonce(body=b""):
    return b"N" * 16 + body


def lay_log(im, recs, tip=None, n=None):
    """Records oldest first, then a v2 root. Returns (root, [record objects])."""
    objs, last = [], 0
    for r in recs:
        p = [r.seq, len(r.payload), 0] + id4(r.id) + id4(r.prev) + [r.flags | (FLAG_ACTOR if r.actor != ZERO else 0)]
        if r.actor != ZERO:
            p += id4(r.actor)
        o = im.obj(D_EV, p + cells_of(r.payload), r.ln)
        if last:
            im.c[o + 2 + 2] = (last - o) & M64
            im.reseal(o, len(p) + len(cells_of(r.payload)) if r.ln is None else r.ln)
        objs.append(o)
        last = o
    root = im.obj(D_LOG, [len(recs) if n is None else n, 0, 1 if tip else 0] + id4(tip or ZERO) + [2])
    if last:
        im.link(root, 1, last)
    return root, objs


# ---- DOC ------------------------------------------------------------------------------------
def heads_tab(im, rows):
    """rows: (actor, seq, tip, seal object or 0)."""
    p = [len(rows)]
    for a, s, t, _ in rows:
        p += id4(a) + [s] + id4(t) + [0]
    t = im.obj(D_HEADS, p)
    for i, r in enumerate(rows):
        if r[3]:
            im.link(t, 1 + 10 * i + 9, r[3])
    return t


def key_obj(im, actor, revoked=0, pk=b"K" * 8):
    return im.obj(D_KEY, id4(actor) + [65, 0, revoked, 0, len(pk)] + cells_of(pk))


def keys_tab(im, rows, m=None):
    """rows: (actor, key object)."""
    p = [len(rows) if m is None else m]
    for a, _ in rows:
        p += id4(a) + [0]
    t = im.obj(D_KEYS, p)
    for i, r in enumerate(rows):
        im.link(t, 1 + 5 * i + 4, r[1])
    return t


def seal_obj(im, actor, seq, tip, sig=b"S" * 8):
    return im.obj(D_SEAL, [1] + id4(actor) + [seq] + id4(tip) + [0, 0, 0, 65, len(sig)] + cells_of(sig))


def seg_obj(im, first, last, root=b"R" * 32, rows=(), redacted=0, declared=0):
    p = [1, first, last] + id4(root) + [0, 0, redacted, declared, len(rows)]
    for a, s, t, sch in rows:
        p += id4(a) + [s] + id4(t) + [sch]
    return im.obj(D_SEG, p)


def seg_hash(first, last, root, rows, redacted, declared, below=ZERO):
    m = b"ein-seg/1" + below + root + b"\0" + struct.pack("<QQQQQ", first, last, redacted, declared, len(rows))
    for a, s, t, sch in rows:
        m += a + struct.pack("<Q", s) + t + struct.pack("<q", sch)
    return hashlib.sha256(m).digest()


def truncate_body(first, last, root, rows, redacted=0, declared=0, prev_root=ZERO):
    return (b"ein-truncate/1" + root + prev_root + struct.pack("<QQQQ", first, last, redacted, declared)
            + seg_hash(first, last, root, rows, redacted, declared))


def doc_root(im, log=0, tree=0, heads=0, keys=0, spec=0, anch=0, seg=0, flags=1):
    d = im.obj(D_DOC, [flags, 0, 0, 0, 0, 5, 0, 5, 0, 0])
    for i, t in ((1, log), (2, tree), (3, heads), (4, keys), (6, spec), (8, anch), (9, seg)):
        if t:
            im.link(d, i, t)
    return d
