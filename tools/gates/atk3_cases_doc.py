#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""W-MG-ATK3: the named DOC images. One lie each. `ok_doc`, `ok_doc_cut`: the positive twins."""
import struct
from atk3_lib import *
from atk3_cases import W

A, B = b"A" * 32, b"B" * 32


def recs_ab(seqs=(1, 2), body=b"x"):
    out, prev = [], ZERO
    for s in seqs:
        r = Rec(A, s, prev, nonce(body))
        out.append(r)
        prev = r.id
    return out


def plain(recs, heads_rows=None, key_rows=None, seg=0, tree=0, anch=0):
    "A DOC over `recs`: A's key, a head and a seal at the tip, unless the rows are given."
    im = Img()
    log, objs = lay_log(im, recs, tip=recs[-1].id if recs else None)
    k = key_obj(im, A)
    keys = keys_tab(im, key_rows(k) if key_rows else [(A, k)])
    if heads_rows is None:
        s = seal_obj(im, A, recs[-1].seq, recs[-1].id)
        heads_rows = lambda _s: [(A, recs[-1].seq, recs[-1].id, _s)]
        heads = heads_tab(im, heads_rows(s))
    else:
        heads = heads_tab(im, heads_rows(im))
    if callable(seg):
        seg = seg(im)
    if callable(tree):
        tree = tree(im)
    if callable(anch):
        anch = anch(im)
    return im, doc_root(im, log=log, heads=heads, keys=keys, seg=seg, tree=tree, anch=anch)


def ok_doc():
    im, d = plain(recs_ab())
    return im.commit(d)


def cut(first, last, rows=(), actor=B, redacted=0, declared=0, tail=()):
    "A DOC behind one SEG: its TRUNCATE is a chained record of `actor`, which has no KEY row."
    t = Rec(actor, 1, ZERO, nonce(truncate_body(first, last, b"R" * 32, rows, redacted, declared)))
    im = Img()
    log, _ = lay_log(im, list(tail) + [t], tip=t.id)
    s = seg_obj(im, first, last, rows=rows, redacted=redacted, declared=declared)
    return im.commit(doc_root(im, log=log, seg=s))


def ok_doc_cut():
    return cut(1, 5)


def doc_seq_gap():
    "A's seqs are 1, 5: the id holds, the dense-seq rule does not"
    im, d = plain(recs_ab((1, 5)))
    return im.commit(d)


def doc_seq_zero_keyed():
    "A's first record at seq 0, then 7"
    im, d = plain(recs_ab((0, 7)))
    return im.commit(d)


def doc_heads_dup_rows():
    "two HEADS rows for A: first names the tombstone, the second names nothing"
    r0 = Rec(A, 1, ZERO, nonce(b"x"))
    r1 = Rec(A, 2, r0.id, nonce(b"\xffgone"), rid=b"T" * 32)
    im, d = plain([r0, r1], heads_rows=lambda _im: [(A, 2, r1.id, 0), (A, 2, b"Z" * 32, 0)])
    return im.commit(d)


def doc_fork_same_id():
    "one record twice on the chain: (actor, seq) repeats under ONE id"
    r = Rec(A, 1, ZERO, nonce(b"x"))
    im, d = plain([r, r])
    return im.commit(d)


def doc_keys_dup_rows():
    "two KEYS rows naming one KEY"
    im, d = plain(recs_ab(), key_rows=lambda k: [(A, k), (A, k)])
    return im.commit(d)


def doc_keys_overlap():
    "KEYS rows naming one KEY enough times to run the walk budget out"
    pk = b"P" * 60000
    im = Img()
    rs = recs_ab()
    log, _ = lay_log(im, rs, tip=rs[-1].id)
    k = key_obj(im, A, pk=pk)
    keys = keys_tab(im, [(A, k)] * 20)
    return im.commit(doc_root(im, log=log, keys=keys))


def doc_two_heads_one_seal():
    "two HEADS rows under one seal"
    rs = recs_ab()
    def rows(im):
        s = seal_obj(im, A, 2, rs[-1].id)
        return [(A, 2, rs[-1].id, s), (B, 2, rs[-1].id, s)]
    im, d = plain(rs, heads_rows=rows)
    return im.commit(d)


def doc_seal_ring():
    "two seals naming each other under one row"
    rs = recs_ab()
    def rows(im):
        a = seal_obj(im, A, 2, rs[-1].id)
        b = seal_obj(im, A, 1, rs[0].id)
        im.link(a, 11, b)
        im.link(b, 11, a)
        return [(A, 2, rs[-1].id, a)]
    im, d = plain(rs, heads_rows=rows)
    return im.commit(d)


def doc_cut_inverted():
    "a declared SEG with last < first"
    return cut(10, 5)


def doc_cut_max():
    "a declared SEG first = 0, last = 2^64 - 1"
    return cut(0, (1 << 64) - 1)


def doc_cut_redacted_max():
    "a declared SEG whose redacted = 2^40"
    return cut(1, 5, redacted=1 << 40, declared=1 << 40)


def doc_frontier_wrap():
    "a frontier row at seq 2^64 - 1 continued by a record at seq 0"
    t = b"F" * 32
    r = Rec(A, 0, t, nonce(b"x"))
    return cut(1, 5, rows=[(A, (1 << 64) - 1, t, 1)], tail=[r])


