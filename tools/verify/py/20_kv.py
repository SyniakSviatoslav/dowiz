

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
