#!/usr/bin/env python3
"""shrink.py — T77 minimal counterexample shrinker + the T39 fuzz classifier.

  shrink.py --classify prog.bp     classify like fuzz.sh (fuzz.sh calls THIS):
                                   prints "CAT<TAB>expected<TAB>got"
  shrink.py repro.bp [--out min.bp] [--name NAME]
                                   ddmin (lines, then expression-level tokens)
                                   to the smallest program with the same
                                   category (CRASH keeps its signal); prints
                                   the minimal program, an H:|DID:|GOT:|VERDICT:
                                   journal stub and a bench/parity_constructs/
                                   cNN_<name>.bp guard skeleton (stdout only,
                                   nothing under bench/ is written)

Categories: OK DIVERGE COMPILEFAIL CRASH TIMEOUT BPREF-ERROR BPREF-DEPTH.
Oracle = tools/bpref.py; SUT = $BEBOP_BIN (default bebop.bin) via seed.
Every seed/compiled-program run has cwd = the scratch dir, so a stray file
written by a misbehaving binary lands in scratch, never in the repo root.
env: BEBOP_BIN, BEBOP_TMP (scratch root, default /tmp/opencode/agentB-fuzz),
RUN_T (run timeout in seconds, default 5; TIMEOUT class shrinks faster with 1).
"""
import glob
import hashlib
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
SEED = os.path.join(ROOT, 'seed', 'build', 'seed')
BPREF = os.path.join(ROOT, 'tools', 'bpref.py')
BIN = os.path.abspath(os.environ.get('BEBOP_BIN', os.path.join(ROOT, 'bebop.bin')))
RUN_T = float(os.environ.get('RUN_T', '5'))
KEYWORDS = {'fn', 'let', 'in', 'if', 'then', 'else', 'while', 'match', 'enum', 'i64',
            'main', 'none', 'some', 'many', 'opt', 'zeros'}


def run(cmd, t, cwd):
    try:
        p = subprocess.run(cmd, cwd=cwd, capture_output=True, timeout=t)
        return p.returncode, p.stdout.decode(errors='replace'), p.stderr.decode(errors='replace')
    except subprocess.TimeoutExpired:
        return 124, '', ''


def oracle(bp, wd):  # the semantic oracle; fuzz_batch.py rebinds this to an in-process fork of bpref
    return run(['python3', BPREF, bp], 40, wd)  # D11-D widened generator: 40 s oracle budget


def last_line(s):
    s = s.strip()
    return s.split('\n')[-1] if s else ''


def classify(src=None, bp=None, wd=None):
    """-> (category, got, expected). Either src (written to wd/<md5>.bp) or an
    existing bp path; wd defaults to bp's directory."""
    if bp is None:
        wd = wd or os.path.join(os.environ.get('BEBOP_TMP', '/tmp/opencode/agentB-fuzz'), 'shrink.%d' % os.getpid())
        os.makedirs(wd, exist_ok=True)
        bp = os.path.join(wd, hashlib.md5(src.encode()).hexdigest()[:10] + '.bp')
        open(bp, 'w').write(src)
    bp = os.path.abspath(bp)
    wd = wd or os.path.dirname(bp)
    bn = bp[:-3] + '.bin'
    rc, out, err = oracle(bp, wd)
    if rc == 3:
        return 'BPREF-DEPTH', err.strip()[-120:], ''
    if rc == 124:  # the oracle timed out: generator too heavy, not a compiler verdict
        return 'BPREF-TIMEOUT', 'bpref > 20 s', ''
    if rc in (80, 81, 82):  # T118: the oracle predicts a capacity trap
        exp_trap = rc
        bn = bp[:-3] + '.bin'
        rc2, out2, err2 = run([SEED, BIN, 'compile', bp, bn], 20, wd)
        if rc2 != 0 or not os.path.exists(bn):
            return 'COMPILEFAIL', 'compile rc=%d' % rc2, 'rc=%d' % exp_trap
        rc3, out3, err3 = run([SEED, bn], RUN_T, wd)
        if rc3 in (80, 81, 82):  # any capacity trap: bpref models the arena only, order may differ
            return 'TRAP-OK', 'rc=%d' % rc3, 'rc=%d' % exp_trap
        if rc3 == 124:  # the oracle trapped early, bebop is still computing: a heavy generator case
            return 'TRAP-TIMEOUT', 'rc=124', 'rc=%d (trap)' % exp_trap
        return 'DIVERGE', 'rc=%d %s' % (rc3, out3.strip()[-60:]), 'rc=%d (trap)' % exp_trap
    if rc != 0:
        return 'BPREF-ERROR', 'rc=%d %s' % (rc, err.strip()[-120:].replace('\n', ' ')), ''
    exp = last_line(out)
    rc, out, err = run([SEED, BIN, 'compile', bp, bn], 20, wd)
    if rc != 0 or not os.path.exists(bn) or os.path.getsize(bn) == 0:
        return 'COMPILEFAIL', 'compile rc=%d %s' % (rc, (out + err).strip()[-80:].replace('\n', ' ')), exp
    rc, out, err = run([SEED, bn], RUN_T, wd)
    got = last_line(out)
    if rc == 124:
        return 'TIMEOUT', 'timeout %gs' % RUN_T, exp
    if rc in (80, 81, 82):  # T118: a capacity trap the oracle did not predict (frame heap is not modelled)
        return 'TRAP-%d' % rc, 'rc=%d' % rc, exp
    if rc < 0 or rc >= 128:  # a CRASH is re-run 3x before it is believed
        sig = -rc if rc < 0 else rc - 128
        n = sum(1 for _ in range(3) if run([SEED, bn], RUN_T, wd)[0] not in range(0, 124))
        return 'CRASH', 'signal %d (%d/3 reruns crashed)' % (sig, n), exp
    if got == exp:
        return 'OK', got, exp
    return 'DIVERGE', got + (' (exit rc=%d)' % rc if rc else ''), exp


