# BLUEPRINT — Item 55: K3 verdict-class retrofit across roadmap verdict surfaces

- **Date:** 2026-07-19 · **Tier:** spec-level amendment (roadmap §K, item-50 shape) · **Status:**
  BLUEPRINT (planning artifact, no code) — this item lands **zero code of its own**; each amendment's
  code cost rides its host item's own build. This doc is DONE when the class enum + policy sentence
  are recorded against every host item and each host item's proof section names its planted-class
  red→green obligation.
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 55
  (lines 934–963) + the host-item entries (item 9 §H lines 368–373; item 45 line 640; item 35 line
  683ff); `AUDIT-BINARY-VS-KLEENE-LOGIC-2026-07-19.md` (findings 1/3/4/7/8 = this item; finding 6
  applied in item 12 §E; findings 2/5 = item 56); `docs/audits/hardening/CHECKLIST.md` (the 5-point
  standard); ground-truth code: `kernel/src/ct_gate.rs`, `kernel/src/spectral.rs`,
  `kernel/src/markov.rs`.
- **Prerequisites:** NONE — READY NOW. This is the classifier-consistency sibling of item 56 and the
  spec-coordination twin of item 50 (verdict-surface amendments in place, never a fork).

---

## 1. Scope & goal

**Goal.** Apply the Kleene-logic audit's remaining spec findings to every roadmap *verdict surface*
so that a **cannot-evaluate** outcome is a distinct, recorded, typed cause — not laundered into the
nearest binary pole and lost. **The invariant for every amendment, verbatim:** the *behavioral*
collapse to the safe pole is KEPT (no runtime change), and a distinct typed cause is ADDED to the
*record*. **There is no third control-flow arm anywhere in this item.** A two-armed gate stays
two-armed; only the forensic record gains a third value.

**Non-goals.**
- NOT a runtime behavior change on any surface (that would violate the golden pins on every one).
- NOT a new `Verdict`/`DriftClass` variant on the two kernel classifiers — that is item 56, and it is
  explicitly a *basis* field, not a fourth enum arm (CLI JSON + `wire_code` are golden-pinned).
- NOT a K3 rewrite of the 27 findings the audit ruled "keep binary" or the 11 "already correct."

**Why this is the honest form.** A binary verdict that folds "I could not measure" into "PASS" (or
"FAIL") is a *silent* epistemic error: the consumer cannot tell a measured result from an unevaluated
one, and mis-routes the response (a bound bump vs a code fix; a bench-stabilization ticket vs a
manufactured CONFIRMED). Kleene's third value `⊥` (undetermined) is exactly this missing state. The
audit found 8 surfaces that need it; 5 are roadmap verdict surfaces (this item), 2 are the kernel
classifiers (item 56), and 1 was item 12's temporal-TMR re-scope (finding 6, already applied).

## 2. Current-state grounding (the six amendment targets)

Each sub-item names its host, the host's *current* two-valued verdict, and the third cause to add.

| # | Host item | Current verdict (grounded) | K3 cause to ADD to the record |
|---|---|---|---|
| (a) | Item 33 — bench-delta | Binary claim vs a measured `fold_transitions` delta; the doc's own `±40%` CI noise-bound already exceeds the `+16.6%` claim | `{Confirmed(cmd), Refuted(cmd), Unresolvable(cause)}` — a claimed delta *smaller than the measured CI* is `Unresolvable`, recorded with **measured CI + claimed delta side-by-side** + a bench-stabilization ticket, never a manufactured CONFIRMED/REFUTED |
| (b) | Items 7/10/11 — Kani/TLC | `BLUEPRINT-ITEM-07` §6.4 already collapses `Refuted\|Undecidable → RED`; the class is not yet on the job artifact | per-target `{Proved, Refuted(cex), Undecidable(cause: bound/timeout/resource)}` rides the artifact — an exhausted bound needs a bound bump, a counterexample needs a code fix |
| (c) | Item 9 (+21 inherits) — breaker | Item 9 not yet built (`kernel/src/breaker/`, roadmap §H lines 368–373: `Result<Permit, Tripped>`); trip cause is currently unstated, and the `Reading::Unavailable`-input policy is undocumented | `TripCause::{Exceeded(named-threshold), Unevaluable(Absence)}`; **input policy becomes law:** a trip predicate over a `Reading::Unavailable` input takes the CONSERVATIVE pole (trip-eligible, never silently healthy), logged distinctly |
| (d) | Item 43 — CT inference gate | Classification law (secret vs public input plane) is silent on the residual case | `Unclassifiable ⇒ treated as secret-adjacent` (mandatory dudect branch), recorded as its own classification value so the fail-closed default is *visible* |
| (e) | Items 6/43 dudect | `ct_gate.rs:35` `T_THRESHOLD = 4.5`; the gate today is a boolean accept/reject on `|t| < 4.5` with a fixed sample count | `{LeakFound, NoLeakAtSamples(n), Inconclusive(underpowered)}` + recorded sample/class counts — a green run is citable as "no leak detected at power N," never "CT proven"; `Inconclusive ⇒ RED`; planted-leak positive control stays |
| (f) | Item 35 — emitter | Emitter refusal is currently one undifferentiated reject | `{BoundViolated, BoundUnprovable}` (consistency note, **no new state** — refusal already exists, only the reason is split) |

