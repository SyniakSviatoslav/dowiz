# Consistency Audit — Binary vs Three-Valued (Kleene) Logic Across the Space-Grade Roadmap

**Date:** 2026-07-19 · **Role:** consistency-enforcement audit (one of three parallel audit
dimensions), read-only over the four session arcs. **Directive audited against (operator,
verbatim):** *"3 states (positive, false, unknown) binary logic is rejected except for the cases
where binary logic is truly possible or helpful."* **Template for a CORRECT application:**
roadmap item 50 — `RejectionClass::{Refuted, Undecidable}` riding item 47's `Rejection`
(`KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` §2.1). **Documents audited:**
`SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` (items 1–54 + §0 rulings),
`DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md`,
`CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-SYNTHESIS-2026-07-19.md`, plus the flagged
kernel precedents (`markov::Verdict`, `spectral::DriftClass`, `fdr::Reading<T>`/`Absence` —
live source read, file:line cited).

## 0. The discipline applied (so this audit does not over-convert)

Item 50's own design is the calibration standard, and it cuts BOTH ways:

1. **A third state is owed only where a genuine epistemic gap exists** — the check could fail
   to *evaluate* (incomplete evidence, resource exhaustion, unreadable input, statistical
   underpower), not merely fail its assertion. A structural fact (byte equality, set inclusion,
   hash match, compile success, exhaustive-domain test outcome) is genuinely binary; forcing
   `Unknown` onto it is the over-engineering the ponytail/ADR rules ban.
2. **The third state is a RECORD class, almost never a third control-flow arm.** Item 50's rule:
   at a decision seam, Kleene-Unknown MUST behave exactly like Kleene-False (the safe pole) —
   otherwise "Unknown" becomes a leniency loophole. What three-valuedness adds is that the
   *logged/typed cause* distinguishes "demonstrably violated" from "could not evaluate."
   Every SHOULD-BE-3-VALUED finding below proposes exactly that shape: behavioral collapse to
   the safe pole KEPT, distinct typed cause ADDED.
3. **A completed CI run is binary.** RED/GREEN on a job that ran is a fact. The genuine third
   state for CI — "the check never actually exercised anything" — is already systematically
   handled in this roadmap by the anti-forgery clauses (zero-match filter ⇒ RED, item 6;
   re-execute-never-presence-check, §10/P7) and by CI's own infra-failure ⇒ RED collapse.
   Those are classified ALREADY-CORRECT, not converted.
4. **Named absence ≠ three-valued logic.** `Reading<T> = Value(T) | Unavailable(Absence)` is
   two-state because a *measurement* is not a proposition — there is no "False" reading. It is
   the correct shape for its plane and is not "upgraded."

## 1. Summary counts

| Classification | Count |
|---|---|
| KEEP-BINARY (genuinely no epistemic gap, or gap already collapses safely at infra level) | **27** |
| SHOULD-BE-3-VALUED (real epistemic gap, concrete third state proposed) | **8** |
| ALREADY-CORRECT (properly three-valued / named-absence / explicit-unknown today) | **11** |
| **Total decision points audited** | **46** |

## 2. Full classification table

Location = roadmap item unless a file is named. "Collapse kept" = the behavioral binary
outcome stays; only the record gains a class (the item-50 shape).

### 2.1 Roadmap items 1–32 (+ §0)

