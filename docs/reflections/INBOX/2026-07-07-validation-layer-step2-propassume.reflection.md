# Reflection — `prop_assume` on a sparse predicate silently exhausts the sampler; condition INSIDE the body instead

**Date:** 2026-07-07 · **Slug:** validation-layer-step2-propassume
**Qualified because:** kernel-dir change (red-line — Sovereign Core Validation Layer step 2,
`ActorNotAuthorized`) + a proptest that FAILED on first run and had to be rediagnosed.

## CONTEXT
Extending the Validation Layer with the `ActorNotAuthorized` invariant (lift
`policy::assert_owner_target_allowed`), I added an isolated actor-dimension proptest that ranged over
`status × transition-command` but is only meaningful on machine-LEGAL edges, so I gated it with
`prop_assume!(assert_transition(status, cmd.target()).is_ok())`. It FAILED — but NOT on an assertion:
proptest aborted with "1024 times ... assert_transition(...).is_ok()", i.e. it rejected too many
generated cases and gave up.

## WHY (causal)
The order machine's legal edges are SPARSE — ~14 legal edges out of 10 statuses × 9 commands ≈ 15%.
`prop_assume` DISCARDS every case the predicate rejects and asks for another; when the predicate is
true only ~15% of the time, the local rejection counter blows past proptest's limit (1024) before
enough passing cases accrue, and the test dies as a HARNESS failure — not a green, not a red on the
property. A `prop_assume` is only safe when the kept fraction is high; on a sparse predicate it turns a
sound property into a flaky/aborting one.

The fix is not to loosen the limit (that hides the smell) but to CONDITION INSIDE the test body:
`if assert_transition(...).is_ok() { ...assert... }`. Every generated case now runs to completion; the
machine-illegal majority becomes a cheap no-op (already covered by the sibling machine+actor property),
and the machine-legal minority exercises the actor assertion. Same coverage, zero rejections. (The
concrete RED case — all three owner-forbidden cancel edges — is ALSO pinned deterministically by an
inline unit test, so the rare Err branch never depends on the sampler hitting it.)

## HOW TO APPLY
- Before `prop_assume!(pred)`, estimate the KEPT fraction. If `pred` is true for only a small share of
  the generated space (sparse state machines, "is a legal edge", "is a valid combo"), do NOT assume —
  either (a) condition inside the body with `if pred { ... }`, or (b) build a generator that only
  produces passing cases. Reserve `prop_assume` for predicates that reject a small minority.
- Keep a DETERMINISTIC unit test for the specific RED case a sparse property is meant to catch, so
  falsifiability never rides on the random sampler happening to hit a 1-in-60 input.
- A proptest that dies with "N times ... <assume-expr>" is an over-rejection smell, not a logic bug —
  read it as "your filter is too aggressive for random generation," not "the code is wrong."

Links: [[verified-by-math-2026-07-07]] · step-1 reflection (2026-07-07-validation-layer-step1) for the
shared design principles (type-as-proof, soundness-superset, one dimension per invariant) ·
VALIDATION-LAYER-SPEC.
