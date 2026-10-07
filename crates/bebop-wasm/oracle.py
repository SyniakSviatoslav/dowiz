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

S-1 (SPEC-BEBOP-DAG-RUNTIME §6.1): every VALID superblock keeps cells 13-14 at
zero; an image that breaks it is refused (status 5) before anything is read.

`--proj <image>` (DG5): a log image that carries a projection memo (superblock
cell 5 -> PROJTAB -> PROJ -> OUT), read to `proj status=0 n=<records>
root=<log fold> memo=<memo value>`; the log fold is FNV-1a 64 over
seq||payload_len||payload oldest first (crates/bebop-wasm/src/lib.rs). The memo
must be current: its input generation = the root's h1 low 32 bits, its tip = the
newest record's id0/id1, its count = the root's cell 0 (status 7 otherwise; 6 =
no memo).

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


def image(path):
    """(cells, n, live superblock or None, data root or 0, S-1 holds)."""
    raw = open(path, "rb").read()
    n = len(raw) // 8
    cells = struct.unpack("<%dq" % n, raw[: n * 8])

    def sb_ok(at):
        if at + 16 > n or cells[at] != MAGIC:
            return False
        crc = zlib.crc32(struct.pack("<15q", *cells[at : at + 15]))
        return crc == cells[at + 15] & 0xFFFFFFFF

    live = None
    s1 = True
    for at in (0, 512):
        if sb_ok(at):
            s1 = s1 and cells[at + 13] == 0 and cells[at + 14] == 0
            if live is None or cells[at + 2] > cells[live + 2]:
                live = at
    if live is None or cells[live + 4] > n:
        return cells, n, None, 0, s1
    pt = cells[live + 3]
    root = cells[pt + 18] if pt > 0 and pt + 18 < n else 0
    return cells, n, live, (root if 0 < root and root + 2 <= n else 0), s1


