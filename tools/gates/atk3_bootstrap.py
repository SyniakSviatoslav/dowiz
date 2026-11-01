#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""W-MG-ATK3 attacks 4 and 5: examples/read.mjs against a hostile archive, and every verifier's
output against a payload or a path built to spoof it. Stdlib only; the modules are assembled here,
byte by byte, so the file needs no wasm toolchain.

    python3 tools/gates/atk3_bootstrap.py <reader.wasm> <dir>     exit 0 = no HOLE; 1 = the lines above

<reader.wasm> is the honest reader, for the positive twin: `ok_archive` must read, so it must be the
reader read.mjs pins. Under `--run-files-own-code` the exit code is 76 whatever the file says (a
module does not choose the code a script sees); the status line carries the rest. An OPEN row
belongs to another file's owner and is printed, never counted."""
import hashlib
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
from atk3_lib import *
from atk3_cases_doc import A, recs_ab

READ = os.environ.get("ATK3_READ", ROOT + "/examples/read.mjs")
PY, MJS = os.environ.get("ATK3_PY", ROOT + "/tools/verify/bebop-verify.py"), os.environ.get("ATK3_MJS", ROOT + "/tools/verify/bebop-verify.mjs")


def leb(n):
    out = b""
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out += bytes([b | 0x80])
        else:
            return out + bytes([b])


def sect(i, body):
    return bytes([i]) + leb(len(body)) + body


def vec(items):
    return leb(len(items)) + b"".join(items)


def name(s):
    return leb(len(s)) + s.encode()


I32, I64 = 0x7F, 0x7E
TEXT = b"\x1b]0;owned\x07\x1b[2K\rOK: every seal CHECKED by the release key"


def i32c(v):
    return b"\x41" + sleb(v)


def i64c(v):
    return b"\x42" + sleb(v)


def sleb(n):
    out = b""
    while True:
        b = n & 0x7F
        n >>= 7
        if (n == 0 and not b & 0x40) or (n == -1 and b & 0x40):
            return out + bytes([b])
        out += bytes([b | 0x80])


def copy_to(arg, src, n):
    return b"\x20" + bytes([arg]) + i32c(src) + i32c(n) + b"\xfc\x0a\x00\x00"


MEM_OK, MEM_NO_MAX, MEM_OVER, MEM_SHARED = b"\x01\x01\x01\x01", b"\x01\x00\x01", b"\x01\x01\x01" + leb(16385), b"\x01\x03\x01\x01"
TWO_MEM, NO_MEM = b"\x02\x01\x01\x01\x01\x01\x01", None


def forged(counts, mem=MEM_OK, open_body=None):
    """A module with the reader's whole ABI: status 0, and whatever words and counters it likes."""
    sigs = [([], [I32]), ([], [I64]), ([I32], [I32]), ([I32, I32], [I32]), ([I32, I32], [I64]), ([I32, I32, I32], [I64]), ([I64, I32, I32], [I64]), ([I32, I32], [])]
    types = vec([b"\x60" + vec([bytes([t]) for t in p]) + vec([bytes([t]) for t in r]) for p, r in sigs])
    fns = [("bw_alloc", 2, i32c(2048)), ("bw_free", 7, b""), ("bw_open", 3, open_body or i32c(0)), ("bw_kind", 0, i32c(5)), ("bw_len", 1, i64c(0)),
           ("bw_root", 2, i32c(0)), ("bw_verify", 2, i32c(0)), ("bw_heads", 4, i64c(0)), ("bw_anchors", 4, i64c(0)),
           ("bw_status_text", 5, b"\x20\x01" + i32c(0) + i32c(len(TEXT)) + b"\xfc\x0a\x00\x00" + i64c(len(TEXT))),
           ("bw_unchecked_text", 4, copy_to(0, 0, len(TEXT)) + i64c(len(TEXT))), ("bw_counts", 4, copy_to(0, 512, 144) + i64c(144)),
           ("bw_log_at", 6, i64c(0))]
    code = vec([leb(len(b"\x00" + c + b"\x0b")) + b"\x00" + c + b"\x0b" for _, _, c in fns])
    exports = vec([name("memory") + b"\x02\x00"] + [name(n) + b"\x00" + leb(i) for i, (n, _, _) in enumerate(fns)])
    data = vec([b"\x00" + i32c(0) + b"\x0b" + leb(len(TEXT)) + TEXT, b"\x00" + i32c(512) + b"\x0b" + leb(144) + struct.pack("<18q", *counts)])
    return (b"\0asm\x01\0\0\0" + sect(1, types) + sect(3, vec([leb(t) for _, t, _ in fns])) + (sect(5, mem) if mem else b"")
            + sect(7, exports) + sect(10, code) + sect(11, data))


