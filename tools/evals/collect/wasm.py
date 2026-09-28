#!/usr/bin/env python3
"""SUITE ci . WASM: the Worker bundle's bytes, sections, code bytes per crate, largest functions.

Reads workers/api/build/index_bg.wasm (what wrangler uploads); builds nothing. That file is
STRIPPED after the glue (workers/api/scripts/strip-wasm.mjs, 96c2b790): sizes come from it, but
function NAMES -- and so the per-crate bytes and worker_share_permille -- come from the kept
unstripped twin build/unstripped/index_bg.<first 16 hex of sha256(stripped)>.wasm when it exists.
With neither a kept copy nor a name section of its own, the name-derived indicators are
UNVERIFIED (null + why), never 0: a 0 would ratchet the baseline down to nothing (it did, 09-27). The box's
wasm-objdump is too old for the reference types the build uses, so this is the 40-line LEB128
walker of BLUEPRINT-OPTIMIZATION-AND-EVALS-2026-09-24 section A.1, as a module with tests.

Prints a JSON list of indicators in the shape of tools/evals/rules.mjs `ind()`.
Usage: wasm.py [path/to/index_bg.wasm]
"""
import gzip
import hashlib
import json
import os
import re
import sys
import time

SECTION_NAMES = {0: "custom", 1: "type", 2: "import", 3: "function", 4: "table", 5: "memory",
                 6: "global", 7: "export", 8: "start", 9: "element", 10: "code", 11: "data",
                 12: "datacount", 13: "tag"}
BIG_FN = 40 * 1024


def leb(buf, i):
    """Unsigned LEB128 at buf[i]; returns (value, next index)."""
    n = shift = 0
    while True:
        b = buf[i]
        i += 1
        n |= (b & 0x7F) << shift
        if b < 0x80:
            return n, i
        shift += 7


def name_of(buf, i):
    ln, i = leb(buf, i)
    return buf[i:i + ln].decode("utf-8", "replace"), i + ln


def sections(buf):
    """[(label, start, size)] for every section; a custom section is labelled `custom:<name>`."""
    if buf[:4] != b"\0asm":
        raise ValueError("not a wasm module")
    out, i = [], 8
    while i < len(buf):
        sid = buf[i]
        size, j = leb(buf, i + 1)
        label = SECTION_NAMES.get(sid, "id%d" % sid)
        if sid == 0:
            label = "custom:" + name_of(buf, j)[0]
        out.append((label, j, size))
        i = j + size
    return out


def imported_functions(buf, start):
    count, i = leb(buf, start)
    fns = 0
    for _ in range(count):
        _, i = name_of(buf, i)
        _, i = name_of(buf, i)
        kind = buf[i]
        i += 1
        if kind == 0:
            _, i = leb(buf, i)
            fns += 1
        elif kind == 1:
            i += 1
            flags, i = leb(buf, i)
            _, i = leb(buf, i)
            if flags & 1:
                _, i = leb(buf, i)
        elif kind == 2:
            flags, i = leb(buf, i)
            _, i = leb(buf, i)
            if flags & 1:
                _, i = leb(buf, i)
        elif kind == 3:
            i += 2
        else:  # tag
            i += 1
            _, i = leb(buf, i)
    return fns


def body_sizes(buf, start):
    count, i = leb(buf, start)
    out = []
    for _ in range(count):
        size, j = leb(buf, i)
        out.append(size)
        i = j + size
    return out


def function_names(buf, start, size):
    """{function index: name} from the `name` section's function subsection (id 1)."""
    end, i, names = start + size, start, {}
    _, i = name_of(buf, i)
    while i < end:
        sub = buf[i]
        ln, i = leb(buf, i + 1)
        if sub == 1:
            count, k = leb(buf, i)
            for _ in range(count):
                idx, k = leb(buf, k)
                nm, k = name_of(buf, k)
                names[idx] = nm
        i += ln
    return names


def crate_of(name):
    """The crate a symbol belongs to: `_ZN<len><crate>` (legacy mangling) or `crate::…`."""
    if name.startswith("_ZN"):
        i = 3
        while i < len(name) and name[i].isdigit():
            i += 1
        if i > 3:
            n = int(name[3:i])
            return name[i:i + n]
        return "?"
    name = re.sub(r"\[[0-9a-f]+\]", "", name)
    if name.startswith("<"):
        inner = name[1:].lstrip("&*").replace("mut ", "", 1)
        head, _, trait = inner.partition(" as ")
        return crate_of(head if "::" in head.split(">")[0] else trait)
    if "::" in name:
        return name.split("::")[0] or "?"
    return "?"


def kept_copy(path, buf):
    """The unstripped twin strip-wasm.mjs --keep wrote beside `path`, or None when there is none."""
    digest = hashlib.sha256(buf).hexdigest()[:16]
    kept = os.path.join(os.path.dirname(path), "unstripped", "index_bg.%s.wasm" % digest)
    return kept if os.path.exists(kept) else None


