#!/usr/bin/env python3
"""axiom_ledger.py -- the axiom census: everything the proof layer ASSUMES.

There was no axiom census in this tree. `grep deps selfhost/tcheck*.bp
tools/kcheck.py` returns nothing, and the trusted base of the proof layer is
spread over three languages: `axiom` declarations in formal/*.lean, rules that
selfhost/tcheck.bp states in a comment and then relies on, the kernel's
treatment of `axiom n : T` as a name with no body, and prose counts in
formal/Bebop/Basic.lean. This file writes that base down.

EVERY ENTRY IS ANCHORED BY CONTENT, NEVER BY LINE NUMBER, and the line number is
RE-DERIVED on every run. This is not fastidiousness: the brief that commissioned
this file cited `selfhost/tcheck.bp:2222` for lemma D and `:2370` for the ring
rule, and BOTH were wrong -- lemma D's text is at 2423 and the ring rule at
1934 in the tree as checked out. A ledger whose locations are stale is worse
than no ledger, because a reader who follows one and finds unrelated code stops
trusting the rest. So: an anchor that no longer matches is a LOUD FAILURE
(exit 2, `NOT MEASURED`), never a silently skipped row.

CLASSIFICATION, which is the part that decides what work exists:
  computable   -- the statement is a Pi_1 sentence over a FINITE domain of
                  ground i64 values, so a counterexample is a single point and
                  tools/axiom_refute.py sweeps for it.
  exhaustive   -- computable AND the domain is small enough to enumerate
                  completely, so a clean sweep is a PROOF, not evidence.
  not          -- the statement quantifies over something a sweep cannot
                  enumerate (all polynomials, all states, satisfiability of a
                  composed circuit), or it is not a proposition at all but a
                  property of the checker's machinery. Each of these carries a
                  `discharge` field saying what WOULD settle it. Inventing a
                  sweep for one of these would be the `tv_fragments.py` defect:
                  that instrument printed `PASS (0/0)` for its entire life.

Output: `lean_axioms: <n>` (a RATCHET -- may only go down as axioms become
theorems), `bp_rule_axioms: <n>`, `kernel_axiom_decls: <n>`, then one line per
entry. `--json` emits the machine-readable ledger.
"""

import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

COMPUTABLE = "computable"
EXHAUSTIVE = "exhaustive"
NOT = "not"


def E(eid, path, anchor, statement, kind, comp, refuter=None, discharge=None, note=None):
    return dict(id=eid, path=path, anchor=anchor, statement=statement, kind=kind,
                computable=comp, refuter=refuter, discharge=discharge, note=note)


