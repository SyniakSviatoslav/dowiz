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
import collections, hashlib, os, re, subprocess, sys

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
    # Negative fixtures exist precisely to be REJECTED, and their functions are never
    # meant to be called; counting them as dead code would drown the real signal.
    skip = ("/.claude/", "/attic/", "/bench/fuzz/", "/bench/wip/", "/seed/",
            "/typecheck_neg/", "/parity_constructs/neg/", "/kernel_neg/", "/repros/")
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

# --- CHECK 7: object header cells are off-limits to writers -----------------------
# Incident 2026-09-12 (24fcb57): st_alloc lays an object out as header at `off`, crc cell
# at `off+1`, payload from `off+2` -- the convention st_get/st_ref/st_len/st_seal all share.
# st_parttab_write wrote its 19 cells from base[pt_off + 0], overwriting the header st_alloc
# had just written, so st_len returned the low word of the store magic (1329743170) and
# st_seal crc'd 10.6 GB out of a 64 KB mapping. One off-by-two, nine red gates, a day to find.
# Only st_alloc and st_seal may write cells 0 and 1 of an object.
def check_object_header_writes(r):
    allow = {"st_alloc", "st_alloc_p", "st_seal", "st_sb_write", "st_sb_write_m",
             "st_sb_write_ch", "st_commit_m", "st_commit_p", "st_commit_2pc_go",
             "st_init_p", "st_compact", "st_cobj", "st_publish"}
    bad = []
    for p in bp_sources():
        fn, params = None, set()
        for i, line in enumerate(open(p, errors="replace"), 1):
            m = re.match(r"^fn\s+(\w+)\s*\(([^)]*)\)", line)
            if m:
                fn = m.group(1)
                # only PARAMETERS can be an object offset handed in from an allocator;
                # a local `let off = pt_off + 18 + q * 3` is a payload cursor, not a header.
                params = {q.split(":")[0].strip() for q in m.group(2).split(",") if ":" in q}
                params = {q for q in params if q == "off" or q.endswith("_off") or q == "obj"}
            if fn in allow or not params: continue
            for q in params:
                if re.search(r"\b\w*base\w*\[\s*" + re.escape(q) + r"\s*(\+\s*[01]\s*)?\]\s*=", line) \
                   or re.search(r"\b(new|old|b2)\[\s*" + re.escape(q) + r"\s*(\+\s*[01]\s*)?\]\s*=", line):
                    bad.append("%s:%d fn %s writes header cell of param `%s`" %
                               (os.path.relpath(p, ROOT), i, fn, q))
    if bad:
        fail("object-header", "only st_alloc/st_seal may write cells 0 and 1 of an object: "
             + "; ".join(sorted(set(bad))[:8]))
    else:
        note("object-header: no writer touches cells 0/1 of an allocated object")

# --- CHECK 8: a gate expectation must be a DERIVATION, not a magic number ---------
# Incidents 2026-09-12 (4383f77, b8c6887, e0e6384): schain, sevolve and scompact each hid a
# layout assumption inside a bare integer -- `live == 4028`, `== 10060`, `== 5008`. When B5
# step 1 changed the layout, nothing could tell those numbers apart from a fold value, and
# each one was found separately, by hand, hours apart. Written as `2000 * 4 + 2003 + 5 + 21`
# the same constant states what it is made of and the next layout change can be checked
# against it.
def check_gate_constants_are_derived(r):
    cap = r.get("magic_literal_min", 1000)
    bad = []
    for p in sorted(glob_bp(os.path.join(ROOT, "bench", "vs_rust", "std_tests"))):
        rel = os.path.relpath(p, ROOT)
        if ("magic_exempt:" + rel) in r: continue
        for i, line in enumerate(open(p, errors="replace"), 1):
            if line.lstrip().startswith("//"): continue
            # Only comparisons that carry a LAYOUT assumption. A gate comparing against its
            # expected FOLD is normal and must not be flagged; a gate comparing a cell
            # count, a cursor or a superblock field is asserting the file format.
            if re.search(r"\bcrc\w*\(", line): continue        # a crc is a fold, not a cell count
            if not re.search(r"\b(live|used|sup|superseded|freed|sb\s*\+)\b", line): continue
            for mm in re.finditer(r"==\s*(\d{4,})\b", line):
                v = int(mm.group(1))
                if v < cap: continue
                tail = line[mm.end():mm.end() + 40]
                if re.match(r"\s*[-+*/]", tail): continue      # already a derivation
                bad.append("%s:%d compares against bare %d" % (rel, i, v))
    if bad:
        fail("magic-constant", "a gate expectation hides a layout assumption in a bare number; "
             "write it as the arithmetic it is: " + "; ".join(bad[:8]))
    else:
        note("magic-constant: gate expectations are written as derivations")

