# BLUEPRINT — Item 50: K3 Admission-Verdict Extension + Validity Terminology Binding

- **Date:** 2026-07-19 · **Tier:** roadmap §J (fourth wave) · **Status:** BLUEPRINT v1 (planning
  artifact, no code). Spec-level amendment to item 47; **starts only when the operator dispatches
  it AND item 47 exists** (see §7).
- **Sources (read this session):**
  `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §J item 50 (lines 776–798) + §I item 47
  (lines 683–699); `KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` Part 1 (terminology
  RULING) + §2.1 (where "Unknown" lives); `docs/audits/hardening/CHECKLIST.md` (the 5-point
  standard); `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` (Kani wiring this item extends).
- **Ground-truth code cited (branch `main`, verified in-tree this session, NOT taken on doc
  citation):** `kernel/src/ports/agent/admission.rs`; `kernel/src/kani_selftest.rs`;
  `kernel/src/fdr/schema.rs`; `kernel/src/fdr/ring.rs`; `docs/audits/hardening/HOT-PATHS.tsv`;
  `.github/workflows/ci.yml` (kani-gate).
- **Upstream (hard):** item 47 (Guardian admission gate) — its `Proposal`/`Invariants`/
  `ValidatedProposal`/`Rejection` types **DO NOT EXIST YET** (verified, §2.2). Item 50 EXTENDS
  them; it can be **specced now** but cannot **land** before item 47's types land.
- **Downstream:** item 51 (shadow mode) consumes `RejectionClass`; item 9 (breaker) weights
  `Refuted` vs `Undecidable`; item 7 (kani-gate) gains the K3-fold harness.

---

## 0. The one correction that changes this item's dependency status

The roadmap §J header (lines 768–771) states *"items 1–49's actual CODE … still lives ONLY on the
unmerged `exec/space-grade-tier0-2026-07-19` branch — `main` has documents only."* **That flag is
now STALE.** Verified this session: `main` HEAD is
`6701bbb6f Merge remote-tracking branch 'origin/exec/space-grade-tier0-2026-07-19'` — the exec
branch is merged. The entire FDR module (`kernel/src/fdr/`), item-6 `hardening-gate` + `ct_gate.rs`,
item-7 `kani-gate` + `kani_selftest.rs`, and `HOT-PATHS.tsv` are all **live in-tree**. Consequences
for item 50:
- "the K3 fold joins item 7's Kani target list … executed under item 7" is now an extension of a
  **shipped** gate (`.github/workflows/ci.yml:528` `kani-gate`; `HOT-PATHS.tsv` has real `mode=kani`
  rows), not a future one.
- The one prerequisite that genuinely remains is **item 47 itself** (§2.2) — unaffected by the
  merge, because item 47 was never on the exec branch.

## 1. Scope / goal

Extend item 47's admission gate so its refusal carries a **two-valued cause** and its internal
step-combinator is **strong-Kleene three-valued** — WITHOUT growing the public seam a third
control-flow arm. Bind the terminology: the RAW-PROMPT-6 "content-based truthfulness" concept is
**Validity** (derivational validity), disjoint from the swarm arc's byte-reproducibility
"Truthfulness" (synthesis Part 1, RULING — already applied in the roadmap text). Deliverable is a
spec + proof design; no runtime lane, no kernel-wide truth type.

**Non-goals (explicit):** no third public arm on `admit`; no `Option<TruthState>` leaking out of the
admission module; no reading of model confidence/logits (Evidence-based Unknown, §3.4); no proptest
for the truth tables (exhaustive enumeration supersedes it — synthesis §2.1); no rename of the
swarm-safety "Truthfulness" property (blast-radius reason, synthesis §1.2).

## 2. Current-state grounding

### 2.1 There are TWO `admit`s — do not conflate them

The kernel already has an `admit(...)` at `kernel/src/ports/agent/admission.rs:394`:

```
pub fn admit<S: EventStore>(&mut self, frame: &SignedFrame, roster: &AnchorRoster,
    chain: &[Delegation], revocations: &RevocationSet, event_log: &mut EventLog<S>,
    conn_id: u64, now: u64) -> Result<AdmissionRecord, AdmissionError>
