# BLUEPRINT — Item 56: kernel classifier epistemic-basis retrofit (`markov::Verdict` fail-open · `spectral::DriftClass` conflated-cause)

- **Date:** 2026-07-19 · **Tier:** code (roadmap §K, item 56) · **Status:** BLUEPRINT (planning
  artifact, no code).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 56
  (lines 964–989); `AUDIT-BINARY-VS-KLEENE-LOGIC-2026-07-19.md` findings **2** (markov, the only
  fail-open-to-lenient instance) + **5** (spectral, its fail-closed sibling); ground-truth code:
  `kernel/src/markov.rs`, `kernel/src/bin/markov_attractor.rs`, `kernel/src/spectral.rs`,
  `kernel/src/fdr/schema.rs`, `kernel/src/fdr/pmu.rs`, `kernel/src/fdr/mod.rs`.
- **Prerequisites (split):** the pure-kernel halves (`markov.rs` / `spectral.rs` basis fields) have
  **none** — both files live on `main`. The FDR-record-field halves join **after the exec-branch FDR
  merge** (they extend `emit_verdict_pmu`'s record / item-27's optional-field discipline).

---

## 1. Scope & goal

**Goal.** Give the kernel's two classifiers a *typed epistemic basis* so the forensic record can
distinguish **"measured"** from **"could not evaluate"** — WITHOUT changing either classifier's
behavior or its byte-pinned wire/CLI contract. This is item 55's discipline applied to the two
in-kernel classifiers the audit singled out: one fail-*open* (markov — the headline, the only
lenient-collapse instance found), one fail-*closed* (spectral — correct collapse, still conflated).

**The two invariants, verbatim:** behavior KEPT; wire/CLI contract KEPT; only the *record* gains a
basis. The basis is a **new field on the report**, never a fourth enum variant (a fourth `Verdict`
would break the golden CLI JSON; a fourth `DriftClass` would break the pinned `wire_code` 0/1/2).

**Non-goals.**
- NOT a fourth `Verdict` or `DriftClass` arm — the classifiers keep 3 variants each.
- NOT a change to any threshold, damping, band, or fail pole.
- NOT making the basis a decision input anywhere — it is P3-plane forensic data only (item 27 law).

## 2. Current-state grounding

### 2a. markov — fail-open-to-lenient (audit finding 2, the headline)

- `kernel/src/markov.rs:41–49` — `enum Verdict { Healthy, LimitCycle, StrangeAttractor }`. Advisory
  (the harness gates decide); `Healthy` = no intervention.
- `kernel/src/markov.rs:110–133` — `analyze_detailed` maps **window-too-short** (`l < MIN_EVENTS`,
  `MIN_EVENTS = 8` at `:23`) to the `cold` report: `verdict: Verdict::Healthy`, `reason: "window too
  short"`. So "not enough data to analyze" is emitted at the **most lenient pole**.
- `kernel/src/bin/markov_attractor.rs:34–38` — a stdin read error also maps to
  `{"verdict":"HEALTHY","reason":"analyzer error"}` (fail-open: "never break the hook").
- `kernel/src/markov.rs:98–105` — `DetailedReport::verdict_str()` emits `"HEALTHY" / "LIMIT_CYCLE" /
  "STRANGE_ATTRACTOR"`. `kernel/src/fdr/mod.rs:400–414` (`sink::emit_verdict`) records **only**
  `verdict_str()` as the `("verdict", …)` field — so in telemetry a *window-too-short* Healthy is
  **byte-identical** to a *measured* Healthy. This is the exact blind spot: "couldn't analyze" ≡
  "measured healthy."

**Ruling (from the roadmap, endorsed):** the fail-open *stays* — for an advisory hook, "no evidence
⇒ no intervention" is the correct behavior; escalating on a cold start would be a false-positive
generator. What is wrong is only that the *record* cannot tell the two apart.

### 2b. spectral — fail-closed-but-conflated (audit finding 5)

- `kernel/src/spectral.rs:692–699` — `enum DriftClass { Damped, Resonant, Unstable }`.
- `kernel/src/spectral.rs:734–749` — `drift_guards_ok` rejects three *cannot-evaluate* causes
  before any indexing: (i) non-finite entry (NaN/±inf poison), (ii) ragged rows (jagged-matrix OOB),
  (iii) `Mat::from_vecvec_checked` Err.
- `kernel/src/spectral.rs:764–769` — `classify_drift` collapses **all three** cannot-evaluate causes
  into `DriftClass::Unstable` (fail-closed). Correct pole — a NaN-poisoned operator must not be
  admitted as healthy — but a genuinely-diverging loop (`ρ > 1`) and a NaN-poisoned input are now
  the *same recorded value*.
