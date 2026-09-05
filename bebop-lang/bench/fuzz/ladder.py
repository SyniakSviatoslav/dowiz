#!/usr/bin/env python3
"""ladder.py — the cut-and-return ladder as a tool (roadmap 1c, 2026-09-06).

  ladder.py repro.bp [--out min.bp] [--name NAME] [--jobs 2] [--budget 300]

The T77 ddmin (shrink.py) cut LINES and TOKENS: most cuts of a gen.py program broke a loop
update or a brace, every broken candidate cost a 40 s oracle timeout, and 5 KB took 90 min
without a result. The hand ladder that found 20056 and 42122 in 4-6 probes cuts STATEMENTS:
  1. the tail expression -> each symbol it mentions (which value diverges);
  2. drop whole top-level statements of every fn (balanced blocks as one unit), ddmin
     over statements, every candidate of a level classified IN PARALLEL (--jobs);
  3. the same inside every surviving block (while / if bodies), deeper each round;
  4. drop fns nobody calls;
  5. shrink.py's token pass (paren / call / ident / literal cuts), biggest cut first, for
     the last bytes under the time budget.
Before 2 every unit is HOISTED first: `let x = 0;` (arrays keep their length) per symbol
it binds -- lets leak out of blocks, so deleting a block breaks every later use, and a
9-symbol caller (42122) must keep its symbol count while losing the code. A candidate is
compiled before the oracle runs (a broken cut is 0.1 s, not a 5 s bpref); the deadline
applies to every rung.
The oracle budget is 5 s and the run budget 2 s (a cut that makes a loop endless is
just "not the same category"), so a failed candidate is cheap. Same classifier, same
category rule (shrink.same) and the same output as shrink.py (MIN header, journal stub,
construct-guard stub). env: BEBOP_BIN, BEBOP_TMP.
"""
import argparse
import multiprocessing as mp
import os
import re
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
os.environ.setdefault('RUN_T', '2')
import shrink        # noqa: E402
import fuzz_batch    # noqa: E402

fuzz_batch.ORACLE_T = float(os.environ.get('BPREF_T', '5'))
shrink.oracle = fuzz_batch.forked_bpref   # in-process oracle, 5 s budget
WD = os.path.join(os.environ.get('BEBOP_TMP', '/tmp/opencode/agentB-fuzz'), 'ladder.%d' % os.getpid())
POOL = None
WANT = None
TESTS = 0
DEADLINE = float('inf')


def same(a, b):
    """shrink.same, and an error keeps its kind: `bpref error: RuntimeError: use after loop
    release` must not shrink into a SyntaxError."""
    kind = lambda c: ':'.join(c[1].split(':')[1:3]) if c[0] in ('BPREF-ERROR', 'COMPILEFAIL') else ''
    return shrink.same(a, b) and kind(a) == kind(b)


def _test(src):  # worker: same category as WANT?
    wd = os.path.join(WD, 'w%d' % os.getpid()); os.makedirs(wd, exist_ok=True)
    if WANT[0] != 'COMPILEFAIL':  # compile first: a broken cut must not pay the oracle
        bp = os.path.join(wd, 'pre.bp'); open(bp, 'w').write(src)
        if shrink.run([shrink.SEED, shrink.BIN, 'compile', bp, bp[:-3] + '.bin'], 20, wd)[0] != 0:
            return False
    return same(shrink.classify(src, wd=wd), WANT)


def first_ok(cands):
    """The first candidate (in order) with the same category, testing --jobs at a time."""
    global TESTS
    cands = list(cands)
    for i in range(0, len(cands), POOL._processes):
        if time.time() > DEADLINE:
            return None
        chunk = cands[i:i + POOL._processes]
        TESTS += len(chunk)
        for c, r in zip(chunk, POOL.map(_test, chunk)):
            if r:
                return c
    return None


def results(cands):
    global TESTS
    cands = list(cands)
    if time.time() > DEADLINE:
        return [False] * len(cands)
    TESTS += len(cands)
    return POOL.map(_test, cands)


# ---- statement structure ---------------------------------------------------
def depth_delta(line):
    return line.count('{') - line.count('}')


def units(lines):
    """Split lines into statement units at the current depth: a unit is one line, or a
    balanced block (a line opening more braces than it closes up to the line that
    closes it)."""
    out, i = [], 0
    while i < len(lines):
        d = depth_delta(lines[i]); j = i
        while d > 0 and j + 1 < len(lines):
            j += 1; d += depth_delta(lines[j])
        out.append(lines[i:j + 1]); i = j + 1
    return out


