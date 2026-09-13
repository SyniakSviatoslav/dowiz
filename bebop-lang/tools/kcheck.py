#!/usr/bin/env python3
"""kcheck.py -- the kernel's `bpref`. A REFERENCE type checker for Core terms.

  *** OUTSIDE THE TRUST ROOT. This is a differential twin and an executable
      specification, never the checker the tree trusts. ***

  `tools/bpref.py` is the compiler's reference implementation; `tools/certcheck.py`
  is the certificate layer's; this is the TERM layer's. It exists so that
  `tcheck.bp` (ROADMAP F6) has a twin to disagree with from its FIRST commit
  rather than its last -- which is the whole point of building the refutation
  instrument before the kernel. A half-built kernel that cannot be refuted on a
  number is the one shape of work this tree does not allow.

  Two numbers come out of this file and its corpus, and both exist today:
      kernel_neg:    accepted of N   -- MUST be 0. A kernel that accepts an
                                        unsound term is unsound, and that is
                                        knowable before the kernel exists.
      kernel_parity: k/N             -- terms where kcheck.py and tcheck.bp
                                        agree. With no tcheck.bp it reads 0/N,
                                        which is the honest STARTING value and
                                        not a failure.

WHERE THIS CALCULUS COMES FROM, and a caveat that must not be lost
------------------------------------------------------------------
The brief cites "the report's section 4" for the calculus. **That report is not
in this tree** (2026-09-09, `e171367`): no file under `docs/` mentions Hurkens,
Girard, non-positivity or `imax`. This is the fourth cited-but-absent source
today, after `docs/RESEARCH-VERIFICATION-2026-09-09.md` (since added),
`docs/blueprints/F0-trap-census.md` and the Fable design report. So the calculus
below is SPECIFIED HERE, from the constraints that are actually stated, and it is
the thing to argue with -- it is not a transcription of section 4 and must not be
read as one.

The calculus (predicative, concrete levels, de Bruijn):

    Sort n                n a CONCRETE natural. Infinite hierarchy: Sort n : Sort (n+1).
    var k                 de Bruijn index, 0 = innermost binder.
    pi A B                dependent function type; B is checked under A.
    lam A b               abstraction with domain annotation.
    app f a               application.
    const c               a previously DECLARED name (def or axiom).

    Sort n            : Sort (n+1)
    pi A B            : Sort (max u v)      where A : Sort u and, under A, B : Sort v
    lam A b           : pi A B              where, under A, b : B
    app f a           : B[a]                where f : pi A B and typeof(a) == A up to conversion

**By default there is no `imax`, and there are no level variables.** (Since
2026-09-13 `KCHECK_LEVEL=imax` switches this twin to the kernel's second arm --
Sort 0 impredicative, nothing above it -- so both arms can be measured here as
well as in `tkernel.bin`; the default and every gate stay on `max`.) The elaborator
instantiates concrete levels. What that costs is worth stating plainly, because
it is the question under adversarial review: dropping `imax` does NOT shorten
the hierarchy -- `Sort 0 : Sort 1 : Sort 2 : ...` is still infinite -- it removes
IMPREDICATIVITY. `imax` exists to make `pi A B : Prop` when `B : Prop`
regardless of A's level; without it every `pi` lands at `max u v` and the system
is predicative throughout. So "an infinite universe hierarchy" is satisfied;
"impredicative Prop" is not, and nothing here claims it. That is the trade to
review, and it makes the Girard/Hurkens negatives MORE important, not less:
those paradoxes are exactly what impredicativity plus a careless level rule buys.

Sharing: a node may be referenced by AT MOST ONE parent. The file is DAG-shaped
syntax carrying a tree. This is deliberate and it is a real restriction: with de
Bruijn indices a subterm's meaning depends on its binder DEPTH, so sharing one
node at two different depths is unsound, and a kernel that permits it must track
depth per reference. Forbidding sharing removes the class entirely for the cost
of duplication in the elaborator, which is the right trade for a kernel that has
to be small and auditable. `n08_shared_subterm` is the negative that pins it.

Usage:
    tools/kcheck.py <file.core>            check one file
    tools/kcheck.py --corpus <dir>         check a corpus, print the two numbers
"""
import os
import re
import sys