# ---------------------------------------------------------------------------
# HAND-WRITTEN ENTRIES: assumptions stated in PROSE that no `grep axiom` finds.
# These are the ones a census must be written for; the `axiom` keyword entries
# below are derived mechanically instead.
# ---------------------------------------------------------------------------
STATIC = [
    # --- selfhost/tcheck.bp: the F6 certificate checker's own axioms ---------
    E("lemmaD_symbolic",
      "selfhost/tcheck.bp",
      r"^//\s+floor\(floor\(x/m\)/n\) == floor\(x/\(m\*n\)\)\s+for m, n > 0\.",
      "floor(floor(x/m)/n) == floor(x/(m*n)) for m, n > 0, for ARBITRARY integer "
      "polynomials x over Z/2^64",
      "bp_rule_axiom", NOT,
      discharge="A Lean theorem over Int (the scalar case) plus a coefficient-wise "
                "lifting argument for the polynomial case. The file says so itself: "
                "'lemma D is an AXIOM of this rule ... It is not proved here.'",
      note="The tree's own honesty note is the anchor's neighbour. What IS checked "
           "today: consistent application (hash-consing), agreement with ground truth "
           "where computable, and that the miter closes -- none of which is the lemma."),

    E("lemmaD_ground",
      "selfhost/tcheck.bp",
      r"^fn nd_floor\(R: \[i64\], x: i64, d: i64, out: i64\) -> i64 \{",
      "The SCALAR INSTANCE of lemma D: for all x : i64 and all m, n with m, n >= 2 "
      "and m*n representable, fdiv(fdiv(x, m), n) == fdiv(x, m*n)",
      "bp_rule_axiom", COMPUTABLE, refuter="lemmaD_ground",
      note="A necessary condition of the symbolic lemma, and the one a sweep CAN see. "
           "Refuting it would refute the symbolic lemma; passing it does not prove it."),

    E("D1_remainder_bound",
      "selfhost/tcheck.bp",
      r"^\s+// live bound check -- this FIRES if the floor correction above is ever wrong",
      "For every coefficient c and every non-zero divisor d, the split r = c - fdiv(c,d)*d "
      "satisfies 0 <= r < d when d > 0, and d < r <= 0 when d < 0",
      "bp_rule_axiom", COMPUTABLE, refuter="D1_remainder_bound",
      note="dv_split records reject 74 if this fails at run time, so the assumption is "
           "GUARDED in the checker -- but the guard has never been swept over the "
           "boundary, and reject 74 firing would be a checker outage, not a proof."),

    E("D1_euclid",
      "selfhost/tcheck.bp",
      r"^fn dv_euclid\(R: \[i64\], x: i64, d: i64, q: i64, r: i64, s1: i64, s2: i64\) -> i64 \{",
      "x == d*q + r in Z/2^64, where q = fdiv(x,d) and r = x - d*q, for every x and "
      "every non-zero d",
      "bp_rule_axiom", COMPUTABLE, refuter="D1_euclid"),

    E("D2_order_transfer",
      "selfhost/tcheck.bp",
      r"^fn dv_transfer\(R: \[i64\], q: i64, d: i64, x: i64, s1: i64, s2: i64, out: i64\) -> i64 \{",
      "For d > 0, `q <= x fdiv d` may be rewritten to the division-free `q*d <= x`",
      "bp_rule_axiom", NOT,
      discharge="A Lean theorem over Int. The rewrite is about the ORDER on symbolic "
                "polynomials, and a polynomial normal form has no order -- the function "
                "itself only reports whether the elimination is LICENSED.",
      note="Reject 75 when d <= 0. The license condition is checked; the lemma the "
           "license appeals to is not."),

    E("ring_rule",
      "selfhost/tcheck.bp",
      r"^// THE rule: two word-level terms are equal over Z/2\^64 iff their canonical",
      "Two word-level terms are equal over Z/2^64 IFF their canonical polynomials are "
      "identical (rg_eq)",
      "bp_rule_axiom", NOT,
      discharge="Soundness (identical normal forms => equal) is the easy half and needs "
                "a canonicality proof for rg_canon. COMPLETENESS (equal => identical "
                "normal forms) is FALSE in general over Z/2^64 -- x^64 and 0 agree "
                "nowhere-relevant but, more cheaply, 2^63*x and 2^63*x^2 agree on every "
                "i64 while their canonical polynomials differ. So the `iff` in the "
                "comment overstates what the rule can have; what the rule NEEDS is only "
                "the soundness direction, and the comment should say so.",
      note="FINDING. rg_eq answers 0 ('not proved') on any internal rejection, so an "
           "incomplete rule cannot manufacture a proof -- the overstatement is in the "
           "COMMENT, not the code. But the comment is the trust document."),

    E("nd_pow2",
      "selfhost/tcheck.bp",
      r"^fn nd_pow2\(d: i64\) -> i64 \{",
      "nd_pow2(d) == k exactly when d == 2^k for some k in [0,62], and -1 otherwise",
      "bp_rule_axiom", COMPUTABLE, refuter="nd_pow2"),

    E("nd_gran",
      "selfhost/tcheck.bp",
      r"^fn nd_gran\(R: \[i64\], x: i64, d: i64\) -> i64 \{",
      "nd_gran returns the largest power of two m with 2 <= m < d that some |coefficient| "
      "of x reaches, or 1 when none does; in particular m is always a power of two and "
      "1 <= m < d",
      "bp_rule_axiom", COMPUTABLE, refuter="nd_gran",
      note="Its magnitude step is `let ac = if c < 0 then (0 - c) else c`, and 0 - MIN "
           "wraps to MIN. The refuter tests the |c| claim at that point."),

    E("tseitin_clause_sets",
      "selfhost/tcheck.bp",
      r"^// --- bit-blaster and Tseitin encoder ---",
      "For each gate encoder (bb_and, bb_or, bb_xor, bb_mux, bb_maj), the emitted clause "
      "set is satisfied by an assignment IFF the output literal equals the gate function "
      "of the inputs",
      "bp_rule_axiom", EXHAUSTIVE, refuter="tseitin_clause_sets",
      note="FINDING. tcheck.bp:2424 calls these axioms and the commissioning inventory "
           "classed them as 'about satisfiability' and therefore unsweepable. They are "
           "not: a fixed gate over k inputs plus one output is a Pi_1 sentence over "
           "2^(k+1) <= 16 points, so EXHAUSTIVE enumeration decides it. The refuter "
           "PARSES the clause literals out of the source -- it does not re-implement "
           "the encoder, so a transcription error cannot make it pass."),

    E("tseitin_fold_agreement",
      "selfhost/tcheck.bp",
      r"^// --- Tseitin-level constant folding ---",
      "Each *_fold returns a literal that is EQUAL to the gate's output whenever it "
      "returns non-zero, over the literal domain {CONST_TRUE, CONST_FALSE, p, -p, q, -q}",
      "bp_rule_axiom", EXHAUSTIVE, refuter="tseitin_fold_agreement",
      note="A folded gate costs no variable and no clause, so a WRONG fold is a silent "
           "soundness hole that the clause-set check above cannot see -- the clauses are "
           "never emitted."),

    E("bb_maj_bcp_complete",
      "selfhost/tcheck.bp",
      r"^// majority-of-three -- the carry OUT of a full adder\.",
      "With all three inputs of bb_maj assigned, one of its six clauses is always unit "
      "on the output literal (BCP-complete)",
      "bp_rule_axiom", EXHAUSTIVE, refuter="bb_maj_bcp_complete",
      note="Claimed in the comment, never checked. BCP completeness is what lets the "
           "checker decide a ground circuit without search."),

    # --- selfhost/tcheck_kernel.bp: the kernel's trusted surface ------------
    E("kernel_axiom_no_body",
      "selfhost/tcheck_kernel.bp",
      r"^// k_def_add / k_def_find: the constant table\. `axiom n : T` records a name with no",
      "`axiom n : T` records a name whose body is absent; k_def_body returns 0 and delta "
      "reduction stops there",
      "kernel_mechanism", NOT,
      discharge="Not a proposition. It is the kernel's DEFINITION of an axiom, and the "
                "thing to audit is not its truth but the SIZE of the set of names "
                "introduced this way in any accepted file -- which is what "
                "`kernel_axiom_decls` below counts.",
      note="This is the entry the census exists for: it is trusted surface with no "
           "`axiom` keyword anywhere near a proposition."),

    E("kernel_ops_opaque",
      "selfhost/tcheck_kernel.bp",
      r"^// `absv` are axioms, so `mul \(num 2\) \(num 3\)` does not reduce to `num 6`\.",
      "The arithmetic operators in a kernel statement (mul, fdiv, absv) are axioms, so "
      "`mul (num 2) (num 3)` does NOT reduce to `num 6`",
      "kernel_mechanism", NOT,
      discharge="One primitive-reduction rule per operator inside the kernel, each with "
                "its own negative fixture. Until then a kernel-accepted arithmetic "
                "STATEMENT carries no arithmetic MEANING, and a fixture that 'checks' "
                "such a statement has checked a shape.",
      note="FINDING with reach: ROADMAP F9 counts `p20_fp_mul_statement -> 0` (78 nodes, "
           "accepted) as progress. With the operators opaque, acceptance of that "
           "statement is independent of whether the statement is TRUE."),

    E("kernel_numtype_nominated",
      "selfhost/tcheck_kernel.bp",
      r"^// THE NUMERAL TYPE IS NOMINATED, NOT BUILT IN\.",
      "Two numerals are convertible iff their VALUES are equal; a numeral inhabits the "
      "nominated numeral type, and one with nothing nominated is refused (32)",
      "kernel_mechanism", NOT,
      discharge="Sound here only because `num` is a ONE-FIELD node -- k_conv's leaf path "
                "compares k_a and ignores k_b. A second one-field node kind added to that "
                "set would be fine; a TWO-field one would silently ignore half of itself. "
                "What would discharge it: a mechanical check that every node kind reaching "
                "k_conv's leaf path has arity <= 1.",
      note="The comment states the hazard. Nothing enforces it."),

    # --- formal/: counts asserted in prose ---------------------------------
    E("footprint_count_claim",
      "formal/Bebop/Basic.lean",
      r"^\s+26 sys_\* are axiomatised; 7 threading builtins are OUT OF",
      "Exactly 26 sys_* are axiomatised, and 7 threading builtins are outside the "
      "single-thread semantics",
      "prose_count", COMPUTABLE, refuter="footprint_count_claim",
      note="A count in prose is a Pi_0 sentence and therefore the cheapest thing in this "
           "ledger to refute. Cross-checked against `grep -c '^axiom sys_'`, "
           "Conformance.lean's axiomatisedBuiltinCount, and the footprint table."),

    E("syscall_footprints_declared",
      "formal/Bebop/Syscalls.lean",
      r"^-- 6\. Footprint table \(for 5 syscalls with declared footprints\)",
      "Only 5 of the 26 axiomatised syscalls have a declared footprint; the remaining 21 "
      "are axiomatised without one",
      "prose_count", COMPUTABLE, refuter="syscall_footprints_declared"),

    E("dispatch_syscall_none",
      "formal/Bebop/Syscalls.lean",
      r"^-- WHAT CHANGED, 2026-09-14, and the rule it follows\.",
      "13 of the 26 syscalls are now MODELLED in dispatchSyscall, but NONE of them is "
      "proven against its `_spec` axiom, so the axioms are still CONNECTED to nothing",
      "trust_gap", NOT,
      discharge="Prove each modelled dispatchSyscall arm against its `_spec` axiom. "
                "Modelling narrowed this gap but did not close it: a modelled arm and an "
                "axiom that are never related are two independent claims, not one checked "
                "one.",
      note="FINDING, NARROWED 2026-09-14 (F4, `82866a6`) and re-anchored in the same "
           "commit because the old anchor asserted `NOTHING is modelled yet`, which the "
           "same change made false -- the ledger REFUSED to measure rather than reporting "
           "a stale number, which is the behaviour this instrument is for. What changed: "
           "dispatchSyscall was `none` for all 26; 13 now have a declared effect and a "
           "declared value, and the other 13 stay `none` and are NAMED in the STUCK "
           "message. What did NOT change: no `_spec` axiom is referenced outside its own "
           "declaration in any hand-written `.lean` file, so the specs remain unreachable "
           "from any evaluation and the trust gap stands."),
]


