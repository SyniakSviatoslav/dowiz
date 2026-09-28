"""wasm.py over a hand-built module: every import kind, a name section, and the crate rules.

python3 -m unittest tools/evals/collect/test_wasm.py   (from the repo root)
"""
import io
import json
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

sys.path.insert(0, os.path.dirname(__file__))
import wasm  # noqa: E402


def u(n):
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        out.append(b | (0x80 if n else 0))
        if not n:
            return bytes(out)


def name(s):
    b = s.encode()
    return u(len(b)) + b


def section(sid, body):
    return bytes([sid]) + u(len(body)) + body


def imports():
    entries = [
        name("m") + name("f") + b"\x00" + u(0),                   # function
        name("m") + name("t") + b"\x01" + b"\x70" + u(1) + u(1) + u(9),  # table with max
        name("m") + name("t2") + b"\x01" + b"\x70" + u(0) + u(1),  # table without max
        name("m") + name("mem") + b"\x02" + u(1) + u(1) + u(2),    # memory with max
        name("m") + name("mem2") + b"\x02" + u(0) + u(1),          # memory without max
        name("m") + name("g") + b"\x03" + b"\x7f\x00",             # global
        name("m") + name("e") + b"\x04" + b"\x00" + u(0),          # tag
    ]
    return section(2, u(len(entries)) + b"".join(entries))


def module(big=0, with_name=True):
    bodies = [b"\x00" * 3, b"\x00" * (200 + big), b"\x00" * 5]
    code = section(10, u(len(bodies)) + b"".join(u(len(b)) + b for b in bodies))
    fnames = [(1, "_ZN4core3fmt5write17h0E"), (2, "dowiz_api_worker::route::h1"), (3, "<alloc::vec::Vec<T> as core::ops::Drop>::drop")]
    sub1 = u(len(fnames)) + b"".join(u(i) + name(n) for i, n in fnames)
    body = name("name") + bytes([0]) + u(2) + name("m") + bytes([1]) + u(len(sub1)) + sub1
    custom = section(0, body) if with_name else b""
    data = section(11, u(0) + b"\x00" * 7)
    return b"\0asm\x01\0\0\0" + section(1, u(0)) + imports() + code + data + custom + section(99, b"")


def keep(d, stripped, unstripped):
    """What strip-wasm.mjs --keep writes: build/index_bg.wasm + build/unstripped/index_bg.<digest>.wasm."""
    import hashlib
    p = os.path.join(d, "index_bg.wasm")
    with open(p, "wb") as f:
        f.write(stripped)
    if unstripped is not None:
        os.makedirs(os.path.join(d, "unstripped"), exist_ok=True)
        k = os.path.join(d, "unstripped", "index_bg.%s.wasm" % hashlib.sha256(stripped).hexdigest()[:16])
        with open(k, "wb") as f:
            f.write(unstripped)
    return p


def run_main(p):
    s = io.StringIO()
    with redirect_stdout(s):
        rc = wasm.main(["wasm.py", p])
    return rc, {i["id"]: i for i in json.loads(s.getvalue())}


class Stripped(unittest.TestCase):
    """96c2b790: the shipped file has no name section; its kept twin names its functions."""

    def test_names_come_from_the_kept_copy(self):
        with tempfile.TemporaryDirectory() as d:
            stripped = module(with_name=False)
            p = keep(d, stripped, module())
            self.assertIsNotNone(wasm.kept_copy(p, stripped))
            rc, out = run_main(p)
            self.assertEqual(rc, 0)
            # Sizes are the shipped file's, names are the twin's.
            self.assertEqual(out["wasm.raw"]["value"], len(stripped))
            self.assertEqual(out["wasm.section.name"]["value"], 0)
            share = out["wasm.worker_share_permille"]
            self.assertEqual(share["value"], round(1000 * 200 / 208))
            self.assertIn("unstripped", share["note"])
            self.assertEqual(out["wasm.crate.dowiz_api_worker"]["value"], 200)
            self.assertNotIn("wasm.crate.unnamed", out)

    def test_no_kept_copy_is_unverified_never_zero(self):
        with tempfile.TemporaryDirectory() as d:
            stripped = module(with_name=False)
            p = keep(d, stripped, None)
            self.assertIsNone(wasm.kept_copy(p, stripped))
            rc, out = run_main(p)
            self.assertEqual(rc, 0)
            share = out["wasm.worker_share_permille"]
            self.assertIsNone(share["value"])
            self.assertIn("no kept copy", share["unverified"])
            self.assertFalse([k for k in out if k.startswith("wasm.crate.")])
            self.assertEqual(out["wasm.raw"]["value"], len(stripped))

    def test_a_kept_copy_of_another_build_is_refused(self):
        with tempfile.TemporaryDirectory() as d:
            p = keep(d, module(with_name=False), module(big=1))
            with self.assertRaises(ValueError):
                run_main(p)

    def test_a_kept_copy_without_names_falls_back_to_the_file(self):
        # An unstripped file at the path (an old build) names itself even if a
        # nameless twin sits beside it.
        with tempfile.TemporaryDirectory() as d:
            full = module()
            p = keep(d, full, module(with_name=False))
            _, out = run_main(p)
            self.assertIn("names from", out["wasm.worker_share_permille"]["note"])
            self.assertGreater(out["wasm.worker_share_permille"]["value"], 900)

    def test_helpers(self):
        self.assertIsNone(wasm.names_in(module(with_name=False)))
        self.assertEqual(wasm.names_in(module())[2], "dowiz_api_worker::route::h1")
        self.assertEqual(wasm.body_sizes_of(module()), [3, 200, 5])
        self.assertEqual(wasm.body_sizes_of(b"\0asm\x01\0\0\0"), [])


