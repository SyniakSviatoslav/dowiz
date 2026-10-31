

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


def read_keys(im, t):
    """`keys::read_keys`: (actor, admit ref) per row whose KEY reads; an unreadable KEY is skipped."""
    if t is None or im.digest(t) != DIGEST_KEYS or im.obj_cells(t) < 1:
        return []
    m, out = min(max(im.get(t, 0), 0), (im.obj_cells(t) - 1) // 5), []
    for i in range(m):
        k = im.follow(t, 1 + 5 * i + 4)
        if k is None or im.digest(k) != DIGEST_KEY or im.obj_cells(k) < 9:
            continue
        pk_len = im.get(k, 8)
        if 0 <= pk_len <= MAX_PK_BYTES and im.obj_cells(k) == 9 + (pk_len + 7) // 8:
            out.append((im.id_at(k, 0), im.follow(k, 7)))
    return out


def read_segs(im, newest):
    """`seg::read_segs`: every SEG from the DOC's ref SEG back, NEWEST FIRST, each crc checked (10); a
    chain object that is not a SEG, or one that never ends -> 2. A segment is a dict: first, last,
    root, prev_root, redacted, declared, frontier [(actor, seq, tip, scheme)]."""
    out, cur, cap = [], newest, im.n // 14 + 1
    while cur is not None:
        if len(out) >= cap:
            raise Refuse(2)
        im.need_crc(cur)
        if im.digest(cur) != DIGEST_SEG or im.obj_cells(cur) < 12 or im.get(cur, 0) != 1:
            raise Refuse(2)
        m = max(im.get(cur, 11), 0)
        if im.obj_cells(cur) != SEG_HEAD + SEG_ROW * m:
            raise Refuse(2)
        prev = im.follow(cur, 7)
        prev_root = im.id_at(prev, 3) if prev is not None and im.digest(prev) == DIGEST_SEG and im.obj_cells(prev) >= 12 and im.get(prev, 0) == 1 else bytes(32)
        out.append({"first": im.get(cur, 1) & M64, "last": im.get(cur, 2) & M64, "root": im.id_at(cur, 3), "prev_root": prev_root,
                    "redacted": im.get(cur, 9) & M64, "declared": im.get(cur, 10) & M64,
                    "frontier": [(im.id_at(cur, SEG_HEAD + SEG_ROW * i), im.get(cur, SEG_HEAD + SEG_ROW * i + 4) & M64,
                                  im.id_at(cur, SEG_HEAD + SEG_ROW * i + 5), im.get(cur, SEG_HEAD + SEG_ROW * i + 9)) for i in range(m)]})
        cur = prev
    return out


def read_anchors(im, newest):
    """`anch::read_anchors`: (kind, over, time_ms, proof_len) NEWEST FIRST; each crc checked (10) as the
    reader does at open. PRESENT, never verified: this file has no adapter for any kind."""
    out, cur, cap = [], newest, im.n // 11 + 1
    while cur is not None and len(out) < cap:
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
    return hashlib.sha256(r["prev"] + r["payload"]).digest()


def is_legacy(r):
    """`ReaderIds::legacy`: never for a FLAG_SHA256 record; else FNV x4 over prev||payload or over the payload."""
    return not r["flags"] & FLAG_SHA256 and r["id"] in (legacy_id(r["prev"] + r["payload"]), legacy_id(r["payload"]))


def is_redacted(r):
    return bool(r["flags"] & FLAG_SHA256) and len(r["payload"]) >= NONCE_LEN and r["payload"][:NONCE_LEN] == b"0" * NONCE_LEN


def body_of(r):
    return r["payload"][NONCE_LEN:] if r["flags"] & FLAG_SHA256 else r["payload"]


def doc_open(im, root):
    """bebop-wasm-reader `open_doc`: the root's crc, the log as a log is read, HEADS / KEYS / SPEC
    crc (10), the SEG chain and every anchor crc-checked, a v4 TREE's entries, another TREE version -> 2.
    Returns (records, heads, keys, spec, tree entries, log tip, segments, anchors)."""
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
    return recs, read_heads(im, heads_at), read_keys(im, keys_at), spec, entries, log_tip(im, log), segs, anchors
