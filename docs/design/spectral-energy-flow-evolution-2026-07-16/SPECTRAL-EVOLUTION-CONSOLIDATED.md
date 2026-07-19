# SPECTRAL-EVOLUTION — Consolidated Index (2026-07-16)

> Arc: `docs/design/spectral-energy-flow-evolution-2026-07-16/`, branch
> `feat/spectral-energy-flow-evolution` (worktree). **Planning artifact only — no code is written or
> edited by this arc.** This document is the entry point; the five source documents below remain the
> source of truth and are deliberately KEPT, not merged away (see §6 for why the Detailed Planning
> Protocol's delete-intermediates step is not applied here).

---

## §0 — Index and how to read

This arc classified 10 external concept clusters (physics, AI-systems, statistics — an operator
reference package) against the live "energy spectral circuit-flow" architecture (`spectral.rs`,
`csr.rs`, `impedance.rs`, `harmonic.rs`, `kalman.rs`, `noether.rs`, `field_frame.rs`, `causal.rs`,
`evals.rs`, …) under the Hermetic audit's V6 metaphor-discipline standard: a concept "connects" only
if a named, computed, falsifiable thing in the codebase would be extended or corrected. Three
clusters survived as STRONG, one as a borrowed pattern, six were rejected; the survivors were
verified by a decorrelated pass and blueprinted. Read in this order:

1. **`RESEARCH-AND-REASONING.md`** — live-code grounding (§1), the 10-cluster classification (§2),
   sketches (§3), honest verdict (§4). Corrections from the verification pass are already applied
   in-place as `⚠ CORRECTED` markers.
2. **`RESEARCH-VERIFICATION.md`** — the decorrelated re-check of the 5 load-bearing claims. All 5
   survived; one real symbol-name error was found (`append_jsonl` → `append_to`/`to_jsonl`) plus a
   wiring caveat on `bridge.rs:125`.
3. **`BLUEPRINT-E1-gradient-lyapunov-and-laplacian-parity.md`** — execution-ready.
4. **`BLUEPRINT-E2-clt-uncertainty-primitive.md`** — execution-ready.
5. **`BLUEPRINT-E3-self-harness-loop-for-llm-harness.md`** — Phase A execution-ready; Phase B
   explicitly gated (see §4).

This document indexes; it does not re-blueprint. Where a summary sentence here and a blueprint
disagree, the blueprint wins.

---

## §1 — The classification, restated compactly

Full derivations and file:line evidence: `RESEARCH-AND-REASONING.md` §2 (table) and §3 (sketches).

| # | Cluster | Bucket | One-line reason |
|---|---------|--------|-----------------|
| 1 | Gradient unification (field = −∇potential) | **STRONG** (narrowed) | Both halves live (`online.rs` monotone descent; `field_frame.rs` diffusion); missing computed objects = Dirichlet-energy Lyapunov gate + incidence parity-bind for the 4×-fractured Laplacian. → E1 |
| 2 | Central Limit Theorem | **STRONG** | Already implemented once, imprisoned in a single test (`causal.rs:2226-2256`); absent everywhere a number is reported (recall@5, brier/ece/aurc, `RegressionGate` tol). → E2 |
| 3 | Apache Spark (9 concepts) | **BORROWED PATTERN** | One concept — fault-tolerance-via-lineage — dowiz already reimplemented natively (event log + folds + `boot_verify`); the other 8 rejected; Spark never a dependency (M6). → §3 below |
| 4 | Self-Harness (arXiv:2606.09498) | **STRONG** | The 3-stage propose/validate/rollback loop exists in embryo at one-scalar scale (`evals.rs` `SelfAdaptator`/`RegressionGate`); extension = lift to the LLM-harness config surface. → E3 |
| 5 | AIDE² recursive self-improvement | **WEAK** / merges into #4 | Its one non-duplicated idea (hidden survival score) is already structurally present (Noether guard independent of eval loss); no distinct falsifiable change. |
| 6 | Quantum steering trust bound | **WEAK** / rejected | No computable H(B\|E) anywhere; mesh trust is signed capability, never a score (M12). V6 fail. |
| 7 | Series/parallel circuits (KCL/KVL, R_eq) | **WEAK** / rejected with precision | KCL already lives as the tested `L·1 = 0` invariant (`csr.rs:794-815`); `impedance.rs` is queueing, docstring rejects the circuit metaphor, zero callers (extending a stranded organ violates P2). R_eq = L⁺ is real math with no named consumer (closest: unbuilt M7 heal, audit #26). |
| 8 | Prioritization frameworks (Pareto, RICE, Kano) | **WEAK** / rejected | Operator planning heuristics, not architecture; no code hit. |
| 9 | Claude Skills authoring guide | **WEAK** / rejected as architecture | Dev-tooling docs; zero contact with the spectral-flow architecture. |
| 10 | Speculative physics (CTCs, ER=EPR, volatility) | **WEAK** / rejected | Novikov↔replay-idempotency is word-play over an already precise, tested vocabulary; no code hit for any member. |

Tally: 3 STRONG (1, 2, 4) · 1 BORROWED (3) · 6 rejected (5-10).

---

## §2 — The three STRONG blueprints, summarized with pointers

**E1 — Gradient/Lyapunov energy gate + Laplacian parity-bind**
(`BLUEPRINT-E1-gradient-lyapunov-and-laplacian-parity.md`). Core decision: do **not** merge the ≥4
Laplacian implementations (they serve three different hot paths); instead land a small canonical
reference operator — new `kernel/src/incidence.rs`, `L = BᵀWB` emitting the **positive** `(D−A)`
convention — and parity-bind every other implementation to it by test, plus a thin one-sided
`noether::lyapunov_nonincreasing` helper so the field integrator finally carries an
energy-monotonicity certificate. The most load-bearing finding: a **live, unpinned opposite-sign
split** — `field_frame.rs:103` emits `−(D−A)` while `csr.rs:316-325` and `spectral.rs:287-297` emit
`+(D−A)`; today, flipping `field_frame.rs:103`'s sign turns **no test red** (E1 §4.2). Each side is
internally correct — this is an unpinned seam hazard (anti-diffusion for any future caller crossing
it), not a defect in either module; `RESEARCH-VERIFICATION.md` Check 1 is explicit that the
integrator itself is physically correct. E1 closes the *code half* of Hermetic finding #8, whose own
backlog revisit-trigger ("`csr::laplacian_spmv` gains its first production caller",
`HERMETIC-REMEDIATION-PLAN.md` §5) has fired at `engine/src/bridge.rs:125` — with the verification
pass's caveat that `apply_field` is a wired public API not yet driven by a live loop.

**E2 — CLT uncertainty primitive** (`BLUEPRINT-E2-clt-uncertainty-primitive.md`). Core decision: a
new zero-dep leaf `kernel/src/stats.rs` (divergence from edit-don't-create justified by layering:
consumers span `causal.rs`, `evals.rs`, and retrieval tests, and putting it in `evals.rs` would force
`causal.rs` to import upward from the harness layer), relocating the byte-identical CLT envelope from
`causal.rs:2226-2256` plus `mean_se`/`wilson_interval`/seeded `bootstrap_interval`. The number:
**Wilson 95% lower bound ≈ 0.76** for the live recall@5 oracle — which is **12 queries**, not the
29-question figure memory notes cite (that was the earlier 2026-07-07 oracle, whose bound is ≈ 0.88).
The headline "recall@5 = 1.0" becomes a falsifiable statement instead of a self-certified constant,
directly answering the memory arc's own "NEXT bigger oracle" by quantifying what a bigger oracle
buys. Wilson is chosen over Wald (degenerates to `[1,1]` at p̂ = 1) and over Clopper-Pearson
(transcendental, per-target-only under `rng.rs`'s reproducibility doctrine — deliberately unshipped).

**E3 — Self-Harness loop for the LLM harness**
(`BLUEPRINT-E3-self-harness-loop-for-llm-harness.md`). Core decision: a hard **phase split**. Phase A
(buildable now, advisory-only): give the appender-only `EvalRow` trace log (`evals.rs:489`,
`append_to` — the verification pass's corrected name) its first reader via a weakness-mining reducer;
proposals are DATA on a **single-axis, enumerable, bounded lattice** over the real harness knobs
(`StackBuilder`, `ChatRequest`, model-per-`TaskClass` allowlist); validation re-runs the frozen
mint-log-pinned metamorphic suite and tags `recommend`/`flag-regression` — a human applies, never the
loop. Phase B (auto-apply) is RC-2 self-certification unless validation is re-executed by an
independent identity, so it hard-depends on P06 `key_V` (**corrected 2026-07-19: no longer zero
code hits — key_V's `HybridSigner` closed `58987d79d` 2026-07-18, real bebop2-kv sign/verify + TLV
sig field, survived the later merge wave; the named precondition is now met**). Phase B remains
not-designed and un-started (no `HarnessConfig`/self-harness-loop code exists anywhere in-tree as
of this correction) — closing the blocker makes Phase B startable, it does not make it started;
re-verify key_V's live state before resuming. The sharpest finding: the blueprint's own live re-read **killed
a knob the research sketch had invented** — there is no retry policy anywhere in the harness
(`dispatch.rs` is degrade-closed, `quirks.rs` holds only wire-correctness deltas), so the lattice
excludes it; meanwhile `track_record.jsonl` already flows from every real dispatcher call, giving the
reducer live bootstrap data before `EvalRow` emission is even wired.

---

## §3 — The borrowed pattern, as an E53-form backlog entry

Form per `BLUEPRINT-P02-canon-repair-operator-decisions.md` §4:
`{what, why-suspended, named-owner, falsifiable-revisit-trigger, date}`.

- **Spark-lineage-as-native-pattern → shared `Projection`/replay seam.**
  *What:* dowiz's event substrate already IS lineage-based fault tolerance — content-addressed
  hash-chained events (`event_log.rs:148`), structural replay idempotency (`:305`),
  decide-before-persist (`:339`), per-subsystem fold reducers (`order_machine.rs:140`,
  `analytics.rs:13-22`, `intake.rs:47`), and post-restart re-derivation (`hydra.rs:253`
  `boot_verify`). State is recomputed from the transformation log, never copied. The borrowed lesson
  (Spark is pattern donor only, never a dependency — M6 zero-dep) is that lineage scales when replay
  is first-class: extract one `Projection` seam (`fn apply(&mut State, &MeshEvent) -> Result<…>`) +
  one `replay(log, projection)` driver, with snapshot-as-checkpoint and snapshot/replay parity checks
  (`RESEARCH-AND-REASONING.md` §3.4).
  *Why-suspended:* only two-ish fold consumers exist today; extraction without a third forcing
  consumer is speculative unification — the same deferral P2 itself prescribes for dead/unforced
  primitives.
  *Named-owner:* whoever implements P7 — the **same owner as `HERMETIC-REMEDIATION-PLAN.md` §5
  backlog row #21**, which this entry deliberately joins rather than duplicates.
  *Falsifiable-revisit-trigger:* **row #21's trigger, verbatim — do not mint a second one:** the
  third fold consumer appears; named-and-near, since P07 §5 routes money ledger-entry events through
  `commit_after_decide`, plausibly that third consumer — "the P7 implementer must check this trigger
  at build time, not after." One design question rides the trigger and must be resolved then: whether
  opaque `MeshEvent.payload` bytes or a typed event enum is the replay unit
  (`RESEARCH-AND-REASONING.md` §3.4, open question).
  *Date:* 2026-07-16.

Explicitly rejected members of the same cluster (no backlog entry, by design): RDD/partitioning/
shuffle (no distributed dataset), lazy DAG/Catalyst (kernel is deliberately eager and deterministic),
DataFrames (no relational layer). Caching is real but already the independently-4×-corroborated P0 —
attributing it to Spark adds no new claim.

---

## §4 — Sequencing

**Cross-checked against each blueprint's own scope/migration sections — and the "fully disjoint"
assumption does NOT survive the check.** The actual file footprints:

- **E1** (§2-§4): new `kernel/src/incidence.rs`; a ~15-line one-sided helper in
  `kernel/src/noether.rs`; parity/sign-pin tests kernel-side; the energy gate as an **engine-side
  test module** around `FieldFrame::step` (no integrator change; `field_frame.rs` runtime code and
  all public signatures untouched, E1 §4.6); one `pub mod` line in `kernel/src/lib.rs`.
- **E2** (§3): new `kernel/src/stats.rs` + one `pub mod` line in `kernel/src/lib.rs`; test-only
  rewrite in `causal.rs`; interval assertions in `retrieval/tests.rs` and `living_knowledge.rs`;
  **and `evals.rs`** — `brier_ci`/`ece_ci`/`aurc_ci` companions (step 5) and an interval-aware
  `RegressionGate` constructor (step 6).
- **E3 Phase A** (§3): **`evals.rs`** — the weakness-mining reducer (step 1, explicitly
  edit-not-create) and validation wiring over the frozen suite (step 4); the `HarnessConfig` +
  single-axis lattice as a new data type over the `llm-adapters`/`ports/llm.rs` config surface (it
  *reads* `compose.rs`/`ports/llm.rs`/`ollama.rs`; exact home of the new type is an implementation
  decision); wiring the harness eval run to call `append_to`.

**Verdict: logically independent, not fully file-disjoint.** No blueprint consumes another's output —
each is buildable alone, and E3's header states Phase A has no hard dependency. But two collisions
exist at the file level: (1) **E2 ∩ E3 = `kernel/src/evals.rs`** — different functions, same hot
file; (2) E1 ∩ E2 = one registration line each in `kernel/src/lib.rs` (trivial, but a merge point).
Note that E2's own header ("parallel-safe with every other spectral-evolution blueprint") is true of
logical dependency and overstated for concurrent file edits. Per the repo's own fan-out rule (never
mutate the same file concurrently; keep shared registration for the lead), the honest sequencing is:

- **E1 runs fully parallel** to everything (its only shared touch is the `lib.rs` one-liner).
- **E2 and E3-Phase-A are parallel as work-streams but their `evals.rs` edits must land serially.**
  Cheapest resolution: land E2 first — it is the smaller diff, and its SE-derived `RegressionGate`
  tolerance is exactly what E2 §5 says makes E3's accept/reject "statistically grounded." That is a
  soft quality edge, not a hard dependency; building E3 first with the hand-tuned tol is legal, just
  worse.
- **E3 Phase B is NOT sequenced.** It was blocked on P06 `key_V` independent re-execution; that
  precondition closed `58987d79d` (2026-07-18) — see the §3 correction above. Still no date, no
  design, and no code beyond the named precondition; the block is dissolved but nothing has picked
  the work up yet.

---

## §5 — The 2-question doubt audit (per `AGENTS.md`, blueprint-organization stage)

**Q1 — least confident about, in this consolidation specifically.**

1. **File-disjointness — cross-checked, not asserted, and the check changed the answer.** The task
   framing assumed E1/E2/E3 touch disjoint files; reading each blueprint's migration steps shows E2
   step 5-6 and E3 step 1/4 both edit `evals.rs`. §4 reflects the corrected picture. Residual
   uncertainty: E1's engine-side energy-gate tests could land inside `field_frame.rs`'s `#[cfg(test)]`
   module rather than a separate test file — the blueprint pins "no runtime contract change" but not
   the test file's name; if they land in-file, E1 "touches" `field_frame.rs` test-only. No collision
   with E2/E3 either way.
2. **Drift check — one phrase actively resisted.** The task brief describes "a live sign-convention
   bug"; the sources are more careful — each module is internally correct and internally tested, the
   hazard is the *unpinned relationship* across the seam (`RESEARCH-VERIFICATION.md` Check 1 states
   the integrator is not buggy). This document says "split/hazard," not "bug," on purpose.
3. **I did not independently re-read the `.rs` files.** This consolidation trusts the
   research + decorrelated-verification pair for all file:line claims (that pairing is the arc's
   whole verification design), but a third read was not performed. Routine risk, stated.
4. **E3's `HarnessConfig` home is genuinely unpinned** — the blueprint names this as an
   implementation-time decision rather than inventing a path; this index preserves the gap rather
   than papering it (protocol step 4's "naming a real gap honestly" case).
5. **The Wilson numbers (≈0.76 / ≈0.88) are carried, not recomputed here.** E2 itself orders the
   implementer to reproduce them exactly (E2 §4.3-4) — that check is where the numbers get their
   second party, not this document.

Item 1 was the only "real risk" bucket entry, and it was investigated to root (the blueprints' own
migration steps), not left as a footnote.

**Q2 — the biggest thing this arc might be missing.**

The named open questions all survived consolidation — none were dropped: E1's bebop `field.rs:82`
cross-repo bind (scoped out per finding #18's cross-repo-pin lesson, named future work), E1's
boundary-mask choice and empirically-pinned `tol_E`, E2's non-iid EMA-stream decision (preferred:
move the statistic upstream of smoothing; fallback: seeded circular moving-block bootstrap; rejected:
Hoeffding) and its deliberately-unshipped Clopper-Pearson, E3's Phase-B `key_V` precondition and
runner-identity residual (E3 §5), and the borrowed pattern's payload-vs-typed-enum question (§3
above). What the arc **does** quietly lack: the rejected clusters leave no structural trace. In
particular cluster 7's genuinely real fragment — effective resistance R_eq = L⁺ with M7 mesh-heal
(audit #26) as its only plausible consumer — is rejected "until a consumer names it," but unlike the
Spark pattern it gets no E53 row and no trigger; if M7 heal work starts, nothing forces the R_eq note
to resurface. 🔴 Flagged, not solved: remediation backlog row #26 (M7 topology) is the natural
carrier, and its owner should add one sentence when that row's own trigger fires. This document does
not edit the remediation plan.

---

## §6 — Anu (logic) & Ananke (organization) check

**Anu.** The §4 sequencing is derived, not asserted: the parallelism claim was re-derived from each
blueprint's own migration-step file lists, and the derivation *changed the conclusion* (from "fully
disjoint" as briefed to "logically independent with one shared hot file") — which is what a real
dependency re-check looks like, per protocol step 2. The one ordering preference (E2 before E3) is
explicitly labeled a soft quality edge with its reason (SE-derived tol grounds E3's gate) rather than
promoted to a fake hard dependency; E3-Phase-B's block is traced to a named, checkable precondition
(P06 `key_V`, zero code hits) rather than "later." One sibling-contradiction was found and named
rather than left standing: E2's "parallel-safe with every other spectral-evolution blueprint" header
vs. the `evals.rs` collision (§4, §5 Q1.1).

**Ananke.** Does this document get found without someone remembering it? Partially: every source doc
is one hop away for anyone who reaches the arc directory — useless for anyone who doesn't. The cheap
structural fix, named concretely: **MEMORY.md's active-arcs index should carry this arc as one line
pointing at THIS file** (the index-line-per-arc pattern MEMORY.md already enforces), so the
established discovery route — memory index → arc file — lands here first. Additionally, §3's E53
entry deliberately reuses remediation row #21's owner and trigger, so the backlog check that already
must happen at P7 build time structurally re-surfaces the lineage decision without anyone recalling
this arc exists. One protocol deviation, stated: step 7 says merge working docs and delete
intermediates; here the three blueprints are kept as execution artifacts (source of truth) and the
research + verification pair as the claim-provenance chain (corrections live in-place as
`⚠ CORRECTED` markers) — this document is the single navigable entry point over them, serving step
7's purpose (one file to reconcile, not five drifting ones) without destroying provenance.

---

*Consolidation written 2026-07-16 on `feat/spectral-energy-flow-evolution`. Sources: the five arc
documents (all read in full), `HERMETIC-REMEDIATION-PLAN.md` §5 (rows #8, #21, #26),
`BLUEPRINT-P02-canon-repair-operator-decisions.md` §4 (E53 form), `AGENTS.md` (2-question ritual,
Detailed Planning Protocol, Anu/Ananke doctrine). No code written or edited.*