| Location | Current logic shape | Classification | If SHOULD: proposed third state |
|---|---|---|---|
| §0 operator rulings (GCRA, mesh, optical, ARINC, SIHFT, eqc-IR) | ADOPT/PURSUE decisions | KEEP-BINARY | Rulings are acts of will, not truth claims — no epistemic gap by construction. |
| Item 2 — FileEventStore dual check (a)/(b) | pass/fail per sub-check | KEEP-BINARY | "Constructed anywhere" is an exhaustive-search fact; adversarially re-verified. The universally-quantified negative was discharged by full-workspace search, not sampling. |
| Item 3 — golden signature + 1e-12 oracle + zero-alloc counter | test green/red | KEEP-BINARY | Deterministic test outcomes on enumerable surfaces. |
| Items 15/16/17/19 — read-only audits (eigen-surface, GraphSpectrum, engine table, retrieval) | defect-found / no-defect | KEEP-BINARY | Findings are cited facts (file:line); "not found" was backed by exhaustive grep, and unresolved smells were filed as tickets rather than silently passed — the honest third outcome ("new scope, not built") is already used as a *disposition*, which is the right plane for it. |
| Item 18 — Laplacian parity pin | bit/epsilon equality | KEEP-BINARY | Differential test = fact. |
| Item 22 — mesh real-port vs stub classification | per-symbol verdict table | KEEP-BINARY | Caller-or-NONE is a structural fact per symbol. |
| Item 30 — FSM proliferation audit | INDEPENDENT/shared per module | KEEP-BINARY | Structural fact; the one epistemic wrinkle ("2 confirmed silent defects" phrase) was correctly resolved as UNSOURCED — an explicit refusal to force an unverifiable claim into true/false without evidence. |
| Item 31 (both halves) — dependency rulings + per-crate gate | KEEP/DEFER/DEFECT; gate GREEN 24/24 | KEEP-BINARY | Manifest/lockfile facts. Note: the CI-poison bug it found (failing `git` access corrupting the next cargo probe → *wrong* verdict, not a failed one) is exactly why gates must make couldn't-run distinguishable-or-RED; the fix (no-pathspec `ls-tree` probe) restored that. |
| Items 1+13 — zero-dep gate A/B/C | tree⊆allowlist / monotonic / hash | KEEP-BINARY | Set inclusion + hash equality = facts. The item-5 latent abort (filter-empties-everything → script abort under `pipefail`) was a couldn't-run state; it aborted (RED-equivalent), never reported a false verdict — safe collapse, since fixed. |
| Item 14 — toolchain-bump gate | vacuous-green / RED / GREEN | KEEP-BINARY | "Did `channel` change in this diff" is a diff fact; vacuous-green on non-bump is not-applicable, keyed to a fact, not a judgment. The baseline artifact's own honesty (assembly audit PARTIAL, per-branch proof DEFERRED to item 7 — "no fabricated clean claim") is the claim-level three-valuedness done right in prose. |
| Items 4+29 — logger/FDR byte-identity + kill-9 proofs | byte comparison | KEEP-BINARY | Byte equality = fact. |
| Items 4+29 — `hw.joules_uj` = `unavailable:no_rapl_interface` | named absence, never fabricated 0 | ALREADY-CORRECT | The `Reading<T>`/`Absence` pattern: absence is explicit and reasoned, never coerced to a value. |
| Item 5 — pattern matcher | match/no-match + `Err(PatternError::UnsupportedMeta)` | ALREADY-CORRECT | Three genuine outcomes: matched, not-matched, *cannot-evaluate-this-pattern* (typed refusal, degrade-closed). The unsupported-metacharacter arm is precisely "Unknown handled as refusal with a named cause." Match/no-match itself is a deterministic fact — correctly binary. |
| Item 6 — hardening-gate | RED/GREEN + zero-match-filter ⇒ RED | ALREADY-CORRECT | The anti-forgery clause IS the third-state detector: "the check never ran against anything" is caught and collapsed fail-closed with a distinct cause (exit path names the zero-match row). `KNOWN-RED(P91.2)` ledgering likewise refuses to launder a known gap into green. |
| Item 6 / item 43 — dudect Welch-t verdict (\|t\| < 4.5) | leak / no-leak binary | SHOULD-BE-3-VALUED (minor, claim-level) | A statistical test's "pass" is *no leak detected at power N*, never *proven constant-time*; underpower is a real epistemic gap. The planted-leak positive control already catches gross underpower (harness that can't see \|t\|≈300 goes RED) — keep it. ADD: the gate's recorded verdict carries `{LeakFound, NoLeakAtSamples(n), Inconclusive(underpowered/insufficient-classes)}` — sample count + input-class count recorded so a green run is never citable as "CT proven." Gate outcome stays binary (Inconclusive ⇒ RED). |
| Item 7 — Kani wiring (also item 10 TLC, item 11 TLC model) | proof pass/fail | SHOULD-BE-3-VALUED | Bounded model checking has three native outcomes: **Proved** (within bounds), **Refuted** (counterexample), **Undecidable** (solver timeout / unwinding bound exceeded / resource exhaustion). Treating undecidable as "failed" behaviorally is correct (CI RED), but the harness must record the class distinctly — an exhausted bound needs a bound bump, a counterexample needs a code fix; conflating them mis-routes the response. Proposal: per-target verdict enum `{Proved, Refuted(cex), Undecidable(cause)}` in the Kani/TLC job's result artifact; CI collapses Refuted\|Undecidable → RED; the class rides the job log/artifact (item-50 shape verbatim). Applies identically to TLC state-space exhaustion in items 10/11. |
| Item 8 — GCRA differential oracle | bit equality | KEEP-BINARY | Differential = fact. (Its Kani interleaving half inherits the item-7 finding above.) |
| Item 9 — breaker `Result<Permit, Tripped>` | two typed poles | SHOULD-BE-3-VALUED (inside `Tripped`, seam unchanged) | The seam is correct and MUST stay two-armed (item-50 rule: no lenient third arm; tripped-but-permitting unconstructible). Two real epistemic gaps in the trip *decision*: (a) trip predicates will consume telemetry that can be `Reading::Unavailable` (PMU/Hw stamps) — the roadmap nowhere states the policy for evaluating a trip condition over an Unavailable input; silently treating absence as 0/healthy would be the fail-open bug class. (b) A `Tripped` caused by "threshold demonstrably exceeded" vs "could not evaluate health (inputs unavailable)" are different facts for the operator and for item 21's gain-scheduling. Proposal: Blueprint-A build carries `TripCause::{Exceeded(named-threshold), Unevaluable(Absence)}` on `Tripped` (mirror of `RejectionClass`), policy stated: Unavailable input ⇒ conservative pole (evaluate as trip-eligible, never as healthy), logged distinctly. Item 21 inherits the same input policy; item 27's response half already routes here. |
| Item 12 — SIHFT triple-vote pilot | vote-mismatch ⇒ trip | SHOULD-BE-3-VALUED (record class; behavior kept) | Any-mismatch ⇒ trip is the right binary collapse. But 2-of-3 (one replica dissents — fault localized, majority value known) vs 3-way disagreement (no majority — correct value UNKNOWN) are epistemically different events with different forensic value. Proposal: the FDR entry carries `VoteOutcome::{Unanimous, SingleDissent(replica-id), NoMajority}`; both non-unanimous classes trip identically. Design-only item — bake this into the design doc now, zero code cost. |
| Item 26 — batching measurements | measured numbers; "PMU unavailable" fallback | KEEP-BINARY | Measurement-only; the PMU gap was named, not fabricated ("wall-clock + strace fallback, no fabricated counters") — honest absence handling. (Item 27 later *corrected* the unavailability premise for root — an instance of why named-absence beats a silent 0: the record was revisitable.) |
| Item 27 — `PmuStamp`, all fields `Reading<u64>` | Value / Unavailable(named Absence) | ALREADY-CORRECT | Every failure mode degrades to a *named* absence (`NoPmuInterface`, `PermissionDenied`), "never a fabricated 0, never a panic." The measurement-plane analog of evidence-based Unknown, already law. |
| Items 20/23/24/28/32 | standard test-green proofs only | KEEP-BINARY | No judgment-shaped decision points in their roadmap text beyond deterministic proofs; item 28's plane-boundary structural test is set membership. |