```

This is the **B1 agent-admission path** (admitting a signed *agent manifest* / capability chain).
Its failure enum `AdmissionError` (`admission.rs:75`) is a flat, single-class typed reject
(`BadSignature`, `Expired`, `Revoked`, `RedLineViolation`, …). **This is NOT item 47's seam** and
item 50 does not touch it. The naming collision is real and must be called out in the item-47/50
module docs so a future reader does not extend the wrong `admit`.

### 2.2 Item 47's seam does not exist yet (verified)

`grep` over `kernel/src/` for `struct Invariants` / `struct Proposal` / `struct ValidatedProposal` /
`enum Rejection` / `fn admit(proposal` / `ValidatedProposal`: **zero hits**. Item 47 is spec-level
(roadmap:683–699: "spec after item 35; full wiring after item 42"). The seam item 50 extends —
`admit(Proposal, &Invariants) -> Result<ValidatedProposal, Rejection>` (roadmap:688) — is future
code. **Item 50 is therefore a co-spec of item 47's error surface, not an edit to existing code.**

### 2.3 The Kani surface item 50 joins is live

`docs/audits/hardening/HOT-PATHS.tsv` carries real `mode=kani` rows (dsa/keccak/selftest, lines
50–53); `kernel/src/kani_selftest.rs` is the planted-fault self-test template
(`kani_selftest.rs:20–28` — a deliberate i32 overflow under `#[kani::should_panic]`);
`scripts/kani-gate.sh` + `.github/workflows/ci.yml:528` run it. Item 50's K3-fold harness is one new
`mode=kani` row + one `#[cfg(kani)]` module in item 47's admission file — the mechanism is proven.

### 2.4 The FDR event surface the `RejectionClass` rides is live

`kernel/src/fdr/schema.rs:186` `enum Kind { Event, SpanClose, Alarm, PostMortem, Tuning,
CleanShutdown }` is the closed record-kind enum; `FdrEvent` (`schema.rs:212`) carries a
`fields: Vec<(&'static str, String)>` free-field bag. Item 47's "every `Rejection` emits an FDR
event" clause (roadmap:696) uses this surface; item 50 adds one field
(`rejection_class = "refuted" | "undecidable"`) to that record — no new `Kind`, no schema break.

## 3. Implementation plan (spec that item 47's author implements alongside item 47)

1. **`RejectionClass` on `Rejection`.** Item 47's `Rejection` (future type) gains a two-class cause:
   ```
   #[repr(u8)] pub enum RejectionClass { Refuted = 0, Undecidable = 1 }
   ```
   - `Refuted` = a named invariant/inference rule was demonstrably violated (Kleene False).
   - `Undecidable` = the evidence/derivation chain is incomplete, absent, or exceeded the checker's
     bounded budget (Kleene Unknown).
   `Rejection` carries `class: RejectionClass` plus the existing named-invariant identifier.
2. **The seam stays two-arm.** `admit(Proposal, &Invariants) -> Result<ValidatedProposal,
   Rejection>` is unchanged in shape. Kleene-False and Kleene-Unknown are **behaviorally identical
   at the seam** — both are `Err(Rejection)`, both take the deterministic path, `ValidatedProposal`
   stays constructible only through `admit` (illegal-state-unrepresentable). The class is *metadata
   on the refusal*, never a third branch a caller could handle leniently (synthesis §2.1).
3. **`TruthState` — internal combinator only.** Inside the admission module:
   ```
   #[repr(u8)] enum TruthState { False = 0, True = 1, Unknown = 2 }
   ```
   Each individual invariant/inference sub-check returns `TruthState`. It is module-private — never
   exported, never a kernel-wide type (synthesis §2.1: "without exporting a kernel-wide truth type
   nobody else needs").
4. **Strong-Kleene fold governs.** The admission verdict is the K3-AND over the conjunction of all
   sub-checks:
   - any `False` short-circuits → `Err(Rejection{class: Refuted})` (`False & Unknown = False` — the
     kernel may act on a definite refutation despite gaps elsewhere);
   - no `False` but any `Unknown` → `Err(Rejection{class: Undecidable})` (`True & Unknown = Unknown`);
   - all `True` → `Ok(ValidatedProposal)`.
   NOT (`match`-based, statically dispatched — the `order_machine` style item 47 mandates,
   roadmap:692–693):
   ```
   fn not(a: TruthState) -> TruthState { match a { False => True, True => False, Unknown => Unknown } }
   fn and(a: TruthState, b: TruthState) -> TruthState { /* the 9-cell strong-Kleene table */ }
   ```
5. **Evidence-based Unknown, forced.** A sub-check returns `Unknown` iff the evidence chain to `True`
   or `False` is missing/incomplete/over-budget. Model confidence/logits are **structurally not an
   input** to `admit` — the signature takes `&Invariants` + `Proposal` data only, no confidence
   field is threaded (synthesis §3.4 / RAW-6 verbatim). Document this as a red line in the fn doc.
6. **`None` ≠ `Unknown` — kept distinct in logs.** The seam's `Option<Proposal>` `None` (advice
   absent — item 45/47) and `Undecidable` (advice present, unevaluable) both take the deterministic
   path; both log as **distinct FDR facts** (`advice_absent` vs `rejection_class=undecidable`), never
   collapsed. A test pins the two log strings differ.
7. **FDR event field.** Item 47's per-`Rejection` FDR event (roadmap:696) gains
   `("rejection_class", class.as_str())` in its `fields` bag (`fdr/schema.rs:212` surface). No `Kind`
   growth; records without the field (non-rejection events) stay byte-identical.
8. **Module doc collision-note.** Add to item 47's admission-module header: "This `admit` is the AI
   *Proposal* gate (item 47/50). It is NOT `ports/agent/admission.rs::admit` (the agent-manifest
   admission, B1). Do not merge the two surfaces." (Grounded in §2.1.)

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

The checklist's four items + the Kani fifth, mapped to this item:

1. **Oracle — exhaustive, not randomized.** The input space of each binary K3 operator is **9 cases**
   (3×3) and NOT is **3 cases** — the FULL space, written out literally as `#[test]` truth-table
   assertions (`and`, `or` if used, `not`). This is CHECKLIST item 1's "exhaustive where the input
   space permits" at its purest. proptest is explicitly rejected (synthesis §2.1: "adding it would
   be theater").
2. **dudect gate — `N/A`.** No secret-dependent timing exists in a 3-state fold over invariant
   checks (the checks themselves may have CT concerns, but that is item 47's sub-check surface,
   not the fold's). Manifest records `N/A(no-secret-timing)`.
3. **Debug cross-check — the fold vs a table.** `debug_assert_eq!(and(a,b), STRONG_KLEENE_AND[a][b])`
   at each fold step against a `const [[TruthState;3];3]` table — the dual-representation cross-check
   idiom (`order_machine`'s `FSM_ADJ` precedent, CHECKLIST item 3). Compiled out of release.
4. **Assembly spot-check — inherits item 14.** The fold is branchy `match`, not a branch-free CT
   path, so CHECKLIST item 4 (asm audit on compiler bump) adds no obligation here beyond the standing
   `toolchain-bump-gate`.
5. **Kani — the K3-fold reachability proof (joins item 7).** A `#[cfg(kani)] proof_k3_fold_total`
   harness: for symbolic `TruthState` inputs (bounded to the 3 valid discriminants via
   `kani::assume`), the fold never reaches an unhandled state and always returns exactly one of the
   three seam outcomes (RAW-7's "prove `TruthState` can never reach an unhandled state"). One new
   `mode=kani` row in `HOT-PATHS.tsv` (executor computes the min-harness count; `kani_selftest.rs`
   planted-fault self-test already guards the gate itself).

**RED→GREEN proofs (P7, in the PR):**
- a planted incomplete-evidence `Proposal` (one required evidence field withheld) lands
  `Err(Rejection{class: Undecidable})` — asserted; restoring the field → `Ok`;
- a planted invariant violation (a `Result.velocity > MAX_SAFE_SPEED`-class breach) lands
  `Err(Rejection{class: Refuted})` — asserted;
- the item-47 `None`-path bit-identity test (deterministic output identical with the extension
  present) still green — proves item 50 added no behavior to the `None` branch;
- the K3-fold Kani harness, with its `should_panic`-style planted unhandled-state fault, goes RED
  when the fault is present and GREEN when removed.

## 5. Falsifiable acceptance criteria

- `admit`'s public signature is byte-identical to item 47's (`Result<ValidatedProposal, Rejection>`);
  a diff adding a third `enum`-arm return type to `admit` fails review.
- `TruthState` has no `pub` export outside the admission module (`grep` for `pub use.*TruthState`
  and `pub enum TruthState` outside the file = zero).
- The 9+3 truth-table tests exist and pass; deleting any one drops the `order_machine`-style min
  count and would be caught by the hardening-gate floor once the manifest row lands.
- `admit` has no parameter, field, or call path that carries model confidence/logits (structural
  grep + review).
- Undecidable and `None` produce two distinct FDR log strings (asserted in a test).
- The K3-fold Kani harness verifies SUCCESSFUL under `kani-gate`; its planted-fault variant verifies
  FAILED (RED-path recorded).

## 6. Dependency gates (honest)

| Gate | Status | Effect on item 50 |
|---|---|---|
| Item 47 types exist (`Proposal`/`Invariants`/`ValidatedProposal`/`Rejection`) | **NOT MET** (verified §2.2) | **Hard blocker to LAND.** Item 50 can be fully specced (this doc) but its code merges *with or after* item 47. Recommended: item 47's author implements `RejectionClass` + `TruthState` in the same change, per this spec. |
| Item 47 gating (spec after 35, wiring after 42) | inherited | item 50 rides these exactly (roadmap:898). |
| exec branch merged (FDR, kani-gate, ct_gate) | **MET** (§0 — stale roadmap flag corrected) | the FDR event surface + kani-gate are live; no wait. |
| Item 7 (kani-gate) | **MET / shipped** | K3-fold harness slots into the existing `mode=kani` manifest mechanism. |
| Item 9 (breaker) | not required | `RejectionClass` is *produced* now; item 9 *consumes* the weighting later. Design does NOT gate on item 9 (roadmap:792). |

## 7. Operator-decision points (flagged, not guessed)

1. **Terminology is already RULED, not open.** "Validity" vs "Truthfulness" was decided by the
   synthesis (Part 1) and applied in the roadmap text. This blueprint does not reopen it. If the
   operator wants the swarm-arc "Truthfulness" renamed instead, that is a cross-lane edit the
   synthesis explicitly declined (blast radius) — needs an explicit operator override.
2. **Where `RejectionClass` physically lives.** It can be a field on `Rejection` (recommended) or a
   sibling enum returned in a tuple. This blueprint recommends the field (keeps the seam a single
   `Result`), but item 47's author owns the final type layout.
3. **Budget for "over-budget ⇒ Undecidable".** The checker's bounded evidence-evaluation budget
   (what counts as "exceeds the bounded budget", roadmap:786) is a policy constant item 47 sets;
   item 50 only requires that hitting it yields `Undecidable`, never a silent `True`.
