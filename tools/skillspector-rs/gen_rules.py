#!/usr/bin/env python3
"""gen_rules.py — generate src/rules.rs from the upstream Python Skillspector.

Mechanical, exact port: uses `ast` to parse each `static_patterns_*.py` module
and `pattern_defaults.py`, extracting:
  - `P{n}_PATTERNS = [(regex, conf), ...]`  -> Rule{rule_id, regex, conf}
  - DEFAULT_EXPLANATIONS / DEFAULT_REMEDIATIONS / RULE_ID_TO_CATEGORY /
    PATTERN_NAMES  -> (key, value) metadata tables

No regex-on-Python-text hacks: we walk the real AST, so the emitted Rust
regex literals and metadata are byte-for-byte identical to the source.

Run from tools/skillspector-rs/:  python3 gen_rules.py
"""
from __future__ import annotations
import ast
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "..", "skillspector", "src", "skillspector")
ANALYZERS = os.path.join(SRC, "nodes", "analyzers")
PATTERNS_GLOB = "static_patterns_*.py"


def parse_pattern_modules() -> list[tuple[str, str, float]]:
    """Return [(rule_id, regex, confidence)] across all pattern modules."""
    rules: list[tuple[str, str, float]] = []
    files = sorted(
        f for f in os.listdir(ANALYZERS)
        if f.startswith("static_patterns_") and f.endswith(".py")
    )
    for fn in files:
        path = os.path.join(ANALYZERS, fn)
        tree = ast.parse(open(path, encoding="utf-8").read(), filename=fn)
        for node in ast.walk(tree):
            if not isinstance(node, ast.Assign):
                continue
            for tgt in node.targets:
                if not isinstance(tgt, ast.Name):
                    continue
                # module-level `P1_PATTERNS = [...]` -> rule_id "P1"
                stem = tgt.id
                if not stem.endswith("_PATTERNS"):
                    continue
                rule_id = stem[: -len("_PATTERNS")]
                # value must be a List of Tuples (regex_const, conf_const)
                if not isinstance(node.value, (ast.List, ast.Tuple)):
                    continue
                for item in node.value.elts:
                    if not isinstance(item, (ast.Tuple, ast.List)):
                        continue
                    if len(item.elts) < 2:
                        continue
                    rx = item.elts[0]
                    cf = item.elts[1]
                    if not isinstance(rx, ast.Constant) or not isinstance(rx.value, str):
                        continue
                    if not isinstance(cf, ast.Constant) or not isinstance(cf.value, (int, float)):
                        continue
                    rules.append((rule_id, rx.value, float(cf.value)))
    return rules


def parse_metadata() -> tuple[dict[str, str], dict[str, str], dict[str, str], dict[str, str]]:
    """Parse the four DEFAULT_* dicts from pattern_defaults.py."""
    path = os.path.join(ANALYZERS, "pattern_defaults.py")
    tree = ast.parse(open(path, encoding="utf-8").read(), filename="pattern_defaults.py")
    expl: dict[str, str] = {}
    rem: dict[str, str] = {}
    cat: dict[str, str] = {}
    names: dict[str, str] = {}
    targets = {
        "DEFAULT_EXPLANATIONS": expl,
        "DEFAULT_REMEDIATIONS": rem,
        "RULE_ID_TO_CATEGORY": cat,
        "PATTERN_NAMES": names,
    }
    # Collect PatternCategory enum members -> string value.
    enum_vals: dict[str, str] = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef) and node.name == "PatternCategory":
            for n in node.body:
                if isinstance(n, ast.Assign) and len(n.targets) == 1 and \
                   isinstance(n.targets[0], ast.Name) and isinstance(n.value, ast.Constant) and \
                   isinstance(n.value.value, str):
                    enum_vals[n.targets[0].id] = n.value.value
                elif isinstance(n, ast.AnnAssign) and isinstance(n.target, ast.Name) and \
                   isinstance(n.value, ast.Constant) and isinstance(n.value.value, str):
                    enum_vals[n.target.id] = n.value.value

    def resolve_val(v: ast.AST) -> str | None:
        if isinstance(v, ast.Constant) and isinstance(v.value, str):
            return v.value
        # PatternCategory.MEMBER (or PatternCategory.MEMBER.value) -> enum string
        if isinstance(v, ast.Attribute) and isinstance(v.value, ast.Attribute) and \
           isinstance(v.value.value, ast.Name) and v.value.value.id == "PatternCategory" and \
           v.value.attr in enum_vals:
            return enum_vals[v.value.attr]
        if isinstance(v, ast.Attribute) and isinstance(v.value, ast.Name) and \
           v.value.id == "PatternCategory" and v.attr in enum_vals:
            return enum_vals[v.attr]
        return None

    names_node = None
    value = None
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign):
            for tgt in node.targets:
                if isinstance(tgt, ast.Name) and tgt.id in targets:
                    names_node = tgt.id
                    value = node.value
                    break
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            if node.target.id in targets and node.value is not None:
                names_node = node.target.id
                value = node.value
        if names_node is None:
            continue
        if not isinstance(value, ast.Dict):
            continue
        out = targets[names_node]
        for k, v in zip(value.keys, value.values):
            kk = resolve_val(k)
            vv = resolve_val(v)
            if kk is not None and vv is not None:
                out[kk] = vv
    return expl, rem, cat, names