### 2.2 Items 33–44 (Deterministic AI Inference arc)

| Location | Current logic shape | Classification | If SHOULD: proposed third state |
|---|---|---|---|
| **Item 33 — bench re-measurement: "each raw-prompt number explicitly CONFIRMED … or REFUTED"** | forced two-valued verdict | **SHOULD-BE-3-VALUED (strongest finding in this audit)** | The arc's own grounding proves the third state occurs: `fold_transitions` is documented **NOISE-BOUND at ±40% CI** (AI synthesis §1.2) — a claimed +16.6% regression on a ±40%-noise bench can be neither confirmed nor refuted at achievable power. Forcing CONFIRMED/REFUTED will manufacture a false verdict on exactly the benches most likely to be disputed. Proposal: per-number verdict `{Confirmed(cmd), Refuted(cmd), Unresolvable(cause: noise-bound CI wider than claimed effect / bench absent / environment non-reproducible)}`; `Unresolvable` requires the measured CI + the claimed delta side-by-side, and files a bench-stabilization ticket instead of a fake refutation. MISSING→RED tracker semantics stay as-is (couldn't-measure already collapses fail-closed and visibly — correct). |
| Item 34 — pilot ruling | operator decision | KEEP-BINARY | An act of will, recorded. |
| Item 35 — number-format spec: "refuse-never-fall-back on any unprovable bound" | prove-or-refuse | ALREADY-CORRECT | This IS the K3 pattern: provable-safe / provable-violated / **unprovable ⇒ refuse** — Unknown handled as the safe pole with a named cause. Minor consistency note (no new item): the emitter's refusal should carry the cause class `{BoundViolated, BoundUnprovable}` — a violated bound needs a model change, an unprovable one may need a better lemma; item-50 vocabulary fits verbatim. |
| Item 36 — eqc emitter "refuses an inexpressible node" | accept/refuse | KEEP-BINARY | Node-in-grammar is a structural fact; no epistemic gap. |
| Item 37 — reference oracle, zero divergence | bit equality | KEEP-BINARY | Differential fact. |
| Item 38 — arena: zero-alloc counter; overlapping layout fails to construct | test facts / compile facts | KEEP-BINARY | Counting-allocator result and unconstructibility are facts. |
| Item 39 — SIMD differential, bit-exact both paths | bit equality | KEEP-BINARY | Fact. |
| Item 40 — per-layer golden CRC32: match/mismatch | binary integrity verdict | KEEP-BINARY (with one observability sub-flag) | Genuinely binary at runtime: the CRC is computed over addressable in-process memory against a build-time-baked golden — there is no I/O to fail, no "couldn't read" state that yields a *verdict*; an unreadable page is a process-fatal fault (SIGSEGV) landing in item 48's crash class, a different mechanism, never a silent wrong answer. Sub-flag (not a third state): "checksum-silent clean run" is indistinguishable from "spot-check never ran" — silence conflates verified-clean with never-verified. Cheap fix: a checks-performed counter rides item 48's heartbeat progress counters, making not-running observable. Same flag applies to item 54. |
| Item 41 — SHA3 init self-check | match/mismatch at init | KEEP-BINARY | Same reasoning as item 40; init-time, embedded static, no I/O. |
| Item 42 — bit-identical outputs across runs/targets | byte equality | KEEP-BINARY | Fact. |
| Item 43 — input-plane classification ("if secret-adjacent … if provably public-plane") | two-way classification | SHOULD-BE-3-VALUED (minor — one sentence of law) | "Provably public" and "provably secret-adjacent" do not partition the space: a plane that *cannot be proven either way* is the unstated third case. The toy pilot is settled (public by construction), but the deferred real-product pilots will hit genuinely ambiguous planes (e.g. features derived from product data at several removes). Proposal: state the default now — `Unclassifiable ⇒ treated as secret-adjacent` (mandatory dudect branch), recorded as its own classification value so the fail-closed default is visible, not implicit. |
| Item 44 — artifact-less diff ⇒ CI RED | binary gate | KEEP-BINARY | Diff-content fact + re-execution (anti-forgery inherited). |