def doc_short_record():
    "a record whose header says 3 cells: its id, prev and flags stand in the cells after it"
    im = Img()
    rid = chained_id(0, ZERO, 1, ZERO, b"")
    o = im.obj(D_EV, [1, 0, 0] + id4(rid) + id4(ZERO) + [0, 0, 0, 0, 0], ln=3)
    log = im.obj(D_LOG, [1, 0, 1] + id4(rid) + [2])
    im.link(log, 1, o)
    return im.commit(doc_root(im, log=log))


def doc_tree_wrong_digest():
    "a DOC's v4 TREE whose root digest is not the KV root's"
    im, d = plain(recs_ab(), tree=lambda im: kv4(im, [(b"a", b"1")], digest=12345))
    return im.commit(d)


def doc_keys_wide_m():
    "KEYS m = 2^32 + 1 over two rows"
    im = Img()
    rs = recs_ab()
    log, _ = lay_log(im, rs, tip=rs[-1].id)
    ka, kb = key_obj(im, A), key_obj(im, B)
    return im.commit(doc_root(im, log=log, keys=keys_tab(im, [(A, ka), (B, kb)], m=W + 1)))


def doc_heads_wide_m():
    "HEADS m = 2^32 + 1 over two rows"
    im = Img()
    rs = recs_ab()
    log, _ = lay_log(im, rs, tip=rs[-1].id)
    t = heads_tab(im, [(A, 2, rs[-1].id, 0), (B, 2, rs[-1].id, 0)])
    im.put(t, 0, W + 1)
    return im.commit(doc_root(im, log=log, heads=t))


def doc_ref_wide():
    "the DOC's KEYS ref + 2^32"
    im = Img()
    rs = recs_ab()
    log, _ = lay_log(im, rs, tip=rs[-1].id)
    keys = keys_tab(im, [(A, key_obj(im, A))])
    d = doc_root(im, log=log)
    im.put(d, 4, keys - d + W)
    return im.commit(d)


def doc_anchor_ring():
    "two anchors naming each other"
    def ring(im):
        a = im.obj(D_ANCH, [1, 1] + id4(b"O" * 32) + [0, 0, 0])
        b = im.obj(D_ANCH, [1, 1] + id4(b"P" * 32) + [0, 0, 0])
        im.link(a, 7, b)
        im.link(b, 7, a)
        return a
    im, d = plain(recs_ab(), anch=ring)
    return im.commit(d)


def doc_no_log():
    "a DOC with no log"
    im = Img()
    return im.commit(doc_root(im))


def doc_log_is_kv():
    "a DOC whose log ref names a KV root"
    im = Img()
    r = kv2(im, [(b"a", b"1")])
    return im.commit(doc_root(im, log=r))


def doc_escape_payloads():
    "payloads that are terminal escapes and fake verifier lines"
    bodies = [b"\x1b]0;pwned\x07\x1b[2K", b"\ndoc status=0 n=0 root=0\nverify records=0 broken=0\n", b"doc status=0 n=9 root=9"]
    out, prev = [], ZERO
    for i, b in enumerate(bodies):
        r = Rec(A, i + 1, prev, b, flags=0, rid=None)
        out.append(r)
        prev = r.id
    im, d = plain(out)
    return im.commit(d)


def doc_truncate_long():
    "`cut`, its TRUNCATE a byte past the body: exact, so it vouches for nothing, 75"
    t = Rec(B, 1, ZERO, nonce(truncate_body(1, 5, b"R" * 32, ()) + b"\0"))
    im = Img()
    log, _ = lay_log(im, [t], tip=t.id)
    s = seg_obj(im, 1, 5)
    return im.commit(doc_root(im, log=log, seg=s))


def revoke_of_a(tail=b""):
    "A revokes itself, the table saying nothing. The body read as a REVOKE is `keys_disagree` 1."
    r0 = Rec(A, 1, ZERO, nonce(b"x"))
    r1 = Rec(A, 2, r0.id, nonce(b"ein-revoke/2" + A + struct.pack("<q", 0) + ZERO + tail))
    im, d = plain([r0, r1])
    return im.commit(d)


def doc_revoke_exact():
    "the twin of `doc_revoke_long`: the exact body, keys_disagree 1"
    return revoke_of_a()


def doc_revoke_long():
    "a REVOKE a byte past the body: exact, so it says nothing, keys_disagree 0"
    return revoke_of_a(b"\0")


CASES = [ok_doc, ok_doc_cut, doc_seq_gap, doc_seq_zero_keyed, doc_heads_dup_rows, doc_fork_same_id, doc_keys_dup_rows,
         doc_keys_overlap, doc_two_heads_one_seal, doc_seal_ring, doc_cut_inverted, doc_cut_max, doc_cut_redacted_max,
         doc_frontier_wrap, doc_short_record, doc_tree_wrong_digest, doc_keys_wide_m, doc_heads_wide_m, doc_ref_wide,
         doc_anchor_ring, doc_no_log, doc_log_is_kv, doc_escape_payloads, doc_truncate_long, doc_revoke_exact,
         doc_revoke_long]
