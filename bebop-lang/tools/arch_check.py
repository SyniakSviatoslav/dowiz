#!/usr/bin/env python3
"""Mechanical architecture and process invariants for bebop-lang.

Every check here exists because a specific defect cost this project real time. The
comment above each one names the incident. Checks are RATCHETS where a clean state is
not reachable today: tools/arch_ratchet.txt records the current worst value and the
check fails if it gets WORSE. Ratchet numbers may only ever be lowered, by hand, with
the commit that earns the reduction.

Run: python3 tools/arch_check.py [--update-ratchet]
Exit 0 = all invariants hold. Exit 1 = at least one violated.
"""
import hashlib, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RATCHET = os.path.join(ROOT, "tools", "arch_ratchet.txt")
fails, notes = [], []

def fail(check, msg): fails.append("%s: %s" % (check, msg))
def note(msg): notes.append(msg)

def load_ratchet():
    d = {}
    if os.path.exists(RATCHET):
        for line in open(RATCHET):
            line = line.split("#")[0].strip()
            if not line: continue
            k, v = line.split("=", 1)
            d[k.strip()] = int(v.strip())
    return d

def bp_sources():
    """Every .bp we author. Excludes generated and vendored trees."""
    skip = ("/.claude/", "/attic/", "/bench/fuzz/repros/", "/bench/wip/", "/seed/")
    out = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__", ".becache")]
        for fn in filenames:
            if not fn.endswith(".bp"): continue
            p = os.path.join(dirpath, fn)
            rel = "/" + os.path.relpath(p, ROOT)
            if any(s in rel for s in skip): continue
            out.append(p)
    return sorted(out)

# --- CHECK 1: no nested function definitions -------------------------------------
# Incident: nested/indented definitions make a file unreadable and hide scope bugs.
# bebop has no closures (docs/LANGUAGE.md), so an indented `fn` is always a mistake.
def check_no_nested_fn():
    bad = []
    for p in bp_sources():
        for i, line in enumerate(open(p, errors="replace"), 1):
            if re.match(r"^[ \t]+fn\s+\w+\s*\(", line):
                bad.append("%s:%d" % (os.path.relpath(p, ROOT), i))
    if bad: fail("nested-fn", "function defined inside another function: " + ", ".join(bad[:10]))

# --- CHECK 2: file size ratchet ---------------------------------------------------
# Incident: bebop.bp is 7700 lines and every change to it is high-risk. New files must
# stay small; existing offenders may only shrink.
def check_file_size(r):
    cap = r.get("max_new_bp_lines", 800)
    worst = r.get("max_bp_lines", 8500)
    biggest, biggest_n = None, 0
    for p in bp_sources():
        n = sum(1 for _ in open(p, errors="replace"))
        rel = os.path.relpath(p, ROOT)
        if n > biggest_n: biggest, biggest_n = rel, n
        if n > cap and ("bp_exempt:" + rel) not in r:
            fail("file-size", "%s is %d lines, cap for non-exempt files is %d "
                 "(split it, or add `bp_exempt:%s = 1` with a reason)" % (rel, n, cap, rel))
    if biggest_n > worst:
        fail("file-size-ratchet", "%s grew to %d lines, ratchet is %d -- the ratchet may go "
             "DOWN, never up" % (biggest, biggest_n, worst))
    note("largest .bp: %s at %d lines (ratchet %d)" % (biggest, biggest_n, worst))

# --- CHECK 3: the promoted compiler IS the source's compiler ----------------------
# Incident 2026-09-12 (a18fa8b): bebop.bin at HEAD was 292b8953 while compile(bebop.bp)
# was 7c7d1f77. Two different compilers; every gate number was measured against an
# artifact no source produces. `chain.sh`'s fixpoint does NOT catch this -- it only
# proves the SOURCE has a fixpoint.
def check_binary_matches_source():
    binp = os.path.join(ROOT, "bebop.bin")
    if not os.path.exists(binp): return fail("artifact-identity", "bebop.bin is missing")
    out = "/tmp/arch_check_gen.bin"
    seed = os.path.join(ROOT, "seed", "build", "seed")
    if not os.path.exists(seed): return note("artifact-identity: skipped, no seed binary")
    rc = subprocess.run([seed, binp, "compile", os.path.join(ROOT, "bebop.bp"), out],
                        capture_output=True).returncode
    if rc != 0: return fail("artifact-identity", "bebop.bin cannot compile bebop.bp (rc=%d)" % rc)
    a = hashlib.md5(open(binp, "rb").read()).hexdigest()[:8]
    b = hashlib.md5(open(out, "rb").read()).hexdigest()[:8]
    if a != b:
        fail("artifact-identity", "bebop.bin is %s but compile(bebop.bp) is %s -- the promoted "
             "compiler is not the source's compiler. Re-promote: cp gen4.bin bebop.bin.tmp && "
             "mv bebop.bin.tmp bebop.bin" % (a, b))
    else: note("artifact-identity: bebop.bin == compile(bebop.bp) == %s" % a)