def glob_bp(d):
    import glob as _g
    return _g.glob(os.path.join(d, "*.bp"))

# --- CHECK 9: a document may not cite a file that does not exist --------------------
# Incident (HISTORY.md:1900, found in the 2026-09-12 full-record analysis): a task cited a
# precedent file "BUG-LEDGER-WEEK" that was never in the tree, and the citation stood as
# evidence. Same class as the roadmap row that cited commit c2f943e, which is not a valid
# object. A claim that points at nothing is worse than no claim, because it reads as proof.
def check_cited_files_exist(r):
    import glob as _g
    docs = [os.path.join(ROOT, f) for f in ("ROADMAP.md", "TASKS.md", "AGENTS.md")]
    docs += _g.glob(os.path.join(ROOT, "docs", "*.md"))
    pat = re.compile(r"\b((?:bench|tools|selfhost|docs|formal|seed)/[A-Za-z0-9_./-]+\.(?:sh|py|bp|md|lean|txt))")
    missing = {}
    for d in docs:
        if not os.path.exists(d): continue
        for i, line in enumerate(open(d, errors="replace"), 1):
            for m in pat.finditer(line):
                rel = m.group(1)
                if os.path.exists(os.path.join(ROOT, rel)): continue
                missing.setdefault(rel, []).append("%s:%d" % (os.path.basename(d), i))
    worst = r.get("max_missing_citations", 0)
    if len(missing) > worst:
        fail("cited-file", "%d cited file(s) do not exist (ratchet %d) -- a citation that points "
             "at nothing reads as evidence: %s" % (len(missing), worst,
             "; ".join("%s (%s)" % (k, v[0]) for k, v in sorted(missing.items())[:6])))
    else:
        note("cited-file: %d missing citations (ratchet %d)" % (len(missing), worst))

# --- CHECK 10: every builtin the compiler emits is exercised somewhere -------------
# codegen/miscompile is the largest defect class in this project (69 journal entries, 38 %
# of HISTORY's defects) and the only defence against it is differential testing. A builtin
# that no corpus program calls has never been compared against tools/bpref.py or run at all,
# so a miscompile in it is invisible by construction. This check names those.
def check_builtin_coverage(r):
    import glob as _g
    src = open(os.path.join(ROOT, "bebop.bp"), errors="replace").read()
    emitted = set(re.findall(r"^fn emit_(sys_\w+)\(", src, re.M))
    corpus = []
    for d in ("bench/parity_constructs", "bench/vs_rust/std_tests", "selfhost/std",
              "selfhost/prelude", "bench/tq_sqlite"):
        corpus += _g.glob(os.path.join(ROOT, d, "*.bp"))
        corpus += _g.glob(os.path.join(ROOT, d, "**", "*.bp"), recursive=True)
    text = "".join(open(f, errors="replace").read() for f in set(corpus))
    # the surface name may drop the sys_ prefix: emit_sys_scan dispatches `scan(...)`.
    # Accept either spelling before calling a builtin uncovered.
    def called(b):
        return (b + "(") in text or (b[4:] + "(") in text
    uncovered = sorted(b for b in emitted if not called(b))
    worst = r.get("max_uncovered_builtins", 0)
    if len(uncovered) > worst:
        fail("builtin-coverage", "%d builtin(s) the compiler emits are called by NO corpus "
             "program, so no differential test can ever see a miscompile in them (ratchet %d): %s"
             % (len(uncovered), worst, ", ".join(uncovered[:10])))
    else:
        note("builtin-coverage: %d emitted builtins, %d uncovered (ratchet %d)"
             % (len(emitted), len(uncovered), worst))

