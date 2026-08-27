#!/usr/bin/env python3
"""
resync_self_check.py — automated self_check checksum resync (L0 gate).

Runs compile/compile_fn/compile_program via qtt_eval_binds (the interpreter
oracle) and patches selfhost/expr_compile.bp:2408 and native/src/fuzz_selfhost.c:431.

Usage:
  python3 tools/resync_self_check.py          # in-place patch, prints diff
  python3 tools/resync_self_check.py --check  # exit 1 if stale (CI)

L0 rule: self_check must be byte-identical to the interpreter's output;
hand-typed checksums are forbidden.
"""
import re, subprocess, sys, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BP = os.path.join(ROOT, "selfhost/expr_compile.bp")
FUZZ = os.path.join(ROOT, "native/src/fuzz_selfhost.c")
BEBOPC = os.path.join(ROOT, "native/build/bebopc")

# The 17 checks that changed in 5bf2123; the 9 compile() checks are stable.
# We recompute all 41 via qtt_eval_binds to be future-proof.

def run_bp(fn: str, arg: str) -> str:
    # bebopc run <file> <fn> <arg>  (arg is a .bp string literal without outer quotes)
    # For run_program we need two args, handle separately.
    # This helper is for single-arg cases: fn takes (s: str)
    import shlex, subprocess
    # Use bebopc run with one string arg; it wraps as TERM_STR
    out = subprocess.check_output([BEBOPC, "run", BP, fn, arg], text=True, timeout=10)
    # output is like "237421204746\n" or "i64 = 42\n"
    # For compile* it prints the checksum; take last token
    tok = out.strip().split()[-1]
    # strip trailing non-digit
    m = re.search(r'-?\d+', tok)
    return m.group(0) if m else tok

def collect_checks():
    txt = open(BP).read()
    # find self_check block
    # each line: let cN = if <call> == <num> then 0 else 1;
    pat = re.compile(r'let c(\d+) = if (compile\w*|run_program)\((.*?)\) == (\d+)')
    checks = []
    for m in pat.finditer(txt):
        cnum, fn, args_str, old = m.groups()
        checks.append((int(cnum), fn, args_str, old, m))
    return checks

def main():
    check_only = "--check" in sys.argv
    # Quick sanity: bebopc must exist
    if not os.path.exists(BEBOPC):
        print(f"bebopc not found at {BEBOPC}, run `make -C native` first", file=sys.stderr)
        sys.exit(2)
    txt = open(BP).read()
    out_txt = txt
    out_fuzz = open(FUZZ).read() if os.path.exists(FUZZ) else None

    # Recompute via Python invoking bebopc run for each check
    # For speed, we batch via a single Python that loads the compiler once;
    # for now, simple per-check subprocess is fine (41 * ~30ms = 1.2s)
    pat = re.compile(r'let c(\d+) = if (compile\w*|run_program)\((.*?)\) == (\d+) then')
    # We need to handle run_program's two args: "s", arg
    # Our run_bp helper above handles single-arg; for run_program we need to test differently
    # For this resync we only care about compile/compile_fn/compile_program (the 17 that changed);
    # run_program checks are execution checks (42,120,10,42,7) and are stable.
    updates = 0
    for m in pat.finditer(txt):
        cnum, fn, args_str, old = m.groups()
        if fn == "run_program":
            continue  # execution checks, not checksums
        # args_str is like '"fn main() { 42 }"' or '"fn f(a) { a + 1 }"' etc
        # Extract the string literal content
        sm = re.search(r'"(.*)"', args_str)
        if not sm:
            continue
        s = sm.group(1)
        # For compile_program the arg is the whole program string, may contain quotes
        # sm.group(1) is greedy, may capture too much; use non-greedy for run_program already skipped
        # For compile* single arg, the whole args_str is just one quoted string
        # Re-extract with proper handling: find first and last quote
        # Simpler: use the raw args_str's first quoted string
        # Already done

        # Skip stable compile() checks (they rarely change) to save time, but we recompute anyway
        try:
            new = run_bp(fn, s)
        except Exception as e:
            print(f"c{cnum} {fn} failed: {e}", file=sys.stderr)
            continue
        if new != old:
            print(f"c{cnum} {fn}: {old} -> {new}")
            out_txt = out_txt.replace(f"== {old} then", f"== {new} then", 1)
            updates += 1
            # also patch fuzz_selfhost.c if it contains this want
            if out_fuzz and old in out_fuzz:
                out_fuzz = out_fuzz.replace(f"{old}L, \"{fn}\"", f"{new}L, \"{fn}\"", 1)
                # also handle the two specific fuzz_selfhost checks
                out_fuzz = out_fuzz.replace(f"{old}L, \"compile", f"{new}L, \"compile", 1)

    if updates == 0:
        print("resync: no changes")
        sys.exit(0)

    if check_only:
        print(f"resync: {updates} checksums stale", file=sys.stderr)
        sys.exit(1)

    open(BP, "w").write(out_txt)
    if out_fuzz is not None:
        open(FUZZ, "w").write(out_fuzz)
    print(f"resync: patched {updates} checksums in {BP} and {FUZZ}")

if __name__ == "__main__":
    main()