### 2.3 Items 45–49 (Whole-System Determinism arc)

| Location | Current logic shape | Classification | If SHOULD: proposed third state |
|---|---|---|---|
| Item 45 — ai-optional-gate: AI present/absent | feature-flag + dependency-direction check | KEEP-BINARY | The audit question ("does build/CI context introduce couldn't-determine?") — answered NO: cargo feature membership is set membership; "core module references AI paths outside the gate" is decidable from source structurally; the planted-import red-proof (P7) verifies the detector works. A job that fails to *run* collapses to CI RED (infra-failure, fail-closed, cause distinct in the CI log) — the standard safe collapse, no roadmap change owed. |
| Item 46 — float containment: "zero unclassified transcendental sites" | classified/unclassified per site | ALREADY-CORRECT | The design makes "unclassified" an explicit tracked state that must be driven to zero, rather than defaulting unvisited sites into a pass — explicit-exhaustiveness is the inventory-plane form of refusing a silent Unknown. |
| Item 47 — `admit → Result<ValidatedProposal, Rejection>`; `Option<Proposal>` None-path | two-armed seam + first-class absence | ALREADY-CORRECT (via item 50) | Checked for consistency with item 50 as tasked: consistent. Item 50 amends `Rejection` in place (no parallel type, no third arm); `None ≠ Undecidable` is explicit (absent advice vs unevaluable advice — both deterministic path, logged distinctly); item 47's planted-invalid proof is refined by item 50's split into planted-Refuted AND planted-Undecidable. No divergence found between §I item 47's text and §J item 50's amendment. |
| Item 48 — heartbeat liveness + panic hook | missed-heartbeat ⇒ declared dead | KEEP-BINARY | The liveness *judgment* is a threshold on a fact (heartbeat arrived or not) with a deliberately safe collapse: a false "hung" verdict merely converts to the kill-9 crash class the system provably survives. Note done right: the clean-shutdown final heartbeat is precisely the third-state discriminator {alive, hung, cleanly-stopped} — already designed in, preventing stop-vs-hang conflation. |
| Item 49 — replay budget exceeded / not | threshold on measurement | KEEP-BINARY | Operator-read measurement doc, methodology stated; not an automated verdict. |

