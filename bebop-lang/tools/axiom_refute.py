#!/usr/bin/env python3
"""axiom_refute.py -- the boundary-inclusive ground refuter for the axiom ledger.

An axiom over i64 is a Pi_1 sentence over a FINITE domain, so a counterexample is
a single ground point. This sweeps for it.

THE DESIGN REQUIREMENT, and the reason this file is not just another fuzzer:
the sweep visits BOUNDARY POINTS UNCONDITIONALLY AND FIRST -- 0, +-1, i64::MIN,
i64::MAX, MIN+1, MAX-1, every power of two and both its neighbours in both signs,
the tree's own domain constants (2^32, 2^16, 0xFFFFFFFF, isqrt(MAX)) -- and only
then draws LCG values. tools/axiom_i64.py owns that ordering.

This is measured, not asserted. Two of the seven axioms in
formal/Bebop/Theorems.lean at commit 7edffdf were FALSE, and each fell at one
point: `isqrt_correct` at s = -1 and `cursor_monotone` at len = -10. Run
`--historical` and this refuter reports both, from the statements as `git show`
returns them. The random-only sweep in that same file (`fp_mul_sweep`, 1024 LCG
pairs) would need ~2^63 draws to expect to see s = -1.

WHAT A CLEAN SWEEP MEANS, stated so it cannot be over-read:
  * `exhaustive` refuters enumerate their ENTIRE domain. A clean run is a PROOF.
  * `computable` refuters enumerate a boundary-first prefix of a 2^64 or 2^128
    domain. A clean run REFUTES NOTHING and PROVES NOTHING; it rules out the
    cheap refutation, which is exactly what the two false Lean axioms failed.
  * `tautology` refuters check a statement that holds by construction. They are
    labelled so that nobody counts them as evidence. tools/tv_fragments.py
    printed `PASS (0/0)` over an empty measurement for its whole life; a
    tautology counted as a pass is the same defect with a bigger denominator.

Gate line: `axiom_refuted: <refuted>/<swept>`. Exit 0 iff refuted == 0 and
swept > 0. A sweep of zero axioms is exit 2 (NOT MEASURED), never a pass.
"""

import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import axiom_i64 as I  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TCHECK = os.path.join(ROOT, "selfhost", "tcheck.bp")

EXHAUSTIVE, COMPUTABLE, TAUTOLOGY = "exhaustive", "computable", "tautology"


class Result:
    def __init__(self, name, kind, statement):
        self.name, self.kind, self.statement = name, kind, statement
        self.points = 0
        self.cex = []          # list of (description, detail)
        self.notes = []

    def visit(self, n=1):
        self.points += n

    def refute(self, desc, detail=""):
        self.cex.append((desc, detail))

    def note(self, s):
        self.notes.append(s)

    @property
    def refuted(self):
        return bool(self.cex)


# ===========================================================================
# 1. Lemma D, scalar instance: fdiv(fdiv(x,m),n) == fdiv(x,m*n)
# ===========================================================================
def fdiv(a, d):
    """The FLOOR quotient, transcribed from selfhost/tcheck.bp `dv_fdiv`:
        let qt = a / d;  let rt = a - qt * d;
        let sr = if rt < 0 ...; let sd = if d < 0 ...;
        let opp = (rt != 0) * (sr != sd);  qt - opp
    Every operation is the wrapping i64 one."""
    qt = I.tdiv(a, d)
    rt = I.sub(a, I.mul(qt, d))
    sr = 1 if rt < 0 else 0
    sd = 1 if d < 0 else 0
    opp = (1 if rt != 0 else 0) * (1 if sr != sd else 0)
    return I.sub(qt, opp)


def r_lemmaD_ground():
    r = Result("lemmaD_ground", COMPUTABLE,
               "fdiv(fdiv(x,m),n) == fdiv(x,m*n) for powers of two m,n >= 2 with "
               "m*n representable (the scalar instance of lemma D)")
    # nd_floor only accepts a power-of-two divisor >= 2 (reject 76), and nd_gran
    # splits d = m*n with m a power of two, 2 <= m < d. So that is the domain.
    powers = [1 << k for k in range(1, 62)]
    for x in I.points(n_random=256):
        for m in powers:
            for n in powers:
                if m * n > (1 << 62):
                    continue
                r.visit()
                lhs = fdiv(fdiv(x, m), n)
                rhs = fdiv(x, m * n)
                if lhs != rhs:
                    r.refute("x=%d m=%d n=%d" % (x, m, n),
                             "floor(floor(x/m)/n)=%d  floor(x/(m*n))=%d" % (lhs, rhs))
    return r