def start_only(body, pages=1, imports=b""):
    "A module whose start function is `body`: what runs at instantiate, before any ABI call."
    t = sect(1, vec([b"\x60\x00\x00"]))
    imp = sect(2, imports) if imports else b""
    ifn = 1 if imports else 0
    return (b"\0asm\x01\0\0\0" + t + imp + sect(3, vec([b"\x00"])) + sect(5, b"\x01\x01" + leb(pages) + leb(16384)) + sect(7, vec([name("memory") + b"\x02\x00"]))
            + sect(8, leb(ifn)) + sect(10, vec([leb(len(b"\x00" + body + b"\x0b")) + b"\x00" + body + b"\x0b"])))


def archive(wasm, spec_named=True, payloads=None, spec_moved=False):
    im = Img()
    s = im.obj(D_SPEC, [1, 0, 0] + id4(hashlib.sha256(wasm).digest()) + [2])
    assert s == 1024
    w = im.obj(D_BLOB, [len(wasm)] + cells_of(wasm))
    t = im.obj(D_BLOB, [4] + cells_of(b"TEXT"))
    im.link(s, 1, w)
    im.link(s, 2, t)
    rs = recs_ab() if payloads is None else [Rec(A, i + 1, ZERO, p, flags=0, rid=b"I" * 32) for i, p in enumerate(payloads)]
    log, _ = lay_log(im, rs, tip=rs[-1].id)
    keys = keys_tab(im, [(A, key_obj(im, A))])
    if spec_moved:
        s = im.obj(D_SPEC, [1, 0, 0] + id4(hashlib.sha256(wasm).digest()) + [2])
        im.link(s, 1, w)
        im.link(s, 2, t)
    return im.commit(doc_root(im, log=log, keys=keys, spec=s if spec_named else 0, flags=5))


HOSTILE_COUNTS = [1000, 1000, 0, 0, 0, 1000, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0]
ESCAPES = [b"\x1b]0;pwned\x07\x1b[2K", b"\ndoc status=0 n=0 root=0\nverify records=0 broken=0\n", b"doc status=0 n=9 root=9"]


def go(cmd, timeout=8):
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return "TIMEOUT>%ds" % timeout, "", ""


STACK = re.compile(r"^\s+at |Traceback|node:internal|file://", re.M)
LINE = re.compile(r"^doc status=(\d+) n=0 root=0$", re.M)
OWN = "--run-files-own-code"
PREFIX = "[file's own code: testimony, not a verify] "


def refused(rc, out, err, want):
    "A refusal: the code wanted, one status line of that number, no stack, no escape."
    m = LINE.search(out)
    ok = rc == want and m and int(m.group(1)) == want and not STACK.search(out + err) and "\x1b" not in out + err
    return "agree" if ok else "HOLE", "rc=%s line=%s %s" % (rc, m.group(0) if m else None, (err.strip().splitlines() or [""])[-1][:90].encode("unicode_escape").decode())