### 2.4 Items 50–54 (Validity/K3 arc)

| Location | Current logic shape | Classification | If SHOULD: proposed third state |
|---|---|---|---|
| Item 50 — `RejectionClass::{Refuted, Undecidable}` + internal `TruthState` K3 fold | full three-valued design | ALREADY-CORRECT | The template itself: evidence-based Unknown, behavioral collapse at the seam, distinct telemetry class, exhaustive 9-case truth-table proof, Kani via item 7. |
| Item 51 — shadow-mode `Kind::ShadowDivergence` | verdict class + agreement bit + digests | ALREADY-CORRECT (one totality note) | Refuted vs Undecidable vs Admitted already carried per record; emission policy distinguishes the classes. Note (spec sentence, not a state): define the agreement bit's totality — it is total iff every constructible `Proposal` carries a comparable action (typed struct ⇒ true today); record that reasoning so a future partial/malformed-proposal variant can't leave the bit silently undefined. |
| Item 52 — miri-gate | pass / UB / **unsupported-under-interpretation** | ALREADY-CORRECT | The third state is explicitly modeled and documented: SIMD/PMU intrinsic bodies "exercise the fallback / error under interpretation, documented not silently green"; a green gate "is never read as SIMD/PMU is Miri-clean." Exemplary claim-level honesty. |
| Item 53 — lint-gate (clippy/fmt) | per-lint RED/GREEN | KEEP-BINARY | Deterministic lint verdicts on a completed run; infra failure ⇒ RED. |
| Item 54 — Sentinel live-struct CRC | match/mismatch at transition points | KEEP-BINARY (two notes) | Same in-memory-fact reasoning as item 40. Notes: (a) the acknowledged missed-re-hash "false trip" is a *wrong-False* risk (not an epistemic third state) and is already bounded by centralizing write sites — correct treatment; (b) inherits item 40's never-ran observability sub-flag (checks-performed counter on the heartbeat). |