# --- CHECK 11: every compile-time trap is documented and has a negative fixture ----
# docs/TRAPS.md opens by saying "a code that is not here is a bug" and then, by its own
# admission, the compiler emits codes the table does not carry (105, 106, 107). An
# undocumented trap is a failure whose meaning has to be reverse-engineered at 2am.
def check_traps_documented(r):
    src = open(os.path.join(ROOT, "bebop.bp"), errors="replace").read()
    codes = set(int(c) for c in re.findall(r"diag_exit\(\s*\w+\s*,\s*\w+\s*,\s*(\d+)\s*\)", src))
    traps = open(os.path.join(ROOT, "docs", "TRAPS.md"), errors="replace").read()
    documented = set(int(c) for c in re.findall(r"^\|\s*(\d+)\s*\|", traps, re.M))
    undoc = sorted(codes - documented)
    worst = r.get("max_undocumented_traps", 0)
    if len(undoc) > worst:
        fail("trap-doc", "compiler emits trap code(s) that docs/TRAPS.md does not carry "
             "(ratchet %d): %s" % (worst, ", ".join(map(str, undoc))))
    else:
        note("trap-doc: %d compile-time trap codes, all documented" % len(codes))

# --- CHECK 12: no `zeros()` inside a loop body (law L8) -----------------------------
# AGENTS.md law L8, prose until now: an allocation inside a loop body grows the arena
# monotonically and the failure lands as trap 80 far from the cause. This is the
# memory/bounds class (25 journal entries) and it is purely syntactic, so it should never
# have been a matter of remembering.
def check_no_alloc_in_loop(r):
    bad = []
    for p in bp_sources():
        depth, loop_at = 0, []
        for i, line in enumerate(open(p, errors="replace"), 1):
            code = line.split("//")[0]
            if re.search(r"\bwhile\b.*\{", code): loop_at.append(depth)
            opens, closes = code.count("{"), code.count("}")
            if loop_at and re.search(r"\bzeros\s*\(", code):
                bad.append("%s:%d" % (os.path.relpath(p, ROOT), i))
            depth += opens - closes
            while loop_at and depth <= loop_at[-1]: loop_at.pop()
    worst = r.get("max_alloc_in_loop", 0)
    if len(bad) > worst:
        fail("alloc-in-loop", "%d `zeros()` inside a loop body (law L8, ratchet %d) -- the arena "
             "grows monotonically and trap 80 lands far from the cause: %s"
             % (len(bad), worst, ", ".join(bad[:8])))
    else:
        note("alloc-in-loop: %d zeros() inside a loop body (ratchet %d)" % (len(bad), worst))

# --- CHECK 13: a gate line is accepted only with a committed oracle (law L17) -------
# Ranked the single highest-value check by the 2026-09-12 full-history analysis: it would
# have caught four separate defects where a row was called DONE and the gate behind it had
# no independent check at all. A gate whose expected value has no oracle is a number that
# agrees with itself.
def check_gate_has_oracle(r):
    g = os.path.join(ROOT, "bench", "vs_rust", "std_golden.sh")
    if not os.path.exists(g): return
    names = re.findall(r"^\s*gate\s+([A-Za-z0-9_]+)\s", open(g, errors="replace").read(), re.M)
    missing = [n for n in sorted(set(names))
               if not os.path.exists(os.path.join(ROOT, "bench", "oracles", n + ".py"))]
    worst = r.get("max_gates_without_oracle", 0)
    if len(missing) > worst:
        fail("gate-oracle", "%d gate(s) have no bench/oracles/<name>.py (ratchet %d) -- a gate "
             "without an oracle is a number that agrees with itself: %s"
             % (len(missing), worst, ", ".join(missing[:10])))
    else:
        note("gate-oracle: %d gates, %d without an oracle (ratchet %d)"
             % (len(set(names)), len(missing), worst))