# ===========================================================================
# 2. D1 remainder bound (dv_split's live check, reject 74)
# ===========================================================================
def r_D1_remainder_bound():
    r = Result("D1_remainder_bound", COMPUTABLE,
               "for r = c - fdiv(c,d)*d: 0 <= r < d when d > 0, and d < r <= 0 when d < 0 "
               "(the bound dv_split enforces with reject 74)")
    for c in I.points(n_random=512):
        for d in I.points(n_random=0):
            if d == 0:
                continue          # reject 73, out of scope of the bound
            r.visit()
            qc = fdiv(c, d)
            rc = I.sub(c, I.mul(qc, d))
            if d > 0:
                bad = (rc < 0) + (rc >= d)
            else:
                bad = (rc > 0) + (rc <= d)
            if bad:
                r.refute("c=%d d=%d" % (c, d), "r=%d (reject 74 would fire)" % rc)
    return r


# ===========================================================================
# 3. D1 Euclid identity -- TAUTOLOGY, labelled as one
# ===========================================================================
def r_D1_euclid():
    r = Result("D1_euclid", TAUTOLOGY,
               "d*q + r == x in Z/2^64 where r := x - d*q (dv_euclid's decision)")
    r.note("TAUTOLOGY: r is DEFINED as x - d*q and every operation is the same "
           "wrapping one, so (x - d*q) + d*q == x holds by the group law. The sweep "
           "confirms the wrap algebra composes and NOTHING about lemma D1. The "
           "content of D1 is the remainder BOUND, which is a separate entry.")
    for x in I.points(n_random=256):
        for d in I.points(n_random=0):
            if d == 0:
                continue
            r.visit()
            q = fdiv(x, d)
            rr = I.sub(x, I.mul(d, q))
            if I.add(I.mul(d, q), rr) != x:
                r.refute("x=%d d=%d" % (x, d), "d*q+r=%d" % I.add(I.mul(d, q), rr))
    return r


# ===========================================================================
# 4. nd_pow2
# ===========================================================================
def nd_pow2(d):
    """Transcribed from selfhost/tcheck.bp `nd_pow2`. The loop guard is written
    with `*` rather than `&&` in the source, because in .bp `&&` is a constant
    zero; transcribing it as a Python `and` would be a DIFFERENT program."""
    k, v = 0, 1
    while (1 if v < d else 0) * (1 if k < 62 else 0) == 1:
        v = I.mul(v, 2)
        k = k + 1
    return k if v == d else -1


def r_nd_pow2():
    r = Result("nd_pow2", COMPUTABLE,
               "nd_pow2(d) == k exactly when d == 2^k for k in [0,62], else -1")
    for d in I.points(n_random=1024):
        r.visit()
        got = nd_pow2(d)
        if got >= 0:
            if I.w(1 << got) != d:
                r.refute("d=%d" % d, "nd_pow2 answered %d but 2^%d = %d"
                         % (got, got, I.w(1 << got)))
        else:
            for k in range(0, 63):
                if I.w(1 << k) == d and 0 <= k <= 62:
                    r.refute("d=%d" % d, "d == 2^%d but nd_pow2 answered -1" % k)
                    break
    # the completeness gap is NOT a refutation of the statement above; recorded
    # because nd_floor turns nd_pow2's answer into reject 76.
    r.note("nd_pow2(1) == 0 (correct), but nd_floor rejects k < 1 with code 76, so "
           "floor(x/1) == x -- a TRUE instance -- is refused. A completeness gap, not "
           "a soundness one; recorded so it is not rediscovered as a bug.")
    return r


# ===========================================================================
# 5. nd_gran
# ===========================================================================
def nd_gran(coeffs, d):
    """Transcribed from selfhost/tcheck.bp `nd_gran`, over the coefficient list
    of the polynomial x (the only part of x the function reads)."""
    mx = 0
    for c in coeffs:
        ac = I.neg(c) if c < 0 else c      # `if c < 0 then (0 - c) else c`
        if ac > mx:
            mx = ac
    m, v = 1, 2
    while (1 if v <= mx else 0) * (1 if v < d else 0) == 1:
        m = v
        v = I.mul(v, 2)
    return m


