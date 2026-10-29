# SPDX-License-Identifier: MIT OR Apache-2.0
"""The bebop KV database from Python: create, commit, reopen, read, scan, and name a refusal.

Build the library once (cd crates/bebop-ffi && cargo build --release), then: python3 examples/kv.py
"""
import os
import sys
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "bindings", "python"))
import bebop_store as bs  # noqa: E402

d = tempfile.mkdtemp()
path = os.path.join(d, "shop.bebop")

with bs.Db.create(path) as db:
    with db.write() as w:  # commits when the block ends, aborts on an exception
        w.put(b"order/0001", b"new")
        w.put(b"order/0002", b"paid")
        w.put(b"zone/north", b"open")
    print("committed generation", w.generation)

with bs.Db.open(path, verify=bs.REACHABLE) as db, db.read() as r:
    print("reopened: %d entries, generation %d" % (len(r), r.generation))
    print("order/0002 =", r.get(b"order/0002").decode())
    for k, v in r.prefix(b"order/"):
        print("  %s = %s" % (k.decode(), v.decode()))
    print("fold root", r.fold_root())
    w = db.write()
    try:
        w.put(b"k" * 70000, b"v")
    except bs.Refusal as e:
        print("refused as expected:", e.name, e.code)
    finally:
        w.abort()