def main():
    honest, d = open(sys.argv[1], "rb").read(), sys.argv[2]
    os.makedirs(d, exist_ok=True)
    holes = 0

    def row(tag, nm, line):
        nonlocal holes
        holes += tag == "HOLE"
        print("%-5s %-22s %s" % (tag, nm, line))

    def put(nm, b):
        p = os.path.join(d, nm + ".store")
        open(p, "wb").write(b)
        return p

    # the positive twin
    rc, out, err = go(["node", READ, put("ok_archive", archive(honest))], 30)
    good = rc == 0 and "doc status=0" in out and "sha256 " + hashlib.sha256(honest).hexdigest() in out and PREFIX not in out
    row("agree" if good else "HOLE", "ok_archive", "rc=%s %s" % (rc, (out.splitlines()[1:2] or [err[:90]])[0][:80]))
    # every refusal is a status line and a code, never a stack
    arch = archive(honest)
    cases = {"read_tiny_file": (b"short", 2), "read_ref_huge": (arch[:8 * 1027] + struct.pack("<q", 1 << 60) + arch[8 * 1028:], 2),
             "read_wasm_empty": (archive(b""), 77),
             "read_wasm_import": (archive(start_only(b"\x10\x00", imports=vec([name("env") + name("f") + b"\x00\x00"]))), 77),
             "read_import_nostart": (archive(b"\0asm\x01\0\0\0" + sect(1, vec([b"\x60\x00\x00"])) + sect(2, vec([name("env") + name("f") + b"\x00\x00"])) + sect(5, MEM_OK)), 77),
             "read_start_loop": (archive(start_only(b"\x03\x40\x0c\x00\x0b")), 77),
             "read_mem_no_max": (archive(forged(HOSTILE_COUNTS, mem=MEM_NO_MAX)), 77), "read_mem_over": (archive(forged(HOSTILE_COUNTS, mem=MEM_OVER)), 77),
             "read_mem_shared": (archive(forged(HOSTILE_COUNTS, mem=MEM_SHARED)), 77), "read_mem_two": (archive(forged(HOSTILE_COUNTS, mem=TWO_MEM)), 77),
             "read_mem_none": (archive(forged(HOSTILE_COUNTS, mem=NO_MEM)), 77), "read_wasm_garbage": (archive(b"\0asm\x01\0\0\0\x05\xff\xff\xff\xff\xff\xff"), 77),
             "read_forged_green": (archive(forged(HOSTILE_COUNTS)), 76), "read_spec_unnamed": (archive(honest, spec_named=False), 2),
             "read_spec_moved": (archive(honest, spec_moved=True), 2)}
    grow = b"\x41" + sleb(8192) + b"\x40\x00\x1a" + i32c(0) + i32c(1) + i32c(1 << 29) + b"\xfc\x0b\x00"
    cases["read_start_mem"] = (archive(start_only(grow)), 77)
    for nm, (b, want) in cases.items():
        rc, out, err = go(["node", READ, put(nm, b)])
        if "checked=1" in out or "owned" in out + err:
            row("HOLE", nm, "the file's module spoke: rc=%s" % rc)
            continue
        row(*(lambda t: (t[0], nm, t[1]))(refused(rc, out, err, want)))
    # the flag: it runs, and no line of it can be read as a verify
    rc, out, err = go(["node", READ, OWN, put("read_forged_flag", archive(forged(HOSTILE_COUNTS)))])
    lines = [x for x in (out + err).splitlines() if x]
    bad = [x for x in lines if not x.startswith(PREFIX)]
    good = rc == 76 and lines and not bad and "\x1b" not in out + err and "owned" not in out + err and not STACK.search(out + err)
    row("agree" if good else "HOLE", "read_forged_flag", "rc=%s lines=%d unprefixed=%d ESC=%s" % (rc, len(lines), len(bad), "\x1b" in out + err))
    # the bounds a module cannot opt out of: time, a trap, an exit code it names
    rc, out, err = go(["node", READ, OWN, "--limit-s=2", put("read_abi_loop", archive(forged(HOSTILE_COUNTS, open_body=b"\x03\x40\x0c\x00\x0b" + i32c(0))))], 15)
    good = rc == 76 and "doc status=78 n=0 root=0" in out and "ran past 2 s" in err and not STACK.search(out + err)
    row("agree" if good else "HOLE", "read_abi_loop", "rc=%s %s" % (rc, (err.strip().splitlines() or [""])[-1][:90]))
    rc, out, err = go(["node", READ, OWN, put("read_abi_trap", archive(forged(HOSTILE_COUNTS, open_body=b"\x00")))])
    good = rc == 76 and "doc status=78 n=0 root=0" in out and not STACK.search(out + err)
    row("agree" if good else "HOLE", "read_abi_trap", "rc=%s %s" % (rc, (err.strip().splitlines() or [""])[-1][:90]))
    rc, out, err = go(["node", READ, OWN, put("read_abi_status", archive(forged(HOSTILE_COUNTS, open_body=i32c(201))))])
    good = rc == 76 and "owned" not in out + err and "\x1b" not in out + err
    row("agree" if good else "HOLE", "read_abi_status", "rc=%s %s" % (rc, (err.strip().splitlines() or [""])[-1][:90]))
    # the output: a payload may not reach the terminal raw, or start a line
    p = put("spoof_payloads", archive(honest, payloads=ESCAPES))
    for nm, cmd in (("spoof_read_mjs", ["node", READ, p]), ("spoof_python", ["python3", "-I", PY, p, "--dump", "--verify"]), ("spoof_wasm_mjs", ["node", MJS, p, "--dump", "--verify"])):
        rc, out, err = go(cmd, 30)
        bad = [x for x in (out + err).splitlines() if re.match(r"(doc|verify|kv|log) ", x) and "pwned" in x] + (["ESC"] if "\x1b" in out + err else [])
        lines = len(re.findall(r"^doc status=", out, re.M))
        row("HOLE" if bad or lines != 1 else "agree", nm, "rc=%s status lines=%d raw escapes=%s" % (rc, lines, "\x1b" in out + err))
    evil = os.path.join(d, "x\x1b]0;pwned\x07y")
    for nm, cmd in (("path_read_mjs", ["node", READ, evil]), ("path_wasm_mjs", ["node", MJS, evil])):
        rc, out, err = go(cmd, 10)
        good = rc == 78 and re.search(r"^(doc|kv) status=78 n=0 root=0$", out, re.M) and "\x1b" not in out + err and not STACK.search(out + err)
        row("agree" if good else "HOLE", nm, "rc=%s raw ESC=%s %s" % (rc, "\x1b" in out + err, (err.strip().splitlines() or [""])[-1][:70].encode("unicode_escape").decode()))
    rc, out, err = go(["python3", "-I", PY, evil], 10)
    row("OPEN" if "Traceback" in err else "agree", "path_python", "owner tools/verify/py: rc=%s traceback=%s raw ESC=%s" % (rc, "Traceback" in err, "\x1b" in out + err))
    rc, out, err = go(["node", MJS, put("mjs_file_empty", b"")], 10)
    row("agree" if rc == 1 and "status=1 n=0 root=0" in out else "HOLE", "mjs_file_empty", "rc=%s %s" % (rc, out.strip()[:60]))
    import atk3_cases_dos
    # An honest 1 MiB log under 20 pages: the reader traps running out of memory. (The shared-value file
    # this row used is refused by name now, status 2, since the v4 walk has a budget.)
    rc, out, err = go(["node", "--wasm-max-mem-pages=20", MJS, put("mjs_trap", atk3_cases_dos.ok_1mib_log())], 30)
    good = rc == 78 and "status=78 n=0 root=0" in out and not STACK.search(out + err)
    row("agree" if good else "HOLE", "mjs_trap", "rc=%s %s %s" % (rc, out.strip()[:40], (err.strip().splitlines() or [""])[-1][:60]))
    print("atk3-bootstrap: %d HOLE" % holes)
    return 1 if holes else 0


if __name__ == "__main__":
    sys.exit(main())