Grounding confirmations read this session:
- `kernel/src/ct_gate.rs:33–55` — the dudect gate is a Welch-t over interleaved samples, boolean
  accept at `|t| < T_THRESHOLD (4.5)`; the planted-leak self-test (`naive_eq`) is the positive
  control. There is **no** `Inconclusive`/underpowered class on the verdict today — (e) adds it.
- `kernel/src/spectral.rs:701–716` — `DriftClass::wire_code()` is the *precedent shape* item 55
  generalizes: an exhaustive `match` that forces every variant to be assigned a stable code
  consciously. K3 causes on non-wire surfaces follow the same "exhaustive-match forces a decision"
  discipline, without touching the pinned 0/1/2 wire codes (that constraint is item 56's).
- Roadmap §H lines 368–373 — item 9 is *unbuilt*; (c) is therefore a pre-build spec amendment,
  cheapest possible (the `TripCause` enum ships in item 9's first commit, not retrofitted later).

## 3. Implementation plan (spec-coordination, numbered)

This item's "implementation" is **recorded amendments**, not `.rs` diffs. The executor performs, per
host item, exactly two mechanical acts and one proof-obligation registration:

1. **(a) Item 33** — amend item 33's blueprint/results-doc text to define the `Unresolvable(cause)`
   row and require the side-by-side `(measured CI, claimed delta)` capture + a bench-stabilization
   ticket reference. Register the proof obligation: item 33's results doc must contain **at least the
   capability to record an `Unresolvable` row** (a schema field/column, red→green).
2. **(b) Items 7/10/11** — amend `BLUEPRINT-ITEM-07` §6.4 (and the TLC items' entries) so the job
   artifact carries per-target `{Proved, Refuted(cex), Undecidable(cause)}`. CI collapse to RED is
   unchanged; the class is an artifact field. Cross-check: item 07's manifest `gap` column already
   distinguishes "CAPPED-at-lemma" (a bounded-`Undecidable`) from a real failure — this makes that
   distinction a first-class recorded value rather than a prose note.
3. **(c) Item 9** — amend item 9's (pending) blueprint to define `TripCause::{Exceeded(threshold),
   Unevaluable(Absence)}` and to **state the `Reading::Unavailable`-input conservative policy before
   build**. Register the obligation: item 9's blueprint must state the Unavailable-input policy, and
   item 9's first commit must ship the `TripCause` enum + a test that a `Reading::Unavailable` trip
   input yields a distinct `Unevaluable`-caused trip (never silent-healthy).
4. **(d) Item 43** — amend item 43's classification law with the `Unclassifiable ⇒ secret-adjacent`
   third value; register the obligation that item 43's dudect wiring defaults an unclassifiable input
   into the secret-timed branch, recorded distinctly.
5. **(e) Items 6/43 dudect** — amend the dudect verdict definition to `{LeakFound, NoLeakAtSamples(n),
   Inconclusive(underpowered)}`, recording sample/class counts. Register the obligation: an
   underpowered run (too few samples to distinguish at the noise floor) is `Inconclusive ⇒ RED`,
   and the planted-leak positive control (`ct_gate.rs` `naive_eq`) stays green-by-being-caught.
6. **(f) Item 35** — annotate item 35's emitter-refusal path with `{BoundViolated, BoundUnprovable}`
   as a consistency note (no new state).

**Standing K3 shape law (record it in each host doc):** *behavioral collapse to the safe pole KEPT;
distinct typed cause ADDED to the record; no third control-flow arm.* Any reviewer can falsify a
violation by finding a third `if`/`match` arm that changes runtime flow — that is out of scope and
RED.

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

This item lands no code, so its *own* proof is documentary (the amendments exist). The 5-point
standard applies to each **host item's** build; item 55 records the obligation per host:

| Checklist item | Applicability to the K3 amendments | Disposition |
|---|---|---|
| 1. Oracle | Each K3 cause must be *reachable and recordable* | **PER-HOST**: each host item's proof section must name a planted-class red→green (e.g. item 33's results doc must record an `Unresolvable` row; item 9 must record an `Unevaluable`-caused trip) |
| 2. Dudect gate | Only (e) touches timing — the dudect verdict itself | **APPLIES to (e)**: the `Inconclusive` class must go RED under an underpowered run; the planted-leak `naive_eq` positive control (`ct_gate.rs`) must stay caught. No new secret-timed code is introduced by item 55. |
| 3. Debug cross-check | K3 causes are enum values, not arithmetic | **N/A(enum-classification)** — the exhaustive-`match` discipline (compile-fail on a new unassigned variant, `wire_code` precedent) is the structural equivalent |
| 4. ASM spot-check | No branch-free arithmetic path added | **N/A** — item 55 adds no hot-path arithmetic |
| 5. Kani/formal | (b) is *about* the Kani/TLC artifacts | **APPLIES to (b)**: item 07's kani-gate artifact must carry the `{Proved, Refuted(cex), Undecidable}` class; no new harness |

**Anti-forgery clause (reused from item 6/7):** because the item lands no test of its own, the
forgery surface is "an amendment claimed but not recorded." Mitigation: a per-host **grep proof** —
each host item's doc must contain its K3 enum name and policy sentence; a checklist row in the arc's
tracking asserts presence. This is documentary, so it inherits the CHECKLIST §"sanctioned
presence-check exception" caveat honestly: item 55's *completion* is presence-checked (an amendment
is a doc edit, not re-executable), but each host item's *class* is re-executed by that host's own
red→green (the teeth are downstream, stated plainly).

## 5. Falsifiable acceptance criteria

Item 55 is DONE iff **all** hold:
1. Each of (a)–(f)'s host item text contains the named class enum + the K3 shape-law sentence
   ("collapse kept, cause added, no third arm").
2. Each host item's proof section names its planted-class red→green obligation (item 33 →
   `Unresolvable`; item 9 → `Unevaluable`-caused trip + Unavailable-input policy stated *before*
   build; item 43 → `Unclassifiable ⇒ secret-adjacent`; items 6/43 dudect → `Inconclusive ⇒ RED`;
   items 7/10/11 → artifact-class; item 35 → refusal split).
3. A reviewer can confirm, by inspection, that no amendment introduces a third *control-flow* arm on
   any surface (behavior byte-identical; only the record widens).
4. Consistency with item 56: item 55 does **not** add a `Verdict`/`DriftClass` enum variant (that
   would collide with item 56's basis-field design and break the golden pins).

**Falsifier:** any host surface where "cannot evaluate" is still indistinguishable from a measured
pole in the record, OR any amendment that changes runtime behavior, OR a fourth kernel-classifier
enum variant appearing under item 55's banner.

## 6. Dependency gates

- **Upstream:** NONE — READY NOW. Each sub-item is a text amendment against an already-specified host.
- **Sibling coordination:** item 56 (kernel classifiers) — items 55 and 56 partition the audit's 8
  three-valued findings (55 = 1/3/4/7/8; 56 = 2/5); they must not both amend the same surface. The
  boundary: item 55 never touches `markov::Verdict` or `spectral::DriftClass` (those are 56's
  fail-open/fail-closed record-basis cases); item 56 never touches the roadmap-verdict surfaces here.
- **Downstream teeth:** the class of each amendment only *executes* when its host item builds — (c)'s
  `TripCause` with item 9; (b)'s artifact class with items 7/10/11; (e)'s `Inconclusive` with the
  next dudect coverage expansion (items 7/8). Item 55 gates none of them; it pre-loads their spec.

## 7. Operator-decision points & accepted risks

- **[OPERATOR] Is (c)'s Unavailable-input conservative policy the desired default for item 9?** The
  amendment makes "trip predicate sees a `Reading::Unavailable` input ⇒ trip-eligible (conservative)"
  a *law* before the breaker is built. This is the fail-safe reading (never silently healthy on
  missing evidence) and matches the FDR named-absence doctrine, but it means a *sensor outage* can
  trip the breaker. Flagged for explicit operator ratification because it sets breaker semantics
  before item 9's design round. **Owner:** operator (breaker behavior is Tier-0 reliability).
- **[ACCEPTED] Documentary completion.** Item 55's own DoD is presence-checked (doc amendments), an
  honest exception to the re-execute-never-presence-check law — justified because the teeth are each
  host item's re-executed red→green. **Owner:** arc lead.
- **[ACCEPTED] Item 33 bench noise.** Recording `(measured CI, claimed delta)` side-by-side surfaces
  that the current `fold_transitions` bench is too noisy to confirm a +16.6% claim; the honest output
  is `Unresolvable` + a stabilization ticket, not a fabricated verdict. This is a *feature* of the
  amendment, not a regression. **Owner:** item 33.
