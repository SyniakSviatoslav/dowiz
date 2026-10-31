

def log_chain(im, root):
    """`EvLog::chain_crc`: the root and every record's crc (10), the count held to the root's
    claim and the walk bounded (4). The record objects, NEWEST FIRST."""
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
    return chain


def log_detail(im, root):
    """Every record OLDEST FIRST as a dict: obj, seq, flags, id, prev, actor, payload (bebop-log
    `walk`: v2 packs the payload eight bytes to a cell after the optional actor; v1 one per cell)."""
    chain = log_chain(im, root)
    ver = im.get(root, 7) if im.obj_len(root) >= 8 and im.get(root, 7) > 0 else 1
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
