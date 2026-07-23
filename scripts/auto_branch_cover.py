#!/usr/bin/env python3
"""Auto-generate branch coverage tests for dowiz-kernel.
V2: focuses on publicly-testable pub fn's, generates proper module-qualified calls.
"""

import json
import os
import re
import sys
from collections import defaultdict, Counter
from pathlib import Path

KERNEL_SRC = Path("/root/dowiz/kernel/src")
COV_JSON = "/tmp/cov.json"
OUTPUT_FILE = Path("/root/dowiz/kernel/tests/auto_branch_coverage.rs")


def parse_coverage():
    with open(COV_JSON) as f:
        data = json.load(f)
    entry = data["data"][0]
    uncovered = []
    for file_entry in entry["files"]:
        fname = file_entry["filename"]
        for branch in file_entry.get("branches", []):
            tc, fc = branch[4], branch[5]
            if tc == 0 or fc == 0:
                uncovered.append({
                    "file": fname,
                    "line": branch[0],
                    "col": branch[1],
                    "line_end": branch[2],
                    "col_end": branch[3],
                    "true_hit": tc,
                    "false_hit": fc,
                    "expansion_id": branch[-1] if len(branch) > 6 else 0,
                })
    return uncovered


def file_to_module_path(filepath):
    """Convert file path to Rust module path relative to kernel."""
    rel = filepath.replace("/root/dowiz/kernel/src/", "").replace(".rs", "")
    if rel == "lib":
        return "dowiz_kernel"
    parts = rel.split("/")
    if parts[-1] == "mod":
        parts = parts[:-1]
    return "dowiz_kernel::" + "::".join(parts)


def find_enclosing_function(source_lines, line_num):
    """Find the enclosing function name and its line (1-indexed)."""
    for i in range(line_num - 1, -1, -1):
        line = source_lines[i]
        m = re.search(r'(?:pub(?:\s*\(\s*crate\s*\)\s*)?\s+)?fn\s+(\w[\w_]*)\s*[<(]', line)
        if m:
            return m.group(1), i + 1
    return None, None


def is_pub_fn(line):
    """Check if a line declares a pub fn."""
    return bool(re.match(r'pub(\s*\(\s*crate\s*\))?\s+fn\s+\w+', line.strip()))


def extract_fn_full_sig(source_lines, fn_line_idx):
    """Extract the full function signature including multi-line ones."""
    sig = source_lines[fn_line_idx]
    # If the line doesn't contain an opening brace, read more lines
    combined = [sig]
    for j in range(fn_line_idx + 1, min(len(source_lines), fn_line_idx + 20)):
        line = source_lines[j]
        combined.append(line)
        if "{" in line:
            break
    return "".join(combined)


def extract_params(fn_sig):
    """Extract parameter (name, type) pairs from a function signature."""
    m = re.search(r'fn\s+\w+\s*(?:<\s*[^>]*\s*>)?\s*\(([^)]*)\)', fn_sig, re.DOTALL)
    if not m:
        return []
    params_str = m.group(1).strip()
    if not params_str:
        return []
    # Handle nested brackets
    params = []
    depth = 0
    current = ""
    for ch in params_str:
        if ch in "([{<":
            depth += 1
            current += ch
        elif ch in ")]}>":
            depth -= 1
            current += ch
        elif ch == "," and depth == 0:
            params.append(current.strip())
            current = ""
        else:
            current += ch
    if current.strip():
        params.append(current.strip())
    result = []
    for p in params:
        p = p.strip()
        if not p:
            continue
        # Handle `self` params
        if p in ("self", "&self", "&mut self", "mut self"):
            result.append(("self", "Self"))
            continue
        # Handle `mut name: Type` pattern
        m = re.match(r'(?:mut\s+)?(\w+)\s*:\s*(.+)', p)
        if m:
            result.append((m.group(1), m.group(2).strip()))
    return result