# ---------------------------------------------------------------------------
# DERIVED ENTRIES: anything the `axiom` keyword itself finds. Counted, never
# listed by hand, so the numbers cannot drift from the files.
# ---------------------------------------------------------------------------
LEAN_AXIOM_RE = re.compile(r"^axiom\s+(\w+)")
CORE_AXIOM_RE = re.compile(r"^axiom\s+(\S+)\s*:\s*(\S+)")

# Shape classification for a Lean axiom body. An axiom whose body's top-level
# quantifier is EXISTENTIAL is Sigma_1: it is satisfied by a single witness, so it
# constrains nothing and (if a witness exists) is a THEOREM waiting to be proved,
# not an assumption. That distinction is invisible to `grep axiom` and is the
# single largest finding in this census.
EXISTS_RE = re.compile(r"^\s*(?:∃|\\exists)")


def lean_axioms(paths):
    out = []
    for rel in paths:
        p = os.path.join(ROOT, rel)
        if not os.path.exists(p):
            raise OSError("ledger source missing: %s" % rel)
        lines = open(p, encoding="utf-8").read().splitlines()
        for i, line in enumerate(lines):
            m = LEAN_AXIOM_RE.match(line)
            if not m:
                continue
            # body = the lines after the declaration head up to the next blank or decl
            body = []
            for j in range(i + 1, min(i + 40, len(lines))):
                nxt = lines[j]
                if not nxt.strip():
                    break
                if nxt.startswith(("axiom ", "theorem ", "def ", "/--", "/-", "-- =")):
                    break
                body.append(nxt)
            btxt = "\n".join(body)
            shape = "exists" if EXISTS_RE.match(btxt) else "forall"
            out.append((rel, i + 1, m.group(1), shape, " ".join(b.strip() for b in body)[:300]))
    return out


