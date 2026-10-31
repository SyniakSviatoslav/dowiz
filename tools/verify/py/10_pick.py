

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
