Status: 2026-09-09 CURRENT (ROADMAP F6 preparation, lane C6, session 30, tree `e171367`, bebop.bin `045d0380`). The refutation instrument for the kernel, built BEFORE the kernel. Every number below was produced by running the artifacts in this tree; none is inherited. The kernel itself is NOT written here.

# Refuting the kernel before it exists

## 0. Why this comes first

Every other row in this tree dies on a number when it is wrong. A half-built CIC
kernel does not: it is a large artifact whose defects are silent, and "the kernel
seems to work" is not a measurement. So the instrument is built first, and
`tcheck.bp` gets numbers from its first commit instead of its last.

Two numbers, both live today:

    kernel_neg:      0 accepted of 13     MUST stay 0. A kernel that accepts an
                                          unsound term is unsound, and that is
                                          knowable with no kernel in existence.
    kernel_pos:      0 rejected of 3      the kernel must accept the good terms too;
                                          a checker that rejects everything scores
                                          a perfect kernel_neg.
    kernel_internal: 0 of 16              a CRASH is not a rejection and is never
                                          scored as one (see §3).
    kernel_parity:   0/16                 terms where `kcheck.py` and `tcheck.bp`
                                          agree. **0/16 is the honest STARTING
                                          value, not a failure** -- there is no
                                          `tcheck.bp`. It becomes k/16 on its
                                          first commit and 16/16 when it is done.

Run: `python3 tools/kcheck.py --corpus bench/kernel_neg`

## 1. Measured size, against the estimate

The report estimated ~1,000 lines for the reference kernel. Measured:
**`tools/kcheck.py` is 459 lines, 383 of them non-comment**, plus a 69-line
module docstring that carries the calculus specification. The corpus is 16 files,
166 lines. So the instrument came in at roughly **40 % of the estimate** — and
the reason is worth recording rather than banked as a win: it type-checks terms
and declares inductives, but it does **not** implement recursors or eliminators.
Those are where the remaining lines are, and any re-estimate of `tcheck.bp`
should start from "459 lines bought everything except elimination".

## 2. A caveat that must not be lost: the cited calculus is absent

The brief specifies the calculus by reference to "the report's section 4". **That
report is not in this tree.** No file under `docs/` mentions Hurkens, Girard,
non-positivity or `imax`. This is the fourth cited-but-absent source in a day,
after `docs/RESEARCH-VERIFICATION-2026-09-09.md` (since added),
`docs/blueprints/F0-trap-census.md` (planned, never written -- the census script re-derives it), and now the Fable design report.

So the calculus is **specified in `tools/kcheck.py`'s docstring**, from the
constraints that were actually stated, and it is the thing to argue with. It is
not a transcription and must not be read as one. If section 4 differs, the
corpus is what has to move, and it is cheap to move because every negative names
the rule it pins.

## 3. Design decisions, and what each one costs

**Concrete levels, no `imax`, no level variables** — as instructed. The honest
statement of what that does, since it is under adversarial review: dropping
`imax` does **not** shorten the hierarchy. `Sort 0 : Sort 1 : Sort 2 : ...` is
still infinite, so "an infinite universe hierarchy" is satisfied. What it removes
is **impredicativity**. `imax` exists so that `pi A B` lands in `Prop` whenever
`B : Prop`, whatever A's level; without it every `pi` lands at `max u v` and the
system is predicative throughout. **So the thing at risk is not the hierarchy, it
is impredicative Prop**, and nothing here claims to have it. That makes the
Girard/Hurkens negatives more important, not less: those paradoxes are precisely
what impredicativity plus a careless level rule buys, and `n02` is the file that
discriminates the two rules.

**Sharing is forbidden — a node may have at most one parent.** With de Bruijn
indices a subterm's meaning depends on its binder *depth*, so one node used at
two depths is unsound, and a kernel that allows sharing must track depth per
reference. Forbidding it removes the class for the cost of duplication in the
elaborator, which is the right trade for a kernel that must be small and
auditable. `n08` pins it.

**No eta.** Every rule the kernel does not have is a rule nobody has to verify;
eta costs a case and buys what the elaborator can do.

**A crash is not a rejection.** This one was found by mutation testing rather
than designed, and it is the most important line in the file. A mutant with the
context bounds check removed did not accept anything — it raised `IndexError`,
and the first version of the scorer counted that as "rejected", so **an unsound
kernel scored as safe**. `run_file` now returns a third outcome and
`kernel_internal` is reported separately. In Bebop the same defect reads past the
context array and types anything, so this is the failure mode that matters most.

