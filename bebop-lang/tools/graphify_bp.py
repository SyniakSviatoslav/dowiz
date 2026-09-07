#!/usr/bin/env python3
"""graphify with Bebop support.

graphify has no extractor for .bp, so the whole compiler was absent from the graph.
This wrapper registers a regex extractor (fn / struct / use / calls) at runtime and
then runs the normal graphify CLI:  <graphify python> tools/graphify_bp.py update .
Run with the interpreter in graphify-out/.graphify_python (the uv tool env).
Self-check:  tools/graphify_bp.py --selftest
"""
import re
import sys
from pathlib import Path

from graphify import detect, extract as gx
from graphify.extractors.base import _make_id

FN = re.compile(r"^\s*fn\s+([A-Za-z_]\w*)\s*\(")
STRUCT = re.compile(r"^\s*struct\s+([A-Za-z_]\w*)")
USE = re.compile(r'^\s*use\s+"([^"]+)"')
CALL = re.compile(r"\b([A-Za-z_]\w*)\s*\(")
KW = {"fn", "if", "while", "let", "then", "else", "struct", "use", "ref", "return"}


def _resolve_use(path: Path, rel: str) -> Path | None:
    for d in [path.parent, *path.parent.parents]:
        if (d / rel).is_file():
            return d / rel
    return None


def extract_bp(path: Path) -> dict:
    try:
        src = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return {"error": f"cannot read {path}"}
    sp, stem = str(path), path.stem
    nodes, edges, raw_calls, seen = [], [], [], set()

    def add_node(nid, label, line):
        if nid not in seen:
            seen.add(nid)
            nodes.append({"id": nid, "label": label, "file_type": "code",
                          "source_file": sp, "source_location": f"L{line}"})

    def add_edge(s, t, rel, line):
        edges.append({"source": s, "target": t, "relation": rel, "confidence": "EXTRACTED",
                      "confidence_score": 1.0, "source_file": sp, "source_location": f"L{line}",
                      "weight": 1.0})

    file_nid = _make_id(sp)
    add_node(file_nid, path.name, 1)
    lines = [l.split("//", 1)[0] for l in src.splitlines()]
    fns = {}  # name -> (nid, start_idx)
    for i, l in enumerate(lines):
        m = FN.match(l)
        if m:
            nid = _make_id(stem, m.group(1))
            add_node(nid, f"{m.group(1)}()", i + 1)
            add_edge(file_nid, nid, "contains", i + 1)
            fns[m.group(1)] = (nid, i)
            continue
        m = STRUCT.match(l)
        if m:
            nid = _make_id(stem, m.group(1))
            add_node(nid, m.group(1), i + 1)
            add_edge(file_nid, nid, "contains", i + 1)
            continue
        m = USE.match(l)
        if m:
            tgt = _resolve_use(path, m.group(1))
            tgt_nid = _make_id(str(tgt)) if tgt else _make_id(m.group(1))
            add_node(tgt_nid, tgt.name if tgt else m.group(1), i + 1)
            add_edge(file_nid, tgt_nid, "imports", i + 1)
    # ponytail: fn body = lines from its header to the next fn header (no brace matching);
    # upgrade to brace counting if nested fns ever appear.
    starts = sorted((s, n) for n, (_, s) in fns.items())
    for k, (s, name) in enumerate(starts):
        end = starts[k + 1][0] if k + 1 < len(starts) else len(lines)
        caller = fns[name][0]
        pairs = set()
        for j in range(s, end):
            for m in CALL.finditer(re.sub(r'"[^"]*"', '""', lines[j])):
                c = m.group(1)
                if c in KW or (j == s and c == name):
                    continue
                if c in fns:
                    if fns[c][0] != caller and (caller, fns[c][0]) not in pairs:
                        pairs.add((caller, fns[c][0]))
                        add_edge(caller, fns[c][0], "calls", j + 1)
                elif (caller, c) not in pairs:
                    pairs.add((caller, c))
                    raw_calls.append({"caller_nid": caller, "callee": c, "is_member_call": False,
                                      "source_file": sp, "source_location": f"L{j + 1}"})
    return {"nodes": nodes, "edges": edges, "raw_calls": raw_calls}


detect.CODE_EXTENSIONS.add(".bp")
gx._DISPATCH[".bp"] = extract_bp
gx._LANG_FAMILY_BY_EXT[".bp"] = "bebop"


def _selftest():
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "t.bp"
        p.write_text('use "missing.bp"\nstruct K { n: i64 }\n// fn ghost(x)\nfn a(x: i64) -> i64 { b(x) + zeros(3) }\n'
                     "fn b(x: i64) -> i64 { x }\n")
        r = extract_bp(p)
        labels = {n["label"] for n in r["nodes"]}
        rels = [(e["relation"]) for e in r["edges"]]
        assert {"a()", "b()", "K", "t.bp", "missing.bp"} == labels, labels
        assert rels.count("calls") == 1 and rels.count("contains") == 3 and "imports" in rels, rels
        assert [c["callee"] for c in r["raw_calls"]] == ["zeros"], r["raw_calls"]
    print("selftest ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        _selftest()
    else:
        from graphify.__main__ import main
        main()