- `kernel/src/spectral.rs:701–716` — `DriftClass::wire_code()` pins `Damped=0/Resonant=1/Unstable=2`
  across the FE-07 kernel→engine bridge (`wasm.rs::spectral_flat_logic` encodes, `engine/src/
  bridge.rs` decodes, round-trip pinned in `drift_wire_code_is_canonical`, `:841–846`). A fourth
  variant is therefore **wrong** — the wire contract forbids it. The basis must be out-of-band.
- `kernel/src/spectral.rs:780–785` — `classify_drift_with_rho` is the single-pass report path
  (`graph_spectrum` at `:656` already routes through it); this is the natural home for a
  `classify_drift_with_rho`-shaped basis-returning variant.

### 2c. the FDR seam (both halves)

- `kernel/src/fdr/schema.rs:210–230` — `FdrEvent` carries `hw` + an **optional** `pmu:
  Option<PmuStamp>` (present only on verdict-emission records, `:223–228`) + `fields`. The
  optional-field discipline (`pmu` absent ⇒ record byte-identical to pre-item-27) is the exact
  precedent for adding an *optional basis field*.
- `kernel/src/fdr/mod.rs:281–283` / `:396–424` — `emit_verdict_pmu(name, verdict, pmu)` →
  `sink::emit_verdict`, which records `vec![("verdict", verdict.to_string())]` + the PMU stamp. This
  is where the markov basis attaches. The spectral companion follows the same optional-FDR shape.

## 3. Implementation plan (numbered)

### markov half (finding 2)

1. Add `pub enum Basis { Measured, WindowTooShort, AnalyzerError }` to `markov.rs` (a small closed
   enum, `Copy`). Add a `pub basis: Basis` field to `Report` (or to `DetailedReport` if that keeps
   the CLI serializer simpler — executor's call, but it must **not** appear in the golden CLI JSON
   key set, see step 3).
2. Set the basis at the two lenient sites: `analyze_detailed`'s cold path (`markov.rs:113–133`) sets
   `Basis::WindowTooShort`; the measured path sets `Basis::Measured`. `markov_attractor.rs:34–38`'s
   stdin-error branch is `Basis::AnalyzerError` (it does not call `analyze_detailed`, so it sets the
   basis on the record it emits, not on a `Report`).
3. **Golden-CLI invariance is the load-bearing constraint.** The stdout JSON contract
   (`markov_attractor.rs:79–96`) is pinned byte-identical to the deleted Python. The basis field
   must therefore be emitted **only into the FDR record**, never the CLI stdout JSON. Concretely:
   `Verdict`/`verdict_str()` stay 3-valued and the CLI serializer is untouched; the basis rides the
   FDR record via a new optional field.
4. Extend `emit_verdict_pmu` (`fdr/mod.rs:281`) with an optional basis: add
   `emit_verdict_pmu_basis(name, verdict, basis: Option<&str>, pmu)` OR carry the basis in the
   existing `fields` vec of `sink::emit_verdict` (`fdr/mod.rs:412`) as an **optional** `("basis", …)`
   field. Follow item-27's optional-field discipline: absent ⇒ record byte-identical to pre-item-56.
5. **Downstream law (record in items 9/21 docs):** items 9/21 must never count an
   unevaluated-`Healthy` (basis `WindowTooShort`/`AnalyzerError`) window as *health evidence*. This
   is a spec obligation on the breaker/gain-scheduler, registered here (composes with item 55(c)'s
   Unavailable-input policy — a `WindowTooShort` verdict is an evidence-absence, treated conservative).

### spectral half (finding 5)

6. Add `pub enum DriftBasis { Measured, IllFormedInput(IllCause) }` where `IllCause ∈ {NonFinite,
   Ragged, Unbuildable}` mirrors the three `drift_guards_ok` rejection causes (`spectral.rs:734–748`).
   Small closed enum, `Copy`.
7. Add a report-path variant that returns the basis alongside the class — the natural extension of
   `classify_drift_with_rho` (`spectral.rs:780`): e.g. `classify_drift_report(a) -> (DriftClass,
   DriftBasis)` where the guard-fail path returns `(Unstable, IllFormedInput(cause))` and the happy
   path returns `(band(ρ), Measured)`. **`classify_drift` / `classify_drift_with_rho` themselves stay
   byte-identical** (they are the pinned surfaces); the new fn is additive.
8. `wire_code` (`spectral.rs:709`) is **untouched** — the basis is out-of-band, carried on the FDR
   companion record (item-27-style optional field), never on the 0/1/2 wire. The FE-07 bridge sees
   the identical 3-valued code.
9. Wire an optional spectral-drift FDR companion at the drift-emission site (where a `DriftClass` is
   logged for forensics), carrying the `DriftBasis` as an optional field — same optional-field
   discipline as the markov half.

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