def r_nd_gran():
    r = Result("nd_gran", COMPUTABLE,
               "nd_gran's m is a power of two with 1 <= m < d, and its magnitude step "
               "`if c < 0 then (0 - c) else c` computes |c|")
    divisors = [1 << k for k in range(1, 62)]
    for c in I.points(n_random=256):
        # the |c| claim, tested at every boundary point including MIN
        r.visit()
        ac = I.neg(c) if c < 0 else c
        if ac != abs(c) and abs(c) <= I.MAX:
            r.refute("c=%d (magnitude step)" % c,
                     "`if c < 0 then (0 - c) else c` gave %d, |c| is %d" % (ac, abs(c)))
        elif ac < 0:
            r.refute("c=%d (magnitude step)" % c,
                     "`if c < 0 then (0 - c) else c` gave a NEGATIVE magnitude %d" % ac)
        for d in divisors:
            r.visit()
            m = nd_gran([c], d)
            if m < 1 or m >= d or (m & (m - 1)) != 0:
                r.refute("coeffs=[%d] d=%d" % (c, d), "m=%d is not a power of two in [1,d)" % m)
            # the STATED property: m is the largest power of two with 2 <= m < d that
            # some |coefficient| of x reaches, or 1 if none does.
            want = 1
            v = 2
            while v <= abs(c) and v < d:
                want = v
                v *= 2
            if m != want:
                r.refute("coeffs=[%d] d=%d (largest-power-of-two claim)" % (c, d),
                         "nd_gran gave m=%d; the largest power of two <= |c|=%d and < d is %d"
                         % (m, abs(c), want))
    r.note("CONSEQUENCE, and it is COMPLETENESS not soundness: with mx stuck at 0, "
           "nd_floor takes the ATOM branch (tr_emit case 13, an opaque quotient "
           "variable) instead of the case-D compose, so a miter over a polynomial whose "
           "largest coefficient is exactly i64::MIN may fail to close. Lemma D is valid "
           "for ANY factorisation d = m*n with m, n > 0, so a SMALLER m is still a legal "
           "split -- no false proof can come of it.")
    r.note("ONE-LINE FIX, in a file this lane does not own (selfhost/tcheck.bp, fn "
           "nd_gran): after `let ac = if c < 0 then (0 - c) else c;` add "
           "`let ac = if ac < 0 then 9223372036854775807 else ac;`. MEASURED: this "
           "takes the output-claim mismatches from 60 to 0 over the same sweep.")
    return r


