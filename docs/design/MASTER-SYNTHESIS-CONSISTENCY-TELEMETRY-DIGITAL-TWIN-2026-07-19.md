# MASTER SYNTHESIS — Consistency Corrections · Pervasive Telemetry · Digital Twin · Governed Self-Evolution (2026-07-19)

**Kind:** master synthesis + blueprint pass (Fable), operator-facing summary. **Planning only —
the operator has instructed NO execution dispatch after this pass; every deliverable below is a
document awaiting review.** Every claim herein is GROUNDED in the seven inputs (or in code
spot-checked this pass, cited where so) or explicitly PROPOSED; the COVERAGE-COMPLETE,
PRECISION-HONEST discipline of input 6 is applied to this document itself.

## 0. What this pass merged

Seven inputs (six staged docs + one operator directive delivered mid-pass):

1. `AUDIT-SPACE-GRADE-CONSISTENCY-DEPLOYMENT-NATIVENESS-2026-07-19.md` — 4 deployment-context
   corrections + 1 ambiguous framing + 2 nativeness gaps (all applied below; §1).
2. `AUDIT-BINARY-VS-KLEENE-LOGIC-2026-07-19.md` — 46 decision points: 8 SHOULD-BE-3-VALUED /
   27 correctly binary / 11 already correct → items 55–56 (+ item 12's VoteOutcome).
3. `AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` — 13 gaps (G1–G13), the
   work-normalized cost ledger, the honest AI-optional verdict → items 57–63.
4. `RESEARCH-OS-ARCHITECTURE-PATTERNS-ADOPTION-2026-07-19.md` — 3 genuine adoptions + 1 small
   gap; full seL4/OTP/IPC-bus/preemptive-RTOS ruled category mismatches → items 64–66 + the
   item-12 temporal-TMR merge.
5. `RESEARCH-NATIVE-KANI-REPLACEMENT-FEASIBILITY-2026-07-19.md` — 16/22 native, 0/22 need SAT;
   already enacted as item 7's blueprint v2 (16 native + 4 Kani now + 2 → item 8). No new item;
   roadmap item 7's entry updated for consistency (it still carried the pre-rescope wording).
6. `RESEARCH-RESOURCE-FOOTPRINT-ZERO-BLINDSPOT-RELATIONAL-TELEMETRY-2026-07-19.md` — five
   threads: derived footprint views, zero-UN-NAMED-blind-spots, FDR relational linkage, the
   completeness procedure, the predictive-oracle principle + digital-twin split → items 62,
   67–72 + the new standing procedure doc.
7. **Operator directive (mid-pass, three parts, verbatim in roadmap §L):** AI may build/change
   the internal OS but never core/red-lines, only under explicit human approval, at the same
   space-grade telemetry/predictability standard; self-healing and self-upgrading included; the
   gate itself structurally unbypassable → items 73–78.

**Item-7 execution status at this pass:** no DONE marker/commit SHAs in the roadmap — treated as
"executing per its rescoped blueprint," not blocked on.

## 1. Corrections applied (Part 1 — confirmed real, edited in place, all staged)

| # | Where | What changed |
|---|---|---|
| 1a | `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS…` §6 | The SIHFT valuation sentence ("On ECC-RAM Hetzner hosts the residual value is modest… not a recommendation") — the exact rejected cloud-ECC reasoning the operator reversed for Sentinel/item 54, never retro-corrected — WITHDRAWN and replaced with the corrected premise: local, offline-first, non-ECC consumer target ⇒ compute-path SEU class material, software voting recommendation-grade for its narrow scope. Shared-silicon caveat + pure-computation scoping kept (genuine engineering ground). |
| 1b | Same doc §9 item 12 | "optional, operator's defense-in-depth call" corrected to PURSUE with the retro-corrected premise; points at the merged item-12 re-scope. |
| 1c | Roadmap §0 + §E item 12 | Ruling rows annotated with the corrected premise (the §J/item-54 treatment); **item 12 re-scoped in place as TEMPORAL TMR** — the audit-A finding and doc-4's temporal-TMR research are the same underlying redundancy concept, MERGED into one refined item (no new number): sequential triple-run + trivial-equality vote on µs-scale pure functions; additive over item 54 (at-rest vs compute-time halves); honest partial-defense caveats carried; `VoteOutcome::{Unanimous, SingleDissent, NoMajority}` FDR class added (Kleene finding 6). Still design-only behind items 9 + FDR — the sequencing was never premise-poisoned, only the valuation. |
| 2 | Same doc §11 T2 | "This kernel targets x86 Hetzner hosts" → local, offline-first consumer hardware (x86_64 + wasm32; not Hetzner-specific, not x86-only). T2 gate-closed verdict unchanged on its independent ground. |
| 3 | `BLUEPRINT-ITEM-27…` §5 + line ~113 | Dev-host-vs-deploy-target conflation fixed: the probed box is an AMD EPYC-Milan KVM **cloud dev guest**, not the deploy target; on the actual local target the availability picture typically inverts (RAPL usually present; `perf_event_paranoid` normally 2, not 4). Named-absence design unchanged — it is deployment-agnostic. |
| 4 | Roadmap §C item 31 | **`kernel::json` vs `fdr::json` dedup ticket FILED** (dual-Keccak ticket format): two JSON-write/escaping surfaces = the §10/P2 failure shape; consolidate under `kernel::json::write` OR record a permanent escaper parity pin + one-escaper rule. Owed to item 25's ledger. |
| 5 | Roadmap item 46 | Fleet-heterogeneity sentence added (audit finding 1.5, ambiguous-class): multi-ISA reopening trigger evaluated against heterogeneous local-first peers, not a single-host assumption. (The parallel framing in the CRASH synthesis doc was left untouched — the roadmap is the live truth; noted here for the record.) |
| 6 | Roadmap item 51 | Digest primitive named in-spec (audit 3.3): in-kernel CRC32 or truncated in-kernel SHA3 — never a third ad-hoc hash. |
| 7 | Roadmap §C item 7 | Entry rewritten to reflect the enacted v2 rescope (16 native exhaustive / 4 Kani / 2 → item 8 / 0 SAT; Kani = CI-time terminal-state-(c) boundary; no longer gates on Kani toolchain bootstrap). Status annotated as executing, no DONE marker. |

Zero other deployment-context violations existed (the audit cleared 7 correct instances and 18
legitimate scope-downs); nothing else was touched.

## 2. New roadmap items (Part 2 — §K items 55–72, §L items 73–78; 24 items total)

### §K — Consistency retrofit, pervasive telemetry, digital twin (55–72)

- **55 — K3 verdict-class spec retrofit** (Kleene findings 1/3/4/7/8 as in-place amendments,
  item-50 shape: safe-pole collapse kept, typed cause added — item 33 `Unresolvable(noise-bound)`;
  Kani/TLC `{Proved, Refuted, Undecidable}`; item 9 `TripCause` + the Reading-Unavailable
  conservative-pole policy; item 43 `Unclassifiable ⇒ secret-adjacent`; dudect
  pass-at-power-N, never "CT proven").
- **56 — kernel classifier epistemic-basis retrofit** (the one fail-open-to-lenient instance
  found: markov window-too-short/analyzer-error ⇒ `Healthy`, FDR-indistinguishable from measured
  health — behavior kept, `Basis` record added; spectral `DriftBasis` out-of-band, wire codes
  untouched).
- **57 — telemetry-completeness procedure RATIFIED + HOT-PATHS `eff` column** (the enforcement
  spine; the item-25 pattern; G9 posture ruled: cheap envelope always-compiled, heavy stamps
  feature-gated).
- **58 — work-normalized cost ledger** (pairs-never-ratios; closed workload-kind enum; E/C/T
  tier ladder self-describing via `Reading<T>`; cross-tier ratios structurally uncomputable).
- **59/60/61 — the three gap-closure deployments** (agent-turn timing G1+G2+G12 — the only
  boundary where work is counted but time is not; engine frame/voice G3+G11 — engine has zero
  `Instant` today; kernel counters G5+G6+G7+G8 — event-log feed for the operator's group-commit
  gate, subprocess rusage, eigensolver spans, crypto-span gating fix). G10 (bebop NTT — zero
  perf numbers on the heaviest new math) and G13 (apps/api) recorded as named out-of-arc flags,
  not items.
- **62 — FDR relational linkage** (`span_id` + `parent_span_id: Reading<u64>`,
  `Unavailable(NoParent)` at roots — the schema is FLAT/UNLINKED today, grep-confirmed; P3
  plane, ~16 bytes; + the wasm clock leg closing G4).
- **63 — item-45 spec extension** (disposition table over `micrograd`/`online`/`attention`/
  ports/engine-voice: CORE-DETERMINISTIC / AI-EDGE / SANCTIONED-SEAM; build-provenance FDR
  record; feature-matrix CI legs; audit's P1 "dispatch item 45 now" recorded as a
  recommendation to the operator).
- **64 — capability-secure declarative composition root** (the strongest adoption — the only
  pattern backed by a proven defect: item 2's "no production composition root constructs the
  durable store"; DAG-validated init order reusing the `order_machine` proof kit; per-module
  declared capabilities, fail-closed; subsumes the item-2 fix; sole minter of capability
  tokens).
- **65 — typed in-process AI/agent capability boundary** (the proportionate seL4 slice;
  unforgeable zero-sized token required by signature at core ports; reuses `capability_cert`
  machinery; + uniform per-port fail-closed containment test — the OTP slice).
- **66 — periodic durable-log scrub** (the one journaling-FS gap; existing CRC32/SHA3 only;
  gated on 64).
- **67 — cost-oracle classification backfill** (every HOT-PATHS row: ORACLE-EXACT /
  ORACLE-BOUNDED / MEASURED-ONLY + traceable evidence; unclassified = forbidden; reuses the
  Kani B/C split as evidence; CT functions inherit EXACT from dudect for free).
- **68 — cost capture as a correctness-test byproduct** (cycle capture inside item 7's native
  exhaustive sweeps; analytic intervals for fixed schedules; the honest noise caveat carried at
  the exact end too).
- **69 — water/carbon as derived views** (joules × operator-supplied regional constant; on-site
  water a PERMANENT named absence; no new measured field — adding `water_ml` to a stamp is a
  standard violation).
- **70 — state-mirroring digital twin (A): REAL, near-term** — a composition of 67+68 + the
  aggregate call-graph layer (ρ(A)/`classify_drift`/Laplacian/markov reused AS-IS, graph-level
  only) + the `eqc-rs` precedent; the forced-metaphor guard binding: no spectral quantity ever
  presented as a per-leaf cost (doc 6's Anu/Ananke finding, carried exactly).
- **71 — cost-aware eqc-rs rewrite-extraction (B′)**: the ONE honestly-scoped near-term step
  toward auto-optimization — finite hand-curated rewrite set, op-count extraction, existing
  proof-program re-proof; no e-graph, no SMT, zero deps; operator-gated whether to build.
- **72 — auto-optimizing twin (B): LONG-TERM ASPIRATION, explicitly NOT promised** — named
  (STOKE/Souper/egg-egglog superoptimization) with entry criteria instead of proof conditions;
  zero commitment.

### §L — Governed Self-Evolution: AI-proposed change governance (73–78)

- **73 — the Gate-Root Invariant (FIRST, governing)**: the approval gate, red-line registry, and
  verification machinery are themselves CORE — structurally unmodifiable by the pipeline they
  govern (no gate-mutation capability exists in the type system; dependency-direction CI check;
  gate changes out-of-band ONLY, never through the pipeline, no beneficial-change exception —
  the corrigibility/tamper-proof-oversight problem addressed structurally, not by policy).
- **74 — the red-line/core registry**: machine-readable, every row cited — product red-lines
  (money/auth/RLS/migrations/bulk, the project's standing rules), the proven FSM +
  golden-signature, `import_unit`'s replay gate, event-log/FDR (the forensic truth surface),
  `pq`/`ct_gate`/`capability_cert`, ALL proof machinery (zero-dep/hardening/toolchain/miri/
  lint/ai-optional gates), items 9/54/64/65 as they land, and §L's own gate + registry
  (recursion recorded as rows).
- **75 — the change-proposal pipeline**: typed `ChangeProposal` → step-zero red-line refusal →
  the kernel's OWN gates re-executed (never a lighter checklist) → explicit human "apply" token
  (unconstructible without out-of-band operator action; silence ≠ consent) — item 47/50's
  grammar at the code plane; eqc-rs as the pre-verified-arrival precedent.
- **76 — lineage + cost-classified impact at approval**: full FDR causal tree per proposal
  (trigger → gates → approval → application, via item 62's linkage); the human sees an
  ORACLE-EXACT/BOUNDED/MEASURED-ONLY impact classification with evidence before approving —
  never blind, never a fabricated estimate.
- **77 — self-healing specialization**: health tracking already exists (grep-verified this
  pass: `DriftClass` has a real fail-closed consumer, `RetainedBase::admit`; `markov::Verdict`
  is observational-only — the missing piece is a consumer, not a classifier); process-level
  restart-recovery STAYS automatic (already proven, kill-9 300/300); code-level fixes for
  RECURRING measured-basis verdicts take the FULL pipeline — never autonomous, not a lighter
  gate. MAPE-K (Kephart & Chess 2003) cited as the fitting reference model with one stated
  deviation: the human operator IS the Execute gate.
- **78 — self-upgrading specialization**: improvement proposals (twin-identified bottlenecks,
  pre-proven rewrites) — same pipeline, cost-delta at approval, and the item-73 law binding
  hardest: no beneficial-change path to the gate, ever.

**Dependency spine:** 55/56/57/63/64 ready now → 58 → {59∥60∥61}, 62∥58, 65/66 behind 64,
67→68→70, 69 small, 71 independent/gated, 72 entry-gated; §L: 73→74→75→{76,77,78}, consuming
§K's 56/62/64/65/67/68 and §I/J's 45/47/50. FDR-dependent halves inherit the standing
exec-branch-merge prerequisite (§J's flag). Nothing new gates items 1–54.

## 3. Deliverables of this pass (all staged on main, no commit — operator reviews first)

1. `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` — 3 retro-corrections (§6, §9.12, §11 T2).
2. `BLUEPRINT-ITEM-27-pmu-classifier-input-2026-07-19.md` — 2 dev-host/deploy-target corrections (§5 + §3).
3. `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` — Part-1 corrections (items 0/7/12/31/46/51) + new §K (55–72) + new §L (73–78).
4. `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md` — NEW: the 13-step binding procedure (item 57 ratifies).
5. This document — the operator-facing summary.
6. `CORE-ROADMAP-INDEX.md` — rows for the six inputs + this pass's outputs.

**Honesty ledger for this pass itself:** no code was changed; no execution was dispatched; item
7's live execution was neither blocked on nor assumed complete; every new item's proof condition
is falsifiable; item 72 promises nothing; the two out-of-arc telemetry gaps (G10, G13) are
flagged, not silently dropped; the one place this pass declined an edit (CRASH synthesis §2.3
fleet framing) is recorded in §1 row 5.