## 4. The corpus, and what each file pins

Negatives (`bench/kernel_neg/n*.core`) — a kernel that accepts any is unsound:

| file | pins |
|---|---|
| `n01_type_in_type` | `Sort n : Sort (n+1)`, never `Sort n : Sort n` — Girard |
| `n02_hurkens_shape` | `pi` at `max u v`, not `imax` — **the discriminator between the predicative and impredicative rule** |
| `n03_nonpositive_inductive` | strict positivity: `mk : (Bad -> Bad) -> Bad` |
| `n04_ill_typed_app` | argument conversion |
| `n06_unbound_var` | context bounds |
| `n07_forward_reference` | node ordering (but see §5 — it does not pin what it appears to) |
| `n08_shared_subterm` | the no-sharing rule |
| `n09_pi_domain_not_a_type` | a `pi` domain must be a type |
| `n10_def_type_mismatch` | a `def`'s body must have its declared type |
| `n11_hurkens_pow` | nested `pi` levels at depth — **and measured NOT to discriminate `imax`; kept with that stated** |
| `n12_larger_id_reference` | the strictly-smaller-id invariant, properly |
| `n13_apply_non_function` | the head of an application must be a `pi` |
| `n14_ctor_wrong_return` | a constructor must return the type it declares |

Positives (`p*.core`) — the kernel must accept all three: the polymorphic
identity, an application at a declared type, and a **strictly positive**
inductive (`succ : Nat -> Nat`). The last is the twin of `n03`: a kernel that
rejects it is too strict, and an early version of the positivity check did
exactly that, so the positive is not decoration.

`n05_level_overflow` is **deliberately absent**, held pending the adversarial
review of whether concrete levels satisfy "an infinite universe hierarchy". §3
is this lane's answer to that question; the test lands when the review does.

## 5. Does the corpus actually work? Mutation testing says: 10 of 12

A corpus nobody has seen fail is not an instrument. Twelve mutants, each removing
exactly one kernel rule from `kcheck.py`, run against the corpus
(`/root/s30/outC6/mutation.txt`):

**Caught: 10 of 12** — `Type : Type`, impredicative `imax`, no positivity check,
conversion always true, sharing allowed, unchecked context bounds, no forward
check, no constructor-return check, no `def` type check, and a no-op de Bruijn
shift (caught by a *positive* failing, which is why positives are in the corpus).

Two survive, and the reasons are findings rather than gaps:

* **`M8_app_no_pi_check`** — removing "the head must be a `pi`" changes nothing
  observable *in Python*, because indexing a `('const','A')` tuple yields a
  string and conversion then fails structurally: a different error, the same
  rejection. The check is untestable in this reference by construction. **In
  Bebop it is exactly the dangerous case**: `tcheck.bp` reads a tag from a cell,
  and with unchecked array bounds the same removal reads a neighbouring cell that
  may well look like a `pi`. **So this one must be pinned in `tcheck.bp` by
  construction** — a tag switch whose default traps — and not by a corpus term.
  That is a requirement on the kernel, discovered before the kernel.
* **`M10_lam_domain_unchecked`** — after `check` was hardened to validate its
  claimed type, a bad `lam` domain is caught by two independent rules. No
  single-mutant test can isolate it. That is defence in depth, not a hole.

Three corpus files (`n12`, `n13`, `n14`) exist **because** mutation testing showed
the rules they pin were untested: `n07` turned out to be caught by the
undefined-node check rather than by the ordering rule it was written for, no test
applied a non-function, and no test had a constructor with the wrong return type.
The instrument found gaps in itself before the kernel could hide in them.

## 6. What `tcheck.bp` inherits from this

1. **A twin from commit one.** `kernel_parity` moves off 0/16 the first time the
   Bebop kernel checks a corpus file, and every disagreement is a named term.
2. **A soundness floor that is already 0.** `kernel_neg` cannot regress quietly.
3. **Two construction requirements**, both from §5: the tag dispatch must trap on
   an unknown tag rather than fall through (M8), and the context read must be
   bounds-checked in the kernel itself rather than relying on the corpus (M6).
4. **A rule list it must not exceed.** Every rule `kcheck.py` does not have —
   eta, sharing, level variables, `imax` — is a rule `tcheck.bp` must not have
   either, or the twin stops being a twin.