MAXFUEL = 100000          # conversion fuel; exhaustion FAILS LOUDLY, never accepts
LEVEL_RULE = os.environ.get('KCHECK_LEVEL', 'max')   # 'max' (default) or 'imax'; see infer's pi case


class KError(Exception):
    """A rejection. The kernel saying no is a RESULT, not a crash."""


# ---------------------------------------------------------------- term syntax
# A term is a tuple: ('sort', n) ('var', k) ('pi', A, B) ('lam', A, b)
#                    ('app', f, a) ('const', name)

def parse(path):
    """Parse a .core file into (decls, checks). Nodes are numbered, arguments
    refer to STRICTLY SMALLER ids, so one forward pass establishes both
    well-formedness and acyclicity -- the same discipline as the certificate
    obligation format."""
    nodes, refs, decls, checks = {}, {}, [], []
    with open(path) as f:
        raw = f.read().split('\n')
    for lineno, line in enumerate(raw, 1):
        t = line.strip()
        if not t or t.startswith('%'):
            continue
        w = t.split()
        if w[0] in ('def', 'axiom', 'inductive', 'check'):
            decls.append((lineno, w))
            continue
        try:
            nid = int(w[0])
        except ValueError:
            raise KError('line %d: expected a node id or a declaration, got %r'
                         % (lineno, w[0]))
        if nid in nodes:
            raise KError('line %d: duplicate node id %d' % (lineno, nid))
        tag, args = w[1], w[2:]

        def ref(a):
            k = int(a)
            if k >= nid:
                raise KError('line %d: node %d refers to %d -- arguments must be '
                             'STRICTLY smaller (one forward pass must suffice)'
                             % (lineno, nid, k))
            if k not in nodes:
                raise KError('line %d: node %d refers to undefined node %d'
                             % (lineno, nid, k))
            refs[k] = refs.get(k, 0) + 1
            if refs[k] > 1:
                raise KError('line %d: node %d is referenced %d times. Sharing is '
                             'forbidden: with de Bruijn indices a subterm\'s meaning '
                             'depends on its binder DEPTH, so one node at two depths '
                             'is unsound. Duplicate it in the elaborator.'
                             % (lineno, k, refs[k]))
            return nodes[k]

        if tag == 'sort':
            n = int(args[0])
            if n < 0:
                raise KError('line %d: Sort %d -- levels are naturals' % (lineno, n))
            nodes[nid] = ('sort', n)
        elif tag == 'var':
            nodes[nid] = ('var', int(args[0]))
        elif tag == 'pi':
            nodes[nid] = ('pi', ref(args[0]), ref(args[1]))
        elif tag == 'lam':
            nodes[nid] = ('lam', ref(args[0]), ref(args[1]))
        elif tag == 'app':
            nodes[nid] = ('app', ref(args[0]), ref(args[1]))
        elif tag == 'const':
            nodes[nid] = ('const', args[0])
        else:
            raise KError('line %d: unknown tag %r' % (lineno, tag))
    return nodes, decls, checks


# ------------------------------------------------------------- de Bruijn shift

def shift(t, d, cutoff=0):
    tag = t[0]
    if tag == 'var':
        return ('var', t[1] + d) if t[1] >= cutoff else t
    if tag in ('sort', 'const'):
        return t
    if tag == 'pi':
        return ('pi', shift(t[1], d, cutoff), shift(t[2], d, cutoff + 1))
    if tag == 'lam':
        return ('lam', shift(t[1], d, cutoff), shift(t[2], d, cutoff + 1))
    if tag == 'app':
        return ('app', shift(t[1], d, cutoff), shift(t[2], d, cutoff))
    raise KError('shift: bad term %r' % (t,))


def subst(t, j, s):
    """t[j := s], with s shifted as it goes under binders."""
    tag = t[0]
    if tag == 'var':
        return s if t[1] == j else (('var', t[1] - 1) if t[1] > j else t)
    if tag in ('sort', 'const'):
        return t
    if tag == 'pi':
        return ('pi', subst(t[1], j, s), subst(t[2], j + 1, shift(s, 1)))
    if tag == 'lam':
        return ('lam', subst(t[1], j, s), subst(t[2], j + 1, shift(s, 1)))
    if tag == 'app':
        return ('app', subst(t[1], j, s), subst(t[2], j, s))
    raise KError('subst: bad term %r' % (t,))