def main(path):
    cells, n, live, root, s1 = image(path)
    if not s1:
        print("kv status=5 n=0 root=0")
        return 5
    if live is None:
        print("kv status=1 n=0 root=0")
        return 1
    if root <= 0:
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
    if ver > 3:
        print("kv status=3 n=0 root=0")
        return 3
    per_cell = 8 if ver >= 2 else 1

    count = cells[root + 2]
    arrays = [follow(root, i) for i in (1, 2, 3, 4)]
    if count < 0 or any(a is None for a in arrays):
        print("kv status=2 n=0 root=0")
        return 2
    kidx, kblob, vidx, vblob = arrays

    # W-CRC: the root and the four arrays must match their header crc (bebop_store
    # Kv::load_checked; abi BAD_CRC = 10).
    def crc_ok(o):
        ln = obj_len(o)
        if o + 2 + ln > n:
            return False
        return zlib.crc32(struct.pack("<%dq" % ln, *cells[o + 2 : o + 2 + ln])) == (cells[o + 1] >> 32) & 0xFFFFFFFF

    if not all(crc_ok(o) for o in [root] + arrays):
        print("kv status=10 n=0 root=0")
        return 10

    # W-DELTA (v3): the chain, newest first, every claim checked (crates/bebop-store/src/kv/delta.rs
    # chain_in), re-derived here from the format, not from the Rust.
    chain = []
    if ver == 3:
        if obj_len(root) < 8 or root + 2 + 8 > n:
            print("kv status=2 n=0 root=0")
            return 2
        d = cells[root + 2 + 7]
        budget = n
        cur = follow(root, 6)
        while cur is not None:
            ln = obj_len(cur)
            if len(chain) >= d or ln < 4 or cur + 2 + ln > n or budget < ln + 2:
                print("kv status=2 n=0 root=0")
                return 2
            budget -= ln + 2
            op, kl, vl = cells[cur + 3], cells[cur + 4], cells[cur + 5]
            if op not in (1, 2) or kl < 0 or vl < 0 or (op == 2 and vl != 0) or 4 + (kl + 7) // 8 + (vl + 7) // 8 != ln:
                print("kv status=2 n=0 root=0")
                return 2
            chain.append((cur, op, kl, vl))
            cur = follow(cur, 0)
        if len(chain) != d:
            print("kv status=2 n=0 root=0")
            return 2
        # the crcs AFTER the whole shape held, as Kv::load_checked orders it (chain_in, then check_chain)
        if not all(crc_ok(o) for o, _, _, _ in chain):
            print("kv status=10 n=0 root=0")
            return 10

    def packed(o, at, ln):
        return bytes((cells[o + 2 + at + (b >> 3)] >> (8 * (b & 7))) & 0xFF for b in range(ln))

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
    entries = []
    for i in range(count):
        k, v = slice_of(kidx, kblob, i), slice_of(vidx, vblob, i)
        if k is None or v is None:
            print("kv status=2 n=0 root=0")
            return 2
        entries.append((k, v))
    if chain:
        if any(entries[i][0] >= entries[i + 1][0] for i in range(len(entries) - 1)):
            print("kv status=2 n=0 root=0")  # a chain on an unsorted base is refused
            return 2
        m = dict(entries)
        for o, op, kl, vl in reversed(chain):  # oldest first
            k = packed(o, 4, kl)
            if op == 1:
                m[k] = packed(o, 4 + (kl + 7) // 8, vl)
            else:
                m.pop(k, None)
        entries = sorted(m.items())
    h = FNV_OFFSET
    for k, v in entries:
        h = fnv(h, struct.pack("<Q", len(k)))
        h = fnv(h, k)
        h = fnv(h, struct.pack("<Q", len(v)))
        h = fnv(h, v)
    print(f"kv status=0 n={len(entries)} root={to_i64(h)}")
    return 0


def proj_mode(path):
    cells, n, live, root, s1 = image(path)

    def say(status, *v):
        print("proj status=%d n=%d root=%d memo=%d" % ((status,) + (v if v else (0, 0, 0))))
        return status

    if not s1:
        return say(5)
    if live is None or root <= 0:
        return say(1)

    def olen(o):
        return cells[o] & 0xFFFFFFFF

    def get(o, i):
        return cells[o + 2 + i] if o + 2 + i < n else 0

    def follow(o, i):
        off = get(o, i)
        t = o + off
        return t if off != 0 and 0 <= t and t + 2 <= n else None

    def crc_ok(o):
        ln = olen(o)
        if o + 2 + ln > n:
            return False
        return zlib.crc32(struct.pack("<%dq" % ln, *cells[o + 2 : o + 2 + ln])) == (cells[o + 1] >> 32) & 0xFFFFFFFF

    # the log: root {n, ref LAST, has_tip, tip0..3, VERSION}; record v2 {seq, plen, ref PREV, id0..3,
    # prev0..3, flags, [actor0..3 if flags & 1], payload 8 bytes/cell LE} or v1 {.., actor0..3, 1 byte/cell}
    ver = get(root, 7) if olen(root) >= 8 and get(root, 7) > 0 else 1
    recs, cur = [], follow(root, 1)
    while cur is not None and len(recs) <= get(root, 0):
        recs.append(cur)
        cur = follow(cur, 2)
    if len(recs) != get(root, 0):
        return say(4)
    # W-CRC: the log root and every record match their header crc (EvLog::chain_crc).
    if not all(crc_ok(o) for o in [root] + recs):
        return say(10)
    h = FNV_OFFSET
    for r in reversed(recs):
        at = (16 if olen(r) >= 16 and get(r, 11) & 1 else 12) if ver >= 2 else 15
        have = olen(r) - at
        cap = have * 8 if ver >= 2 else have
        pl = max(0, min(get(r, 1), max(cap, 0)))
        if ver >= 2:
            pay = bytes((get(r, at + (b >> 3)) >> (8 * (b & 7))) & 0xFF for b in range(pl))
        else:
            pay = bytes(get(r, at + b) & 0xFF for b in range(pl))
        h = fnv(h, struct.pack("<q", get(r, 0)) + struct.pack("<Q", pl) + pay)
    # the memo: PROJTAB [m, (key, ref PROJ, kind) x m]; PROJ 9 cells; OUT {value, count}
    t = cells[live + 5]
    if t < 1024 or t + 2 > n or not crc_ok(t) or get(t, 0) < 1:
        return say(6)
    p = follow(t, 2)
    if p is None or olen(p) != 9 or not crc_ok(p) or get(p, 0) != get(t, 1):
        return say(6)
    out = follow(p, 7)
    if out is None or olen(out) != 2 or not crc_ok(out):
        return say(6)
    tip = (get(recs[0], 3), get(recs[0], 4)) if recs else (0, 0)
    if get(p, 4) != cells[root + 1] & 0xFFFFFFFF or (get(p, 5), get(p, 6)) != tip or get(out, 1) != get(root, 0):
        return say(7)
    return say(0, len(recs), to_i64(h), get(out, 0))


# --- the block codec (DG9, SPEC-DATALOG-AND-CODEC §B.2-B.6) ----------------------------------
# A reader of its own, re-derived from the spec text: header, descriptors, canonical layout, the
# offsets VALUES, UTF-8, and an encoder that writes the block again FROM THE DECODED COLUMNS.
# `--block <file>... | @<list>` prints, per block, the line gate.sh compares across four readers:
#   block <schema> n=<n> nnz=<nnz> vals=<fnv64 hex> rt=<ok|diff> k256=<sha256 of the re-encoding>
#   block refused=<code> col=<column|->        (the check that refused, in Rust check()'s order)
# `vals` = FNV-1a 64 over each column's element count, then its elements, each as 8 LE bytes.
BLOCK_SCHEMAS = [
    "menu_prices:v1(dish:i64:0,price:i64:1,tax_ppm:i64:5,mods_ptr:u32rp:0,mods_col:u32:0,mods_val:i64:1)",
    "bom:v1(dish_ptr:u32rp:0,supply:u32:0,qty:i64:0)",
    "stock_levels:v1(supply:i64:0,qty:i64:0,gen:i64:7)",
    "names:v1(id:i64:0,bytes:u8:0,off:u32off:0)",
]
B_I64, B_I32, B_BYTES, B_OFF, B_RP, B_COL, B_VAL = 1, 2, 3, 4, 5, 6, 7
B_TOK = {"i64": B_I64, "i32": B_I32, "u8": B_BYTES, "u32off": B_OFF, "u32rp": B_RP, "u32": B_COL}
B_FIXED, B_DESC, B_MAX = 24, 16, 96 * 1024


def k64_of(b):
    return ((zlib.crc32(b) & 0xFFFFFFFF) << 32) | (len(b) & 0xFFFFFFFF)


def block_parse(s):
    """name, [(col, type, unit)], csr -- `i64` right after a CSR `u32` is the CSR val (type 7)."""
    name, body = s.split(":v", 1)[0], s[s.index("(") + 1:-1]
    cols = []
    for part in body.split(","):
        c, tok, unit = part.split(":")
        t = B_VAL if tok == "i64" and cols and cols[-1][1] == B_COL else B_TOK[tok]
        cols.append((c, t, int(unit)))
    return name, cols, any(t == B_RP for _, t, _ in cols)


BLOCK_TABLE = {k64_of(s.encode()): block_parse(s) for s in BLOCK_SCHEMAS}


class Refused(Exception):
    def __init__(self, code, col="-"):
        super().__init__(code)
        self.code, self.col = code, col


def u16(b, a):
    return struct.unpack_from("<H", b, a)[0]


def u32(b, a):
    return struct.unpack_from("<I", b, a)[0]


def block_want(t, n, nnz):
    return {B_I64: n * 8, B_I32: n * 4, B_OFF: (n + 1) * 4, B_RP: (n + 1) * 4, B_COL: nnz * 4, B_VAL: nnz * 8}.get(t)


def block_check(b):
    """The layout of `b` -- (name, cols, n, nnz, [(off, len)]) -- or Refused, in Rust check()'s order."""
    ln = len(b)
    if ln < B_FIXED + 4:
        raise Refused("too_short")
    if ln > B_MAX:
        raise Refused("too_big")
    if b[:4] != b"DWB1":
        raise Refused("bad_magic")
    if u16(b, 4) != 1:
        raise Refused("bad_version")
    end = ln - 4
    if u32(b, end) != zlib.crc32(b[:end]) & 0xFFFFFFFF:
        raise Refused("bad_crc")
    known = BLOCK_TABLE.get(struct.unpack_from("<Q", b, 16)[0])
    if known is None:
        raise Refused("unknown_schema")
    name, cols, csr = known
    ncols = u16(b, 6)
    if ncols > 4096 or ncols != len(cols):
        raise Refused("bad_ncols")
    if B_FIXED + B_DESC * ncols > end:
        raise Refused("too_short")
    n, nnz = u32(b, 8), u32(b, 12)
    if nnz != 0 and not csr:
        raise Refused("bad_nnz")
    lay, prev = [], B_FIXED + B_DESC * ncols
    for i, (c, t, unit) in enumerate(cols):
        at = B_FIXED + B_DESC * i
        if b[at] != t or b[at + 1] != 0:
            raise Refused("bad_type", c)
        if b[at + 2] != unit:
            raise Refused("bad_unit", c)
        if b[at + 3] != 0 or u32(b, at + 12) != 0:
            raise Refused("bad_reserved", c)
        clen, off = u32(b, at + 4), u32(b, at + 8)
        if off % 8 != 0:
            raise Refused("bad_alignment", c)
        if off != (prev + 7) // 8 * 8 or off + clen > end:
            raise Refused("bad_offsets", c)
        w = block_want(t, n, nnz)
        if w is not None and w != clen:
            raise Refused("bad_length", c)
        if any(b[prev:off]):
            raise Refused("bad_padding", c)
        lay.append((off, clen))
        prev = off + clen
    if prev != end:
        raise Refused("bad_offsets", cols[-1][0] if cols else "")
    for i, (c, t, _) in enumerate(cols):
        if t == B_RP:
            last = nnz
        elif t == B_OFF:
            if i == 0 or cols[i - 1][1] != B_BYTES:
                raise Refused("bad_type", c)
            last = lay[i - 1][1]
        else:
            continue
        v = [u32(b, lay[i][0] + 4 * k) for k in range(n + 1)]
        if v[0] != 0 or any(v[k] < v[k - 1] for k in range(1, n + 1)) or any(x > last for x in v) or v[-1] != last:
            raise Refused("bad_offsets", c)
        if t == B_OFF:
            base = lay[i - 1][0]
            for k in range(n):
                s = b[base + v[k]:base + v[k + 1]]
                try:
                    s.decode("utf-8", "strict")
                except UnicodeDecodeError:
                    raise Refused("not_utf8", cols[i - 1][0])
                if 0 in s:
                    raise Refused("not_utf8", cols[i - 1][0])
    return name, cols, n, nnz, lay


def block_decode(b):
    """(name, n, nnz, [column values]) -- each value a Python int (i64 signed, u32/byte unsigned)."""
    name, cols, n, nnz, lay = block_check(b)
    vals = []
    for (c, t, _), (off, ln) in zip(cols, lay):
        raw = b[off:off + ln]
        if t in (B_I64, B_VAL):
            vals.append(list(struct.unpack("<%dq" % (ln // 8), raw)))
        elif t == B_I32:
            vals.append(list(struct.unpack("<%di" % (ln // 4), raw)))
        elif t == B_BYTES:
            vals.append(list(raw))
        else:
            vals.append(list(struct.unpack("<%dI" % (ln // 4), raw)))
    return name, n, nnz, vals


def block_encode(name, n, nnz, vals):
    """The canonical bytes of a decoded block (§B.2), written from the values alone."""
    key = next(k for k, v in BLOCK_TABLE.items() if v[0] == name)
    cols = BLOCK_TABLE[key][1]
    out = bytearray(b"DWB1" + struct.pack("<HHIIQ", 1, len(cols), n, nnz, key))
    out += bytes(B_DESC * len(cols))
    for i, ((c, t, unit), v) in enumerate(zip(cols, vals)):
        out += bytes(-len(out) % 8)
        off = len(out)
        fmt = {B_I64: "q", B_VAL: "q", B_I32: "i", B_BYTES: "B"}.get(t, "I")
        out += struct.pack("<%d%s" % (len(v), fmt), *v)
        struct.pack_into("<BBBBIII", out, B_FIXED + B_DESC * i, t, 0, unit, 0, len(out) - off, off, 0)
    return bytes(out + struct.pack("<I", zlib.crc32(bytes(out)) & 0xFFFFFFFF))


def block_line(b):
    try:
        name, n, nnz, vals = block_decode(b)
    except Refused as r:
        return "block refused=%s col=%s" % (r.code, r.col)
    h = FNV_OFFSET
    for v in vals:
        for x in [len(v)] + v:
            h = fnv(h, (x & M64).to_bytes(8, "little"))
    again = block_encode(name, n, nnz, vals)
    rt = "ok" if again == b else "diff"
    return "block %s n=%d nnz=%d vals=%016x rt=%s k256=%s" % (name, n, nnz, h, rt, hashlib.sha256(again).hexdigest())


def block_mode(args):
    paths = open(args[0][1:]).read().split() if len(args) == 1 and args[0].startswith("@") else args
    for p in paths:
        try:
            b = open(p, "rb").read()
        except OSError as e:
            print("block unreadable=%s" % e)
            continue
        print(block_line(b))
    return 0


def block_schemas_mode():
    for s in BLOCK_SCHEMAS:
        print("schema %s k64=%016x %s" % (block_parse(s)[0], k64_of(s.encode()), s))
    return 0


# --- the published menu (BN3, src/block_view.rs `menu_line`) -------------------------------
# `--menu <menu_prices.dwb> <names.dwb>`: the hub's `Catalogue::row_of` over the two PUBLISHED
# blocks -- a dish found by its K64 and CONFIRMED by its bytes in `names` -- folded as
#   menu rows=<n> resolved=<k> fold=<fnv64: per resolved row, len(id) 8 LE, id, price, rate>
#   menu refused=<code> col=<column|->
def menu_line(pb, nb):
    try:
        p = block_decode(pb)
        n = block_decode(nb)
    except Refused as r:
        return "menu refused=%s col=%s" % (r.code, r.col)
    if p[0] != "menu_prices" or n[0] != "names":
        return "menu refused=mismatch col=-"
    if any(r < 0 or r >= n[1] for r in p[3][4][:p[2]]):
        return "menu refused=bad_offsets col=mods_col"
    off, text = n[3][2], bytes(n[3][1])
    dish_rows = {k: r for r, k in enumerate(p[3][0])}
    name_rows = {k: r for r, k in enumerate(n[3][0])}
    def named(key):
        r = name_rows.get(key)
        if r is None:
            return None
        s = text[off[r]:off[r + 1]]
        return s if to_i64(k64_of(s)) == key else None
    resolved, h = 0, FNV_OFFSET
    for r, key in enumerate(p[3][0]):
        s = named(key)
        if s is None or dish_rows.get(to_i64(k64_of(s))) != r:
            continue
        resolved += 1
        for x in (len(s).to_bytes(8, "little"), s, (p[3][1][r] & M64).to_bytes(8, "little"), (p[3][2][r] & M64).to_bytes(8, "little")):
            h = fnv(h, x)
    return "menu rows=%d resolved=%d fold=%016x" % (p[1], resolved, h)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--proj":
        sys.exit(proj_mode(sys.argv[2]))
    if len(sys.argv) == 3 and sys.argv[1] == "--key":
        sys.exit(key_mode(sys.argv[2]))
    elif len(sys.argv) >= 3 and sys.argv[1] == "--block":
        sys.exit(block_mode(sys.argv[2:]))
    elif len(sys.argv) == 4 and sys.argv[1] == "--menu":
        print(menu_line(open(sys.argv[2], "rb").read(), open(sys.argv[3], "rb").read()))
        sys.exit(0)
    elif len(sys.argv) == 2 and sys.argv[1] == "--schemas":
        sys.exit(block_schemas_mode())
    elif len(sys.argv) == 2:
        sys.exit(main(sys.argv[1]))
    else:
        print("usage: oracle.py <image> | oracle.py --key compile|proj|empty | oracle.py --block <file>...|@<list> | oracle.py --menu <prices> <names> | oracle.py --schemas", file=sys.stderr)
        sys.exit(2)