| Checklist item | Disposition for item 56 |
|---|---|
| 1. **Oracle** | **markov:** golden CLI JSON byte-identical before/after (the pinned contract IS the regression oracle) — a forced short-window run and a forced analyzer-error run each yield `Healthy` **plus the correct distinct basis in the FDR record** (red→green: today both are byte-identical to measured-healthy). **spectral:** a NaN-poisoned matrix and a genuinely-divergent (`ρ>1`) matrix both classify `Unstable` **with distinct recorded bases**; `wire_code` round-trip test (`drift_wire_code_is_canonical`, `spectral.rs:842`) untouched and green. |
| 2. **Dudect gate** | **N/A(no-secret-timing)** — both classifiers operate on tool-outcome tokens / graph operators, not secrets; the basis is P3 forensic data. No secret-dependent branch introduced. |
| 3. **Debug cross-check** | markov already `N/A(corpus-oracle)` in HOT-PATHS; spectral is `N/A(corpus-oracle)`. The basis fields are enum classifications, not arithmetic — the exhaustive-`match` compile-fail discipline is the structural equivalent (a new `IllCause` without a mapping fails to compile). |
| 4. **ASM spot-check** | **N/A** — no branch-free arithmetic hot path added; the basis is a tag set at a decision the classifier already makes. |
| 5. **Kani/formal** | **N/A** — the property is "the record distinguishes the causes," an oracle/golden concern, not a bounded-model-checking one. |

**Plane-firewall proof (item-27 §4.5 precedent, mandatory here):** a grep proof that neither `Basis`
nor `DriftBasis` is read by any hash/signature/gate/replay/decision surface — the basis is recorded,
never a decision variable. This is the highest-class invariant for this item (a fail-open classifier
whose basis leaked into a decision would be a real regression).

## 5. Falsifiable acceptance criteria

1. **markov golden CLI JSON is byte-identical** pre/post — the pinned Python-contract test is green
   unchanged (proves behavior + wire untouched).
2. A forced `l < 8` run records `Basis::WindowTooShort` in the FDR record and `Verdict::Healthy`;
   a measured run records `Basis::Measured` + `Verdict::Healthy`. The two FDR records **differ** on
   the basis field and **agree** on the verdict field (red→green: today they are identical).
3. `markov_attractor.rs`'s stdin-error path records `Basis::AnalyzerError`.
4. A NaN-poisoned operator and a `ρ>1` operator both yield `DriftClass::Unstable`; their FDR
   companion records carry `DriftBasis::IllFormedInput(NonFinite)` vs `DriftBasis::Measured`
   respectively.
5. `wire_code` round-trip and every existing `classify_drift`/`classify_drift_with_rho` test are
   green **unchanged** (additive-only proof).
6. Grep proof: no `Basis`/`DriftBasis` value reaches a decision/hash/gate surface.

**Falsifier:** any change to the golden CLI JSON key set; any fourth `Verdict`/`DriftClass` variant;
any test on a pinned classifier fn that had to change; any basis value on a decision path.

## 6. Dependency gates

- **Pure-kernel halves (markov/spectral basis enums + report-path fns):** **READY NOW** — both files
  on `main`; additive, behind no feature flag (they add fields/fns, they do not pull deps).
- **FDR-field halves (basis on `emit_verdict_pmu` / spectral companion):** gated on the **exec-branch
  FDR merge** (the `fdr/` module + `emit_verdict_pmu` must be on the target branch). Confirmed present
  in this worktree's base (`kernel/src/fdr/mod.rs:281`), so the gate is the merge, not new code.
- **Consistency with item 55:** item 56 owns findings 2/5 (the kernel classifiers); item 55 owns
  1/3/4/7/8 (roadmap verdict surfaces). Neither touches the other's surfaces (see item 55 §6).
- **Downstream:** items 9/21 inherit the "never count unevaluated-Healthy as health evidence" law
  (step 5) — registered, not built here.

## 7. Operator-decision points & accepted risks

- **[ACCEPTED] The fail-open pole stays.** markov keeps emitting `Healthy` on window-too-short /
  analyzer-error. This is deliberate (advisory hook; no-evidence⇒no-intervention). The item only makes
  the *reason* visible. If the operator ever wants a cold start to be *escalatable*, that is a
  separate behavior change (out of scope) — flagged so the choice is conscious. **Owner:** operator.
- **[ACCEPTED] Basis is P3-only.** By construction the basis can never influence a decision; the
  grep-firewall proof enforces it. The residual risk is a future author reading the basis into a gate
  — mitigated by the standing item-27 plane law + the mandatory grep proof (§4). **Owner:** arc lead.
- **[NOTE] Field placement (`Report` vs `DetailedReport`).** Executor picks the struct that keeps the
  golden CLI serializer untouched; both are viable. Not an operator decision — an implementation
  detail recorded so the golden-invariance constraint is not violated by accident.