def core_axiom_decls():
    """`axiom n : T` declarations in kernel corpus files -- names with no body."""
    out = []
    for dirpath, dirnames, filenames in os.walk(os.path.join(ROOT, "bench")):
        dirnames[:] = [d for d in dirnames if d not in (".git", "__pycache__")]
        for fn in sorted(filenames):
            if not fn.endswith(".core"):
                continue
            p = os.path.join(dirpath, fn)
            rel = os.path.relpath(p, ROOT)
            for i, line in enumerate(open(p, encoding="utf-8").read().splitlines()):
                m = CORE_AXIOM_RE.match(line)
                if m:
                    out.append((rel, i + 1, m.group(1), m.group(2)))
    return out


def resolve(entry):
    """Re-derive path:line from the content anchor. Raises on a stale anchor."""
    p = os.path.join(ROOT, entry["path"])
    if not os.path.exists(p):
        raise OSError("ledger anchor file missing: %s (entry %s)" % (entry["path"], entry["id"]))
    rx = re.compile(entry["anchor"])
    hits = [i + 1 for i, line in enumerate(open(p, encoding="utf-8").read().splitlines())
            if rx.search(line)]
    if not hits:
        raise OSError("STALE ANCHOR: entry %s no longer matches in %s -- pattern %r"
                      % (entry["id"], entry["path"], entry["anchor"]))
    if len(hits) > 1:
        raise OSError("AMBIGUOUS ANCHOR: entry %s matches %d lines in %s (%s)"
                      % (entry["id"], len(hits), entry["path"], hits))
    return hits[0]