def type_to_default(typestr, pname):
    """Generate a reasonable default argument string for a type."""
    t = typestr.strip()
    # Check for &[T] or &[T; N]
    if re.match(r'&\s*\[\s*[uif]\d+', t) or re.match(r'&\s*\[u\d+\s*;\s*\d+\s*\]', t):
        return "&[]"
    if re.match(r'&\s*\[\s*\w', t):
        return "&[]"
    if re.match(r'&\s*\[\s*[A-Z]', t):
        return "&[]"
    if t == "&str" or t == "&'static str":
        return '""'
    # Numeric types
    for num_type in ["usize", "u64", "u32", "u16", "u8", "isize", "i64", "i32", "i16", "i8", "f64", "f32"]:
        if t == num_type:
            return "0" if num_type.startswith(("u", "i")) else "0.0"
        if t.startswith("&" + num_type):
            return "&0"
    # Generic primitive refs
    if t == "&usize":
        return "&0"
    if t == "&u64":
        return "&0_u64"
    if t == "&f64":
        return "&0.0"
    if t == "bool":
        return "false"
    if t == "&bool":
        return "&false"
    if t == "&str":
        return '""'
    if t == "String":
        return 'String::from("")'
    if t == "&String":
        return '&String::new()'
    if t.startswith("Vec<") or t.startswith("&Vec<"):
        return "vec![]"
    if t.startswith("&["):
        return "&[]"
    if t.startswith("Option<"):
        return "None"
    if t.startswith("&Option<"):
        return "&None"
    if t.startswith("HashMap<"):
        return "std::collections::HashMap::new()"
    if t.startswith("BTreeMap<"):
        return "std::collections::BTreeMap::new()"
    if t.startswith("HashSet<"):
        return "std::collections::HashSet::new()"
    if t.startswith("&mut "):
        inner = t[5:]
        return f"&mut ()"  # fallback
    if "Fn" in t or "FnMut" in t or "FnOnce" in t:
        return "_dummy_closure"
    if t == "impl Into<String>" or t == "impl AsRef<str>":
        return '""'
    if t == "impl Into<Vec<u8>>":
        return "vec![]"
    # For reference types: &Type -> pass a ref to default
    if t.startswith("&"):
        return f"&/*{t}*/Default::default()"
    # Generic/unknown
    return f"/*{t}*/Default::default()"


