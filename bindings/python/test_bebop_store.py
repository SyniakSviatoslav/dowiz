# SPDX-License-Identifier: MIT OR Apache-2.0
"""Writes, reopens, reads, and checks Refusals by number. Run: python3 bindings/python/test_bebop_store.py"""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bebop_store as bs  # noqa: E402

FIX = os.path.join(bs._REPO, "crates", "bebop-wasm", "fixtures")


class StoreTest(unittest.TestCase):
    def test_write_reopen_read(self):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "t.bebop")
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
            with self.assertRaises(bs.Refusal) as e:
                db.write()
            self.assertEqual(e.exception.name, "WRITER_BUSY")
            r = db.read()
            with self.assertRaises(bs.Refusal) as e:
                db.close()
            self.assertEqual(e.exception.name, "BUSY")
            w.abort()
            r.close()
        with self.assertRaises(bs.Refusal) as e:
            bs.Db.create(os.path.join(tempfile.gettempdir(), "no-such-dir-bebop", "x"))
        self.assertEqual(e.exception.name, "IO")

    def test_fixture_refusal(self):
        with open(os.path.join(FIX, "kv4stub.store"), "rb") as f:
            data = f.read()
        with self.assertRaises(bs.Refusal) as e:
            bs.Db.from_bytes(data)
        self.assertEqual((e.exception.code, e.exception.name), (12, "UNKNOWN_VERSION"))

    def test_memory_snapshot_roundtrip(self):
        with bs.Db.memory() as db:
            with db.write() as w:
                w.put(b"a", b"1")
            img = db.snapshot()
        with bs.Db.from_bytes(img) as db2, db2.read() as r:
            self.assertEqual(r.get(b"a"), b"1")


if __name__ == "__main__":
    unittest.main(verbosity=2)
