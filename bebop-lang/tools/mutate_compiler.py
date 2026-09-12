#!/usr/bin/env python3
"""Mutation coverage for the COMPILER, not for the gates.

codegen/miscompile is the largest defect class in this project -- 69 journal entries, 38 %
of HISTORY's defects -- and no static check can prove an emitter emits the right words. The
only thing that measures it is this question, asked one emitter at a time:

    if I corrupt the word this emitter emits, does ANY gate go red?

If nothing goes red, that emitter's output is not covered by the test suite, and a real
miscompile in it would be invisible in exactly the same way.

`tools/mutate_gate.sh` is the INVERSE of this tool: it mutates a gate's own program to ask
whether the gate is sensitive to its own arithmetic. This one mutates the compiler.

Usage:  python3 tools/mutate_compiler.py [--limit N] [--emitter NAME] [--out FILE]
Results accumulate in tools/mutation_coverage.txt so a sweep can be run in batches.
"""
import argparse, os, re, subprocess, sys, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SEED = os.path.join(ROOT, "seed", "build", "seed")
SRC = os.path.join(ROOT, "bebop.bp")
OUTDB = os.path.join(ROOT, "tools", "mutation_coverage.txt")
TMP = "/tmp/mutc"
THREAD_BASE = None

def sites():
    """One mutable literal word per emitter: the first `em(insns, n, <literal>)` in each."""
    out, fn = [], None
    for i, line in enumerate(open(SRC, errors="replace").read().split("\n"), 1):
        m = re.match(r"^fn (emit_\w+)\(", line)
        if m: fn = m.group(1); continue
        if fn is None: continue
        mm = re.search(r"\bem\(insns, n, (\d{4,})\)", line)
        if mm and not any(s[0] == fn for s in out):
            out.append((fn, i, int(mm.group(1))))
    return out

def build(text, out):
    p = os.path.join(TMP, "mut.bp")
    open(p, "w").write(text)
    r = subprocess.run([SEED, os.path.join(ROOT, "bebop.bin"), "compile", p, out],
                       capture_output=True, cwd=ROOT)
    return r.returncode == 0 and os.path.exists(out) and os.path.getsize(out) > 0

def threads(binpath):
    """A gate that actually SPAWNS: construct_parity does not, so every concurrency emitter
    survived its mutants -- cond_set, futex_wake, futex_wait_guard, atomic_add,
    exit_thread_guard were all invisible to it. smw exercises all of them in one run: three
    writer threads, real futex handshakes, per-partition atomics. Its fold is P*N + P*(N/100)
    and any corruption of those primitives moves it or hangs it."""
    out = os.path.join(TMP, "smw_mut.bin")
    src = os.path.join(ROOT, "bench", "vs_rust", "std_tests", "smw.bp")
    for f in ("smw.store",):
        try: os.remove(os.path.join(ROOT, f))
        except OSError: pass
    r = subprocess.run([SEED, binpath, "compile", src, out], capture_output=True, cwd=ROOT)
    if r.returncode != 0 or not os.path.exists(out): return "compile-failed"
    try:
        rr = subprocess.run([SEED, out, "2", "200"], capture_output=True, text=True,
                            cwd=ROOT, timeout=90)
    except subprocess.TimeoutExpired:
        return "hang"
    return (rr.stdout.strip().split("\n") or [""])[-1]

def parity(binpath):
    """Return the gate pass count, or -1 if the harness could not run at all."""
    env = dict(os.environ, BEBOP_BIN=binpath, BEBOP_TMP=TMP, NO_SLOT="1")
    try:
        r = subprocess.run(["bash", "bench/vs_rust/construct_parity.sh"], capture_output=True,
                           text=True, cwd=ROOT, env=env, timeout=420)
    except subprocess.TimeoutExpired:
        return -2                                  # a hang is a kill: the mutant broke something
    m = re.search(r"construct parity: pass=(\d+)", r.stdout)
    return int(m.group(1)) if m else -1

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=5)
    ap.add_argument("--emitter")
    ap.add_argument("--baseline", type=int, default=0)
    a = ap.parse_args()
    os.makedirs(TMP, exist_ok=True)
    done = set()
    if os.path.exists(OUTDB):
        for l in open(OUTDB):
            if l.strip() and not l.startswith("#"): done.add(l.split()[0])
    base_text = open(SRC, errors="replace").read()
    global THREAD_BASE
    THREAD_BASE = threads(os.path.join(ROOT, "bebop.bin"))
    print("thread-gate baseline: %s" % THREAD_BASE)
    base = a.baseline
    if not base:
        assert build(base_text, os.path.join(TMP, "base.bin")), "the UNMUTATED source must build"
        base = parity(os.path.join(TMP, "base.bin"))
        print("baseline pass=%d" % base)
    todo = [s for s in sites() if s[0] not in done and (not a.emitter or s[0] == a.emitter)]
    lines = base_text.split("\n")
    with open(OUTDB, "a") as db:
        if os.path.getsize(OUTDB) == 0 if os.path.exists(OUTDB) else True:
            db.write("# emitter  verdict  detail   -- KILLED = some gate noticed; SURVIVED = nothing did\n")
        for fn, ln, word in todo[:a.limit]:
            t0 = time.time()
            mut = list(lines)
            # flip the low bit of the emitted word: still a word, different instruction
            mut[ln - 1] = mut[ln - 1].replace("em(insns, n, %d)" % word,
                                              "em(insns, n, %d)" % (word ^ 1), 1)
            ok = build("\n".join(mut), os.path.join(TMP, "mut.bin"))
            if not ok:
                verdict, detail = "KILLED", "self-compile-failed"
            else:
                p = parity(os.path.join(TMP, "mut.bin"))
                if p == -2:   verdict, detail = "KILLED", "hang"
                elif p == -1: verdict, detail = "KILLED", "harness-failed"
                elif p < base: verdict, detail = "KILLED", "pass=%d<%d" % (p, base)
                else:
                    # construct_parity does not spawn, so a concurrency emitter needs a gate
                    # that does before its mutant can be called dead.
                    t = threads(os.path.join(TMP, "mut.bin"))
                    if t != THREAD_BASE:
                        verdict, detail = "KILLED", "threads=%s!=%s" % (t[:14], THREAD_BASE)
                    else:
                        verdict, detail = "SURVIVED", "pass=%d, threads ok" % p
            db.write("%s %s %s %.1fs\n" % (fn, verdict, detail, time.time() - t0))
            db.flush()
            print("%-28s %-9s %-22s %.1fs" % (fn, verdict, detail, time.time() - t0))
    return 0

if __name__ == "__main__":
    sys.exit(main())
