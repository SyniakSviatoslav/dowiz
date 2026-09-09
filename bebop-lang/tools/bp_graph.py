#!/usr/bin/env python3
"""bp_graph.py (2026-09-09) -- a graphify extractor for Bebop `.bp`, the one language
graphify cannot see.

WHY THIS EXISTS. `graphify update` indexed this tree and produced 2,615 nodes of which
172 had a recognised language and all 172 were bash -- against 497 `.bp` files and 63,257
lines. graphify's extension table (`graphify/extract.py:2558`) and its extractor dispatch
(`:5769`) have no `.bp` entry, so the language the project is WRITTEN IN was invisible to
the tool CLAUDE.md tells every agent to consult first.

WHY IT IS HERE AND NOT A PATCH TO site-packages. Editing
`/usr/local/lib/python3.14/dist-packages/graphify/extract.py` works until the next upgrade
silently reverts it, and a tool whose correctness depends on an unrecorded edit to someone
else's package is the same class of defect as the five documentation-versus-reality gaps
found today. Instead this emits graphify's OWN schema and is combined with its OWN
documented command, `graphify merge-graphs`, so an upgrade cannot undo it.

WHAT IT EXTRACTS. Bebop's surface is small and regular (`docs/LANGUAGE.md`): one `fn` per
definition, no closures, no generics, no methods, no overloading. So a function's callees
are exactly the identifiers followed by `(` that name another `fn` in the same expansion --
which is why a regex extractor is honest here and would not be in a language with dynamic
dispatch. Nodes: one per file, one per `fn`. Edges: `contains` file->fn, `calls` fn->fn,
`uses` file->file for `use "path"` lines.

WHAT IT DOES NOT DO, said plainly rather than discovered later: it does not resolve a call
through `use` expansion order, so a call to a name defined in two included files is
attributed to both; it does not see builtins (they are the compiler's dispatch chain, not
`fn`s -- `tools/builtin_surface.py` owns that surface); and it has no notion of the arena,
the store or the register model. It is a call graph, not a semantics.

Usage:
  tools/bp_graph.py <dir> -o out.json      emit the .bp graph alone
  tools/bp_graph.py --stats <dir>          counts only, no file written
Then:
  graphify merge-graphs graphify-out/graph.json out.json --out graphify-out/graph.json
"""
import json
import os
import re
import sys

FN = re.compile(r"^fn\s+([A-Za-z_][A-Za-z_0-9]*)\s*\(", re.M)
USE = re.compile(r'^\s*use\s+"([^"]+)"', re.M)
CALL = re.compile(r"\b([A-Za-z_][A-Za-z_0-9]*)\s*\(")
# `if`/`while`/`match` are not calls; `zeros(` and the sys_* family are builtins, and the
# builtin surface is owned by tools/builtin_surface.py rather than duplicated here.
KEYWORDS = {"if", "while", "match", "let", "fn", "return", "break", "enum", "struct", "use"}


def node_id(path, name=None):
    base = path.replace("/", "_").replace(".", "_").replace("-", "_")
    return base if name is None else base + "__" + name


def scan(root):
    files = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in (".git", "graphify-out", "attic")]
        for fn in filenames:
            if fn.endswith(".bp"):
                files.append(os.path.join(dirpath, fn))
    return sorted(files)


def build(root):
    nodes, links = [], []
    defs = {}  # fn name -> [node id]
    per_file = {}

    files = scan(root)
    for path in files:
        rel = os.path.relpath(path, ".") if path.startswith("/") else path
        try:
            src = open(path, encoding="utf-8", errors="replace").read()
        except OSError:
            continue
        fid = node_id(rel)
        nodes.append({
            "id": fid, "label": os.path.basename(rel), "file_type": "code",
            "source_file": rel, "source_location": "L1",
            "metadata": {"language": "bebop", "kind": "file"}, "_origin": "ast",
        })
        lines = src.split("\n")
        fns = []
        for m in FN.finditer(src):
            name = m.group(1)
            line = src.count("\n", 0, m.start()) + 1
            nid = node_id(rel, name)
            fns.append((name, nid, line))
            defs.setdefault(name, []).append(nid)
            nodes.append({
                "id": nid, "label": name, "file_type": "code",
                "source_file": rel, "source_location": "L%d" % line,
                "metadata": {"language": "bebop", "kind": "function"}, "_origin": "ast",
            })
            links.append({
                "source": fid, "target": nid, "relation": "contains",
                "confidence": "EXTRACTED", "source_file": rel,
                "source_location": "L%d" % line, "weight": 1.0, "_origin": "ast",
            })
        per_file[rel] = (fid, fns, lines)
        for m in USE.finditer(src):
            links.append({
                "source": fid, "target": node_id(m.group(1)), "relation": "uses",
                "confidence": "EXTRACTED", "source_file": rel,
                "source_location": "L%d" % (src.count("\n", 0, m.start()) + 1),
                "weight": 1.0, "_origin": "ast",
            })

    # second pass: calls, now that every definition is known
    ncalls = 0
    for rel, (fid, fns, lines) in per_file.items():
        for idx, (name, nid, line) in enumerate(fns):
            end = fns[idx + 1][2] - 1 if idx + 1 < len(fns) else len(lines)
            body = "\n".join(lines[line - 1:end])
            seen = set()
            for cm in CALL.finditer(body):
                callee = cm.group(1)
                if callee in KEYWORDS or callee == name or callee in seen:
                    continue
                targets = defs.get(callee)
                if not targets:
                    continue
                seen.add(callee)
                for t in targets:
                    links.append({
                        "source": nid, "target": t, "relation": "calls",
                        "confidence": "EXTRACTED", "source_file": rel,
                        "source_location": "L%d" % line, "weight": 1.0, "_origin": "ast",
                    })
                    ncalls += 1
    return nodes, links, len(files), ncalls


def main():
    args = [a for a in sys.argv[1:]]
    stats = "--stats" in args
    if stats:
        args.remove("--stats")
    out = None
    if "-o" in args:
        i = args.index("-o")
        out = args[i + 1]
        del args[i:i + 2]
    root = args[0] if args else "."

    nodes, links, nfiles, ncalls = build(root)
    fns = sum(1 for n in nodes if n["metadata"]["kind"] == "function")
    sys.stderr.write("bp_graph: %d files, %d fns, %d nodes, %d links (%d calls)\n"
                     % (nfiles, fns, len(nodes), len(links), ncalls))
    if not nodes:
        sys.stderr.write("bp_graph: no .bp files under %s -- refusing to write an empty graph\n" % root)
        return 1
    if stats:
        return 0
    if not out:
        sys.stderr.write("bp_graph: -o <path> required unless --stats\n")
        return 2
    json.dump({"input_tokens": 0, "output_tokens": 0, "failed_sources": [],
               "nodes": nodes, "links": links, "directed": False},
              open(out, "w"), ensure_ascii=False)
    sys.stderr.write("bp_graph: wrote %s\n" % out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
