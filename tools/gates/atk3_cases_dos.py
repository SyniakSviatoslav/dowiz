#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""W-MG-ATK3: the resource images, each at most 1 MiB, each one named structure. `ok_1mib_log` is
the positive twin: a whole megabyte of honest payload, which sets what linear costs here."""
import struct
from atk3_lib import *
from atk3_cases_doc import A, B

MIB = 1 << 20


def _log_records(im, n):
    "n one-cell records, no actor: cheap chain the walks may visit."
    return lay_log(im, [Rec(ZERO, i, ZERO, b"x", flags=0, rid=b"I" * 32) for i in range(n)])


def ok_1mib_log():
    "one honest record of 1 MiB"
    im = Img()
    root, _ = lay_log(im, [Rec(ZERO, 1, ZERO, b"p" * (MIB - 20000), flags=0, rid=b"I" * 32)])
    return im.commit(root)


def res_pad_512m():
    "a 296-byte file whose superblock claims the whole padding bound"
    im = Img()
    im.c = im.c[:37]
    b = im.commit(0, capacity=(1 << 26) - ARENA, used=0)
    return b


def res_kv2_overlap():
    "20 000 index rows naming one 300 KB blob (the KV budget's case; every Rust reader has it)"
    im = Img()
    kblob = im.obj(9, cells_of(b"k"))
    vblob = im.obj(9, cells_of(b"v" * 300000))
    n = 20000
    kidx = im.obj(9, [0, 1] * n)
    vidx = im.obj(9, [0, 300000] * n)
    root = im.obj(D_KV_ROOT, [n, 0, 0, 0, 0, 2])
    for i, a in enumerate((kidx, kblob, vidx, vblob)):
        im.link(root, 1 + i, a)
    return im.commit(root)


def res_kv4_shared_value():
    "2 000 keys of one leaf naming ONE 200 KB value object"
    im = Img()
    v = im.obj(D_VAL, [200000] + cells_of(b"v" * 200000))
    pairs = [(b"%06d" % i, b"") for i in range(2000)]
    return im.commit(kv4(im, pairs, val=lambda i: v))


def res_kv4_overlap_keys():
    "6 000 slots whose 64 KB keys overlap in one 70 KB key area, ascending"
    im = Img()
    v = im.obj(D_VAL, [0])
    m, klen = 6000, 65535
    leaf = len(im.c)
    slots = []
    for i in range(m):  # one zero fewer each slot: strictly ascending
        slots += [(i << 16) | klen, (v - leaf) & M64]
    im.obj(D_LEAF, [(1 << 56) | m] + slots + cells_of(b"\0" * m + b"\1" * klen))
    root = im.obj(D_KV_ROOT, [m, 0, 0, 0, 0, 4, 0, 1])
    im.link(root, 6, leaf)
    return im.commit(root)


def res_seal_ring():
    "5 000 HEADS rows, each on a ring of two 8 KB seals, 1 600 records setting the walk cap"
    im = Img()
    log, _ = _log_records(im, 1600)
    a = seal_obj(im, A, 2, b"T" * 32, sig=b"S" * 8000)
    b = seal_obj(im, A, 1, b"T" * 32, sig=b"S" * 8000)
    im.link(a, 11, b)
    im.link(b, 11, a)
    t = heads_tab(im, [(A, 2, b"T" * 32, a)] * 5000)
    return im.commit(doc_root(im, log=log, heads=t))


def res_seg_ring():
    "two 490 KB SEGs naming each other"
    im = Img()
    rows = [(A, 1, b"T" * 32, 1)] * 6000
    a = seg_obj(im, 1, 2, rows=rows)
    b = seg_obj(im, 3, 4, rows=rows)
    im.link(a, 7, b)
    im.link(b, 7, a)
    return im.commit(doc_root(im, seg=a))


def res_anchor_ring():
    "two 480 KB anchors naming each other"
    im = Img()
    p = b"p" * 480000
    a = im.obj(D_ANCH, [1, 1] + id4(b"O" * 32) + [0, 0, len(p)] + cells_of(p))
    b = im.obj(D_ANCH, [1, 1] + id4(b"P" * 32) + [0, 0, len(p)] + cells_of(p))
    im.link(a, 7, b)
    im.link(b, 7, a)
    return im.commit(doc_root(im, anch=a))


def res_legacy_fnv():
    "1 MiB of payload under no valid id: Python folds it, then tries both FNV lanes"
    im = Img()
    log, _ = lay_log(im, [Rec(A, 1, ZERO, b"p" * (MIB - 20000), flags=0, rid=b"I" * 32)])
    return im.commit(doc_root(im, log=log))


CASES = [ok_1mib_log, res_pad_512m, res_kv2_overlap, res_kv4_shared_value, res_kv4_overlap_keys, res_seal_ring,
         res_seg_ring, res_anchor_ring, res_legacy_fnv]