# ------------------------------------------------------- environment and whnf

class Env:
    def __init__(self):
        self.types = {}      # const name -> type term
        self.bodies = {}     # const name -> body term (defs only; axioms have none)


def whnf(env, t, fuel):
    """Weak head normal form: beta + delta. Fuel exhaustion RAISES -- a kernel
    that gives up and accepts is the failure mode this file exists to prevent."""
    while True:
        if fuel[0] <= 0:
            raise KError('conversion fuel exhausted (%d steps). The kernel refuses '
                         'rather than accepting an unnormalised term -- a checker '
                         'that timed out into "yes" would be unsound.' % MAXFUEL)
        fuel[0] -= 1
        if t[0] == 'const' and t[1] in env.bodies:
            t = env.bodies[t[1]]
            continue
        if t[0] == 'app':
            f = whnf(env, t[1], fuel)
            if f[0] == 'lam':
                t = subst(f[2], 0, t[2])
                continue
            return ('app', f, t[2])
        return t


def conv(env, a, b, fuel):
    """Definitional equality: beta/delta convertibility, structural after whnf.
    No eta -- deliberately: eta costs the kernel a case and buys nothing the
    elaborator cannot do, and every rule the kernel does not have is a rule
    nobody has to verify."""
    a, b = whnf(env, a, fuel), whnf(env, b, fuel)
    if a[0] != b[0]:
        return False
    if a[0] == 'sort':
        return a[1] == b[1]
    if a[0] == 'var':
        return a[1] == b[1]
    if a[0] == 'const':
        return a[1] == b[1]
    if a[0] in ('pi', 'lam'):
        return conv(env, a[1], b[1], fuel) and conv(env, a[2], b[2], fuel)
    if a[0] == 'app':
        return conv(env, a[1], b[1], fuel) and conv(env, a[2], b[2], fuel)
    return False


# --------------------------------------------------------------------- typing

def infer(env, ctx, t, fuel):
    """ctx is a list of types, innermost FIRST (index 0 = var 0)."""
    tag = t[0]
    if tag == 'sort':
        # THE rule that makes or breaks soundness. `Sort n : Sort (n+1)`, never
        # `Sort n : Sort n` -- the latter is Girard's paradox and n01 pins it.
        return ('sort', t[1] + 1)
    if tag == 'var':
        k = t[1]
        if k < 0 or k >= len(ctx):
            raise KError('var %d is unbound (context depth %d)' % (k, len(ctx)))
        return shift(ctx[k], k + 1)
    if tag == 'const':
        if t[1] not in env.types:
            raise KError('const %s is not declared' % t[1])
        return env.types[t[1]]
    if tag == 'pi':
        u = sort_of(env, ctx, t[1], fuel)
        v = sort_of(env, [t[1]] + ctx, t[2], fuel)
        # Default: predicative `max`. KCHECK_LEVEL=imax mirrors the kernel's second
        # arm (tkernel.bp:224-226 selects it by a 4th argv starting with `i`;
        # tcheck_kernel.bp:127-130 is the rule): a pi whose codomain is Sort 0 lands
        # in Sort 0 whatever its domain's level. ONLY Sort 0 is impredicative -- there
        # is no (Sort 2, Sort 1) rule, which is exactly why n18_hurkens_witness is
        # rejected under this arm. Added 2026-09-13 so the twin can measure both arms.
        if LEVEL_RULE == 'imax' and v == 0:
            return ('sort', 0)
        return ('sort', max(u, v))
    if tag == 'lam':
        sort_of(env, ctx, t[1], fuel)   # the domain must be a type
        b = infer(env, [t[1]] + ctx, t[2], fuel)
        return ('pi', t[1], b)
    if tag == 'app':
        ft = whnf(env, infer(env, ctx, t[1], fuel), fuel)
        if ft[0] != 'pi':
            raise KError('application of a non-function: head has type %s'
                         % show(ft))
        at = infer(env, ctx, t[2], fuel)
        if not conv(env, at, ft[1], fuel):
            raise KError('argument type mismatch: expected %s, got %s'
                         % (show(ft[1]), show(at)))
        return subst(ft[2], 0, t[2])
    raise KError('infer: bad term %r' % (t,))


