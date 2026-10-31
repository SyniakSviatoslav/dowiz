

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
