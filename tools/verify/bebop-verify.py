#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""bebop-verify (Python half): ONE FILE, STANDARD LIBRARY ONLY, no repo, no wasm, no C.

    python3 bebop-verify.py <image> [--dump] [--verify]

Prints ONE line, the line every reader of docs/STATUS-CODES.md prints --
    kv status=<s> n=<entries> root=<fold>     log status=<s> n=<records> root=<fold>
    doc status=<s> n=<records> root=<log fold>                  (W-MG-M5, FORMAT §9.4)
-- exits with the status (0 = read), and says the status in words on stderr. `--dump` prints
every entry (key order) or every record (oldest first; a DOC's heads first). `--verify` prints
a DOC's counters on stdout:
    verify records= chained= legacy= redacted= broken= forks= seals= checked= heads= keys= genesis=

WHAT IT CHECKS, in the library's order (docs/FORMAT-BEBOP-IMAGE-v2.md; bebop-image
`pick_checked_in` §3.1 -> 1 and `last_commit_in` §7 -> 10 / 41; then the root's family: bebop-kv
`Kv::load_checked` v1-v3 and `TreeIn::verify` v4 -- every reached object's crc -> 10, any claim or
rule that does not hold -> 2, a version above 4 that is not a DOC -> 12; bebop-log `EvLog::chain_crc`
-> 10, the count -> 4; bebop-log `Doc::open` + `doc::verify` §9.4 -- the root, its log, HEADS, KEYS,
SPEC crc -> 10, then the counters). A green line says the bytes are whole and the layout holds; it
is not a proof that the content is true. A DOC's seals are COUNTED, never checked: this file has
no signature algorithm (`checked=0`). This is a THIRD implementation beside the Rust library (the
wasm half) and crates/bebop-wasm/oracle.py, sharing code with neither; conformance.sh and
tools/verify/gate.sh hold all of them to one answer on every fixture.