def sort_of(env, ctx, t, fuel):
    s = whnf(env, infer(env, ctx, t, fuel), fuel)
    if s[0] != 'sort':
        raise KError('expected a type, got something of type %s' % show(s))
    return s[1]


def show(t):
    tag = t[0]
    if tag == 'sort':
        return 'Sort %d' % t[1]
    if tag == 'var':
        return '#%d' % t[1]
    if tag == 'const':
        return t[1]
    if tag == 'pi':
        return '(%s -> %s)' % (show(t[1]), show(t[2]))
    if tag == 'lam':
        return '(fun %s => %s)' % (show(t[1]), show(t[2]))
    if tag == 'app':
        return '(%s %s)' % (show(t[1]), show(t[2]))
    return repr(t)


# ------------------------------------------------------------ positivity check

def occurs(name, t):
    if t[0] == 'const':
        return t[1] == name
    if t[0] in ('pi', 'lam', 'app'):
        return occurs(name, t[1]) or occurs(name, t[2])
    return False


def occurs_negatively(name, arg):
    """Inside ONE constructor argument, does `name` appear left of an arrow?"""
    while arg[0] == 'pi':
        if occurs(name, arg[1]):
            return True
        arg = arg[2]
    return False


def strictly_positive(name, ctype):
    """`name` may appear in a constructor argument only STRICTLY POSITIVELY.

    The distinction matters and a first cut of this function got it wrong, so it
    is worth writing down: `succ : Nat -> Nat` is FINE -- the argument IS Nat,
    which is an ordinary recursive occurrence -- while
    `mk : (Bad -> Bad) -> Bad` is not, because inside the argument `Bad -> Bad`
    the name appears to the LEFT of an arrow. So the test is applied per
    argument, to that argument's own pi-spine, and never to the constructor's
    own top-level spine. A kernel that confuses the two either rejects every
    inductive (too strict) or admits non-positive ones (unsound); n03 pins the
    second and p03 pins the first."""
    t = ctype
    while t[0] == 'pi':
        if occurs_negatively(name, t[1]):
            return False
        t = t[2]
    return True


def check_inductive(env, name, arity, ctors, fuel):
    sort_of(env, [], arity, fuel)
    env.types[name] = arity
    for cname, ctype in ctors:
        sort_of(env, [], ctype, fuel)
        if not strictly_positive(name, ctype):
            del env.types[name]
            raise KError('inductive %s: constructor %s has a NON-POSITIVE '
                         'occurrence of %s (it appears left of an arrow). Accepting '
                         'this makes the system inconsistent.' % (name, cname, name))
        tail = ctype
        while tail[0] == 'pi':
            tail = tail[2]
        if not occurs(name, tail) and tail != ('const', name):
            del env.types[name]
            raise KError('inductive %s: constructor %s does not return %s'
                         % (name, cname, name))
        env.types[cname] = ctype
    return True


# --------------------------------------------------------------------- driver