# --- CHECK 14: every prose law names its enforcement --------------------------------
# Operator rule, 2026-09-12: for every line of prose in the rules there must be a concrete
# safeguard in the code. This is the check that keeps that true: a law added to AGENTS.md
# without an entry in tools/law_manifest.txt fails the battery, so prose cannot outrun
# enforcement. `MANUAL:` is allowed and must carry a reason.
def check_laws_have_enforcement(r):
    a = open(os.path.join(ROOT, "AGENTS.md"), errors="replace").read()
    laws = set(re.findall(r"^(L\d+)\.", a, re.M))
    man = os.path.join(ROOT, "tools", "law_manifest.txt")
    if not os.path.exists(man): return fail("law-manifest", "tools/law_manifest.txt is missing")
    listed, unjustified = set(), []
    for line in open(man, errors="replace"):
        if line.startswith("#") or "|" not in line: continue
        law = line.split("|")[0].strip()
        listed.add(law)
        body = line.split("|", 1)[1]
        if "MANUAL:" in body and len(body.split("MANUAL:")[1].strip()) < 25:
            unjustified.append(law)
    orphans = sorted(laws - listed)
    if orphans:
        fail("law-manifest", "law(s) in AGENTS.md with no enforcement named in "
             "tools/law_manifest.txt: " + ", ".join(orphans))
    if unjustified:
        fail("law-manifest", "law(s) marked MANUAL without a real reason: " + ", ".join(unjustified))
    named = set(re.findall(r"arch_check:([a-z-]+)", open(man, errors="replace").read()))
    have = set(re.findall(r'fail\("([a-z-]+)"', open(os.path.join(ROOT, "tools", "arch_check.py"),
                                                     errors="replace").read()))
    have |= set(re.findall(r'note\("([a-z-]+):', open(os.path.join(ROOT, "tools", "arch_check.py"),
                                                      errors="replace").read()))
    ghosts = sorted(named - have)
    if ghosts:
        fail("law-manifest", "the manifest names check(s) that do not exist in arch_check.py -- "
             "prose pointing at an imaginary safeguard is worse than prose alone: " + ", ".join(ghosts))
    if not orphans and not unjustified and not ghosts:
        note("law-manifest: %d laws, all with a named enforcement" % len(laws))

# --- CHECK 15: every syscall emitter carries its register table (law L2) ------------
# A syscall emitter writes raw words; the register contract is the only thing that makes
# them reviewable. L2 has required the table since the beginning and nothing checked it.
def check_syscall_comments(r):
    src = open(os.path.join(ROOT, "bebop.bp"), errors="replace").read().split("\n")
    bad = []
    for i, line in enumerate(src):
        m = re.match(r"^fn (emit_sys_\w+)\(", line)
        if not m: continue
        window = "\n".join(src[max(0, i - 12):i])
        if not re.search(r"\bx[0-9]+\b", window):
            bad.append("%s (bebop.bp:%d)" % (m.group(1), i + 1))
    worst = r.get("max_syscall_no_regtable", 0)
    if len(bad) > worst:
        fail("syscall-comment", "%d syscall emitter(s) with no register table in the 12 lines "
             "above them (ratchet %d): %s" % (len(bad), worst, ", ".join(bad[:8])))
    else:
        note("syscall-comment: every emit_sys_* carries its register table")

# --- CHECK 16: journal entries carry all four fields (laws L10, L20) ----------------
# A journal line without GOT: is an opinion; without VERDICT: it is an unfinished thought.
# The journal is this project's only durable record of what was MEASURED, so its shape is
# load-bearing.
def check_journal_format(r):
    p = os.path.join(ROOT, "docs", "exp.journal")
    if not os.path.exists(p): return
    # COST:<seconds> is the time from FIRST SYMPTOM to NAMED CAUSE, not the time to fix.
    # Operator rule, 2026-09-12: the claim that loud failures speed development up must be
    # measurable rather than asserted, and this is the number that measures it. Required on
    # every entry written from COST_SINCE onward; older entries predate the field.
    cost_since = r.get("journal_cost_since", 0)
    bad, nocost = [], []
    for i, line in enumerate(open(p, errors="replace"), 1):
        if not line.strip(): continue
        if not all(f in line for f in ("H:", "DID:", "GOT:", "VERDICT:")):
            bad.append(str(i))
        m = re.match(r"\s*(\d{10})\s", line)
        if m and cost_since and int(m.group(1)) >= cost_since and "COST:" not in line:
            nocost.append(str(i))
    if nocost:
        fail("journal-cost", "%d journal entr(ies) written after the cost field was introduced "
             "carry no COST:<seconds-from-symptom-to-cause>, so the speed-up claim stays an "
             "opinion: lines %s" % (len(nocost), ",".join(nocost[:10])))
    worst = r.get("max_malformed_journal", 0)
    if len(bad) > worst:
        fail("journal-format", "%d journal entries missing one of H:/DID:/GOT:/VERDICT: "
             "(ratchet %d), lines %s" % (len(bad), worst, ",".join(bad[:10])))
    else:
        note("journal-format: %d entries, %d missing a field (ratchet %d), %d written since the "
             "cost field and all carrying COST:"
             % (sum(1 for _ in open(p, errors="replace") if _.strip()), len(bad), worst,
                sum(1 for l in open(p, errors="replace")
                    for m in [re.match(r"\s*(\d{10})\s", l)] if m and cost_since and int(m.group(1)) >= cost_since)))

