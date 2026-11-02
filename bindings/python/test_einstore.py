# SPDX-License-Identifier: MIT OR Apache-2.0
"""Writes, reopens, reads, and checks Refusals by number. Run: python3 bindings/python/test_einstore.py"""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import einstore as bs  # noqa: E402

FIX = os.path.join(bs._REPO, "crates", "dowiz-hub-wasm", "fixtures")


class StoreTest(unittest.TestCase):
    def test_write_reopen_read(self):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "t.ein")
            with bs.Db.create(path) as db:
                with db.write() as w:
                    w.put(b"order/0001", b"new")
                    w.put(b"order/0002", b"paid")
                    w.put("zone/north", "")
                with db.write() as w:
                    self.assertTrue(w.remove(b"order/0001"))
                    self.assertFalse(w.remove(b"nope"))
                    self.assertEqual(w.get(b"order/0001"), None)
                gen = w.generation
            with bs.Db.open(path) as db, db.read() as r:
                self.assertEqual(r.generation, gen)
                self.assertEqual(len(r), 2)
                self.assertEqual(r.get(b"order/0002"), b"paid")
                self.assertEqual(r.get(b"zone/north"), b"")
                self.assertIsNone(r.get(b"order/0001"))
                self.assertEqual(list(r.prefix(b"order/")), [(b"order/0002", b"paid")])
                self.assertEqual([k for k, _ in r.scan()], [b"order/0002", b"zone/north"])
                self.assertEqual(db.verify(bs.WHOLE_ARENA), gen)

    def test_refusals_by_number(self):
        with bs.Db.memory() as db:
            w = db.write()
            with self.assertRaises(bs.Refusal) as e:
                w.put(b"k" * 65536, b"v")
            self.assertEqual((e.exception.code, e.exception.name), (44, "KEY_TOO_LONG"))
            self.assertIn("KeyTooLong", e.exception.message)
            db.write().abort()  # W-ES-P19b: a second writer opens; its commit validates
            r = db.read()
            with self.assertRaises(bs.Refusal) as e:
                db.close()
            self.assertEqual(e.exception.name, "BUSY")
            w.abort()
            r.close()
        with self.assertRaises(bs.Refusal) as e:
            bs.Db.create(os.path.join(tempfile.gettempdir(), "no-such-dir-ein", "x"))
        self.assertEqual(e.exception.name, "IO")

    def test_fixture_refusal(self):
        # kv4stub.expected is the authority: a v4 claim that does not hold is NOT_A_KV since KV v4
        # reads everywhere; a root version this reader does not know is the DOC fixture's 12.
        for name, want in (("kv4stub", (2, "NOT_A_KV")), ("doc", (12, "UNKNOWN_VERSION"))):
            with open(os.path.join(FIX, name + ".store"), "rb") as f:
                data = f.read()
            with self.assertRaises(bs.Refusal) as e:
                bs.Db.from_bytes(data)
            self.assertEqual((e.exception.code, e.exception.name), want)

    def test_memory_snapshot_roundtrip(self):
        with bs.Db.memory() as db:
            with db.write() as w:
                w.put(b"a", b"1")
            img = db.snapshot()
        with bs.Db.from_bytes(img) as db2, db2.read() as r:
            self.assertEqual(r.get(b"a"), b"1")


if __name__ == "__main__":
    unittest.main(verbosity=2)