def run_file(path):
    """Returns (ok, message). ok=False means the kernel REJECTED, which for a
    negative test is the correct outcome."""
    fuel = [MAXFUEL]
    try:
        nodes, decls, _ = parse(path)
        env = Env()
        n_checked = 0
        for lineno, w in decls:
            kind = w[0]
            if kind == 'axiom':
                nm, ty = w[1], nodes[int(w[3])]
                sort_of(env, [], ty, fuel)
                env.types[nm] = ty
            elif kind == 'def':
                nm, ty, bd = w[1], nodes[int(w[3])], nodes[int(w[5])]
                sort_of(env, [], ty, fuel)
                got = infer(env, [], bd, fuel)
                if not conv(env, got, ty, fuel):
                    raise KError('line %d: def %s -- declared %s but the body has '
                                 'type %s' % (lineno, nm, show(ty), show(got)))
                env.types[nm], env.bodies[nm] = ty, bd
            elif kind == 'inductive':
                nm, arity = w[1], nodes[int(w[3])]
                ctors = []
                rest = w[4:]
                while rest and rest[0] == '|':
                    ctors.append((rest[1], nodes[int(rest[3])]))
                    rest = rest[4:]
                check_inductive(env, nm, arity, ctors, fuel)
            elif kind == 'check':
                term, ty = nodes[int(w[1])], nodes[int(w[3])]
                # The CLAIMED type must itself be a type. Added 2026-09-09 after
                # mutation testing: without it a `check` line could assert a term
                # against a claim that is not a type at all, and the assertion
                # would be meaningless rather than false.
                sort_of(env, [], ty, fuel)
                got = infer(env, [], term, fuel)
                if not conv(env, got, ty, fuel):
                    raise KError('line %d: check -- expected %s, inferred %s'
                                 % (lineno, show(ty), show(got)))
                n_checked += 1
        return True, 'accepted (%d checks, %d fuel used)' % (n_checked, MAXFUEL - fuel[0])
    except KError as e:
        return False, str(e)
    except RecursionError:
        return None, ('python recursion limit -- an INTERNAL error, NOT a rejection. '
                      'Note for tcheck.bp: the Bebop kernel must bound its own depth '
                      'explicitly rather than inherit a host limit')
    except Exception as e:
        # A crash is NOT a rejection and must never be scored as one. Mutation
        # testing found this: a mutant with the context bounds check removed
        # crashed with IndexError, and an earlier version of this file counted
        # that as "rejected" -- so the instrument scored an UNSOUND kernel as
        # safe. In Bebop the same defect reads past the context array and types
        # anything, so this is the failure mode that matters most.
        return None, 'INTERNAL %s: %s' % (type(e).__name__, e)


def run_kernel_on_fixture(kernel_bin, fixture_path, seed_path='./seed/build/seed'):
    """Run the kernel binary on a fixture and return the verdict code.
    Returns: (verdict_code, is_internal)
    - verdict_code: the number printed by the kernel (0, 10-31, 71, or None for crash)
    - is_internal: True if the kernel output 71 or crashed (should not be scored as rejection)
    """
    try:
        import subprocess
        # Run via seed binary to avoid permission issues
        result = subprocess.run([seed_path, kernel_bin, fixture_path],
                              capture_output=True, text=True, timeout=5)
        output = result.stdout.strip()
        if not output:
            return None, True  # No output = internal error
        lines = output.split('\n')
        last_line = lines[-1].strip()
        try:
            verdict = int(last_line)
            is_internal = verdict == 71
            return verdict, is_internal
        except ValueError:
            return None, True  # Can't parse output = internal error
    except Exception as e:
        # Timeout, exception, etc. = internal error
        return None, True


def measure_kernel_neg_bin(kernel_bin_path, corpus_dir):
    """Measure how many NEGATIVE fixtures the kernel binary ACCEPTS.
    Returns: count of unsound acceptances by the kernel.
    """
    import subprocess

    neg = sorted(f for f in os.listdir(corpus_dir) if f.startswith('n') and f.endswith('.core'))
    accepted_count = 0

    for f in neg:
        fixture_path = os.path.join(corpus_dir, f)
        kernel_verdict, is_internal = run_kernel_on_fixture(kernel_bin_path, fixture_path)

        # If kernel prints 0 and it's not internal, it accepted an unsound term
        if not is_internal and kernel_verdict == 0:
            accepted_count += 1

    return accepted_count, len(neg)


