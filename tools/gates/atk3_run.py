#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""W-MG-ATK3: one file, every reader, one line each; any disagreement or crash is RED.

    python3 tools/gates/atk3_run.py write <dir>          the images (*.img)
    python3 tools/gates/atk3_run.py run <dir> [name...]  wasm32 / python / oracle, + $dir/native.lines

The wasm32 reader is $ATK3_MJS (default the committed tools/verify/bebop-verify.mjs). native.lines is
written by `ATK3_DIR=<dir> cargo test --test atk3_native` (crates/bebop-wasm-reader), through slot.sh.
Exit 0 = every case agrees; 1 = the HOLE lines above."""
import os
import re
import signal
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
TIMEOUT = int(os.environ.get("ATK3_TIMEOUT", "30"))


# Rows where one reader differs BY A RULE, never by a defect: (case, reader) -> why it is left out.
NAMED = {
    ("sb_used_negative", "zc"): "zc refuses 2 where the rest refuse 1: a refusal either way; KvIn is no reader of status 1 (main, 2026-10-31)",
    ("pt_root_negative", "zc"): "zc refuses 2 where the rest refuse 1: a refusal either way; KvIn is no reader of status 1 (main, 2026-10-31)",
    ("sb_s1_cells", "oracle"): "FORMAT §3.1: a reader MAY check cells 13-14, and the Python reference reader answers 5",
    ("sb_s1_cells", "zc"): "the same image, read past cells 13-14 by KvIn, is a log root and so no KV: 2",
}


def all_cases():
    import atk3_cases
    import atk3_cases_doc
    import atk3_cases_dos
    return atk3_cases.CASES + atk3_cases_doc.CASES + atk3_cases_dos.CASES


class Late(Exception):
    pass


def late(*_):
    raise Late()


def run(cmd, path):
    """(answer, seconds, peak RSS MB): the stdout lines that are the reader's answer, or how it died."""
    t = time.time()
    with tempfile.TemporaryFile() as fo, tempfile.TemporaryFile() as fe:
        p = subprocess.Popen(cmd + [path], stdout=fo, stderr=fe)
        signal.signal(signal.SIGALRM, late)
        signal.alarm(TIMEOUT)
        try:
            _, st, ru = os.wait4(p.pid, 0)
            signal.alarm(0)
        except Late:
            p.kill()
            _, st, ru = os.wait4(p.pid, 0)
            return "TIMEOUT>%ds" % TIMEOUT, time.time() - t, ru.ru_maxrss // 1024
        p.returncode = os.waitstatus_to_exitcode(st)
        fo.seek(0), fe.seek(0)
        out, err = fo.read(), fe.read()
    dt, rc, mb = time.time() - t, p.returncode, ru.ru_maxrss // 1024
    e = err.decode("utf-8", "replace")
    if re.search(r"Traceback|RuntimeError|RangeError|CompileError|LinkError|TypeError|panicked|MemoryError|IndexError", e):
        last = [x for x in e.strip().splitlines() if re.search(r"Error|panicked", x)]
        return "CRASH rc=%d %s" % (rc, (last or ["?"])[-1][:90]), dt, mb
    lines = [x for x in out.decode("utf-8", "replace").splitlines() if re.match(r"(kv|log|doc) status=|verify |anchor ", x)]
    if rc < 0:
        return "KILLED sig=%d" % -rc, dt, mb
    return " || ".join(lines), dt, mb


def canon(a):
    """kv and doc lines the verifier prints for a refusal say `kv status=`: the status is the answer."""
    return re.sub(r"^(kv|log|doc) status=([1-9]\d*) n=0 root=0", r"status=\2", a)


def main():
    mode, d = sys.argv[1], sys.argv[2]
    names = sys.argv[3:]
    if mode == "write":
        os.makedirs(d, exist_ok=True)
        for f in all_cases():
            open(os.path.join(d, f.__name__ + ".img"), "wb").write(f())
        print("atk3: %d images in %s" % (len(all_cases()), d))
        return 0
    mjs = os.environ.get("ATK3_MJS", ROOT + "/tools/verify/bebop-verify.mjs")
    readers = {"wasm32": ["node", mjs, "--verify"], "python": ["python3", "-I", ROOT + "/tools/verify/bebop-verify.py", "--verify"],
               "oracle": ["python3", "-I", ROOT + "/crates/bebop-wasm/oracle.py"]}
    native = {}
    if os.path.exists(d + "/native.lines"):
        for x in open(d + "/native.lines"):
            k, _, v = x.strip().partition(" ")
            native[k] = v
    zc = {}
    if os.path.exists(d + "/zc.lines"):
        for x in open(d + "/zc.lines"):
            k, _, v = x.strip().partition(" ")
            zc[k] = v
    bad = 0
    for f in all_cases():
        n = f.__name__
        if names and n not in names:
            continue
        p = os.path.join(d, n + ".img")
        ans = {}
        for r, cmd in readers.items():
            if r == "oracle" and not (n.startswith(("ok_kv", "wide_kv", "wide_tree", "pad_kv", "kv", "sb_", "pt_", "oracle_", "res_kv"))):
                continue
            a, dt, mb = run(cmd, p)
            ans[r] = canon(a) + ("  [%.1fs %dMB]" % (dt, mb) if dt > 2 or mb > 150 or n.startswith(("res_", "ok_1"))else "")
        if n in native:
            ans["native"] = canon(native[n])
        if n in zc and "oracle" in ans:
            ans["zc"] = canon(zc[n])
        named = {r: w for (c, r), w in NAMED.items() if c == n and r in ans}
        for r in named:
            del ans[r]
        strip = lambda s: re.sub(r"  \[.*MB\]$", "", s)
        vals = {k: strip(v) for k, v in ans.items()}
        # the oracle has no counters and no DOC; hold it to the first line only
        # the oracle and zc have no counters and no fold; each is held to what it prints
        full = {v for k, v in vals.items() if k not in ("oracle", "zc")}
        first = {v.split(" || ")[0] for k, v in vals.items() if k != "zc"}
        st = {re.sub(r" n=.*", "", v.split(" || ")[0]) for v in vals.values()}
        same = len(full) == 1 and len(first) == 1 and len(st) == 1
        crash = any(v.startswith(("CRASH", "TIMEOUT", "KILLED")) or "PANIC" in v for v in vals.values())
        crash = crash or any(int(x) > 256 for v in ans.values() for x in re.findall(r"(\d+)MB\]$", v))
        tag = "agree" if same and not crash else "HOLE"
        bad += tag == "HOLE"
        print("%-5s %s" % (tag, n))
        for r, w in named.items():
            print("        named   %s left out: %s" % (r, w))
        if tag == "HOLE" or names:
            for k, v in ans.items():
                print("        %-7s %s" % (k, v))
    print("atk3: %d HOLE" % bad)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