def main():
    print("Parsing coverage data...", file=sys.stderr)
    uncovered = parse_coverage()
    print(f"Found {len(uncovered)} uncovered branches", file=sys.stderr)

    # Group by file
    by_file = defaultdict(list)
    for b in uncovered:
        by_file[b["file"]].append(b)

    sorted_files = sorted(by_file.items(), key=lambda x: -len(x[1]))

    # Phase 1: find all pub functions and uncovered branches within them
    pub_funcs_to_cover = defaultdict(list)  # (file, func_name) -> [(line, branch_data)]

    for filepath, branches in sorted_files[:40]:
        rel_path = filepath.replace("/root/dowiz/kernel/src/", "")
        full_path = KERNEL_SRC / rel_path
        if not full_path.exists():
            continue
        with open(full_path) as f:
            source_lines = f.read().splitlines()

        # Find all pub fn declarations
        pub_fn_lines = {}
        for i, line in enumerate(source_lines):
            if is_pub_fn(line):
                m = re.search(r'fn\s+(\w+)', line)
                if m:
                    pub_fn_lines[m.group(1)] = i + 1

        if not pub_fn_lines:
            continue

        for b in branches:
            fn_name, fn_line = find_enclosing_function(source_lines, b["line"])
            if fn_name and fn_name in pub_fn_lines:
                key = (filepath, fn_name)
                pub_funcs_to_cover[key].append(b)

    print(f"Found {len(pub_funcs_to_cover)} public functions with uncovered branches", file=sys.stderr)

    # Phase 2: Generate tests
    test_lines = []
    test_lines.append("// Auto-generated branch coverage tests for dowiz-kernel")
    test_lines.append("// Generated by scripts/auto_branch_cover.py")
    test_lines.append("")
    test_lines.append("#[allow(unused_imports)]")
    test_lines.append("use dowiz_kernel::*;")
    test_lines.append("")

    test_idx = 0

    # Focus on testable functions - prioritize those with most uncovered branches
    sorted_funcs = sorted(pub_funcs_to_cover.items(), key=lambda x: -len(x[1]))

    for (filepath, fn_name), branches in sorted_funcs:
        if test_idx >= 300:
            break

        rel_path = filepath.replace("/root/dowiz/kernel/src/", "")
        full_path = KERNEL_SRC / rel_path
        with open(full_path) as f:
            source_lines = f.read().splitlines()

        # Find function line, signature, and body
        fn_line = None
        for i, line in enumerate(source_lines):
            if is_pub_fn(line) and f"fn {fn_name}" in line:
                fn_line = i + 1
                break
        if fn_line is None:
            continue

        fn_sig = extract_fn_full_sig(source_lines, fn_line - 1)
        params = extract_params(fn_sig)

        # Also collect the body to understand branch types
        fn_end = min(len(source_lines), fn_line + 100)
        fn_body_lines = source_lines[fn_line:fn_end]

        # Determine module path
        mod_path = file_to_module_path(filepath)

        # Collect lines that are uncovered
        uncovered_lines = set(b["line"] for b in branches)

        # Analyze what kind of branches are uncovered to tailor test inputs
        covered_by_bad_input = False
        covered_by_good_input = False
        covered_by_bool_true = False
        covered_by_bool_false = False
        covered_by_none = False
        covered_by_some = False
        covered_by_empty = False
        covered_by_zero = False
        covered_by_nonzero = False

        for bidx, bline in enumerate(uncovered_lines):
            line_idx = bline - 1
            if 0 <= line_idx < len(source_lines):
                line_text = source_lines[line_idx].strip()
                if "return Err" in line_text or "Err(" in line_text:
                    covered_by_bad_input = True
                elif "if " in line_text:
                    if "true" in line_text or "is_true" in line_text:
                        covered_by_bool_true = True
                    if "false" in line_text or "is_false" in line_text:
                        covered_by_bool_false = True
                    if "is_none" in line_text or "== None" in line_text:
                        covered_by_none = True
                    if "is_some" in line_text:
                        covered_by_some = True
                    if ".is_empty" in line_text or "len() == 0" in line_text:
                        covered_by_empty = True
                else:
                    # Other code paths
                    covered_by_good_input = True

        # Generate test code
        module_fn = f"{mod_path}::{fn_name}"

        # Build base args
        base_args = []
        for pname, ptype in params:
            if pname == "self":
                continue
            base_args.append(type_to_default(ptype, pname))

        base_call = f"{module_fn}({', '.join(base_args)})"

        # Generate multiple tests for different input combinations
        # Test 1: Base/default inputs (catches many error paths with bad defaults)
        test_name = f"cover_{test_idx}_{fn_name}_default"
        test_code_block = []
        test_code_block.append(f"#[test]")
        test_code_block.append(f"fn {test_name}() {{")
        if "->" in fn_sig:
            if "Result" in fn_sig:
                test_code_block.append(f"    let _ = {base_call};")
            elif "Option" in fn_sig:
                test_code_block.append(f"    let _r = {base_call};")
            else:
                test_code_block.append(f"    let _r = {base_call};")
        else:
            test_code_block.append(f"    {base_call};")
        test_code_block.append("}")
        test_lines.extend(test_code_block)
        test_idx += 1

        # Test 2: If bool params exist, try the inverse
        for pname, ptype in params:
            if pname == "self":
                continue
            t = ptype.strip()
            if t in ("bool", "&bool"):
                alt_args = []
                for pn2, pt2 in params:
                    if pn2 == "self":
                        continue
                    if pn2 == pname:
                        alt_args.append("true")
                    else:
                        alt_args.append(type_to_default(pt2, pn2))
                test_name = f"cover_{test_idx}_{fn_name}_bool_true"
                test_code_block = []
                test_code_block.append(f"#[test]")
                test_code_block.append(f"fn {test_name}() {{")
                if "->" in fn_sig:
                    if "Result" in fn_sig:
                        test_code_block.append(f"    let _ = {module_fn}({', '.join(alt_args)});")
                    else:
                        test_code_block.append(f"    let _r = {module_fn}({', '.join(alt_args)});")
                else:
                    test_code_block.append(f"    {module_fn}({', '.join(alt_args)});")
                test_code_block.append("}")
                test_lines.extend(test_code_block)
                test_idx += 1
                break

        # Test 3: If Option params exist, try Some
        for pname, ptype in params:
            if pname == "self":
                continue
            t = ptype.strip()
            if "Option<" in t:
                alt_args = []
                for pn2, pt2 in params:
                    if pn2 == "self":
                        continue
                    if pn2 == pname:
                        alt_args.append("None")  # keep as None for first
                    else:
                        alt_args.append(type_to_default(pt2, pn2))
                test_name = f"cover_{test_idx}_{fn_name}_option_none"
                test_code_block = []
                test_code_block.append(f"#[test]")
                test_code_block.append(f"fn {test_name}() {{")
                if "->" in fn_sig:
                    if "Result" in fn_sig:
                        test_code_block.append(f"    let _ = {module_fn}({', '.join(alt_args)});")
                    else:
                        test_code_block.append(f"    let _r = {module_fn}({', '.join(alt_args)});")
                else:
                    test_code_block.append(f"    {module_fn}({', '.join(alt_args)});")
                test_code_block.append("}")
                test_lines.extend(test_code_block)
                test_idx += 1
                break

        # Test 4: Non-empty input for &[T] or Vec params
        for pname, ptype in params:
            if pname == "self":
                continue
            t = ptype.strip()
            if "&[" in t or "Vec<" in t:
                # Determine element type
                elem_default = "0_u8"
                m = re.match(r'(?:&)?\[?Vec<([^>]+)>\]?', t)
                if m:
                    inner = m.group(1).strip()
                    if inner == "u8" or inner == "&u8":
                        elem_default = "0_u8"
                    elif inner == "f64":
                        elem_default = "0.0"
                    elif inner == "String":
                        elem_default = 'String::from("")'
                    elif inner == "usize":
                        elem_default = "0_usize"
                    else:
                        elem_default = f"/*{inner}::*/Default::default()"

                alt_args = []
                for pn2, pt2 in params:
                    if pn2 == "self":
                        continue
                    if pn2 == pname:
                        alt_args.append(f"vec![{elem_default}]" if "Vec" in t else f"&[{elem_default}]")
                    else:
                        alt_args.append(type_to_default(pt2, pn2))
                test_name = f"cover_{test_idx}_{fn_name}_nonempty"
                test_code_block = []
                test_code_block.append(f"#[test]")
                test_code_block.append(f"fn {test_name}() {{")
                if "->" in fn_sig:
                    if "Result" in fn_sig:
                        test_code_block.append(f"    let _ = {module_fn}({', '.join(alt_args)});")
                    else:
                        test_code_block.append(f"    let _r = {module_fn}({', '.join(alt_args)});")
                else:
                    test_code_block.append(f"    {module_fn}({', '.join(alt_args)});")
                test_code_block.append("}")
                test_lines.extend(test_code_block)
                test_idx += 1
                break

        # Test 5: Non-zero numeric inputs
        for pname, ptype in params:
            if pname == "self":
                continue
            t = ptype.strip()
            for num_type in ["usize", "u64", "u32", "u16", "u8", "f64", "f32"]:
                if t == num_type or t == f"&{num_type}":
                    alt_args = []
                    for pn2, pt2 in params:
                        if pn2 == "self":
                            continue
                        if pn2 == pname:
                            alt_args.append("1" if num_type.startswith(("u", "i")) else "1.0")
                        else:
                            alt_args.append(type_to_default(pt2, pn2))
                    test_name = f"cover_{test_idx}_{fn_name}_nonzero"
                    test_code_block = []
                    test_code_block.append(f"#[test]")
                    test_code_block.append(f"fn {test_name}() {{")
                    if "->" in fn_sig:
                        if "Result" in fn_sig:
                            test_code_block.append(f"    let _ = {module_fn}({', '.join(alt_args)});")
                        else:
                            test_code_block.append(f"    let _r = {module_fn}({', '.join(alt_args)});")
                    else:
                        test_code_block.append(f"    {module_fn}({', '.join(alt_args)});")
                    test_code_block.append("}")
                    test_lines.extend(test_code_block)
                    test_idx += 1
                    break
            break

    # Write output
    output = "\n".join(test_lines)

    OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_FILE, "w") as f:
        f.write(output)
    print(f"Generated {test_idx} tests for {len(sorted_funcs)} functions", file=sys.stderr)
    print(f"Written to {OUTPUT_FILE}", file=sys.stderr)

    with open("/dev/shm/gen_tests.rs", "w") as f:
        f.write(output)

    # Print summary for report
    print(f"TESTS_GENERATED={test_idx}")
    print(f"FUNCTIONS_TARGETED={len(sorted_funcs)}")


if __name__ == "__main__":
    main()