def measure_kernel_parity(kernel_bin_path, corpus_dir):
    """Measure agreement between twin and kernel on all fixtures.
    Returns: (agreement_count, total_count, internal_count, internals_list)
    """
    import subprocess

    # Get all fixtures
    neg = sorted(f for f in os.listdir(corpus_dir) if f.startswith('n') and f.endswith('.core'))
    pos = sorted(f for f in os.listdir(corpus_dir) if f.startswith('p') and f.endswith('.core'))
    fixtures = neg + pos
    total = len(fixtures)

    agreement_count = 0
    internals = []

    for f in fixtures:
        fixture_path = os.path.join(corpus_dir, f)

        # Get twin's verdict from our checker
        ok, msg = run_file(fixture_path)
        # ok=True means accepted (verdict 0), ok=False means rejected, ok=None means internal

        # Run kernel
        kernel_verdict, is_internal = run_kernel_on_fixture(kernel_bin_path, fixture_path)

        # Check agreement:
        # - Twin accepted (ok=True) and kernel printed 0
        # - Twin rejected (ok=False) and kernel printed 10..31
        # - If kernel outputs 71 or crashes, it's internal (don't count as agreement)
        if is_internal or kernel_verdict is None or kernel_verdict == 71:
            internals.append(f)
        elif ok and kernel_verdict == 0:
            agreement_count += 1  # Both accepted
        elif not ok and 10 <= kernel_verdict <= 31:
            agreement_count += 1  # Both rejected
        # else: disagreement (don't increment agreement_count)

    return agreement_count, total, len(internals), internals


def corpus(d):
    neg = sorted(f for f in os.listdir(d) if f.startswith('n') and f.endswith('.core'))
    pos = sorted(f for f in os.listdir(d) if f.startswith('p') and f.endswith('.core'))
    accepted, rejected_pos, internal = [], [], []
    print('== negatives (a kernel that accepts ANY of these is unsound)')
    for f in neg:
        ok, msg = run_file(os.path.join(d, f))
        lab = 'ACCEPTED <-- UNSOUND' if ok else ('INTERNAL <-- BROKEN' if ok is None else 'rejected')
        print('  %-28s %s  %s' % (f, lab, '' if ok else msg[:96]))
        if ok:
            accepted.append(f)
        elif ok is None:
            internal.append(f)
    print('== positives (the kernel must accept every one)')
    for f in pos:
        ok, msg = run_file(os.path.join(d, f))
        lab = 'accepted' if ok else ('INTERNAL <-- BROKEN' if ok is None else 'REJECTED <-- BUG')
        print('  %-28s %s  %s' % (f, lab, '' if ok else msg[:96]))
        if ok is None:
            internal.append(f)
        elif not ok:
            rejected_pos.append(f)
    total = len(neg) + len(pos)
    print()
    print('kernel_neg: %d accepted of %d' % (len(accepted), len(neg)))
    print('kernel_internal: %d of %d  (a CRASH is not a rejection and is never '
          'scored as one)' % (len(internal), total))
    print('kernel_pos: %d rejected of %d' % (len(rejected_pos), len(pos)))

    # Measure kernel metrics if kernel binary is available
    kernel_bin = os.environ.get('TKERNEL_BIN', './tkernel.bin')
    if os.path.exists(kernel_bin):
        # Measure kernel_neg_bin: how many negatives does the kernel binary accept?
        kernel_neg_bin, neg_total = measure_kernel_neg_bin(kernel_bin, d)
        print('kernel_neg_bin: %d accepted of %d' % (kernel_neg_bin, neg_total))

        # Measure kernel_parity: agreement between twin and kernel
        agreement, parity_total, parity_internal, internals = measure_kernel_parity(kernel_bin, d)
        if parity_internal > 0:
            print('kernel_parity: %d/%d  (kernel_internal: %d)' % (agreement, parity_total, parity_internal))
        else:
            print('kernel_parity: %d/%d' % (agreement, parity_total))
    else:
        print('kernel_neg_bin: NOT MEASURED (kernel binary absent)')
        print('kernel_parity: NOT MEASURED (kernel binary absent)')
        return 1  # Exit non-zero when instrument is absent

    return 1 if (accepted or rejected_pos or internal) else 0


if __name__ == '__main__':
    a = sys.argv[1:]
    if not a:
        sys.stderr.write(__doc__.split('Usage:')[1])
        raise SystemExit(2)
    if a[0] == '--corpus':
        raise SystemExit(corpus(a[1] if len(a) > 1 else 'bench/kernel_neg'))
    ok, msg = run_file(a[0])
    print('%s: %s' % ('ACCEPTED' if ok else 'REJECTED', msg))
    raise SystemExit(0 if ok else 1)