class Walker(unittest.TestCase):
    def test_leb_and_names(self):
        self.assertEqual(wasm.leb(u(624485), 0), (624485, 3))
        self.assertEqual(wasm.name_of(name("abc"), 0), ("abc", 4))

    def test_not_wasm(self):
        with self.assertRaises(ValueError):
            wasm.sections(b"nope\0\0\0\0")

    def test_sections_and_imports(self):
        buf = module()
        labels = [s[0] for s in wasm.sections(buf)]
        self.assertEqual(labels, ["type", "import", "code", "data", "custom:name", "id99"])
        start = [s for s in wasm.sections(buf) if s[0] == "import"][0][1]
        self.assertEqual(wasm.imported_functions(buf, start), 1)

    def test_crate_rules(self):
        self.assertEqual(wasm.crate_of("_ZN4core3fmt5write17h0E"), "core")
        self.assertEqual(wasm.crate_of("_ZNx"), "?")
        self.assertEqual(wasm.crate_of("dowiz_api_worker::route::h[0a1b]"), "dowiz_api_worker")
        self.assertEqual(wasm.crate_of("<alloc::vec::Vec<T> as core::ops::Drop>::drop"), "alloc")
        self.assertEqual(wasm.crate_of("<&mut T as core::fmt::Write>::write"), "core")
        self.assertEqual(wasm.crate_of("::x"), "?")
        self.assertEqual(wasm.crate_of("plain"), "?")

    def test_analyse_maps_bodies_to_names_after_imports(self):
        a = wasm.analyse(module())
        self.assertEqual(a["code"], 208)
        self.assertEqual(a["crates"], {"core": 3, "dowiz_api_worker": 200, "alloc": 5})
        self.assertEqual(a["fns"][0], (200, "dowiz_api_worker::route::h1"))
        bare = wasm.analyse(b"\0asm\x01\0\0\0")
        self.assertEqual((bare["code"], bare["crates"]), (0, {}))

    def test_indicators(self):
        buf = module(big=wasm.BIG_FN)
        out = {i["id"]: i for i in wasm.indicators("x.wasm", buf, 0)}
        self.assertEqual(out["wasm.raw"]["value"], len(buf))
        self.assertIn("artifact built", out["wasm.raw"]["note"])
        self.assertEqual(out["wasm.section.name"]["rule"], "ratchet")
        self.assertEqual(out["wasm.functions_over_40k"]["value"], 1)
        self.assertEqual(out["wasm.crate.dowiz_api_worker"]["rule"], "trend")
        self.assertGreater(out["wasm.worker_share_permille"]["value"], 990)
        self.assertEqual(out["wasm.functions"]["value"], 3)
        self.assertIn("names from x.wasm", out["wasm.worker_share_permille"]["note"])
        # A module with names but no code: the share is a real 0, not unverified.
        named_empty = b"\0asm\x01\0\0\0" + section(0, name("name"))
        empty = {i["id"]: i for i in wasm.indicators("x.wasm", named_empty, 0)}
        self.assertEqual(empty["wasm.worker_share_permille"]["value"], 0)

    def test_main_reads_a_file_or_says_unverified(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "m.wasm")
            with open(p, "wb") as f:
                f.write(module())
            s = io.StringIO()
            with redirect_stdout(s):
                self.assertEqual(wasm.main(["wasm.py", p]), 0)
            self.assertEqual(json.loads(s.getvalue())[0]["id"], "wasm.raw")
            s = io.StringIO()
            with redirect_stdout(s):
                self.assertEqual(wasm.main(["wasm.py", os.path.join(d, "none.wasm")]), 0)
            self.assertIn("no build artifact", json.loads(s.getvalue())[0]["unverified"])
            s = io.StringIO()
            with redirect_stdout(s):
                wasm.main(["wasm.py"])
            self.assertEqual(json.loads(s.getvalue())[0]["id"], "wasm.raw")


if __name__ == "__main__":  # pragma: no cover
    unittest.main()
