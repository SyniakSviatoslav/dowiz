#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""W-MG-ATK3: the named images, KV and log. One lie each; the name says which cell lies and at what.
`ok_*` are the positive twins: every reader must agree on them, or the probe itself is wrong."""
import struct
from atk3_lib import *

W = 1 << 32  # the cell value that is 0 in a wasm32 usize
CAP = 1 << 20


def log1(n=None, payload=b"ABCDEFGH"):
    im = Img()
    root, objs = lay_log(im, [Rec(ZERO, 1, ZERO, payload, flags=0, rid=b"I" * 32)], n=n)
    return im, root, objs[0]


def ok_log():
    im, root, _ = log1()
    return im.commit(root)


def ok_kv2():
    im = Img()
    return im.commit(kv2(im, [(b"a", b"1"), (b"b", b"22")]))


def ok_kv1():
    im = Img()
    return im.commit(kv2(im, [(b"a", b"1"), (b"b", b"22")], ver=1))


def ok_kv4():
    im = Img()
    return im.commit(kv4(im, [(b"a", b"1"), (b"b", b"22")]))


# ---- wasm32: a cell of 2^32 + x is x in `as usize` --------------------------------------------
def wide_log_ref():
    "the log root's newest-record ref + 2^32: outside the image, unless usize is 32 bits"
    im, root, rec = log1()
    im.put(root, 1, rec - root + W)
    return im.commit(root)


def wide_log_count():
    "the log root's count = 2^32 + 1 over a chain of 1"
    im, root, _ = log1()
    im.put(root, 0, W + 1)
    return im.commit(root)


def wide_log_plen():
    "a record's payload length = 2^32 + 3 over 8 bytes: 3 bytes or 8"
    im, root, rec = log1()
    im.put(rec, 1, W + 3)
    return im.commit(root)


def wide_kv_n():
    "a KV's entry count = 2^32 + 1 over one entry"
    im = Img()
    root = kv2(im, [(b"a", b"1")])
    im.put(root, 0, W + 1)
    return im.commit(root)


def wide_kv_key_off():
    "a key's offset = 2^32 inside a 1-byte blob"
    im = Img()
    root = kv2(im, [(b"a", b"1")])
    im.put(root + struct.unpack("<q", struct.pack("<Q", im.c[root + 3]))[0], 0, W)
    return im.commit(root)


def wide_kv_val_len():
    "a value's length = 2^32 + 1 inside an 8-byte blob"
    im = Img()
    root = kv2(im, [(b"a", b"1")])
    vidx = root + struct.unpack("<q", struct.pack("<Q", im.c[root + 5]))[0]
    im.put(vidx, 1, W + 1)
    return im.commit(root)


def wide_kv3_d():
    "a v3 root's chain length D = 2^32 + 1 over one delta record"
    im = Img()
    base = kv2(im, [(b"a", b"1")])
    arrays = [im.c[base + 3 + i] for i in range(4)]
    tgt = [base + struct.unpack("<q", struct.pack("<Q", a))[0] for a in arrays]
    rec = im.obj(9, [0, 1, 1, 1] + cells_of(b"b") + cells_of(b"2"))
    root = im.obj(D_KV_ROOT, [1, 0, 0, 0, 0, 3, 0, W + 1])
    for i, t in enumerate(tgt):
        im.link(root, 1 + i, t)
    im.link(root, 6, rec)
    return im.commit(root)


def wide_kv3_klen():
    "a delta record's KLEN = 2^32 + 1 with one key cell"
    im = Img()
    base = kv2(im, [(b"a", b"1")])
    tgt = [base + struct.unpack("<q", struct.pack("<Q", im.c[base + 3 + i]))[0] for i in range(4)]
    rec = im.obj(9, [0, 1, W + 1, 1] + cells_of(b"b") + cells_of(b"2"))
    root = im.obj(D_KV_ROOT, [1, 0, 0, 0, 0, 3, 0, 1])
    for i, t in enumerate(tgt):
        im.link(root, 1 + i, t)
    im.link(root, 6, rec)
    return im.commit(root)


def wide_tree_m():
    "a leaf's META m = 2^32 + 2 (the bound 65535 is asked of the truncated usize)"
    im = Img()
    root = kv4(im, [(b"a", b"1"), (b"b", b"22")])
    leaf = root + struct.unpack("<q", struct.pack("<Q", im.c[root + 2 + 6]))[0]
    im.put(leaf, 0, (1 << 56) | (W + 2))
    return im.commit(root)


def wide_tree_koff():
    "a slot's key offset = 2^32 inside a 2-byte key area"
    im = Img()
    root = kv4(im, [(b"a", b"1")])
    leaf = root + struct.unpack("<q", struct.pack("<Q", im.c[root + 2 + 6]))[0]
    im.put(leaf, 1, (W << 16) | 1)
    return im.commit(root)


# ---- padding: Rust's n is `capacity`, Python's is the file --------------------------------------
def pad_log_phantom():
    "a record's next ref past the file's end, inside `capacity`: a zero object with a true crc"
    im, root, rec = log1(n=2)
    im.put(rec, 2, 100000 - rec)
    return im.commit(root, capacity=CAP)


def pad_kv_array():
    "a KV's value blob ref past the file's end, inside `capacity`"
    im = Img()
    root = kv2(im, [(b"a", b"")])
    im.put(root, 4, 100000 - root)
    return im.commit(root, capacity=CAP)


def pad_wide_capacity():
    "capacity = 2^32 + 2^20: wasm32 pads to 2^20, 64 bits refuses to, Python never pads"
    im, root, rec = log1(n=2)
    im.put(rec, 2, 100000 - rec)
    return im.commit(root, capacity=W + CAP)


def pad_obj_tail():
    "the last object's length runs past the file by four cells of zeros, its crc over those zeros"
    im, root, _ = log1(n=1)
    rec = len(im.c)
    im.c += [D_EV << 32 | 20, 0, 1, 0, 0] + [0] * 17
    im.c[rec + 1] = im.crc(rec + 2, 20) << 32
    im.link(root, 1, rec)
    return im.commit(root, capacity=CAP, used=len(im.c) - 4, cut=len(im.c) - 4)


# ---- KV semantics ------------------------------------------------------------------------------
def kv2_dup_keys():
    "two entries under one key, v2, no chain"
    im = Img()
    return im.commit(kv2(im, [(b"a", b"1"), (b"a", b"2")]))


def kv2_unsorted():
    "entries out of key order, v2, no chain"
    im = Img()
    return im.commit(kv2(im, [(b"b", b"1"), (b"a", b"2")]))


def kv2_nonutf8_key():
    "a key that is not UTF-8"
    im = Img()
    return im.commit(kv2(im, [(b"\xff", b"1")]))


def kv2_lossy_collision():
    "two keys, different bytes, one lossy string"
    im = Img()
    return im.commit(kv2(im, [(b"\xfe", b"1"), (b"\xff", b"2")]))


def kv2_lossy_shapes():
    "the five shapes UTF-8 decoders split differently"
    im = Img()
    keys = [b"\xc0\x80", b"\xe2\x82", b"\xed\xa0\x80", b"\xf0\x90\x80", b"\xf4\x90\x80\x80"]
    return im.commit(kv2(im, [(k, b"v") for k in keys]))


def kv4_wrong_digest():
    "a v4 root whose digest is not the KV root's"
    im = Img()
    return im.commit(kv4(im, [(b"a", b"1")], digest=12345))


def kv4_keys_overlap():
    "two slots whose keys overlap in the key area (rule 5: packed in slot order, no gap)"
    im = Img()
    v = im.obj(D_VAL, [0])
    leaf = len(im.c)
    im.obj(D_LEAF, [(1 << 56) | 2, (0 << 16) | 2, (v - leaf) & M64, (1 << 16) | 2, (v - leaf) & M64] + cells_of(b"abc"))
    root = im.obj(D_KV_ROOT, [2, 0, 0, 0, 0, 4, 0, 1])
    im.link(root, 6, leaf)
    return im.commit(root)


def kv4_leaf_trailing():
    "a leaf one cell longer than its keys need (rule 5: the object is exact)"
    im = Img()
    v = im.obj(D_VAL, [0])
    leaf = len(im.c)
    im.obj(D_LEAF, [(1 << 56) | 1, (0 << 16) | 1, (v - leaf) & M64] + cells_of(b"a") + [7])
    root = im.obj(D_KV_ROOT, [1, 0, 0, 0, 0, 4, 0, 1])
    im.link(root, 6, leaf)
    return im.commit(root)


def log_version_99():
    "a log root naming version 99"
    im, root, _ = log1()
    im.put(root, 7, 99)
    return im.commit(root)


# ---- the container -----------------------------------------------------------------------------
def file_empty():
    "zero bytes"
    return b""


def file_7_bytes():
    "less than one cell"
    return b"BEBOPST"


def file_zeros():
    "4 KiB of zeros"
    return bytes(4096)


def sb_used_negative():
    "arena_used = -1"
    im, root, _ = log1()
    b = bytearray(im.commit(root))
    struct.pack_into("<q", b, 8 * 4, -1)
    return _resb(b)


def _resb(b):
    import zlib
    struct.pack_into("<Q", b, 8 * 15, zlib.crc32(bytes(b[:120])))
    return bytes(b)


def sb_s1_cells():
    "superblock cell 13 not zero (FORMAT S-1)"
    im, root, _ = log1()
    b = bytearray(im.commit(root))
    struct.pack_into("<q", b, 8 * 13, 7)
    return _resb(b)


def pt_root_negative():
    "the PartTab's root cell = -5"
    im, root, _ = log1()
    return im.commit((-5) & M64)


def oracle_root_at_end():
    "the root is the last two cells of the file, one payload cell behind it"
    im, root, _ = log1()
    r = im.obj(D_KV_ROOT, [0])
    return im.commit(r)


CASES = [ok_log, ok_kv2, ok_kv1, ok_kv4, wide_log_ref, wide_log_count, wide_log_plen, wide_kv_n, wide_kv_key_off,
         wide_kv_val_len, wide_kv3_d, wide_kv3_klen, wide_tree_m, wide_tree_koff, pad_log_phantom, pad_kv_array,
         pad_wide_capacity, pad_obj_tail, kv2_dup_keys, kv2_unsorted, kv2_nonutf8_key, kv2_lossy_collision,
         kv2_lossy_shapes, kv4_wrong_digest, kv4_keys_overlap, kv4_leaf_trailing, log_version_99, file_empty, file_7_bytes, file_zeros, sb_used_negative,
         sb_s1_cells, pt_root_negative, oracle_root_at_end]