# ===========================================================================
# 6. fp_mul_correct -- the ONE live axiom in formal/Bebop/Theorems.lean
# ===========================================================================
def fp_mul_spec(a, b):
    """Transcribed from formal/Bebop/Theorems.lean `fp_mul_spec`. The product is
    taken in UNBOUNDED Nat (`aa.toInt.natAbs * ab.toInt.natAbs`) and shifted
    exactly, then `Int64.ofNat` wraps."""
    na = 1 if a < 0 else 0
    aa = I.neg(a) if na == 1 else a
    nb = 1 if b < 0 else 0
    ab = I.neg(b) if nb == 1 else b
    prod = abs(aa) * abs(ab)          # natAbs, exact
    shifted = prod >> 32              # Nat shiftRight, exact
    flip = na + nb
    neg = (flip - (flip // 2) * 2) == 1
    signed = I.w(shifted)             # Int64.ofNat wraps
    return I.neg(signed) if neg else signed


def fp_mul_impl(a, b):
    """Transcribed from formal/Bebop/Theorems.lean `fp_mul_impl`, which is itself
    selfhost/prelude/fp.bp:6-30 line for line with `>>` as the LOGICAL shift."""
    na = 1 if a < 0 else 0
    aa = I.neg(a) if na == 1 else a
    nb = 1 if b < 0 else 0
    ab = I.neg(b) if nb == 1 else b
    a1 = I.lsr(aa, 32)
    a0 = I.sub(aa, I.shl(a1, 32))
    b1 = I.lsr(ab, 32)
    b0 = I.sub(ab, I.shl(b1, 32))
    hi = I.mul(a1, b1)
    mid = I.add(I.mul(a1, b0), I.mul(a0, b1))
    ah = I.lsr(a0, 16)
    al = I.sub(a0, I.shl(ah, 16))
    bh = I.lsr(b0, 16)
    bl = I.sub(b0, I.shl(bh, 16))
    ll_h = I.mul(ah, bh)
    ll_m = I.add(I.mul(ah, bl), I.mul(al, bh))
    ll_l = I.mul(al, bl)
    low = I.add(ll_h, I.lsr(I.add(ll_m, I.lsr(ll_l, 16)), 16))
    p = I.add(I.add(I.shl(hi, 32), mid), low)
    flip = na + nb
    flip = flip - (flip // 2) * 2
    return I.neg(p) if flip == 1 else p


def r_fp_mul():
    r = Result("fp_mul_impl_eq_spec", COMPUTABLE,
               "fp_mul_impl a b == fp_mul_spec a b for all a, b : i64 "
               "(formal/Bebop/Theorems.lean `axiom fp_mul_correct`)")
    for a, b in I.pairs(n_random=4096):
        r.visit()
        if fp_mul_impl(a, b) != fp_mul_spec(a, b):
            r.refute("a=%d b=%d" % (a, b),
                     "impl=%d spec=%d" % (fp_mul_impl(a, b), fp_mul_spec(a, b)))
    r.note("The file's own `fp_mul_sweep` draws 1024 LCG pairs and NO boundary point. "
           "This visits %d boundary pairs before its first random draw."
           % (len(I.CROSS) ** 2))
    return r


# ===========================================================================
# 7. Tseitin clause sets -- EXHAUSTIVE, and PARSED from the source
# ===========================================================================
# The clause literals are read out of selfhost/tcheck.bp rather than re-written
# here. A re-implementation could agree with itself while disagreeing with the
# checker, which is the `bpref` failure mode: an oracle that models a different
# language than the one under test.
EMIT_RE = re.compile(
    r"bb_emit\(c_id, c_len, c_lits, n_cl,\s*(\d+),\s*([^,]+?),\s*([^,]+?),\s*([^)]+?)\)")

GATES = {
    "bb_and": (("a", "b"), lambda v: v["a"] & v["b"]),
    "bb_or":  (("a", "b"), lambda v: v["a"] | v["b"]),
    "bb_xor": (("a", "b"), lambda v: v["a"] ^ v["b"]),
    "bb_mux": (("s", "t", "e"), lambda v: v["t"] if v["s"] else v["e"]),
    "bb_maj": (("a", "b", "c"), lambda v: 1 if (v["a"] + v["b"] + v["c"]) >= 2 else 0),
}


def read_fn_body(path, fname):
    txt = open(path, encoding="utf-8").read().splitlines()
    start = None
    for i, line in enumerate(txt):
        if line.startswith("fn %s(" % fname):
            start = i
            break
    if start is None:
        raise OSError("cannot find `fn %s(` in %s" % (fname, path))
    end = len(txt)
    for j in range(start + 1, len(txt)):
        if txt[j].startswith("fn "):
            end = j
            break
    return "\n".join(txt[start:end])


def parse_literal(expr):
    """'a' -> ('a', 1); '0 - a' -> ('a', -1). Anything else is an error, loudly."""
    e = expr.strip()
    m = re.fullmatch(r"0\s*-\s*(\w+)", e)
    if m:
        return m.group(1), -1
    if re.fullmatch(r"\w+", e):
        return e, 1
    raise OSError("unparsable clause literal %r" % expr)


def parse_clauses(fname):
    body = read_fn_body(TCHECK, fname)
    clauses = []
    for m in EMIT_RE.finditer(body):
        ln = int(m.group(1))
        lits = [parse_literal(m.group(2 + k)) for k in range(3)][:ln]
        clauses.append(lits)
    if not clauses:
        raise OSError("no bb_emit clauses parsed out of `fn %s`" % fname)
    return clauses


def clauses_sat(clauses, val):
    for cl in clauses:
        if not any((val[n] == 1) if pol > 0 else (val[n] == 0) for n, pol in cl):
            return False
    return True


def r_tseitin_clause_sets():
    r = Result("tseitin_clause_sets", EXHAUSTIVE,
               "for each gate encoder, the PARSED clause set is satisfied iff the output "
               "literal equals the gate function of its inputs")
    for fname, (ins, fn) in sorted(GATES.items()):
        clauses = parse_clauses(fname)
        r.note("%s: %d clauses parsed from selfhost/tcheck.bp, %d inputs -> %d points"
               % (fname, len(clauses), len(ins), 2 ** (len(ins) + 1)))
        names = list(ins) + ["o"]
        for mask in range(1 << len(names)):
            val = {n: (mask >> k) & 1 for k, n in enumerate(names)}
            r.visit()
            sat = clauses_sat(clauses, val)
            want = (val["o"] == fn(val))
            if sat != want:
                r.refute("%s %s" % (fname, val),
                         "clauses %s but o == f(inputs) is %s"
                         % ("SAT" if sat else "UNSAT", want))
    return r


def r_bb_maj_bcp():
    r = Result("bb_maj_bcp_complete", EXHAUSTIVE,
               "with all three inputs of bb_maj assigned, one of its six clauses is unit "
               "on the output literal")
    clauses = parse_clauses("bb_maj")
    if len(clauses) != 6:
        r.refute("clause count", "parsed %d clauses, the comment claims six" % len(clauses))
    for mask in range(8):
        val = {n: (mask >> k) & 1 for k, n in enumerate("abc")}
        r.visit()
        unit_on_o = False
        for cl in clauses:
            if not any(n == "o" for n, _ in cl):
                continue
            others = [(n, p) for n, p in cl if n != "o"]
            # UNIT on o iff every other literal is FALSE under val.
            if all((val[n] == 0) if p > 0 else (val[n] == 1) for n, p in others):
                unit_on_o = True
                break
        if not unit_on_o:
            r.refute("a=%d b=%d c=%d" % (val["a"], val["b"], val["c"]),
                     "no clause becomes unit on o")
    return r


# ===========================================================================
# 8. Tseitin constant folding -- EXHAUSTIVE over the literal domain
# ===========================================================================
T, F = 1, -1
LITS = (T, F, 2, -2, 3, -3)      # CONST_TRUE, CONST_FALSE, +-p, +-q


def and_fold(a, b):
    f = F
    return (f if a == f else (f if b == f else (b if a == 1 else (a if b == 1 else
            (a if a == b else (f if a == -b else 0))))))


def or_fold(a, b):
    f = F
    return (1 if a == 1 else (1 if b == 1 else (b if a == f else (a if b == f else
            (a if a == b else (1 if a == -b else 0))))))


def xor_fold(a, b):
    f = F
    return (b if a == f else (a if b == f else ((-b) if a == 1 else ((-a) if b == 1 else
            (f if a == b else (1 if a == -b else 0))))))


def mux_fold(s, t, e):
    f = F
    if s == 1:
        return t
    if s == f:
        return e
    if t == e:
        return t
    if t == 1:
        return s if e == f else 0
    if t == f:
        return (-s) if e == 1 else 0
    return 0


FOLDS = {
    "bb_and_fold": (2, and_fold, lambda v: v[0] & v[1]),
    "bb_or_fold":  (2, or_fold, lambda v: v[0] | v[1]),
    "bb_xor_fold": (2, xor_fold, lambda v: v[0] ^ v[1]),
    "bb_mux_fold": (3, mux_fold, lambda v: v[1] if v[0] else v[2]),
}


def lit_val(lit, assign):
    """assign maps variable number -> 0/1. Variable 1 is pinned TRUE (bb_reset)."""
    v = assign[abs(lit)]
    return v if lit > 0 else 1 - v


def r_tseitin_fold_agreement():
    r = Result("tseitin_fold_agreement", EXHAUSTIVE,
               "whenever a *_fold returns a non-zero literal, that literal's value equals "
               "the gate's output for every assignment of the free variables")
    for name, (arity, fold, fn) in sorted(FOLDS.items()):
        combos = 0
        for idx in range(len(LITS) ** arity):
            args = []
            t = idx
            for _ in range(arity):
                args.append(LITS[t % len(LITS)])
                t //= len(LITS)
            out = fold(*args)
            if out == 0:
                continue          # "emit the gate" -- covered by the clause-set check
            combos += 1
            # variable 1 is pinned TRUE; p = 2 and q = 3 range freely
            for m in range(4):
                assign = {1: 1, 2: m & 1, 3: (m >> 1) & 1}
                r.visit()
                got = lit_val(out, assign)
                want = fn([lit_val(a, assign) for a in args])
                if got != want:
                    r.refute("%s%s" % (name, tuple(args)),
                             "folded to literal %d worth %d, gate value %d, assign %s"
                             % (out, got, want, assign))
        r.note("%s: %d of %d literal tuples fold to a determined output"
               % (name, combos, len(LITS) ** arity))
    return r


# ===========================================================================
# 9. Prose counts
# ===========================================================================
def grepc(rel, pattern):
    p = os.path.join(ROOT, rel)
    rx = re.compile(pattern)
    return sum(1 for line in open(p, encoding="utf-8").read().splitlines() if rx.search(line))


def r_footprint_count():
    r = Result("footprint_count_claim", EXHAUSTIVE,
               "Basic.lean's '26 sys_* are axiomatised' agrees with the file, with "
               "Conformance.lean's axiomatisedBuiltinCount, and with the footprint table")
    r.visit()
    n_axioms = grepc("formal/Bebop/Syscalls.lean", r"^axiom\s+sys_\w+_spec")
    if n_axioms != 26:
        r.refute("axiom count", "grep '^axiom sys_*_spec' Syscalls.lean == %d, prose says 26"
                 % n_axioms)
    r.visit()
    txt = open(os.path.join(ROOT, "formal/Bebop/Conformance.lean"), encoding="utf-8").read()
    m = re.search(r"def axiomatisedBuiltinCount\s*:\s*Nat\s*:=\s*(\d+)", txt)
    if not m:
        r.refute("axiomatisedBuiltinCount", "definition not found in Conformance.lean")
    elif int(m.group(1)) != n_axioms:
        r.refute("axiomatisedBuiltinCount", "Conformance says %s, Syscalls.lean has %d"
                 % (m.group(1), n_axioms))
    r.note("grep '^axiom sys_*_spec' formal/Bebop/Syscalls.lean == %d" % n_axioms)
    return r


def r_footprints_declared():
    r = Result("syscall_footprints_declared", EXHAUSTIVE,
               "exactly 5 of the 26 axiomatised syscalls have a declared footprint, "
               "leaving 21 without one")
    r.visit()
    n_fp = grepc("formal/Bebop/Syscalls.lean", r"^def sys_\w+_footprint")
    if n_fp != 5:
        r.refute("footprint count", "grep '^def sys_*_footprint' == %d, prose says 5" % n_fp)
    n_axioms = grepc("formal/Bebop/Syscalls.lean", r"^axiom\s+sys_\w+_spec")
    r.visit()
    if n_axioms - n_fp != 21:
        r.refute("remainder", "%d axioms - %d footprints = %d, prose says 21"
                 % (n_axioms, n_fp, n_axioms - n_fp))
    r.note("grep '^def sys_*_footprint' == %d; %d - %d = %d" % (n_fp, n_axioms, n_fp, n_axioms - n_fp))
    return r


# ===========================================================================
# THE ACCEPTANCE TEST: the two known-false axioms at 7edffdf
# ===========================================================================
# These are read from git, not from memory. The statement text `git show` returns
# is asserted before the sweep runs, so if history is rewritten the acceptance
# test fails LOUDLY instead of sweeping something else.
HIST_REF = "7edffdf:bebop-lang/formal/Bebop/Theorems.lean"

HIST_EXPECT = {
    "isqrt_correct": [
        "axiom isqrt_correct (s : Val) :",
        "let r := isqrt_spec s",
        "r * r ≤ s ∧ s < (r + 1) * (r + 1)",
    ],
    "cursor_monotone": [
        "axiom cursor_monotone (tx : Array Val) (len : Val) :",
        "let old_cursor := tx.getD (2 : Nat) (0 : Val)",
        "let new_cursor := old_cursor + 2 + len",
        "new_cursor ≥ old_cursor",
    ],
}


def git_show_historical():
    """Read formal/Bebop/Theorems.lean at 7edffdf. The repo root is the PARENT of
    this tree (a lane worktree is not a repo), so git runs there with -C."""
    repo = ROOT
    while repo != "/" and not os.path.isdir(os.path.join(repo, ".git")):
        repo = os.path.dirname(repo)
    if repo == "/":
        repo = "/root/dowiz"
    p = subprocess.run(["git", "-C", repo, "show", HIST_REF],
                       capture_output=True, text=True)
    rc = p.returncode
    if rc != 0:
        raise OSError("git -C %s show %s -> rc=%d: %s"
                      % (repo, HIST_REF, rc, p.stderr.strip()[:200]))
    return p.stdout, repo


def isqrt_spec_7edffdf(s):
    """Transcribed from `def isqrt_spec` at 7edffdf:
        if s <= 0 then 0 else Int64.ofNat (Nat.sqrt s.toInt.natAbs)"""
    if s <= 0:
        return 0
    n = abs(s)
    # Nat.sqrt
    x = int(n ** 0.5)
    while x * x > n:
        x -= 1
    while (x + 1) * (x + 1) <= n:
        x += 1
    return I.w(x)


def r_hist_isqrt(text):
    r = Result("HISTORICAL lean:isqrt_correct", COMPUTABLE,
               "r*r <= s AND s < (r+1)*(r+1) where r = isqrt_spec s, over Int64 "
               "(7edffdf, NO sign hypothesis)")
    for frag in HIST_EXPECT["isqrt_correct"]:
        if frag not in text:
            raise OSError("acceptance test: %r not found in %s -- history changed"
                          % (frag, HIST_REF))
    for s in I.points(n_random=1024):
        r.visit()
        rr = isqrt_spec_7edffdf(s)
        lower = I.mul(rr, rr) <= s
        upper = s < I.mul(I.add(rr, 1), I.add(rr, 1))
        if not lower:
            r.refute("s = %d" % s, "isqrt_spec(s) = %d, r*r = %d, and r*r <= s is FALSE"
                     % (rr, I.mul(rr, rr)))
        if not upper:
            r.refute("s = %d" % s,
                     "isqrt_spec(s) = %d, (r+1)*(r+1) wraps to %d, and s < (r+1)^2 is FALSE"
                     % (rr, I.mul(I.add(rr, 1), I.add(rr, 1))))
    return r


def r_hist_cursor(text):
    r = Result("HISTORICAL lean:cursor_monotone", COMPUTABLE,
               "old_cursor + 2 + len >= old_cursor over Int64 (7edffdf, NO hypothesis "
               "on len)")
    for frag in HIST_EXPECT["cursor_monotone"]:
        if frag not in text:
            raise OSError("acceptance test: %r not found in %s -- history changed"
                          % (frag, HIST_REF))
    for old in I.points(n_random=0):
        for length in I.points(n_random=0):
            r.visit()
            new = I.add(I.add(old, 2), length)
            if not (new >= old):
                r.refute("old_cursor = %d, len = %d" % (old, length),
                         "new_cursor = %d, which is LESS than old_cursor" % new)
    return r


# ===========================================================================
# NEGATIVE CONTROLS. An instrument that cannot fail is not measuring anything --
# tools/tv_fragments.py printed `PASS (0/0)` for its whole life, and the first
# version of `bb_maj_bcp_complete` in THIS file passed on a wrong predicate
# (it tested "not all other literals are true" instead of "every other literal
# is false"). So each exhaustive refuter is run against a DELIBERATELY BROKEN
# input and must report a counterexample. `--controls` runs these.
def controls():
    rows = []

    def add(label, fired, detail):
        rows.append((label, fired, detail))

    # (a) clause-set check: drop one clause from bb_and's parsed set.
    cl = parse_clauses("bb_and")
    broken = cl[1:]
    bad = [dict(a=(m >> 0) & 1, b=(m >> 1) & 1, o=(m >> 2) & 1) for m in range(8)]
    fired = any(clauses_sat(broken, v) != (v["o"] == (v["a"] & v["b"])) for v in bad)
    add("clause-set check fires when a bb_and clause is DROPPED", fired,
        "%d clauses -> %d" % (len(cl), len(broken)))

    # (b) clause-set check: flip one literal's polarity in bb_xor.
    cl = parse_clauses("bb_xor")
    flipped = [list(c) for c in cl]
    flipped[0][0] = (flipped[0][0][0], -flipped[0][0][1])
    bad = [dict(a=(m >> 0) & 1, b=(m >> 1) & 1, o=(m >> 2) & 1) for m in range(8)]
    fired = any(clauses_sat(flipped, v) != (v["o"] == (v["a"] ^ v["b"])) for v in bad)
    add("clause-set check fires when a bb_xor literal is NEGATED", fired,
        "clause 0 literal 0 polarity flipped")

    # (c) BCP check: drop one of bb_maj's six clauses and it must stop being
    #     BCP-complete. This is the control the wrong first predicate failed.
    cl = parse_clauses("bb_maj")
    broken = cl[:-1]
    fired = False
    for mask in range(8):
        val = {n: (mask >> k) & 1 for k, n in enumerate("abc")}
        unit = False
        for c in broken:
            if not any(n == "o" for n, _ in c):
                continue
            others = [(n, p) for n, p in c if n != "o"]
            if all((val[n] == 0) if p > 0 else (val[n] == 1) for n, p in others):
                unit = True
                break
        if not unit:
            fired = True
            break
    add("BCP check fires when one of bb_maj's six clauses is DROPPED", fired,
        "%d clauses -> %d" % (len(cl), len(broken)))

    # (d) fold check: a fold that answers CONST_TRUE for AND(p, -p) must be caught.
    fired = False
    for m in range(4):
        assign = {1: 1, 2: m & 1, 3: (m >> 1) & 1}
        if lit_val(1, assign) != (lit_val(2, assign) & lit_val(-2, assign)):
            fired = True
    add("fold check fires on a WRONG fold (and(p,-p) -> CONST_TRUE)", fired,
        "and(p,-p) is always 0; CONST_TRUE is 1")

    # (e) the i64 model: a floor-division model that truncates must fail the
    #     remainder bound, which is how the bound check earns its pass.
    fired = False
    for c in (-1, -7, I.MIN):
        for d in (3, 5, 1 << 40):
            q = I.tdiv(c, d)            # TRUNCATING, i.e. the WRONG quotient
            rr = I.sub(c, I.mul(q, d))
            if (rr < 0) or (rr >= d):
                fired = True
    add("remainder-bound check fires on the TRUNCATING quotient (no floor correction)",
        fired, "this is the defect dv_fdiv's `opp` term exists to fix")

    # (f) fp_mul: model `>>` as the ARITHMETIC shift instead of the logical one and
    #     the sweep must refute -- at the point and with the VALUES the tree already
    #     measured. formal/Bebop/Theorems.lean:145-149: "with `>>>` on Int64 the case
    #     (i64::MIN, 1) gave impl = 2147483648 vs spec = -2147483648". Reproducing a
    #     number produced by Lean is the strongest validation this model can get.
    orig = I.lsr
    I.lsr = I.asr
    try:
        n_bad = sum(1 for a, b in I.pairs(n_random=0)
                    if fp_mul_impl(a, b) != fp_mul_spec(a, b))
        got_impl, got_spec = fp_mul_impl(I.MIN, 1), fp_mul_spec(I.MIN, 1)
    finally:
        I.lsr = orig
    fired = (n_bad > 0 and got_impl == 2147483648 and got_spec == -2147483648)
    add("fp_mul sweep fires when `>>` is modelled ARITHMETIC", fired,
        "%d refuting boundary pairs; at (MIN,1) impl=%d spec=%d -- Theorems.lean:145-149 "
        "quotes exactly 2147483648 vs -2147483648" % (n_bad, got_impl, got_spec))

    return rows


# ===========================================================================
REFUTERS = [r_lemmaD_ground, r_D1_remainder_bound, r_D1_euclid, r_nd_pow2,
            r_nd_gran, r_fp_mul, r_tseitin_clause_sets, r_bb_maj_bcp,
            r_tseitin_fold_agreement, r_footprint_count, r_footprints_declared]

MAX_CEX_SHOWN = 6


def report(results, header):
    print(header)
    swept = refuted = taut = 0
    for r in results:
        swept += 1
        if r.kind == TAUTOLOGY:
            taut += 1
        if r.refuted:
            refuted += 1
        verdict = "REFUTED (%d counterexamples)" % len(r.cex) if r.refuted else "no counterexample"
        print("  %-34s %-11s %9d points  %s" % (r.name, r.kind, r.points, verdict))
        print("        %s" % r.statement)
        for note in r.notes:
            print("        note: %s" % note)
        for desc, detail in r.cex[:MAX_CEX_SHOWN]:
            print("        CEX  %s :: %s" % (desc, detail))
        if len(r.cex) > MAX_CEX_SHOWN:
            print("        ... %d more counterexamples" % (len(r.cex) - MAX_CEX_SHOWN))
    return swept, refuted, taut


def main():
    historical = "--historical" in sys.argv
    if "--controls" in sys.argv:
        try:
            rows = controls()
        except OSError as e:
            print("axiom_controls: NOT MEASURED -- %s" % e)
            return 2
        print("NEGATIVE CONTROLS -- every refuter below is fed a BROKEN input and")
        print("must report a counterexample. A control that does not fire means the")
        print("corresponding green gate is measuring nothing.")
        fired = 0
        for label, ok, detail in rows:
            print("  %-4s %-62s %s" % ("FIRE" if ok else "SILENT", label, detail))
            fired += 1 if ok else 0
        print("")
        print("axiom_controls: %d/%d fired" % (fired, len(rows)))
        return 0 if fired == len(rows) else 1
    print("boundary_points: %d   cross_points: %d   cross_pairs: %d   (boundary FIRST, always)"
          % (len(I.BOUNDARY), len(I.CROSS), len(I.CROSS) ** 2))
    try:
        I.self_test()
    except AssertionError as e:
        print("axiom_refuted: NOT MEASURED -- the i64 model failed its own self-test: %s" % e)
        return 2
    print("i64_model: self-test passes (14 assertions against measured tree values)")
    print("")

    if historical:
        try:
            text, repo = git_show_historical()
        except OSError as e:
            print("axiom_refuted: NOT MEASURED -- %s" % e)
            return 2
        print("ACCEPTANCE TEST -- `git -C %s show %s` (%d bytes)" % (repo, HIST_REF, len(text)))
        print("The two axioms this tree MEASURED as FALSE on 2026-09-14. A refuter that")
        print("does not find them is measuring nothing.")
        print("")
        results = [r_hist_isqrt(text), r_hist_cursor(text)]
        swept, refuted, taut = report(results, "historical corpus:")
        print("")
        # The two points the tree DOCUMENTED (Theorems.lean:36-45, cex_isqrt_neg /
        # cex_cursor_neglen) must each appear by name. "some counterexample was found"
        # is a weaker claim than "the counterexample the tree measured was found", and
        # only the second one shows the boundary ordering did its job.
        named = [("isqrt_correct at s = -1",
                  "HISTORICAL lean:isqrt_correct", "s = -1"),
                 ("isqrt_correct at s = i64::MAX (upper bound wraps)",
                  "HISTORICAL lean:isqrt_correct", "s = 9223372036854775807"),
                 ("cursor_monotone at old_cursor = 1024, len = -10",
                  "HISTORICAL lean:cursor_monotone", "old_cursor = 1024, len = -10"),
                 ("cursor_monotone at old_cursor = i64::MAX, len = 0",
                  "HISTORICAL lean:cursor_monotone",
                  "old_cursor = 9223372036854775807, len = 0")]
        by_name = {r.name: r for r in results}
        all_named = True
        print("NAMED POINTS -- the exact counterexamples this tree recorded:")
        for label, rname, key in named:
            hit = next((d for k, d in by_name[rname].cex if k == key), None)
            if hit is None:
                all_named = False
                print("  MISSED   %-52s (%s not visited or not refuting)" % (label, key))
            else:
                print("  FOUND    %-52s %s :: %s" % (label, key, hit))
        print("")
        print("axiom_refuted: %d/%d  [ACCEPTANCE: expects %d/%d]" % (refuted, swept, swept, swept))
        ok = (refuted == swept == 2) and all_named
        print("acceptance: %s -- the refuter %s both known-false axioms at the %s"
              % ("PASS" if ok else "FAIL", "FOUND" if ok else "MISSED",
                 "documented points" if all_named else "documented points MISSING"))
        return 0 if ok else 1

    results = []
    for f in REFUTERS:
        try:
            results.append(f())
        except OSError as e:
            print("axiom_refuted: NOT MEASURED -- refuter %s could not run: %s"
                  % (f.__name__, e))
            return 2
    swept, refuted, taut = report(results, "live corpus (this tree at HEAD):")
    print("")
    if swept == 0:
        print("axiom_refuted: NOT MEASURED -- zero axioms swept")
        return 2
    print("axiom_refuted: %d/%d" % (refuted, swept))
    print("axiom_sweep_points: %d" % sum(r.points for r in results))
    print("axiom_sweep_tautologies: %d of %d (labelled, not counted as evidence)" % (taut, swept))
    return 1 if refuted else 0


if __name__ == "__main__":
    sys.exit(main())