def names_in(buf):
    """{function index: name} from `buf`'s name section, or None when it has none."""
    for label, start, size in sections(buf):
        if label == "custom:name":
            return function_names(buf, start, size)
    return None


def analyse(buf, names=None):
    """`names` overrides the module's own name section (the kept twin's, for a stripped file)."""
    secs = sections(buf)
    by = {}
    for label, start, size in secs:
        by.setdefault(label, (start, size))
        by[label + "#bytes"] = by.get(label + "#bytes", 0) + size
    imports = imported_functions(buf, by["import"][0]) if "import" in by else 0
    bodies = body_sizes(buf, by["code"][0]) if "code" in by else []
    if names is None:
        names = function_names(buf, *by["custom:name"]) if "custom:name" in by else {}
    crates, fns = {}, []
    for n, size in enumerate(bodies):
        nm = names.get(imports + n, "?")
        c = crate_of(nm)
        crates[c] = crates.get(c, 0) + size
        fns.append((size, nm))
    fns.sort(reverse=True)
    return {"secs": {k[:-6]: v for k, v in by.items() if k.endswith("#bytes")},
            "crates": crates, "fns": fns, "code": sum(bodies)}


def body_sizes_of(buf):
    """Every function body's size: equal across the strip, which touches custom sections only."""
    for label, start, _ in sections(buf):
        if label == "code":
            return body_sizes(buf, start)
    return []


def ind(id_, value, unit, rule, source, **extra):
    d = {"id": id_, "value": value, "unit": unit, "rule": rule, "source": source}
    d.update(extra)
    return d


def indicators(path, buf, mtime, kept=None):
    """`kept`: (path, bytes) of the unstripped twin, whose name section names `buf`'s functions."""
    names, named_by = None, None
    if kept is not None:
        names, named_by = names_in(kept[1]), kept[0]
        if names is not None and body_sizes_of(kept[1]) != body_sizes_of(buf):
            raise ValueError("kept copy %s has other code than %s" % (kept[0], path))
    if names is None and names_in(buf) is not None:
        names, named_by = names_in(buf), path
    a = analyse(buf, names or {})
    src = "wasm.py over " + os.path.relpath(path)
    age = "artifact built %s" % time.strftime("%Y-%m-%d %H:%M", time.localtime(mtime))
    out = [ind("wasm.raw", len(buf), "bytes", "ratchet", src, note=age),
           ind("wasm.gzip", len(gzip.compress(buf, 9)), "bytes", "ratchet", src)]
    for label in ("code", "data", "custom:name"):
        key = "wasm.section." + label.replace("custom:", "")
        out.append(ind(key, a["secs"].get(label, 0), "bytes", "ratchet", src))
    rest = len(buf) - sum(a["secs"].get(k, 0) for k in ("code", "data", "custom:name"))
    out.append(ind("wasm.section.other", rest, "bytes", "trend", src))
    if names is None:
        why = ("stripped, and no kept copy at %s: run the [build] command (strip-wasm.mjs --keep)"
               % os.path.relpath(os.path.join(os.path.dirname(path), "unstripped")))
        out.append(ind("wasm.worker_share_permille", None, "permille", "ratchet", src, unverified=why))
    else:
        named = "names from " + os.path.relpath(named_by)
        top = sorted(a["crates"].items(), key=lambda kv: -kv[1])[:12]
        for crate, n in top:
            out.append(ind("wasm.crate." + ("unnamed" if crate == "?" else crate), n, "bytes", "trend", src,
                           note=named))
        own = a["crates"].get("dowiz_api_worker", 0)
        share = round(1000 * own / a["code"]) if a["code"] else 0
        out.append(ind("wasm.worker_share_permille", share, "permille", "ratchet", src,
                       note="dowiz_api_worker code bytes / all code bytes; " + named))
    big = [(s, n) for s, n in a["fns"] if s > BIG_FN]
    out.append(ind("wasm.functions_over_40k", len(big), "functions", "ratchet", src,
                   note="; ".join("%s %d" % (n[:60], s) for s, n in big[:6])))
    out.append(ind("wasm.functions", len(a["fns"]), "functions", "trend", src))
    return out


def main(argv):
    root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
    path = argv[1] if len(argv) > 1 else os.path.join(root, "workers/api/build/index_bg.wasm")
    if not os.path.exists(path):
        print(json.dumps([ind("wasm.raw", None, "bytes", "ratchet", "wasm.py",
                              unverified="no build artifact at %s: run worker-build --release" % path)]))
        return 0
    with open(path, "rb") as f:
        buf = f.read()
    kept = kept_copy(path, buf)
    if kept is not None:
        with open(kept, "rb") as f:
            kept = (kept, f.read())
    print(json.dumps(indicators(path, buf, os.path.getmtime(path), kept)))
    return 0


if __name__ == "__main__":  # pragma: no cover
    sys.exit(main(sys.argv))