def ddmin_units(us, build):
    """Minimal subsequence of units (each a list of lines) keeping WANT; build(kept) -> src.
    Every candidate of a level is classified in parallel; removals that pass are merged."""
    n = 2
    while len(us) >= 2:
        chunk = max(1, -(-len(us) // n))
        starts = list(range(0, len(us), chunk))
        cands = [us[:i] + us[i + chunk:] for i in starts]
        hits = [i for i, r in zip(starts, results(build(c) for c in cands)) if r]
        if hits:
            removed = set(k for i in hits for k in range(i, i + chunk))
            merged = [u for k, u in enumerate(us) if k not in removed]
            if len(hits) > 1 and _test_main(build(merged)):
                us = merged
            else:
                us = cands[starts.index(hits[0])]
            n = max(n - 1, 2)
        elif chunk == 1:
            break
        else:
            n = min(n * 2, len(us))
    return us


def _test_main(src):
    global TESTS
    TESTS += 1
    return POOL.map(_test, [src])[0]


def split_fns(src):
    """[(header_line_idx, body_lines, footer_line)] for every `fn ... {` at column 0."""
    lines = src.split('\n'); fns = []; i = 0
    while i < len(lines):
        if lines[i].startswith('fn ') and lines[i].rstrip().endswith('{'):
            d = 1; j = i
            while d > 0 and j + 1 < len(lines):
                j += 1; d += depth_delta(lines[j])
            fns.append((i, j))
            i = j + 1
        else:
            i += 1
    return lines, fns


def rebuild(lines, fns, bodies):
    out = []; last = 0
    for (i, j), body in zip(fns, bodies):
        out += lines[last:i + 1] + body + [lines[j]]; last = j + 1
    return '\n'.join(out + lines[last:])


def hoist(u):
    """The unit replaced by `let x = 0;` per symbol it binds (lets leak out of blocks in
    bebop, so a deleted block breaks every later use; array lets keep their length for
    the `& 7` index masks): every prefix of that list, shortest first, because a slot
    bug (42122: the 9th symbol) needs the symbol COUNT, not the names. [] when the unit
    binds nothing or is already that."""
    out, seen = [], set()  # `let _` takes a slot, a re-declared name does not (probed on old.bin)
    for line in u:
        for m in re.finditer(r'\blet\s+(\w+)\s*=\s*(\[?)|\b(?:some|many|none)\((\w+)\)', line):
            name = m.group(3) or m.group(1)  # a match-arm binder is a symbol too (the 9-symbol caller of 42122)
            if name in seen and name != '_':
                continue
            seen.add(name)
            if m.group(2):
                j = match_span(line, m.end() - 1, '[', ']')
                n = 1 + sum(1 for k in range(m.end(), j) if line[k] == ',' and depth_at(line, m.end(), k) == 0)
                out.append('  let %s = [%s];' % (name, ', '.join(['0'] * n)))
            else:
                out.append('  let %s = 0;' % name)
    return [] if not out or out == [l.rstrip() for l in u] else [out[:n] for n in range(1, len(out) + 1)]


def match_span(s, i, o, c):
    d = 0
    for j in range(i, len(s)):
        d += (s[j] == o) - (s[j] == c)
        if d == 0:
            return j
    return len(s)


def depth_at(s, i, k):
    return sum((ch in '([{') - (ch in ')]}') for ch in s[i:k])


def hoist_units(us, build):
    """Every unit's hoisted form tested in parallel; the accepted ones merged (verified)."""
    idx = [(i, h) for i, hs in enumerate(hoist(u) for u in us) for h in hs]
    if not idx:
        return us
    rs = results(build(us[:i] + [h] + us[i + 1:]) for i, h in idx)
    hs = {}
    for (i, h), r in zip(idx, rs):
        if r:
            hs.setdefault(i, h)  # the shortest accepted prefix per unit
    hits = sorted(hs)
    if not hits:
        return us
    merged = [hs[i] if i in hits else u for i, u in enumerate(us)]
    if len(hits) == 1 or _test_main(build(merged)):
        return merged
    us = list(us)  # the hits interact: take them one at a time, each verified
    for i in hits:
        cand = us[:i] + [hs[i]] + us[i + 1:]
        if _test_main(build(cand)):
            us = cand
    return us


def cut_blocks(body, build, depth):
    """hoist, ddmin the units of a statement list, then recurse into every surviving block."""
    us = hoist_units(units(body), lambda kept: build(sum(kept, [])))  # kept = a list of units
    us = ddmin_units(us, lambda kept: build(sum(kept, [])))
    for idx in range(len(us)):
        u = us[idx]
        if len(u) > 1 and depth < 6:  # a block: header line, inner lines, closing line
            def b(k, idx=idx):
                return build(sum(us[:idx], []) + [us[idx][0]] + k + [us[idx][-1]] + sum(us[idx + 1:], []))
            us[idx] = [u[0]] + cut_blocks(u[1:-1], b, depth + 1) + [u[-1]]
    return sum(us, [])


def tail_rung(src):
    """main's tail expression -> each symbol it mentions (shortest first)."""
    lines, fns = split_fns(src)
    for (i, j) in fns:
        if lines[i].startswith('fn main'):
            k = j - 1
            while k > i and not lines[k].strip():
                k -= 1
            tail = lines[k]
            if tail.strip().endswith(';'):
                return src
            syms = sorted(set(re.findall(r'\b[A-Za-z_]\w*\b', tail)) - shrink.KEYWORDS, key=len)
            cands = ['\n'.join(lines[:k] + ['  ' + s] + lines[k + 1:]) for s in ['0'] + syms if s != tail.strip()]
            c = first_ok(cands)
            return c if c else src
    return src


def drop_dead_fns(src):
    lines, fns = split_fns(src)
    changed = True
    while changed:
        changed = False
        for (i, j) in fns:
            name = re.match(r'fn (\w+)', lines[i]).group(1)
            if name == 'main':
                continue
            rest = '\n'.join(lines[:i] + lines[j + 1:])
            if not re.search(r'\b%s\s*\(' % name, rest):
                src = rest; lines, fns = split_fns(src); changed = True; break
    return src


def token_rung(src, deadline):
    k = 0; cs = sorted(set(shrink.cands(src)), key=len)  # the biggest cut first (creduce order)
    while k < len(cs) and time.time() < deadline:
        chunk = [c for c in cs[k:k + POOL._processes] if len(c) < len(src)]
        hit = first_ok(chunk) if chunk else None
        if hit is not None:  # regenerate, resume near the same position (the order is by size)
            src = hit; cs = sorted(set(shrink.cands(src)), key=len); k = max(0, k - POOL._processes)
        else:
            k += POOL._processes
    return src


def ladder(src, deadline):
    global TESTS, DEADLINE
    DEADLINE = deadline
    src = tail_rung(src)
    print('rung 1 tail: %d bytes (%d tests)' % (len(src), TESTS), file=sys.stderr)
    for rnd in range(4):
        before = src
        lines, fns = split_fns(src)
        bodies = [lines[i + 1:j] for (i, j) in fns]
        for f in range(len(fns)):
            def build(body, f=f):
                return rebuild(lines, fns, bodies[:f] + [body] + bodies[f + 1:])
            bodies[f] = cut_blocks(bodies[f], build, 0)
            src = rebuild(lines, fns, bodies)
        src = drop_dead_fns(src)
        print('rung 2-4 round %d: %d bytes (%d tests)' % (rnd + 1, len(src), TESTS), file=sys.stderr)
        if src == before or time.time() > deadline:
            break
    src = token_rung(src, deadline)
    print('rung 5 tokens: %d bytes (%d tests)' % (len(src), TESTS), file=sys.stderr)
    return re.sub(r'\n\s*\n', '\n', src).strip() + '\n'


def main():
    global POOL, WANT
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('repro'); ap.add_argument('--out'); ap.add_argument('--name')
    ap.add_argument('--jobs', type=int, default=2); ap.add_argument('--budget', type=float, default=300)
    opts = vars(ap.parse_args()); args = [opts['repro']]
    jobs = opts['jobs']; budget = opts['budget']
    os.makedirs(WD, exist_ok=True)
    src = open(args[0]).read()
    body = '\n'.join(l for l in src.split('\n') if not l.startswith('//'))
    WANT = shrink.classify(body, wd=WD)
    print('want %s got=%s expected=%s (%d bytes, bin %s)' % (WANT[0], WANT[1], WANT[2], len(body), shrink.BIN), file=sys.stderr)
    if WANT[0] in ('OK', 'BPREF-DEPTH', 'BPREF-TIMEOUT'):
        print('program is %s under %s, nothing to shrink' % (WANT[0], shrink.BIN), file=sys.stderr); sys.exit(1)
    POOL = mp.get_context('fork').Pool(jobs)
    t0 = time.time()
    cur = ladder(body, t0 + budget)
    fin = shrink.classify(cur, wd=WD)
    name = opts.get('name') if isinstance(opts.get('name'), str) else fin[0].lower().replace('-', '')
    out = '// MIN %s expected=%s got=%s (%d -> %d bytes, %d classifications, %.0f s, ladder.py)\n%s' % (
        fin[0], fin[2], fin[1], len(body), len(cur), TESTS, time.time() - t0, cur)
    if isinstance(opts.get('out'), str):
        open(opts['out'], 'w').write(out)
    sys.stdout.write(out)
    print('\n--- journal stub (docs/exp.journal) ---')
    print('$(date +%%s) H:%s of %s is one construct | DID:ladder.py %d -> %d bytes, %d classifications in %.0f s, BEBOP_BIN=%s | GOT:%s expected=%s got=%s | VERDICT:confirmed'
          % (fin[0], os.path.basename(args[0]), len(body), len(cur), TESTS, time.time() - t0, os.path.basename(shrink.BIN), fin[0], fin[2], fin[1]))
    print('--- construct-parity guard stub (bench/parity_constructs/c%02d_%s.bp, EXPECT=%s) ---' % (shrink.next_guard_no(), name, fin[2]))
    print(' '.join(cur.split()))
    POOL.terminate()


if __name__ == '__main__':
    main()