# ---- ddmin ----------------------------------------------------------------
def same(a, b):
    return a[0] == b[0] and (a[0] != 'CRASH' or a[1].split(' (')[0] == b[1].split(' (')[0])


cache = {}


def ok(src, want, wd):
    if src not in cache:
        cache[src] = same(classify(src, wd=wd), want)
    return cache[src]


def lines_pass(src, want, wd):
    lines = src.split('\n')
    n = 2
    while len(lines) >= 2:
        chunk = max(1, len(lines) // n)
        changed = False
        i = 0
        while i < len(lines):
            cand = lines[:i] + lines[i + chunk:]
            if ok('\n'.join(cand), want, wd):
                lines = cand
                changed = True
                n = max(n - 1, 2)
            else:
                i += chunk
        if not changed:
            if chunk == 1:
                break
            n = min(n * 2, len(lines))
    return '\n'.join(lines)


def match_paren(s, i, o='(', c=')'):
    d = 0
    for j in range(i, len(s)):
        if s[j] == o:
            d += 1
        elif s[j] == c:
            d -= 1
            if d == 0:
                return j
    return -1


def cands(src):
    """Smaller candidate programs, most aggressive first."""
    for m in re.finditer(r'\(', src):  # (E) -> 0 / 1 / E
        i = m.start()
        j = match_paren(src, i)
        if j >= 0:
            for rep in ('0', '1', src[i + 1:j]):
                yield src[:i] + rep + src[j + 1:]
    for m in re.finditer(r'\b[A-Za-z_]\w*\s*[\[(]', src):  # a[..] / f(..) -> 0
        i = m.end() - 1
        j = match_paren(src, i, src[i], ']' if src[i] == '[' else ')')
        if j < 0:
            continue
        rest = src[j + 1:j + 4].strip()
        if src[i] == '[' and rest.startswith('=') and not rest.startswith('=='):
            continue  # array store: handled by line deletion
        yield src[:m.start()] + '0' + src[j + 1:]
    for m in re.finditer(r'(?<![\w\])])\[', src):  # an array literal -> [0] (ladder.py, 2026-09-06)
        j = match_paren(src, m.start(), '[', ']')
        if j > m.start() + 2:
            yield src[:m.start()] + '[0]' + src[j + 1:]
    for m in re.finditer(r'\bmatch\b', src):  # match -> each arm body
        i = src.find('{', m.start())
        j = match_paren(src, i, '{', '}') if i >= 0 else -1
        if j < 0:
            continue
        for arm in re.split(r',(?![^(]*\))', src[i + 1:j]):
            if '=>' in arm:
                yield src[:m.start()] + arm.split('=>', 1)[1].strip() + src[j + 1:]
    for m in re.finditer(r'\blet\s+(\w+)\s*=', src):  # let x = R in B -> B
        k = src.find(' in ', m.end())
        semi = src.find(';', m.end())
        if k < 0 or (semi != -1 and semi < k):
            continue
        yield src[:m.start()] + src[k + 4:]
    for m in re.finditer(r'\b[A-Za-z_]\w*\b', src):  # ident -> 0
        if m.group() not in KEYWORDS:
            yield src[:m.start()] + '0' + src[m.end():]
    for m in re.finditer(r'\b\d+\b', src):  # num -> 0 / 1 / 2
        for rep in ('0', '1', '2'):
            if m.group() != rep:
                yield src[:m.start()] + rep + src[m.end():]
    yield re.sub(r'[ \t]+', ' ', src)
    yield re.sub(r'\n\s*\n', '\n', src)


def expr_pass(src, want, wd):
    k = 0
    cs = list(cands(src))
    while k < len(cs):
        c = cs[k]
        if len(c) < len(src) and ok(c, want, wd):
            src = c
            cs = list(cands(src))  # regenerate, resume near the same position
        else:
            k += 1
    return src


def shrink(src, want, wd):
    cur = src
    for _ in range(4):
        before = cur
        cur = lines_pass(cur, want, wd)
        cur = expr_pass(cur, want, wd)
        cur = lines_pass(cur, want, wd)
        if cur == before:
            break
    return cur.strip() + '\n'


def next_guard_no():
    ns = [int(m.group(1)) for f in glob.glob(os.path.join(ROOT, 'bench', 'parity_constructs', 'c*.bp'))
          for m in [re.match(r'c(\d+)_', os.path.basename(f))] if m]
    return max(ns, default=0) + 1


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    opts = dict(a[2:].split('=', 1) if '=' in a else (a[2:], True) for a in sys.argv[1:] if a.startswith('--'))
    if 'classify' in opts:
        cat, got, exp = classify(bp=args[0])
        print('%s\t%s\t%s' % (cat, exp, got))
        return
    src = open(args[0]).read()
    body = '\n'.join(l for l in src.split('\n') if not l.startswith('//'))
    wd = os.path.join(os.environ.get('BEBOP_TMP', '/tmp/opencode/agentB-fuzz'), 'shrink.%d' % os.getpid())
    os.makedirs(wd, exist_ok=True)
    want = classify(body, wd=wd)
    print('want %s got=%s expected=%s (%d bytes, bin %s)' % (want[0], want[1], want[2], len(body), BIN), file=sys.stderr)
    if want[0] in ('OK', 'BPREF-DEPTH'):
        print('program is %s under %s, nothing to shrink' % (want[0], BIN), file=sys.stderr)
        sys.exit(1)
    cur = shrink(body, want, wd)
    fin = classify(cur, wd=wd)
    name = opts.get('name') if isinstance(opts.get('name'), str) else fin[0].lower().replace('-', '')
    out = '// MIN %s expected=%s got=%s (%d -> %d bytes, %d classifications)\n%s' % (
        fin[0], fin[2], fin[1], len(body), len(cur), len(cache), cur)
    if isinstance(opts.get('out'), str):
        open(opts['out'], 'w').write(out)
    sys.stdout.write(out)
    print('\n--- journal stub (docs/exp.journal) ---')
    print('$(date +%%s) H:%s of %s is one construct | DID:shrink.py ddmin %d -> %d bytes, %d classifications, BEBOP_BIN=%s | GOT:%s expected=%s got=%s | VERDICT:confirmed'
          % (fin[0], os.path.basename(args[0]), len(body), len(cur), len(cache), os.path.basename(BIN), fin[0], fin[2], fin[1]))
    print('--- construct-parity guard stub (bench/parity_constructs/c%02d_%s.bp, EXPECT=%s; add the case line to bench/vs_rust/construct_parity.sh and freeze the .bin) ---'
          % (next_guard_no(), name, fin[2]))
    print('// %s: <one-line root cause> (<date>, from fuzz repro %s via shrink.py)' % (fin[0], os.path.basename(args[0])))
    print(' '.join(cur.split()))


if __name__ == '__main__':
    main()