# --- CHECK 17: a scripted source patch asserts its anchor is unique (law L3) --------
# Every edit this project makes to a big source file is a scripted string replacement. An
# anchor that matches twice silently edits the wrong place; an anchor that matches zero
# times silently edits nothing and the commit claims a change it did not make.
def check_scripted_patch_assert(r):
    import glob as _g
    bad = []
    for p in sorted(_g.glob(os.path.join(ROOT, "tools", "*.py"))):
        txt = open(p, errors="replace").read()
        if ".replace(" not in txt: continue
        if "count(" not in txt and "assert" not in txt:
            bad.append(os.path.relpath(p, ROOT))
    worst = r.get("max_unasserted_patchers", 0)
    if len(bad) > worst:
        fail("scripted-patch-assert", "%d tool(s) rewrite source with .replace() and never "
             "assert the anchor is unique (ratchet %d): %s" % (len(bad), worst, ", ".join(bad[:8])))
    else:
        note("scripted-patch-assert: every source-rewriting tool asserts its anchor")

# --- CHECK 18: no function is defined and never called ------------------------------
# Operator rule, 2026-09-12: "code that never executes is a huge cause of bugs". It is, and
# for a specific reason: an uncalled function is never compiled through the real path, never
# differentially tested, and never trapped -- so it rots silently and then someone wires it
# up and inherits every defect it accumulated. The 2026-09-12 history analysis found an
# audit reporting 50 of 116 modules unused, with fp_mul defined 14 times over.
def check_no_dead_functions(r):
    # Read every source ONCE. The first version re-read the whole corpus per name, which is
    # quadratic: it took over five minutes and was therefore a check nobody could afford to
    # run. A safeguard that is too slow to run is not a safeguard.
    # DEFINITIONS are only counted for the library and compiler surface: a gate program or
    # a parity construct may legitimately define helpers it does not call (c66_fncap.bp
    # defines a hundred functions on purpose, to test the fn cap). USES are counted over
    # the whole corpus, because a library function called only from a gate is alive.
    owned = ("bebop.bp", "selfhost/prelude/", "selfhost/std/", "selfhost/tools/")
    texts, defs = [], {}
    for p in bp_sources():
        rel = os.path.relpath(p, ROOT)
        txt = open(p, errors="replace").read()
        texts.append(txt)
        if not any(rel == o or rel.startswith(o) for o in owned): continue
        for i, line in enumerate(txt.split("\n"), 1):
            m = re.match(r"^fn\s+(\w+)\s*\(", line)
            if m: defs.setdefault(m.group(1), []).append("%s:%d" % (rel, i))
    uses = collections.Counter(re.findall(r"\b(\w+)\s*\(", "\n".join(texts)))
    entry = {"main", "kernel_main"}
    # `fn foo(` also matches the call pattern, so a function is dead when its only
    # occurrences in the whole corpus are its own definitions.
    dead = sorted("%s (%s)" % (n, w[0]) for n, w in defs.items()
                  if n not in entry and uses[n] <= len(w))
    worst = r.get("max_dead_functions", 0)
    if len(dead) > worst:
        fail("dead-function", "%d function(s) defined and never called (ratchet %d) -- uncalled "
             "code is never differentially tested and rots until someone wires it up: %s"
             % (len(dead), worst, ", ".join(dead[:10])))
    else:
        note("dead-function: %d defined, %d never called (ratchet %d)"
             % (len(defs), len(dead), worst))


def main():
    r = load_ratchet()
    check_no_nested_fn()
    check_file_size(r)
    check_binary_matches_source()
    check_gates_clean_their_stores()
    check_producers_not_memoised()
    check_loud_failures(r)
    check_object_header_writes(r)
    check_gate_constants_are_derived(r)
    check_cited_files_exist(r)
    check_builtin_coverage(r)
    check_traps_documented(r)
    check_no_alloc_in_loop(r)
    check_gate_has_oracle(r)
    check_syscall_comments(r)
    check_journal_format(r)
    check_scripted_patch_assert(r)
    check_no_dead_functions(r)
    check_laws_have_enforcement(r)
    for n in notes: print("  note: " + n)
    if fails:
        print("\narch_check: %d INVARIANT(S) VIOLATED" % len(fails))
        for f in fails: print("  FAIL " + f)
        return 1
    print("\narch_check: all invariants hold")
    return 0

if __name__ == "__main__":
    sys.exit(main())