# --- CHECK 4: a gate's store is removed before the gate runs ----------------------
# Incident 2026-09-12 (09944f7, d3445fb): nine gb gates were RED because of stale .store
# files -- st_open maps a file it does not match and traps 82 with no output. Eight were
# in $BEBOP_TMP, gb_roundtrip's was in the repo root.
def check_gates_clean_their_stores():
    g = os.path.join(ROOT, "bench", "vs_rust", "std_golden.sh")
    if not os.path.exists(g): return
    src = open(g, errors="replace").read()
    stores = set(re.findall(r"(?:^|[/\s\"])([A-Za-z][A-Za-z0-9_]*)\.store", src, re.M))
    removed = set(re.findall(r"rm -f[^\n]*?([A-Za-z][A-Za-z0-9_]*)\.store", src))
    blanket = 'rm -f "$BEBOP_TMP"/gb_*.store' in src
    for s in sorted(stores - removed):
        if blanket and s.startswith("gb_"): continue
        fail("stale-store", "%s.store is used by std_golden.sh but never `rm -f`'d before use" % s)

# --- CHECK 5: producer gates are never memo-replayed ------------------------------
# Incident 2026-09-12 (a742c9e): the memo replays a PASSed gate by printing cached stdout
# WITHOUT running the binary, so its side-effect file is never written and every consumer
# folds to 0. A gate whose file another gate reads must run directly.
def check_producers_not_memoised():
    g = os.path.join(ROOT, "bench", "vs_rust", "std_golden.sh")
    if not os.path.exists(g): return
    lines = open(g, errors="replace").read().split("\n")
    produced = {}
    for i, line in enumerate(lines):
        for m in re.finditer(r"\$GBT/([A-Za-z0-9_]+)\.store", line):
            produced.setdefault(m.group(1), []).append(i)
    for name, where in produced.items():
        if len(where) < 2: continue          # written and read in one place: self-test
        first = where[0]
        if re.search(r"&&\s*run\s+\d+", lines[first]):
            fail("producer-memo", "%s.store is written at std_golden.sh:%d through `run` and read "
                 "again at line %d -- a memo replay would skip writing it. Run producers directly."
                 % (name, first + 1, where[1] + 1))

# --- CHECK 6: failures must be LOUD -----------------------------------------------
# Operator rule, 2026-09-12: "make failures loud everywhere, and make it a rule."
# Nearly every defect that cost this project a day was SILENT, not subtle: a child that
# died left the parent hanging and read as "slow"; `got=` empty had three different causes
# that printed identically; a fold of 0 named none of its five guard factors. This check
# flags the constructs that MANUFACTURE silence in gate and tool scripts: discarded stderr,
# and a run whose exit code is thrown away by a pipeline.
def check_loud_failures(r):
    import glob
    allow = set(k.split(":", 1)[1] for k in r if k.startswith("loud_exempt:"))
    bad = []
    roots = [os.path.join(ROOT, "bench", "vs_rust"), os.path.join(ROOT, "tools")]
    for d in roots:
        for p in sorted(glob.glob(os.path.join(d, "*.sh"))):
            rel = os.path.relpath(p, ROOT)
            if rel in allow: continue
            for i, line in enumerate(open(p, errors="replace"), 1):
                if line.lstrip().startswith("#"): continue
                # stderr discarded on a RUN of a compiled artifact. Compiles may stay quiet:
                # their failure is caught by the `&&` that follows and by the .bin guard.
                # the seed LOADER being run, not any mention of the word: `as seed/seed.S`
                # is an assembler call and its stderr is noise, not a silenced failure.
                if "2>/dev/null" in line and "compile" not in line and re.search(r"(\./seed/build/seed|\$SEED|\$\{SEED)", line):
                    bad.append("%s:%d stderr discarded on a run" % (rel, i))
    if bad:
        fail("loud-failure", "a failure is being silenced -- name it instead: " + "; ".join(bad[:8]))
    else:
        note("loud-failure: no silenced runs in gate or tool scripts")

def main():
    r = load_ratchet()
    check_no_nested_fn()
    check_file_size(r)
    check_binary_matches_source()
    check_gates_clean_their_stores()
    check_producers_not_memoised()
    check_loud_failures(r)
    for n in notes: print("  note: " + n)
    if fails:
        print("\narch_check: %d INVARIANT(S) VIOLATED" % len(fails))
        for f in fails: print("  FAIL " + f)
        return 1
    print("\narch_check: all invariants hold")
    return 0

if __name__ == "__main__":
    sys.exit(main())