### 2.5 Kernel code precedents (task item 4)

| Location | Current logic shape | Classification | If SHOULD: proposed third state |
|---|---|---|---|
| **`markov::Verdict` (`kernel/src/markov.rs:42`) — Healthy/LimitCycle/StrangeAttractor** | 3-state *domain* classification, fail-OPEN on unknowns | **SHOULD-BE-3-VALUED (strong; the one fail-open-in-the-wrong-direction instance found)** | The three variants are fine as domain classes (all definite, measured). The defect is the epistemic edge: `analyze_detailed` (markov.rs:110) maps **window-too-short ⇒ `Verdict::Healthy`**, and `markov_attractor.rs:36` maps **analyzer-error (stdin read failure) ⇒ `"HEALTHY"`** — Unknown emitted as the MOST lenient verdict, the exact "Unknown dissolving into the system" failure RAW-PROMPT-7 warns against, inverted relative to every fail-closed gate in this roadmap. Mitigations exist but leak: the `reason` string carries the truth in CLI JSON, **but item 27's FDR record carries only `verdict_str()` + PMU (`markov_attractor.rs:100`) — in FDR telemetry, "couldn't analyze" is byte-identical to "measured healthy."** Behaviorally, fail-open is *defensible* here (advisory hook, "never break the hook"; no-evidence ⇒ no-intervention is the right decision) — so per the item-50 shape, keep the behavior, fix the record. Proposal: (i) a typed evaluation marker on `Report` (e.g. `evaluated: bool` or a small `Basis::{Measured, WindowTooShort, AnalyzerError}` enum) — NOT a fourth `Verdict` variant (the CLI JSON contract is golden-pinned byte-identical; the FDR field is additive under the item-27 optional-field discipline); (ii) `emit_verdict_pmu` grows an optional basis field so FDR distinguishes measured-Healthy from unevaluated-Healthy. Downstream (item 9's breaker, item 21) must never count an unevaluated-Healthy window as health evidence. |
| **`spectral::DriftClass` (`kernel/src/spectral.rs:674`) — Damped/Resonant/Unstable** | 3-state domain classification, fail-CLOSED on unknowns | SHOULD-BE-3-VALUED (moderate; record only, behavior + wire kept) | Domain classification is fine as-is. `classify_drift` (spectral.rs:705-731) collapses THREE distinct cannot-evaluate causes — non-finite entries, ragged matrix, `from_vecvec_checked` Err — into `DriftClass::Unstable`. The *behavioral* collapse is correct and stays (fail-closed, rejected by `RetainedBase::admit` — exactly item 50's Unknown-acts-as-False rule, and a deliberate fix of an earlier silent-admit bug). What's missing is the distinct record: "measured ρ>1 divergent" vs "operator ill-formed, unevaluable" are conflated in the typed result, so telemetry/forensics cannot separate a genuinely diverging loop from NaN-poisoned input. The pinned wire contract (`wire_code` 0/1/2, round-trip test) makes a fourth variant expensive — don't add one. Proposal: an out-of-band provenance (e.g. `classify_drift_with_rho`'s report path or the item-27-style optional FDR companion field carries `DriftBasis::{Measured, IllFormedInput(cause)}`); `DriftClass` and its wire code stay byte-identical. |
| **`fdr::Reading<T> = Value(T) \| Unavailable(Absence)` (`fdr/schema.rs:62`, exec branch)** | 2-state named absence | ALREADY-CORRECT — and correctly NOT three-valued | A measurement is not a proposition; it has no "False." Two-state-with-named-reason is the right epistemics for its plane (the synthesis's own `None ≠ Unknown` distinction, one level down). Converting it to K3 would be a category error. The real obligation sits with CONSUMERS: any code that folds a `Reading` into a boolean/threshold decision (item 9 trip predicates, item 21 gain-scheduling, item 44 cost reports) must state its Unavailable policy — covered by the item-9 finding above; no change to `Reading<T>` itself. |

## 3. The SHOULD-BE-3-VALUED findings, ranked by significance

1. **Item 33 — CONFIRMED/REFUTED must gain `Unresolvable(noise-bound)`.** The arc's own
   grounding (fold_transitions NOISE-BOUND ±40% CI vs a claimed +16.6% effect) proves the forced
   binary will fabricate verdicts on exactly the contested benches. Third state carries measured
   CI + claimed delta and files a stabilization ticket.
2. **`markov::Verdict` fail-open — Unknown currently reported as Healthy, and the FDR record
   can't tell.** The only place in the audited surface where an epistemic gap collapses to the
   *lenient* pole. Keep the advisory behavior; add the typed basis marker + optional FDR field
   (`markov.rs:110`, `markov_attractor.rs:36,100`).
3. **Item 7 (and 10/11) — Kani/TLC verdicts are natively three-valued**
   ({Proved, Refuted(cex), Undecidable(bound/timeout)}); CI collapses to RED either way, but the
   class must be recorded — the two non-green classes demand different responses.
4. **Item 9 — `Tripped` cause class + the unstated `Reading::Unavailable` input policy** for
   trip predicates (and item 21's gain-scheduling): `TripCause::{Exceeded, Unevaluable}`,
   Unavailable evaluated at the conservative pole, never as healthy.
5. **`spectral::DriftClass` — fail-closed collapse is right, the conflated record is not:**
   ill-formed-input vs measured-divergent logged distinctly; wire code untouched.
6. **Item 12 — vote-outcome class** `{Unanimous, SingleDissent, NoMajority}` on the FDR entry;
   both failure classes trip identically.
7. **Item 43 — state the fail-closed default for an unclassifiable input plane**
   (unprovable-public ⇒ secret-adjacent, recorded as its own classification value).
8. **Item 6/43 dudect — record pass-at-power-N**, never "CT proven"; `Inconclusive` (underpowered)
   distinct from `LeakFound`, both RED.

## 4. What was deliberately NOT converted (over-application guard, per the directive's own "except" clause)

- All completed-run CI gate outcomes (items 1+13, 14, 44, 45, 53; item 6's gate mechanics).
  The genuine third state ("check never exercised anything") is already detected and collapsed
  RED by the anti-forgery clauses — adding a public Unknown arm would create the leniency
  loophole item 50 exists to prevent.
- All byte/bit-equality differentials and goldens (items 3, 5-match, 8, 18, 37, 39, 42) — facts.
- In-memory checksum verdicts (items 40, 41, 54) — no I/O, no couldn't-compute state that
  produces a verdict; unreadable memory is a process-fatal fault owned by item 48's crash class.
  (Observability sub-flag on "never-ran" noted, not a third verdict state.)
- `Reading<T>` — named absence is the correct two-state shape for measurements; K3 would be a
  category error.
- Operator rulings (§0, item 34) — decisions, not truth claims.

**Net assessment:** the roadmap is substantially consistent with the directive already — items
50/52/27/35/46/5 show the pattern applied correctly across four different planes (admission,
interpretation-limits, measurement, provability, parsing). The 8 findings are concentrated where
verdicts summarize *statistical or resource-bounded* processes (33, 6/43-dudect, 7/10), where a
*consumer policy over named absence is unstated* (9/21), and in the two pre-roadmap kernel
classifiers whose epistemic edge cases predate the item-50 discipline (markov fail-open,
spectral conflated fail-closed). None of the 8 requires a third control-flow arm; all follow
item 50's shape — behavioral collapse to the safe pole, distinct typed cause in the record.