def rust_str_literal(s: str) -> str:
    """Emit a Rust string literal (byte-faithful)."""
    out = ['"']
    for ch in s:
        if ch == '\\':
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == '\n':
            out.append("\\n")
        elif ch == '\t':
            out.append("\\t")
        elif ch == '\r':
            out.append("\\r")
        elif ord(ch) < 0x20:
            out.append("\\u{%x}" % ord(ch))
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def main() -> int:
    rules = parse_pattern_modules()
    expl, rem, cat, names = parse_metadata()
    # sort rules for deterministic output
    rules.sort(key=lambda r: (r[0], r[1]))

    lines: list[str] = []
    lines.append("// rules.rs — GENERATED by gen_rules.py. DO NOT EDIT BY HAND.")
    lines.append("// Source of truth: tools/skillspector/src/skillspector/nodes/analyzers/")
    lines.append("//   static_patterns_*.py + pattern_defaults.py")
    lines.append("//")
    lines.append("// Auto-regenerated on every build (build.rs) when the Python")
    lines.append("// sources change, so this stays byte-identical to upstream.")
    lines.append("")
    lines.append("/// A single static pattern rule: rule_id prefix, regex source, default confidence.")
    lines.append("pub struct Rule {")
    lines.append("    pub rule_id: &'static str,")
    lines.append("    pub regex: &'static str,")
    lines.append("    pub conf: f64,")
    lines.append("}")
    lines.append("")
    lines.append("/// All static patterns, mechanically extracted from P{n}_PATTERNS.")
    lines.append("pub static RULES: &[Rule] = &[")
    for rid, rx, cf in rules:
        lines.append(
            "    Rule { rule_id: \"%s\", regex: %s, conf: %s },"
            % (rid, rust_str_literal(rx), repr(cf))
        )
    lines.append("];")
    lines.append("")

    def emit_table(name: str, table: dict[str, str]) -> None:
        lines.append("/// %s (keyed by rule_id prefix)." % name)
        lines.append("pub static %s: &[(&str, &str)] = &[" % name)
        for k in sorted(table.keys()):
            lines.append("    (%s, %s)," % (rust_str_literal(k), rust_str_literal(table[k])))
        lines.append("];")
        lines.append("")

    emit_table("EXPLANATIONS", expl)
    emit_table("REMEDIATIONS", rem)
    emit_table("CATEGORIES", cat)
    emit_table("NAMES", names)
    lines.append("")
    lines.append("// total rules: %d" % len(rules))
    lines.append("")

    out_path = os.path.join(HERE, "src", "rules.rs")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print("gen_rules: wrote %d rules + 4 metadata tables to %s" % (len(rules), out_path))
    return 0


if __name__ == "__main__":
    sys.exit(main())
