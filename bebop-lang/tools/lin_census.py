#!/usr/bin/env python3
"""lin_census -- the python twin of bebop.bp's LIN recurrence detector (ROADMAP A3).

`tools/lin_census.py FILE.bp ...` prints every `while` loop that the A3 shape
detector accepts, with the composed affine map M^k (exact in wraparound i64) and
the trip guard the folded loop must carry. It is the *specification* half of A3
step 1: the emitter in bebop.bp must fold exactly this set and no other, so a
disagreement between this census and the compiler is a detector bug, not a
tuning knob.

Why python and not a diag print in the compiler: an emitted diagnostic changes
the word stream (a WORD_DELTA on every construct), and the whole acceptance of
step 1 is "the non-folded path is byte-identical". A twin costs nothing in the
binary and is auditable line by line against docs/blueprints/A3-*.md section 3.

Shape accepted (blueprint section 3, verbatim):
  * body items are ONLY `let NAME = RHS;` plus an optional trailing `0`;
  * exactly one COUNTER item `let i = i - c;` / `let i = i + c;`, c a positive
    literal and `i` the symbol the condition tests;
  * every other item binds an ACCUMULATOR that was bound BEFORE the loop, at
    most once, with an AFFINE RHS over {accumulators, counter, invariants,
    literals} built from `+ - *` and parens, every `*` having a constant side;
  * m <= 3 accumulators;
  * condition `i OP B`, OP in `> >= < <=`, B a literal or a loop-invariant
    symbol, direction consistent with the counter's sign.
Everything else (a call, an array access, a string, `if`, a nested `while`,
`match`, `return`, `break`, `/ % & | ^ << >>`, a comparison inside an
accumulator RHS) rejects the WHOLE loop -- rejection is the safe answer, and
the reject reason is printed under -v so the census doubles as the argument for
why a given kernel does not fold.

Composition: state z = (s_1..s_m, i, invariants..., 1); one iteration is
z' = M z over Z/2^64; the folded body is row j of M^k. Exactness is the whole
game here (A3 lane card, trap 1), so every product goes through wrap() -- the
same wraparound helper bpref.py uses for the interpreter -- and never through
python's unbounded ints.

Usage:
  tools/lin_census.py [-v] [--matrix] [--k N] FILE.bp ...
  -v        also print every REJECTED loop with its reason (the interesting half)
  --matrix  print the full M and M^k matrices for each accepted loop
  --k N     force k instead of the blueprint rule (m==1 and every |coef|<65536 -> 4, else 2)
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpref  # noqa: E402  -- the twin shares bpref's tokenizer and grammar on purpose

MASK = (1 << 64) - 1
wrap = bpref.wrap

# The operators an accumulator RHS may contain. `/ % & | ^ << >> >>>` and every
# comparison are OUT by section 1: they are not affine over Z/2^64, so no matrix
# composition exists for them at all -- not "hard to fold", *undefined*.
AFFINE_OPS = ('+', '-', '*')

# The blueprint's accumulator cap. `--wide` raises it so the census can answer a
# question the fixed cap hides: of the loops this optimisation refuses, how many
# are refused because the DETECTOR is narrow (a cap, a shape) versus because the
# loop is not an affine recurrence at all (it reads or writes memory, calls a
# function, branches). Only the first kind could ever be recovered by widening.
MAX_M = [3]


class Reject(Exception):
    pass


# ---------------------------------------------------------------- linear forms
# A linear form is {key: coef} with the constant under key None; keys are symbol
# names. wrap() on every coefficient keeps the form exact in i64 the way the
# emitted code will be -- a python-int coefficient that "looks" bigger than 64
# bits is exactly how this class of optimisation silently produces wrong answers.
def lf_const(v):
    return {None: wrap(v)}


def lf_var(name):
    return {name: 1}


def lf_add(a, b, sign=1):
    out = dict(a)
    for k, v in b.items():
        out[k] = wrap(out.get(k, 0) + sign * v)
    return dict((k, v) for k, v in out.items() if v != 0 or k is None)


def lf_is_const(f):
    for k in f:
        if k is not None:
            return False
    return True


def lf_scale(f, c):
    return dict((k, wrap(v * c)) for k, v in f.items())


def affine(node, allowed):
    """Linear form of `node` over the `allowed` symbol names, or Reject."""
    t = node[0]
    if t == 'num':
        return lf_const(node[1])
    if t == 'neg':
        return lf_scale(affine(node[1], allowed), -1)
    if t == 'var':
        if node[1] not in allowed:
            raise Reject('symbol %r is neither an accumulator, the counter, nor a loop invariant' % node[1])
        return lf_var(node[1])
    if t == 'bin':
        op = node[1]
        if op not in AFFINE_OPS:
            raise Reject('operator %r is not affine' % op)
        a = affine(node[2], allowed)
        b = affine(node[3], allowed)
        if op == '+':
            return lf_add(a, b)
        if op == '-':
            return lf_add(a, b, -1)
        # `*` is affine only when one side folds to a constant
        if lf_is_const(b):
            return lf_scale(a, b[None])
        if lf_is_const(a):
            return lf_scale(b, a[None])
        raise Reject('`*` with two non-constant sides is not affine')
    raise Reject('node %r is not an affine form' % (t,))


# ---------------------------------------------------------------- matrices
def mat_mul(A, B, d):
    C = [[0] * d for _ in range(d)]
    for i in range(d):
        Ai = A[i]
        Ci = C[i]
        for l in range(d):
            a = Ai[l]
            if a:
                Bl = B[l]
                for j in range(d):
                    if Bl[j]:
                        Ci[j] = wrap(Ci[j] + a * Bl[j])
    return C


def mat_pow(M, d, k):
    R = [[1 if i == j else 0 for j in range(d)] for i in range(d)]
    for _ in range(k):
        R = mat_mul(R, M, d)
    return R


# ---------------------------------------------------------------- detector
def outer_syms(items):
    """Names bound by `let` at this statement level (the blueprint's sym_is_outer)."""
    s = set()
    for it in items:
        if it[0] == 'let':
            s.add(it[1])
    return s


def detect(cond, body, outer, params):
    """Return a shape report for a foldable loop, else raise Reject."""
    # ---- condition: `i OP B`
    if cond[0] != 'bin' or cond[1] not in ('>', '>=', '<', '<='):
        raise Reject('condition is not `i > >= < <= B`')
    op = cond[1]
    if cond[2][0] != 'var':
        raise Reject('condition LHS is not a bare symbol')
    ctr = cond[2][1]
    b = cond[3]
    if b[0] == 'num':
        bound = ('lit', b[1])
    elif b[0] == 'neg' and b[1][0] == 'num':
        bound = ('lit', wrap(-b[1][1]))
    elif b[0] == 'var':
        bound = ('sym', b[1])
    else:
        raise Reject('condition RHS is neither a literal nor a symbol')

    # ---- body items: only `let`, plus an optional trailing bare `0`
    lets = []
    for idx, it in enumerate(body):
        if it[0] == 'let':
            lets.append(it)
        elif it[0] == 'expr' and it[1] == ('num', 0) and idx == len(body) - 1:
            pass
        elif it[0] == 'while':
            raise Reject('nested while')
        else:
            raise Reject('body item %r is not `let NAME = RHS;`' % (it[0],))
    if not lets:
        raise Reject('empty body')

    # ---- split the counter from the accumulators
    counter = None
    accs = []
    for it in lets:
        name = it[1]
        rhs = it[2]
        if rhs[0] == 'assign':
            raise Reject('chain-assign in the body')
        if name == ctr:
            if counter is not None:
                raise Reject('counter assigned twice')
            if rhs[0] != 'bin' or rhs[1] not in ('+', '-') or rhs[2] != ('var', ctr) or rhs[3][0] != 'num':
                raise Reject('counter update is not `%s = %s +/- <literal>`' % (ctr, ctr))
            c = rhs[3][1]
            if c <= 0:
                raise Reject('counter step is not a positive literal')
            counter = c if rhs[1] == '+' else -c
        else:
            accs.append((name, rhs))
    if counter is None:
        raise Reject('no counter update for the condition symbol %r' % ctr)
    # Is the counter the LAST item? bebop's `let` REBINDS in place, so a body
    # that decrements first makes every later RHS read the NEW counter. The
    # matrix below models that correctly (the forms are substituted in source
    # order), but the emitter in bebop.bp folds only the counter-last subset --
    # see `emit` at the bottom of this function.
    ctr_last = lets[-1][1] == ctr
    # The direction must agree with the test or the guard below is unsound: a
    # counter that walks AWAY from the bound never terminates, and one that
    # walks toward it under the opposite test folds a trip that never ran.
    if counter < 0 and op not in ('>', '>='):
        raise Reject('decreasing counter under a `%s` test' % op)
    if counter > 0 and op not in ('<', '<='):
        raise Reject('increasing counter under a `%s` test' % op)

    names = [a[0] for a in accs]
    if len(names) != len(set(names)):
        raise Reject('an accumulator is assigned twice')
    if len(names) > MAX_M[0]:
        raise Reject('m = %d > 3 accumulators' % len(names))
    for nm in names:
        if nm not in outer:
            raise Reject('accumulator %r is not bound before the loop' % nm)

    # ---- affine forms. Anything that is neither an accumulator nor the counter
    # must be an INVARIANT: an outer symbol (or a parameter) the body never
    # assigns. It gets its own identity row in M, so the composition handles it
    # without a special case.
    assigned = set(names) | set([ctr])
    allowed = set(outer) | set(params) | set([ctr])
    # SEQUENTIAL composition, not simultaneous. `let` in bebop rebinds the
    # existing symbol, so within one trip each RHS sees the values written by
    # the items ABOVE it. `cur[v]` is v's value at this point expressed in the
    # variables the TRIP STARTED with; every RHS is substituted through it
    # before being stored back. Modelling the body as a simultaneous update
    # instead silently produces the wrong map for a counter-first body -- the
    # exact class c99_lin_reject's r3 pins.
    cur = {}

    def subst(f):
        out = lf_const(f.get(None, 0))
        for k, v in f.items():
            if k is None:
                continue
            out = lf_add(out, lf_scale(cur.get(k, lf_var(k)), v))
        return out

    invs = set()
    for it in lets:
        f = subst(affine(it[2], allowed))
        for k in f:
            if k is None or k in assigned:
                continue
            invs.add(k)
        cur[it[1]] = f
    forms = [(nm, cur[nm]) for nm in names]
    if bound[0] == 'sym':
        if bound[1] in assigned:
            raise Reject('the condition bound %r is assigned in the body' % bound[1])
        if bound[1] not in allowed:
            raise Reject('the condition bound %r is not a loop invariant' % bound[1])

    # ---- build M over z = (s_1..s_m, i, invs..., 1)
    order = names + [ctr] + sorted(invs) + [None]
    d = len(order)
    pos = {}
    for i, k in enumerate(order):
        pos[k] = i
    M = [[0] * d for _ in range(d)]
    for j, nmf in enumerate(forms):
        for k, v in nmf[1].items():
            M[j][pos[k]] = wrap(v)
    for k, v in cur[ctr].items():
        M[pos[ctr]][pos[k]] = wrap(v)
    for iv in invs:
        M[pos[iv]][pos[iv]] = 1
    M[d - 1][d - 1] = 1
    # `emit` is the narrower subset bebop.bp's own detector folds: one
    # accumulator, counter last. Everything else here is still a genuine affine
    # recurrence -- the census reports it so the gap between the spec and the
    # emitter stays visible instead of being quietly forgotten.
    return dict(ctr=ctr, c=abs(counter), step=counter, op=op, bound=bound,
                names=names, invs=sorted(invs), order=order, d=d, M=M,
                m=len(names), emit=(len(names) == 1 and ctr_last))


def pick_k(rep, forced=None):
    """Blueprint section 1: k = 4 when m == 1 and every composed coefficient fits
    one movz (|v| < 65536) after folding, else k = 2. A wider coefficient is not
    wrong, it just costs a movz/movk chain per folded trip and eats the win."""
    if forced:
        return forced
    if rep['m'] != 1:
        return 2
    P = mat_pow(rep['M'], rep['d'], 4)
    for j in range(rep['m']):
        for v in P[j]:
            if abs(v) >= 65536:
                return 2
    return 4


def guard_text(rep, k):
    """The folded trip is legal iff all k iterations would have passed the test."""
    sgn = '-' if rep['step'] < 0 else '+'
    return '%s %s %d %s %s' % (rep['ctr'], sgn, (k - 1) * rep['c'], rep['op'], rep['bound'][1])


def widest(P, m):
    w = 0
    for j in range(m):
        for v in P[j]:
            if abs(v) > w:
                w = abs(v)
    return w


def report(path, fnname, ordinal, rep, k, matrix=False):
    P = mat_pow(rep['M'], rep['d'], k)
    print('%s %s %s#%d  m=%d counter=%s step=%+d cond=(%s %s %s) k=%d widest=%d'
          % ('FOLD' if rep['emit'] else 'FOLD(spec-only)', path, fnname, ordinal,
             rep['m'], rep['ctr'], rep['step'],
             rep['ctr'], rep['op'], rep['bound'][1], k, widest(P, rep['m'])))
    lbl = []
    for o in rep['order']:
        lbl.append('1' if o is None else o)
    for j, nm in enumerate(rep['names']):
        terms = ' + '.join('%d*%s' % (P[j][l], lbl[l]) for l in range(rep['d']) if P[j][l])
        print('     %s <- %s' % (nm, terms or '0'))
    print('     %s <- %s %s %d' % (rep['ctr'], rep['ctr'],
                                   '-' if rep['step'] < 0 else '+', k * rep['c']))
    print('     guard: %s' % guard_text(rep, k))
    if matrix:
        print('     order: %s' % ' '.join(lbl))
        for row in rep['M']:
            print('     M   : %s' % ' '.join(str(v) for v in row))
        for row in P:
            print('     M^%d : %s' % (k, ' '.join(str(v) for v in row)))


def expand_uses(path, seen=None, root='.'):
    """Inline `use "rel/path.bp"` the way the compiler's loader does, so the
    census can read bebop.bp and every std_test (all of them start with a `use`
    line, which bpref's own grammar does not carry -- the compiler resolves
    includes before the parser ever sees them). Repeat includes are dropped, as
    the compiler drops them."""
    if seen is None:
        seen = set()
    ap = os.path.normpath(path)
    if ap in seen:
        return ''
    seen.add(ap)
    out = []
    for line in open(path):
        t = line.strip()
        if t.startswith('use ') and t.count('"') >= 2:
            rel = t.split('"')[1]
            cand = os.path.join(root, rel)
            if os.path.exists(cand):
                out.append(expand_uses(cand, seen, root))
                continue
        out.append(line)
    return ''.join(out)


def walk(path, verbose, matrix, forced_k, stats):
    try:
        src = expand_uses(path)
        p = bpref.Parser(src)
        p.program()
    except Exception as e:
        # not every .bp in the tree is a whole program (prelude fragments, `use`
        # includes); a file the oracle cannot parse is not evidence either way.
        print('SKIP %s (%s)' % (path, e))
        stats['skip'] += 1
        return
    for fnname in p.fns:
        params = p.fns[fnname][0]
        body = p.fns[fnname][1]
        seq = [0]

        def rec(items, outer):
            for it in items:
                if it[0] == 'while':
                    seq[0] = seq[0] + 1
                    ordinal = seq[0]
                    stats['seen'] += 1
                    rep = None
                    try:
                        rep = detect(it[1], it[2], outer, params)
                    except Reject as r:
                        stats['rej'] += 1
                        stats['why'][str(r).split(' ')[0]] = stats['why'].get(str(r).split(' ')[0], 0) + 1
                        if verbose:
                            print('rej  %s %s#%d: %s' % (path, fnname, ordinal, r))
                    if rep is not None:
                        stats['fold'] += 1
                        stats['emit'] += rep['emit']
                        report(path, fnname, ordinal, rep, pick_k(rep, forced_k), matrix)
                    rec(it[2], set(outer) | outer_syms(it[2]))
                elif it[0] == 'let':
                    outer = set(outer) | set([it[1]])

        rec(body, set(params))


# ---------------------------------------------------------------- R1/R2 coverage census
# 2026-10-07, lane rfast-lsr (docs/research/2026-10-07-bebop-fastest-runtime.md §12 L1
# STEP 0). The A3 rule: a loop transform that covers 0 loops of the real corpus is refused
# however green it is. `--lsr` answers, BEFORE any emitter change, how many `while` loops
# carry (a) `NAME * LIT` / `LIT * NAME` / `NAME << LIT` with NAME bound before the loop
# and never rebound anywhere in its body (LICM candidate), (b) `IND * LIT` / `IND << LIT`
# with IND the loop counter (`cond` tests IND, the body's own level has exactly one
# `let IND = IND +/- LIT`) -- the LSR candidate, (c) a literal 1..65535 in the body (any
# use, and the narrower "operand of * / %" use that costs a `mov` every trip), and (d)
# self-recursive fns whose TAIL is `f(..)` or `f(..) + f(..)` (TRE candidates; `x + f(..)`
# reported separately). Each loop is counted against its OWN body; a nested `while` is its
# own loop (the depth-0 rule of hoist_scan). A loop or fn reached through several `use`
# expansions is counted once (key = fn name + AST).
def _kids(node):
    for x in node[1:] if isinstance(node, tuple) else node:
        if isinstance(x, (tuple, list)):
            yield x


def _walk_own(node, out, skip_while=True):
    """Every expression node of a loop body, not descending into nested whiles."""
    if isinstance(node, list):
        for it in node:
            if isinstance(it, tuple) and it and it[0] == 'while' and skip_while:
                continue
            _walk_own(it, out, skip_while)
        return
    if not isinstance(node, tuple):
        return
    out.append(node)
    for k in _kids(node):
        _walk_own(k, out, skip_while)


def _rebinds(body):
    """Names (re)bound anywhere in body, nested whiles included."""
    nodes = []
    _walk_own(body, nodes, skip_while=False)
    s = set()
    for nd in nodes:
        if nd[0] in ('let', 'letin', 'assign') and isinstance(nd[1], str):
            s.add(nd[1])
    return s


def _counter(cond, body):
    if cond[0] != 'bin' or cond[2][0] != 'var':
        return None
    ctr = cond[2][1]
    ups = [it for it in body if it[0] == 'let' and it[1] == ctr]
    if len(ups) != 1:
        return None
    r = ups[0][2]
    if r[0] == 'bin' and r[1] in ('+', '-') and r[2] == ('var', ctr) and r[3][0] == 'num':
        return ctr
    return None


def _tails(e):
    t = e[0]
    if t == 'if':
        return _tails(e[2]) + _tails(e[3])
    if t == 'letin':
        return _tails(e[3])
    if t == 'block':
        return _tails(e[1][-1]) if e[1] else []
    if t == 'match':
        out = []
        for arm in e[2]:
            out += _tails(arm[2])
        return out
    return [e]


def lsr_census(files):
    loops = {}
    fns = {}
    skipped = []
    for path in files:
        try:
            src = expand_uses(path)
            p = bpref.Parser(src)
            p.program()
        except BaseException as ex:  # bpref sys.exit(3)s on UNSUPPORTED
            skipped.append((path, str(ex)[:60]))
            continue
        for fnname in p.fns:
            params, body = p.fns[fnname]
            fns.setdefault((fnname, repr(body)), (path, fnname, body))

            def rec(items, outer):
                for it in items:
                    if it[0] == 'while':
                        loops.setdefault((fnname, repr(it)), (path, fnname, it, frozenset(outer)))
                        rec(it[2], set(outer) | outer_syms(it[2]))
                    elif it[0] == 'let':
                        outer = set(outer) | set([it[1]])
            rec(body, set(params))
    n = len(loops)
    ca = cb = cab = cc = ccm = 0
    ex_a = []
    ex_b = []
    for key, (path, fnname, w, outer) in loops.items():
        cond, body = w[1], w[2]
        reb = _rebinds(body)
        ctr = _counter(cond, body)
        nodes = []
        _walk_own(body, nodes)
        ha = hb = hc = hcm = 0
        for nd in nodes:
            if nd[0] == 'bin' and nd[1] in ('*', '<<'):
                a, b = nd[2], nd[3]
                pairs = [(a, b)] + ([(b, a)] if nd[1] == '*' else [])
                for v, l in pairs:
                    if v[0] == 'var' and l[0] == 'num':
                        if ctr is not None and v[1] == ctr:
                            hb = 1
                        elif v[1] in outer and v[1] not in reb:
                            ha = 1
            if nd[0] == 'num' and 1 <= nd[1] <= 65535:
                hc = 1
            if nd[0] == 'bin' and nd[1] in ('*', '/', '%') and (nd[2][0] == 'num' or nd[3][0] == 'num'):
                lit = nd[2] if nd[2][0] == 'num' else nd[3]
                if 1 <= lit[1] <= 65535 and lit[1] & (lit[1] - 1) != 0:
                    hcm = 1  # a power of two is a shift; anything else is mov+mul every trip
        ca += ha
        cb += hb
        cab += 1 if (ha or hb) else 0
        cc += hc
        ccm += hcm
        if ha and len(ex_a) < 12:
            ex_a.append('%s:%s' % (path, fnname))
        if hb and len(ex_b) < 12:
            ex_b.append('%s:%s' % (path, fnname))
    d_tail = []
    d_two = []
    d_acc = []
    for key, (path, fnname, body) in fns.items():
        if not body or body[-1][0] != 'expr':
            continue
        for t in _tails(body[-1][1]):
            if t[0] == 'call' and t[1] == fnname:
                d_tail.append('%s:%s' % (path, fnname))
            elif t[0] == 'bin' and t[1] == '+':
                l = t[2][0] == 'call' and t[2][1] == fnname
                r = t[3][0] == 'call' and t[3][1] == fnname
                if l and r:
                    d_two.append('%s:%s' % (path, fnname))
                elif l or r:
                    d_acc.append('%s:%s' % (path, fnname))
    d_tail = sorted(set(d_tail))
    d_two = sorted(set(d_two))
    d_acc = sorted(set(d_acc))
    pct = lambda x: 100.0 * x / n if n else 0.0
    print('lsr census: %d files read, %d skipped (bpref cannot parse), %d distinct while loops, %d distinct fns'
          % (len(files) - len(skipped), len(skipped), n, len(fns)))
    print('  (a) NAME*LIT / NAME<<LIT, NAME invariant  : %4d loops  %5.1f %%' % (ca, pct(ca)))
    print('  (b) IND*LIT / IND<<LIT, IND the counter   : %4d loops  %5.1f %%' % (cb, pct(cb)))
    print('  (a) or (b)  [R1 GO needs >= 20 %%]          : %4d loops  %5.1f %%  -> R1 %s'
          % (cab, pct(cab), 'GO' if pct(cab) >= 20.0 else 'NO-GO'))
    print('  (c) literal 1..65535 anywhere in the body  : %4d loops  %5.1f %%' % (cc, pct(cc)))
    print('  (c\') non-pow2 literal 1..65535 as * / %% operand: %4d loops  %5.1f %%' % (ccm, pct(ccm)))
    print('  (d) self-recursive tail f(..)              : %4d fns' % len(d_tail))
    print('  (d) self-recursive tail f(..) + f(..)      : %4d fns' % len(d_two))
    print('  (d) [R2 GO needs >= 3]                     : %4d fns  -> R2 %s'
          % (len(d_tail) + len(d_two), 'GO' if len(d_tail) + len(d_two) >= 3 else 'NO-GO'))
    print('  (info) tail x + f(..) (accumulator form)    : %4d fns' % len(d_acc))
    for x in ex_a:
        print('    a-example %s' % x)
    for x in ex_b:
        print('    b-example %s' % x)
    for x in d_tail + d_two:
        print('    d-fn %s' % x)
    for x in d_acc:
        print('    d-acc %s' % x)
    for pth, why in skipped:
        print('    skip %s (%s)' % (pth, why))


# ---------------------------------------------------------------- R2b shrink-wrap census
# 2026-10-07, lane rfast-lsr (R2b). `--leaf`: per distinct fn, does a CALL-FREE RETURN PATH exist
# (a path from entry to return that executes no user-fn call; builtins do not count as calls)?
#   leaf   -- no user call anywhere in the body (a frame-free fn candidate)
#   mixed  -- calls exist, but some return path makes none (the shrink-wrap case proper)
#   guard  -- the narrow shape R2b builds: the body is ONE `if P OP NUM then (P|NUM) else ...`,
#             P a parameter, NUM 0..4095 (the base-case guard that runs before the prologue)
def _nocall(e, fns):
    nodes = []
    _walk_own(e if isinstance(e, list) else [e], nodes, skip_while=False)
    return not any(nd[0] == 'call' and nd[1] in fns for nd in nodes)


def _cf(e, fns):
    t = e[0]
    if t == 'if':
        return _nocall(e[1], fns) and (_cf(e[2], fns) or _cf(e[3], fns))
    if t == 'letin':
        return _cf(e[2], fns) and _cf(e[3], fns)
    if t == 'match':
        return any(_cf(a[2], fns) for a in e[2])
    if t == 'block':
        return all(_cf(x, fns) for x in e[1])
    return _nocall(e, fns)


def _cf_items(items, fns):
    for it in items:
        if it[0] == 'return':
            return _cf(it[1], fns)
        if it[0] == 'while':
            if not (_nocall(it[1], fns) and _nocall(it[2], fns)):
                return False
        elif it[0] in ('let', 'expr'):
            if not _cf(it[2] if it[0] == 'let' else it[1], fns):
                return False
    return True


def leaf_census(files):
    fns = {}
    skipped = 0
    for path in files:
        try:
            p = bpref.Parser(expand_uses(path))
            p.program()
        except BaseException:
            skipped += 1
            continue
        for nm in p.fns:
            params, body = p.fns[nm]
            fns.setdefault((nm, repr(body)), (path, nm, params, body, set(p.fns)))
    n = len(fns)
    leaf = mixed = guard = gmixed = 0
    ex = []
    for key, (path, nm, params, body, allf) in fns.items():
        anycall = not _nocall(body, allf)
        cf = _cf_items(body, allf)
        if not anycall:
            leaf += 1
        elif cf:
            mixed += 1
        if len(body) == 1 and body[0][0] == 'expr' and body[0][1][0] == 'if':
            c, a = body[0][1][1], body[0][1][2]
            ok = (c[0] == 'bin' and c[1] in ('<', '<=', '>', '>=', '==', '!=') and c[2][0] == 'var'
                  and c[2][1] in params and c[3][0] == 'num' and 0 <= c[3][1] <= 4095
                  and ((a[0] == 'var' and a[1] in params) or (a[0] == 'num' and 0 <= a[1] <= 65535)))
            if ok:
                guard += 1
                gmixed += 1 if anycall else 0
                if len(ex) < 12:
                    ex.append('%s:%s' % (path, nm))
    pct = lambda x: 100.0 * x / n if n else 0.0
    print('leaf census: %d files read, %d skipped, %d distinct fns' % (len(files) - skipped, skipped, n))
    print('  leaf  (no user call at all)                 : %4d  %5.1f %%' % (leaf, pct(leaf)))
    print('  mixed (calls, but a call-free return path)  : %4d  %5.1f %%' % (mixed, pct(mixed)))
    print('  any call-free return path (leaf + mixed)    : %4d  %5.1f %%' % (leaf + mixed, pct(leaf + mixed)))
    print('  guard shape R2b builds (all / with calls)   : %4d / %d  %5.1f %% / %5.1f %%' % (guard, gmixed, pct(guard), pct(gmixed)))
    for x in ex:
        print('    guard-example %s' % x)


def main():
    args = sys.argv[1:]
    verbose = '-v' in args
    matrix = '--matrix' in args
    if '--wide' in args:
        MAX_M[0] = 64
    forced_k = None
    if '--k' in args:
        i = args.index('--k')
        forced_k = int(args[i + 1])
        del args[i:i + 2]
    files = [a for a in args if not a.startswith('-')]
    if '--leaf' in args:
        leaf_census(files)
        return
    if '--lsr' in args:
        lsr_census(files)
        return
    stats = dict(seen=0, fold=0, emit=0, rej=0, skip=0, why={})
    for f in files:
        walk(f, verbose, matrix, forced_k, stats)
    print('lin census: %d while loops, %d affine (%d folded by bebop.bp), %d rejected, %d files skipped'
          % (stats['seen'], stats['fold'], stats['emit'], stats['rej'], stats['skip']))
    if verbose:
        for k in sorted(stats['why'], key=lambda x: -stats['why'][x]):
            print('  reject class %-14s %d' % (k, stats['why'][k]))


if __name__ == '__main__':
    main()