def build():
    """The whole ledger. Raises OSError on any stale anchor -- NOT MEASURED, never a skip."""
    entries = []
    for e in STATIC:
        e = dict(e)
        e["line"] = resolve(e)
        entries.append(e)

    lean_paths = ["formal/Bebop/Theorems.lean", "formal/Bebop/Syscalls.lean"]
    for rel, line, name, shape, body in lean_axioms(lean_paths):
        if shape == "exists":
            comp, refuter = NOT, None
            discharge = ("Sigma_1, not Pi_1: the body is an EXISTENTIAL, so it is "
                         "satisfied by one witness and constrains nothing. Witness "
                         "(result := the first listed errno, s' := s) satisfies every "
                         "conjunct, so this is PROVABLE today and should be a theorem. "
                         "No sweep applies -- there is no counterexample to find.")
            note = ("FINDING. A counterexample sweep cannot refute an existential. The "
                    "cost of this axiom is not unsoundness but DEAD TRUSTED SURFACE: it "
                    "is counted as an assumption and asserts nothing.")
        else:
            comp = COMPUTABLE
            refuter = "fp_mul_impl_eq_spec" if name == "fp_mul_correct" else None
            discharge = ("A limb-decomposition proof with a carry bound on "
                         "`ll_m + (ll_l >>> 16)`; bv_decide cannot take it (the spec side "
                         "multiplies in unbounded Nat)." if name == "fp_mul_correct" else None)
            note = None
        entries.append(E("lean:" + name, rel, r"^axiom\s+" + re.escape(name) + r"\b",
                         body, "lean_axiom", comp, refuter, discharge, note) |
                       {"line": line, "shape": shape})

    cores = core_axiom_decls()
    return entries, cores


def main():
    want_json = "--json" in sys.argv
    try:
        entries, cores = build()
    except OSError as e:
        print("axiom_ledger: NOT MEASURED -- %s" % e)
        sys.exit(2)

    n_lean = sum(1 for e in entries if e["kind"] == "lean_axiom")
    n_lean_exists = sum(1 for e in entries if e["kind"] == "lean_axiom" and e.get("shape") == "exists")
    n_bp = sum(1 for e in entries if e["kind"] == "bp_rule_axiom")
    n_mech = sum(1 for e in entries if e["kind"] in ("kernel_mechanism", "trust_gap", "prose_count"))
    n_comp = sum(1 for e in entries if e["computable"] in (COMPUTABLE, EXHAUSTIVE))
    n_exh = sum(1 for e in entries if e["computable"] == EXHAUSTIVE)
    n_not = sum(1 for e in entries if e["computable"] == NOT)

    if want_json:
        print(json.dumps(dict(
            lean_axioms=n_lean, lean_axioms_existential=n_lean_exists,
            bp_rule_axioms=n_bp, other_assumptions=n_mech,
            kernel_axiom_decls=len(cores),
            computable=n_comp, exhaustive=n_exh, not_computable=n_not,
            entries=entries,
            kernel_axiom_decl_sites=[dict(path=p, line=l, name=n, type_uid=t)
                                     for p, l, n, t in cores],
        ), indent=1))
        return 0

    # --- gate lines -------------------------------------------------------
    print("lean_axioms: %d" % n_lean)
    print("bp_rule_axioms: %d" % n_bp)
    print("kernel_axiom_decls: %d" % len(cores))
    print("axiom_ledger: %d entries  computable=%d (exhaustive=%d)  not_computable=%d"
          % (len(entries), n_comp, n_exh, n_not))
    print("lean_axioms_existential: %d of %d (Sigma_1: no counterexample exists to find)"
          % (n_lean_exists, n_lean))
    print("")
    for e in entries:
        tag = {COMPUTABLE: "COMP", EXHAUSTIVE: "EXH ", NOT: "NOT "}[e["computable"]]
        print("[%s] %-28s %s:%d" % (tag, e["id"], e["path"], e["line"]))
        print("        stmt: %s" % e["statement"])
        if e["refuter"]:
            print("        refuter: tools/axiom_refute.py::%s" % e["refuter"])
        if e["discharge"]:
            print("        discharge: %s" % e["discharge"])
        if e["note"]:
            print("        note: %s" % e["note"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