GENERATED: tools/verify/assemble.sh concatenates tools/verify/py/*.py into this one file so the
deliverable stays a single download and each source part stays under 300 lines. Edit the parts.
"""
import hashlib
import struct
import sys
import zlib

MAGIC = 3554557610294396226
M32, M64 = 0xFFFFFFFF, 0xFFFFFFFFFFFFFFFF
DIGEST_KV_ROOT, DIGEST_EVLOG_ROOT = 610082063, 0x45564C47
DIGEST_KV4_LEAF, DIGEST_KV4_NODE, DIGEST_KV4_VAL = 284439384, 3702325768, 44497123
# FORMAT §9.4 / §9.5 (W-MG-M1, M3, M1B, M5): the DOC with cell 9 = ref SEG (digest 2619226223; 3669646107 was
# the M1 layout with cell 9 reserved, of which no image exists).
DIGEST_DOC, ROOT_VERSION_DOC, DOC_CELLS = 2619226223, 5, 10
DIGEST_SEAL, DIGEST_HEADS, DIGEST_KEY, DIGEST_KEYS = 1951851249, 4237402659, 2540354000, 1448248784
# SEG rows carry `scheme` since W-MG-M3b (12 + 10m cells, digest 2192499951; 3288852188 was the 9-cell row).
DIGEST_SEG, DIGEST_ANCH, DIGEST_SPEC, DIGEST_BLOB = 2192499951, 180762881, 849619745, 799774958
SEG_HEAD, SEG_ROW, SCHEME_CHAINED = 12, 10, 1
TRUNCATE_MARK, FORGOTTEN_MARK = b"bebop-truncate/1", b"bebop-forgotten/1"
FNV_OFFSET, FNV_PRIME = 0xCBF29CE484222325, 0x100000001B3
MAX_VALUE_BYTES = 1 << 30
MAX_SIG_BYTES = MAX_PK_BYTES = 65536
MAX_BLOB_BYTES = 16 << 20
FLAG_SHA256, NONCE_LEN = 2, 16
ID_TAG, SEG_TAG = b"bebop-record/1", b"bebop-seg/1"  # id v2, the SEG hash (W-MG-HOLES; FORMAT §9.2, §9.4)
REVOKE_MARK, REDACTED_BIT = b"bebop-revoke/1", 0x80
# Word for word bebop-wasm-reader `status.rs` TABLE: tools/gates/maingoal.sh holds the surfaces to one text.
WORDS = {0: "OK: read; n = entries or records, root = the FNV-1a 64 fold (a DOC's root is its log's fold)",
         1: "NO_SUPERBLOCK: no valid superblock, or the live one was refused structurally (arena_used or PartTab outside, PartTab misplaced or its crc bad, root outside)",
         2: "NOT_A_KV: a superblock but no KV root, or a KV claim (a slice, a chain, a v4 rule) that does not hold, or a DOC whose SPEC or v1-v3 TREE this reader does not read",
         4: "TRUNCATED: the log's root claims more records than its chain delivers, or the chain never ends",
         10: "BAD_CRC: an object the read reached fails its crc (root, an array, a delta record, a node, a value, a log record, HEADS, KEYS, SPEC, an object of a torn last commit)",
         12: "UNKNOWN_VERSION: the KV root names a version above 4 and is not a DOC (root-version 5 in both version cells)",
         41: "LAST_COMMIT: the last commit's range does not hash to its anchor and every object in it checks (a re-sealed object, a changed header); never fallen back past",
         75: "CUT_UNDECLARED: a log segment left this file (a SEG under the DOC) and no TRUNCATE record on the kept chain names it by root and range -- records were dropped without the cut being declared"}
UNCHECKED = ("seals COUNTED, not CHECKED: this reader carries no signature algorithm (stdlib, no ML-DSA); "
             "a count is not a proof -- bring a verifier, or the format text's independent readers")


class Refuse(Exception):
    """The status, as `args[0]`."""


def fnv(h, data):
    for b in data:
        h = ((h ^ b) * FNV_PRIME) & M64
    return h


def to_i64(u):
    return u - (1 << 64) if u >= 1 << 63 else u


def sat(v):
    """A counter past i64 saturates: the line prints signed cells, and a file's claim wraps nothing."""
    return min(v, (1 << 63) - 1)


class Image:
    """The cells; 0 past the end (FORMAT §1). Every claim read out of a cell is bounded here."""

    def __init__(self, raw):
        self.n = len(raw) // 8
        self.b = raw[: self.n * 8]
        self.left = self.n

    def spend(self, o):
        """`doc/budget.rs`: one budget of `n` cells for every walk of a `verify`, `2 + c` per object,
        paid before it is read. The objects of an honest file are disjoint and cannot run it out."""
        self.left -= self.obj_cells(o) + 2
        if self.left < 0:
            raise Refuse(2)

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

    def packed(self, o, at, ln):
        """`ln` bytes packed eight to a cell from payload cell `at` of `o`, bounded by the object."""
        have = max(0, min((ln + 7) // 8, self.obj_cells(o) - at))
        return self.b[(o + 2 + at) * 8: (o + 2 + at + have) * 8][:ln]

    def id_at(self, o, at):
        """32 bytes as four cells (zero where the object is short)."""
        return self.packed(o, at, 32).ljust(32, b"\0")


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
        entries.append((blob_bytes(im, kblob, ver, ko, kl), blob_bytes(im, vblob, ver, vo, vl)))
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
    if any(entries[i][0] >= entries[i + 1][0] for i in range(len(entries) - 1)):
        raise Refuse(2)  # strictly ascending in raw bytes, chain or no chain (W-MG-ATK3, main)
    if not chain:
        return entries
    m = dict(entries)
    for obj, op, kl, vl in reversed(chain):
        k = im.b[(obj + 6) * 8: (obj + 6) * 8 + kl]
        if op == 1:
            m[k] = im.b[(obj + 6 + (kl + 7) // 8) * 8: (obj + 6 + (kl + 7) // 8) * 8 + vl]
        else:
            m.pop(k, None)
    return sorted(m.items())


def node_at(im, obj, leaf):
    """A v4 node's slots `(key, ref)`, every length and ref bounded (`node_at` + `items`), or 2: keys
    packed in slot order with no gap, strictly ascending, the object exactly as long as they need."""
    want = DIGEST_KV4_LEAF if leaf else DIGEST_KV4_NODE
    ln, meta = im.obj_len(obj), im.get(obj, 0) & M64
    kind, hc, m = meta >> 56, (meta >> 48) & 0xFF, meta & 0xFFFFFFFFFFFF
    if im.digest(obj) != want or obj + 2 + ln > im.n or ln == 0 or kind != (1 if leaf else 2) or (leaf and hc) or m == 0 or m > 65535:
        raise Refuse(2)
    stride, karea = 2 + hc, 1 + (2 + hc) * m
    if karea > ln:
        raise Refuse(2)
    slots, used = [], 0
    for i in range(m):
        k = im.get(obj, 1 + stride * i + (0 if leaf else 1)) & M64
        koff, klen = k >> 16, k & 0xFFFF
        if koff != used or koff + klen > (ln - karea) * 8:
            raise Refuse(2)
        key = im.bytes_at((obj + 2 + karea) * 8 + koff, klen)
        if slots and slots[-1][0] >= key:
            raise Refuse(2)
        used += klen
        slots.append((key, im.back(obj, 1 + stride * i + (1 if leaf else 0))))
    if ln != karea + (used + 7) // 8:
        raise Refuse(2)
    return slots


def kv4_walk(im, obj, level, sep, state, out):
    """`TreeIn::verify` + the entries: crc of every node and value (10), rules 1-5 (2)."""
    im.spend(obj)
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
        im.spend(r)
        im.need_crc(r)
        vlen = im.get(r, 0)
        if im.digest(r) != DIGEST_KV4_VAL or not 0 <= vlen <= MAX_VALUE_BYTES or im.obj_len(r) != 1 + (vlen + 7) // 8:
            raise Refuse(2)
        out.append((k, im.bytes_at((r + 3) * 8, vlen)))
        state[0] = k


def kv4_entries(im, root):
    ln, n, depth = im.obj_len(root), im.get(root, 0), im.get(root, 7)
    if im.digest(root) != DIGEST_KV_ROOT or ln != 8 or root + 2 + ln > im.n or any(im.get(root, i) for i in (1, 2, 3, 4)) or n < 0 or not 0 <= depth <= 16:
        raise Refuse(2)
    out, left = [], im.left  # the walk's own budget: a value named twice is an overlap (`budget.rs`)
    if depth == 0:
        if im.get(root, 6) or n:
            raise Refuse(2)
    else:
        kv4_walk(im, im.back(root, 6), depth, None, [None], out)
    im.left = left
    if len(out) != n:
        raise Refuse(2)
    return out


def log_version(im, root):
    """FORMAT §9.2: cell 7 when the root has it and it is above 0, else 1."""
    return im.get(root, 7) if im.obj_len(root) >= 8 and im.get(root, 7) > 0 else 1


def log_chain(im, root):
    """`EvLog::chain_crc`: the root and every record's crc (10), the count held to the root's
    claim and the walk bounded (4), a version above 2 refused before it (12). The record objects, NEWEST FIRST."""
    im.need_crc(root)
    if log_version(im, root) > 2:
        raise Refuse(12)
    head = 12 if log_version(im, root) >= 2 else 15
    cap, budget, cnt, cur, chain = im.n // 14 + 1, im.n, 0, im.follow(root, 1), []
    while cur is not None:
        cnt += 1
        cost = im.obj_cells(cur) + 2
        if cnt > cap or cost > budget:
            raise Refuse(4)
        budget -= cost
        if im.obj_cells(cur) < head:
            raise Refuse(4)  # a record cut short: no field of it is read from the cells behind it
        im.need_crc(cur)
        chain.append(cur)
        cur = im.follow(cur, 2)
    if cnt != im.get(root, 0):
        raise Refuse(4)
    return chain


def log_detail(im, root):
    """Every record OLDEST FIRST as a dict: obj, seq, flags, id, prev, actor, payload (bebop-log
    `walk`: v2 packs the payload eight bytes to a cell after the optional actor; v1 one per cell)."""
    chain = log_chain(im, root)
    ver = log_version(im, root)
    out, budget = [], im.n
    for obj in chain:
        seq, plen = im.get(obj, 0) & M64, im.get(obj, 1) & M64
        rid, prev = im.id_at(obj, 3), im.id_at(obj, 7)
        if ver >= 2:
            flags = im.get(obj, 11)
            named = flags & 1 and im.obj_cells(obj) >= 16
            at = 16 if named else 12
            actor = im.id_at(obj, 12) if named else bytes(32)
            have = min(max(im.obj_cells(obj) - at, 0), budget)
            payload = im.b[(obj + 2 + at) * 8: (obj + 2 + at + have) * 8][: min(plen, have * 8)]
        else:
            flags, actor = 0, im.id_at(obj, 11)
            have = min(max(im.obj_cells(obj) - 15, 0), budget)
            payload = bytes(im.get(obj, 15 + j) & 0xFF for j in range(min(plen, have)))
        budget -= have
        out.append({"obj": obj, "seq": seq, "flags": flags & ~1, "id": rid, "prev": prev, "actor": actor, "payload": payload})
    out.reverse()
    return out


def log_records(im, root):
    """Records OLDEST FIRST as (seq, payload): what the LOG fold hashes."""
    return [(r["seq"], r["payload"]) for r in log_detail(im, root)]


def log_tip(im, root):
    """The chain tip the log root names (cells 3..6 when cell 2 says one is set), else None."""
    return im.id_at(root, 3) if root is not None and im.get(root, 2) else None


def is_doc(im, root):
    """bebop-log `Doc::is_doc`: digest, length and BOTH version cells (FORMAT §9.4)."""
    return (im.digest(root) == DIGEST_DOC and im.obj_len(root) == DOC_CELLS and im.obj_cells(root) == DOC_CELLS
            and im.get(root, 5) == ROOT_VERSION_DOC and im.get(root, 7) == ROOT_VERSION_DOC)


def read_heads(im, t):
    """`seal::read_heads`: rows (actor, seq, tip, seal ref) in table order, bounded by the object."""
    if t is None or im.digest(t) != DIGEST_HEADS or im.obj_cells(t) < 1:
        return []
    m = min(max(im.get(t, 0), 0), (im.obj_cells(t) - 1) // 10)
    return [(im.id_at(t, 1 + 10 * i), im.get(t, 1 + 10 * i + 4) & M64, im.id_at(t, 1 + 10 * i + 5), im.follow(t, 1 + 10 * i + 9)) for i in range(m)]


def read_seal(im, at):
    """`seal::read_seal`: (actor, seq, tip, prev ref) or None for a wrong digest, version or length."""
    if im.digest(at) != DIGEST_SEAL or im.obj_cells(at) < 15 or im.get(at, 0) != 1:
        return None
    sig_len = im.get(at, 14)
    if not 0 <= sig_len <= MAX_SIG_BYTES or im.obj_cells(at) != 15 + (sig_len + 7) // 8:
        return None
    return im.id_at(at, 1), im.get(at, 5) & M64, im.id_at(at, 6), im.follow(at, 11)


def key_rows(im, t):
    """`keys::key_rows`: the KEY object of every row with a ref, in table order."""
    if t is None or im.digest(t) != DIGEST_KEYS or im.obj_cells(t) < 1:
        return []
    m = min(max(im.get(t, 0), 0), (im.obj_cells(t) - 1) // 5)
    return [k for k in (im.follow(t, 1 + 5 * i + 4) for i in range(m)) if k is not None]


def read_keys(im, t):
    """`keys::read_keys`: (actor, admit ref, revoked_at) per row whose KEY reads; an unreadable KEY is skipped."""
    out = []
    for k in key_rows(im, t):
        if im.digest(k) != DIGEST_KEY or im.obj_cells(k) < 9:
            continue
        pk_len = im.get(k, 8)
        if 0 <= pk_len <= MAX_PK_BYTES and im.obj_cells(k) == 9 + (pk_len + 7) // 8:
            out.append((im.id_at(k, 0), im.follow(k, 7), im.get(k, 6)))
    return out


def read_segs(im, newest):
    """`seg::read_segs`: every SEG from the DOC's ref SEG back, NEWEST FIRST, each crc checked (10); a
    chain object that is not a SEG, or one that never ends -> 2. A segment is a dict: first, last,
    root, prev_root, redacted, declared, frontier [(actor, seq, tip, scheme)]."""
    out, cur, cap, left = [], newest, im.n // 14 + 1, im.n
    while cur is not None:
        left -= im.obj_cells(cur) + 2
        if len(out) >= cap or left < 0:
            raise Refuse(2)
        im.need_crc(cur)
        if im.digest(cur) != DIGEST_SEG or im.obj_cells(cur) < 12 or im.get(cur, 0) != 1:
            raise Refuse(2)
        m = max(im.get(cur, 11), 0)
        if im.obj_cells(cur) != SEG_HEAD + SEG_ROW * m or im.get(cur, 2) & M64 < im.get(cur, 1) & M64:
            raise Refuse(2)  # `last` < `first` names no range (W-MG-ATK3, main)
        prev = im.follow(cur, 7)
        prev_root = im.id_at(prev, 3) if prev is not None and im.digest(prev) == DIGEST_SEG and im.obj_cells(prev) >= 12 and im.get(prev, 0) == 1 else bytes(32)
        out.append({"first": im.get(cur, 1) & M64, "last": im.get(cur, 2) & M64, "root": im.id_at(cur, 3), "prev_root": prev_root,
                    "blob": im.follow(cur, 8) is not None, "obj": cur,
                    "redacted": im.get(cur, 9) & M64, "declared": im.get(cur, 10) & M64,
                    "frontier": [(im.id_at(cur, SEG_HEAD + SEG_ROW * i), im.get(cur, SEG_HEAD + SEG_ROW * i + 4) & M64,
                                  im.id_at(cur, SEG_HEAD + SEG_ROW * i + 5), im.get(cur, SEG_HEAD + SEG_ROW * i + 9)) for i in range(m)]})
        cur = prev
    return out


def read_anchors(im, newest):
    """`anch::read_anchors`: (kind, over, time_ms, proof_len) NEWEST FIRST; each crc checked (10) as the
    reader does at open. PRESENT, never verified: this file has no adapter for any kind."""
    out, cur, cap, left = [], newest, im.n // 11 + 1, im.n
    while cur is not None and len(out) < cap:
        left -= im.obj_cells(cur) + 2
        if left < 0:
            break
        if im.digest(cur) != DIGEST_ANCH or im.obj_cells(cur) < 9 or im.get(cur, 0) != 1:
            break
        plen = im.get(cur, 8)
        if not 0 <= plen <= (1 << 20) or im.obj_cells(cur) != 9 + (plen + 7) // 8:
            break
        im.need_crc(cur)
        out.append((im.get(cur, 1), im.id_at(cur, 2), im.get(cur, 6), plen))
        cur = im.follow(cur, 7)
    return out


def read_spec(im, at):
    """§9.5: (wasm ref, text ref, sha256, abi) or None for a wrong digest, version or length."""
    if im.digest(at) != DIGEST_SPEC or im.obj_cells(at) != 8 or im.get(at, 0) != 1:
        return None
    return im.follow(at, 1), im.follow(at, 2), im.id_at(at, 3), im.get(at, 7)


def read_blob(im, at):
    """§9.5 BLOB: its bytes, or None (digest, crc, a length past MAX_BLOB_BYTES)."""
    if at is None or im.digest(at) != DIGEST_BLOB or im.obj_cells(at) < 1 or not im.crc_ok(at):
        return None
    ln = im.get(at, 0)
    if not 0 <= ln <= MAX_BLOB_BYTES or im.obj_cells(at) != 1 + (ln + 7) // 8:
        return None
    return im.packed(at, 1, ln)


def legacy_id(payload):
    """dowiz-hub `chain/scheme.rs` `legacy_id`: FNV-1a over four salted lanes, 32 bytes (pre-M0)."""
    out = b""
    for lane in range(4):
        h = FNV_OFFSET ^ ((lane * 0x9E3779B97F4A7C15) & M64)
        out += struct.pack("<Q", fnv(h, payload))
    return out


def chained_id(r):
    """`doc::id_message` under sha256: the tag, then every cell a reader acts on. `flags` holds no
    FLAG_ACTOR (`log_detail`, `Record::flags`). A tombstone's id no longer answers this."""
    return hashlib.sha256(ID_TAG + struct.pack("<q", r["flags"]) + r["actor"] + struct.pack("<Q", r["seq"]) + r["prev"] + r["payload"]).digest()


def is_legacy(r):
    """`ReaderIds::legacy`: never for a FLAG_SHA256 record; else FNV x4 over prev||payload or over the payload."""
    return not r["flags"] & FLAG_SHA256 and r["id"] in (legacy_id(r["prev"] + r["payload"]), legacy_id(r["payload"]))


def is_redacted(r):
    """`ReaderIds::redacted`: the kind byte's high bit, behind the nonce when there is one. It is the only
    mark: a legacy record has no nonce to erase, so an erased nonce with no bit is an edit (FORMAT §9.4)."""
    b = r["payload"][NONCE_LEN:] if r["flags"] & FLAG_SHA256 else r["payload"]
    return bool(b) and b[0] & REDACTED_BIT != 0


def body_of(r):
    """FORMAT §9.4, the owner's framing: M0's nonce, then the hub's kind byte on a declaration (`D`) or a
    key op (`K`). A body with no kind byte (bebop-log's own fixtures) starts at its mark."""
    b = r["payload"][NONCE_LEN:] if r["flags"] & FLAG_SHA256 else r["payload"]
    return b[1:] if b[:1] in (b"D", b"K") else b


def doc_open(im, root):
    """bebop-wasm-reader `open_doc`: the root's crc, the log as a log is read, HEADS / KEYS / SPEC
    crc (10), the SEG chain and every anchor crc-checked, a v4 TREE's entries, another TREE version -> 2.
    Returns (records, heads, keys, spec, tree entries, log tip, segments, anchors, (HEADS, KEYS))."""
    im.need_crc(root)
    log = im.follow(root, 1)
    recs = log_detail(im, log) if log is not None else []
    heads_at, keys_at, spec_at, tree = im.follow(root, 3), im.follow(root, 4), im.follow(root, 6), im.follow(root, 2)
    for t in (heads_at, keys_at):
        if t is not None:
            im.need_crc(t)
    spec = None
    if spec_at is not None:
        im.need_crc(spec_at)
        spec = read_spec(im, spec_at)
        if spec is None:
            raise Refuse(2)
    entries = []
    if tree is not None:
        im.need_crc(tree)
        if im.get(tree, 5) != 4:
            raise Refuse(2)
        entries = kv4_entries(im, tree)
    segs = read_segs(im, im.follow(root, 9))
    anchors = read_anchors(im, im.follow(root, 8))
    return recs, read_heads(im, heads_at), read_keys(im, keys_at), spec, entries, log_tip(im, log), segs, anchors, (heads_at, keys_at)


def frontier_of(segs, actor):
    """`seg::frontier_of`: the row of the NEWEST segment that names the device, or None."""
    return next((f for s in segs for f in s["frontier"] if f[0] == actor), None)


def first_ok(segs, actor, r):
    """`verify.rs` `first_ok`: a device's OLDEST kept record continues the frontier the newest segment
    naming it recorded (seq + 1, prev = that tip); a device no segment names starts at any seq."""
    row = frontier_of(segs, actor)
    if row is None:
        return True
    return row[1] == (r["seq"] - 1) & M64 and row[2] == r["prev"]


def forgotten_list(b):
    """`seg::parse_forgotten`: the (id, digest) rows of a FORGOTTEN body, or None when it is not one,
    or when `n` is not what the body holds."""
    if not b.startswith(FORGOTTEN_MARK) or len(b) < len(FORGOTTEN_MARK) + 40:
        return None
    n, rest = struct.unpack_from("<Q", b, len(FORGOTTEN_MARK))[0], b[len(FORGOTTEN_MARK) + 40:]
    if len(rest) % 64 or len(rest) // 64 != n:
        return None
    return [(rest[i:i + 32], rest[i + 32:i + 64]) for i in range(0, len(rest), 64)]


def effective(sound):
    """`declare.rs` `declarations` with `signatures` False (this file judges no seal): a declaration counts
    on its chain alone -- its record classed chained, so its `prev` test held, with no tombstone mark.
    `sound` = those records, oldest first. Returns (truncates, forgotten, revokes, erased): `erased` =
    (id, digest, signer) in file order, FORGOTTEN's list."""
    truncates, forgotten, revokes, erased = [], 0, [], []
    for r in sound:
        if is_redacted(r):
            continue
        b = body_of(r)
        if b.startswith(TRUNCATE_MARK) and len(b) >= len(TRUNCATE_MARK) + 128:
            x = b[len(TRUNCATE_MARK):]
            truncates.append((x[:32], x[32:64]) + struct.unpack_from("<QQQQ", x, 64) + (x[96:128],))
        elif (rows := forgotten_list(b)) is not None:
            forgotten += len(rows)
            erased += [(i, d, r["actor"]) for i, d in rows]
        elif b.startswith(REVOKE_MARK) and len(b) >= len(REVOKE_MARK) + 40:
            revokes.append((b[len(REVOKE_MARK):len(REVOKE_MARK) + 32], struct.unpack_from("<q", b, len(REVOKE_MARK) + 32)[0]))
    return truncates, forgotten, revokes, erased


def keys_disagree(keys, revokes):
    """`settle.rs` under a reader with no verifier: per key the earliest `after_seq` of the REVOKEs
    naming it, on their chain alone (`keys::narrower`: 0 = none, -1 = after 0), against the table's
    `revoked_at`. No seal is judged: this says the table and the records' CLAIM differ, nothing more."""
    def cell(actor):
        c = 0
        for who, after in revokes:
            if who == actor:
                this = after if after > 0 else -1
                c = this if c == 0 else min(c, this)
        return c
    return sum(1 for actor, _, table in keys if table != cell(actor))


def seg_message(s, below):
    """`seg::seg_message`: every field a reader acts on; `blob` as a bit, its offset is not a fact."""
    m = SEG_TAG + below + s["root"] + bytes([s["blob"]])
    m += struct.pack("<QQQQQ", s["first"], s["last"], s["redacted"], s["declared"], len(s["frontier"]))
    for actor, seq, tip, scheme in s["frontier"]:
        m += actor + struct.pack("<Q", seq) + tip + struct.pack("<q", scheme)
    return m


def chain_hash(segs):
    """`seg::chain_hash`: `segs` newest first, folded oldest to newest, each hash over the one below."""
    h = bytes(32)
    for s in reversed(segs):
        h = hashlib.sha256(seg_message(s, h)).digest()
    return h


def check_segments(segs, truncates):
    """`declare.rs` `check_segments`: one effective TRUNCATE names the NEWEST segment, its six fields
    and the hash of the whole chain, else 75. Older segments need no TRUNCATE of their own."""
    if not segs:
        return
    s = segs[0]
    if (s["root"], s["prev_root"], s["first"], s["last"], s["redacted"], s["declared"], chain_hash(segs)) not in truncates:
        raise Refuse(75)


def tombs_foreign(recs, tombs, erased, keys):
    """`declare.rs` `foreign_tombs` with the library's reading of `roots` under `Trust::Genesis`: the
    keys with no ADMIT record. WHAT THAT LETS THIS READER SAY: not 0 = the file's own FORGOTTENs do
    not account for a tombstone as it stands (its id is unlisted, the digest differs, the signer is
    neither its author nor a table root), or list a live record. WHAT IT CANNOT SAY: 0 proves no
    authority. No seal is judged, so a FORGOTTEN counts on its chain alone, and the roots are rows of
    a table nobody signs: a forger with no key can write both. Only a verifier with an algorithm and
    its own roots (`Trust::Only`) turns 0 into "nobody but those two erased a record"."""
    roots = {actor for actor, admit, _ in keys if admit is None}
    listed = {i: (d, by) for i, d, by in erased}
    stones, foreign = set(), 0
    for r in tombs:
        stones.add(r["id"])
        d, by = listed.get(r["id"], (None, None))
        foreign += not (d == chained_id(r) and (by == r["actor"] or by in roots))
    return foreign + sum(1 for r in recs if r["id"] in listed and r["id"] not in stones)


def read_seals(im, heads):
    """`doc/seals.rs` `load`: every seal read once, whichever row reached it, paid for first. A walk that
    meets a seal already read stops there, and one that meets a seal it read itself (a ring) counts one
    bad; an object that fails its crc or is no SEAL ends the walk, one bad, no seal. A null `ref SEAL`
    starts nothing. Returns (tips of the seals read, bad)."""
    last_row, tips, bad = {}, [], 0
    for row, (_, _, _, cur) in enumerate(heads):
        while cur is not None:
            if cur in last_row:
                bad += last_row[cur] == row
                last_row[cur] = row
                break
            im.spend(cur)
            seal = read_seal(im, cur) if im.crc_ok(cur) else None
            if seal is None:
                bad += 1
                break
            last_row[cur] = row
            tips.append(seal[2])
            cur = seal[3]
    return tips, bad


def unsealed(recs, tips, segs):
    """`verify.rs` `unsealed` with no seal judged: from the tip cell of EVERY seal reached from HEADS,
    follow `prev` by id. Each record never visited counts 1; each walk that stops on an id absent
    from the file and neither 0^32 nor the tip of any frontier row of any SEG counts 1. A HEADS
    row's own tip cell is never a start: HEADS is signed by nobody."""
    at = {r["id"]: r for r in recs}
    ends = {f[2] for s in segs for f in s["frontier"]} | {bytes(32)}
    seen, dangling = set(), 0
    for i in tips:
        while i in at and i not in seen:
            seen.add(i)
            i = at[i]["prev"]
        if i not in at and i not in ends:
            dangling += 1
    return max(len(recs) - len(seen), 0) + dangling


def spend_walks(im, recs, tables, segs):
    """§9.4: the walks before the seals, from one budget (`verify.rs`); a file that runs it out is 2.
    `tables` = (HEADS, KEYS); `read_seals` pays for the seals out of the same budget."""
    for o in [r["obj"] for r in recs] + [t for t in tables if t is not None] + [s["obj"] for s in segs] + key_rows(im, tables[1]):
        im.spend(o)


def doc_counts(im, recs, heads, keys, log_tip, segs):
    """`doc::verify`'s eighteen counters as the MIT reader computes them (bebop-wasm-reader `ids.rs`,
    `Verifier::checks_signatures` false): ids under dowiz-hub's sha256 scheme or the pre-M0 FNV lanes
    (a legacy id newer than a chained record of its device is broken: the scheme moves once), a
    redaction by the kind byte's high bit, forks by `(actor, seq)`, every seal COUNTED (`checked` 0,
    `genesis` = keys with no ADMIT record), segments declared by TRUNCATEs on their chain alone or 75,
    `keys_disagree` the table against the REVOKEs on their chain alone, `unsealed` from every seal's tip,
    `tombs_foreign` (printed apart: the ABI's eighteen cells do not hold it) against the table's roots."""
    c = dict(records=len(recs), chained=0, legacy=0, redacted=0, broken=0, forks=0, seals=0,
             checked=0, heads=len(heads), keys=len(keys), genesis=sum(1 for _, a, _ in keys if a is None),
             segments=len(segs), cut=sat(sum(s["last"] - s["first"] + 1 for s in segs)),
             cut_redacted=sat(sum(s["redacted"] for s in segs)), declared=0)
    tombs, sound, at = [], set(), {}
    for r in recs:
        at[(r["actor"], r["seq"])] = at.get((r["actor"], r["seq"]), 0) + 1
    c["forks"] = sum(1 for n in at.values() if n > 1)  # by records: one id held twice is a fork too
    by_actor = {}
    for r in recs:
        by_actor.setdefault(r["actor"], []).append(r)
    head_tip = {}
    for h in heads:
        head_tip.setdefault(h[0], h[2])  # the first row of an actor answers for it
    keyed, ids = {k[0] for k in keys}, {r["id"] for r in recs}
    for actor, rs in by_actor.items():
        # M0's forward-once rule continues from the frontier row's `scheme` (W-MG-M3b)
        row = frontier_of(segs, actor)
        tip, moved = head_tip.get(actor, log_tip), row is not None and row[3] == SCHEME_CHAINED
        last, soft = None, None  # the seq of the previous chained or tombstone record; of a legacy one
        for j, r in enumerate(rs):
            prev_ok = first_ok(segs, actor, r) if j == 0 else rs[j - 1]["id"] == r["prev"]
            next_names_me = rs[j + 1]["prev"] == r["id"] if j + 1 < len(rs) else tip == r["id"]
            dense = (actor not in keyed or at[(actor, r["seq"])] > 1
                     or (last + 1 == r["seq"] if last is not None else
                         row is not None or r["seq"] == 1 or (soft is not None and soft + 1 == r["seq"])
                         or (j == 0 and r["prev"] != bytes(32) and r["prev"] not in ids)))
            if not prev_ok:
                c["broken"] += 1
                last = r["seq"]
            elif chained_id(r) == r["id"]:
                moved, last = True, r["seq"]
                if dense:
                    c["chained"] += 1
                    sound.add(id(r))
                else:
                    c["broken"] += 1
            elif is_legacy(r):
                last, soft = None, r["seq"]
                c["broken" if moved else "legacy"] += 1
            elif is_redacted(r) and next_names_me and (dense or last is None):
                last = r["seq"]
                c["redacted"] += 1
                tombs.append(r)
            else:
                last = r["seq"]
                c["broken"] += 1
    tips, bad = read_seals(im, heads)
    c["seals"] = len(tips) + bad
    truncates, forgotten, revokes, erased = effective([r for r in recs if id(r) in sound])
    check_segments(segs, truncates)
    c["declared"] = sat(forgotten + sum(s["declared"] for s in segs))
    c.update(keys_disagree=keys_disagree(keys, revokes), unsealed=unsealed(recs, tips, segs), tombs_foreign=tombs_foreign(recs, tombs, erased, keys))
    return c


def read(raw):
    """(family, entries-or-records, doc-or-None); raises Refuse. A DOC is asked FIRST: to the KV
    path its version cell reads 'above 4'. A DOC's counters are the caller's second question
    (`doc_counts`), as `bw_verify` is the reader's: the status line comes first either way."""
    im = Image(raw)
    at, used, pt = pick(im)
    last_commit(im, at, used)
    root = im.cell(pt + 18) if pt else 0
    if root <= 0:
        raise Refuse(2)
    if is_doc(im, root):
        recs, heads, keys, spec, _entries, tip, segs, anchors, tables = doc_open(im, root)
        return "doc", [(r["seq"], r["payload"]) for r in recs], (im, recs, heads, keys, tip, segs, anchors, spec, tables)
    if im.digest(root) == DIGEST_EVLOG_ROOT:
        return "log", log_records(im, root), None
    im.need_crc(root)
    ver = im.get(root, 5) if im.obj_cells(root) >= 6 and im.get(root, 5) > 0 else 1
    return "kv", (kv4_entries(im, root) if ver == 4 else kv_entries(im, root)), None


COUNT_LINE = ("verify records=%(records)d chained=%(chained)d legacy=%(legacy)d redacted=%(redacted)d broken=%(broken)d "
              "forks=%(forks)d seals=%(seals)d checked=%(checked)d heads=%(heads)d keys=%(keys)d genesis=%(genesis)d "
              "segments=%(segments)d cut=%(cut)d cut_redacted=%(cut_redacted)d declared=%(declared)d keys_disagree=%(keys_disagree)d unsealed=%(unsealed)d")


def main(argv):
    flags = [a for a in argv[1:] if a.startswith("--")]
    paths = [a for a in argv[1:] if not a.startswith("--")]
    if not paths or "--help" in flags:
        print("usage: python3 bebop-verify.py <image> [--dump] [--verify]", file=sys.stderr)
        return 2
    try:
        family, items, doc = read(open(paths[0], "rb").read())
    except OSError as e:
        print("kv status=78 n=0 root=0")
        print("HOST_FAILED: cannot read the file: %s" % (type(e).__name__,), file=sys.stderr)
        return 78
    except Refuse as r:
        print("kv status=%d n=0 root=0" % r.args[0])
        print(WORDS.get(r.args[0], "status %d" % r.args[0]), file=sys.stderr)
        return r.args[0]
    # the two folds (FORMAT §9.1; lib.rs): LOG = seq8 || len8 || payload oldest first,
    # KV = len8 || key || len8 || value in key order; FNV-1a 64 over the bytes. A DOC's is its log's.
    h = FNV_OFFSET
    for a, b in items:
        head = struct.pack("<Q", a) if family in ("log", "doc") else struct.pack("<Q", len(a)) + a
        h = fnv(h, head + struct.pack("<Q", len(b)) + b)
    print("%s status=0 n=%d root=%d" % (family, len(items), to_i64(h)))
    what = {"log": "EvLog", "doc": "DOC (root-version 5)", "kv": "KV"}[family]
    print("OK; %s; every reached object whole, the last commit anchored" % what, file=sys.stderr)
    if doc is not None:
        im, recs, heads, keys, tip, segs, anchors, spec, tables = doc
        out = sys.stdout if "--verify" in flags else sys.stderr
        try:
            spend_walks(im, recs, tables, segs)
            c = doc_counts(im, recs, heads, keys, tip, segs)
        except Refuse as r:
            print("verify status=%d" % r.args[0], file=out)
            print(WORDS.get(r.args[0], "status %d" % r.args[0]), file=sys.stderr)
            return r.args[0]
        print(COUNT_LINE % c, file=out)
        print(UNCHECKED, file=sys.stderr)
        print("tombs_foreign=%d (blind: not 0 = the file's own FORGOTTENs do not account for a tombstone; 0 proves no authority)" % c["tombs_foreign"], file=sys.stderr)
        for kind, over, ms, plen in anchors:
            print("anchor kind=%d over=%s time_ms=%d proof_len=%d (present, NOT verified by this reader)" % (kind, over.hex(), ms, plen), file=out)
        if spec is None:
            print("SPEC: none (a working image; archives carry one)", file=sys.stderr)
        else:
            print("SPEC: embedded reader abi %d sha256 %s (the file's CLAIM; this file runs no wasm), format text ref %s"
                  % (spec[3], spec[2].hex(), spec[1]), file=sys.stderr)
        if "--dump" in flags:
            for actor, seq, tip_, _ in heads:
                print("head actor=%s seq=%d tip=%s" % (actor.hex(), seq, tip_.hex()))
            for i, r in enumerate(recs):
                print("#%d seq=%d id=%s payload=%r" % (i, r["seq"], r["id"].hex(), r["payload"]))
    elif "--dump" in flags:
        for i, (a, b) in enumerate(items):
            print("#%d seq=%d payload=%r" % (i, a, b) if family == "log" else "%r = %r" % (a, b))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
