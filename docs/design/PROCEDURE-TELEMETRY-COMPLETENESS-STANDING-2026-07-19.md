# PROCEDURE — Telemetry Completeness (Standing, Binding on Ratification)

- **Date:** 2026-07-19 · **Kind:** standing procedure (the item-25 pattern:
  `PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md` is the template — a numbered,
  CI-checkable checklist every future blueprint in the arc must walk, with named terminal states
  and red→green proofs). Ratified BINDING by roadmap **item 57**
  (`SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K) once the operator dispatches it;
  until then it is the drafted standard of record.
- **Source:** `RESEARCH-RESOURCE-FOOTPRINT-ZERO-BLINDSPOT-RELATIONAL-TELEMETRY-2026-07-19.md`
  threads 1–5 (steps 1–10 = thread 4 verbatim in structure; steps 11–13 = thread 5.6's cost-oracle
  additions), composed with `AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` §1.3 (the
  work-normalized cost ledger + the HOT-PATHS `eff` enforcement mechanism).
- **Scope:** every future blueprint/design in the space-grade arc that touches a hot path, a
  process/module boundary, or any resource/latency measurement. **A blueprint is not complete
  until it walks steps 1–13 and the enforcement rows (6, 7) are green.**

## 0. The two laws this procedure encodes (read first)

- **Zero UN-NAMED blind spots — not zero blind spots.** Literal "a runtime timer on 100% of call
  sites" is forbidden by a real impossibility triangle: you cannot simultaneously have (1) a stamp
  on 100% of call sites, (2) zero overhead on hot paths measured to the microsecond (item 26:
  FDR encode 3.9 µs, event-log append 637 µs), and (3) byte-deterministic replay — and on
  `wasm32`, `Instant::now()` panics outright. The 100% that IS achievable, and that this
  procedure enforces, is 100% coverage of the *accounting*: every function classified, every
  absence named, mechanically checked. State the triangle in any doc that invokes this standard;
  never promise free universal timing.
- **COVERAGE-COMPLETE, PRECISION-HONEST** (thread 5's named principle). Exact cost prediction for
  arbitrary code is undecidable (WCET reduces to the halting problem — Wilhelm et al., ACM TECS
  2008). "100% prediction" is therefore 100% *classification* coverage with per-class honest
  precision — never 100% exact numbers. A classification may be honestly uncertain with stated
  bounds; it may never be silently absent.

## 1. The thirteen steps

Every step names its violation condition. "Record" always means an FDR-plane artifact or a
manifest row — never prose alone.

1. **Name the workload unit.** Every new hot-path function/boundary states its countable *work*
   from the closed workload-kind enum (`DecisionUnitsImported`, `FdrRecordsAppended`,
   `TransitionsFolded`, `TokensGenerated`, `FramesRendered`, `EigensolvesCompleted`,
   `SignaturesVerified`; closed-enum growth only, item-48 `Heartbeat` precedent). **No unit named
   → the blueprint is not complete.**
2. **Emit pairs, never ratios.** Records carry `(work: Δcount, cost: HwStamp-delta [⊕
   PmuStamp-delta])` as raw `u64`. Efficiency ratios (per-joule / per-cycle / per-tick /
   per-CO₂e / per-litre) are **consumer-side**, exactly like `metrics.rs`'s "CPU-% is a derived
   consumer concern." A ratio field in a record schema is a violation (lossy — it forecloses
   every later question the raw pair could answer).
3. **Resource fields degrade via `Reading<T>`, never omission.** Every cost/footprint field is
   `Value | Unavailable(reason)` with a **closed** reason enum; the serializer always emits the
   key. A missing key, a fabricated 0, or a panic on an unavailable counter is a violation
   (the `joules_uj: {"unavailable":"no_rapl_interface"}` precedent is the normative shape).
4. **No new *measured* footprint field beyond energy.** Joules is the one physical primitive a
   userspace kernel can read ("atoms/molecules consumption" honestly IS silicon power draw =
   joules; item 27's RAPL/PMU machinery already is that mechanism). Carbon and water are
   **derived, constant-multiplied views** (`joules × grid-carbon-intensity`;
   `joules × WUE-source` for off-site water), gated on a joules measurement AND an
   operator-supplied `(region, deployment-class)` constant, degrading to named absence otherwise.
   **On-site water is a permanent named absence** on a local device — a facility cooling
   property software cannot observe. Adding a raw `water_ml`/`co2e` field to a stamp is a
   violation (fabrication risk). (Roadmap item 69 builds the conversion table.)
5. **Every cross-module / cross-process call carries a relational identifier.** New boundaries
   emit `span_id` + `parent_span_id: Reading<u64>` (root → `Unavailable(NoParent)`), extending —
   never replacing — the flat FDR envelope; cross-process edges (subprocess spawns, agent↔LLM)
   seed the parent id across the boundary (OTel propagation reduced to one `u64`). A boundary
   that drops linkage is a blind spot and fails review. (Roadmap item 62 lands the schema.)
6. **Zero un-named blind spots — the enforcement row.** Every function in a `HOT-PATHS.tsv` zone
   is classified `INSTRUMENTED | CHEAP(SamplingDisabled) | EXCLUDED(reason)`; the TSV's `eff`
   column must name a workload-kind/span or carry a ledgered `gap:` reason. The extended
   hardening gate (item-6 machinery, item 57) goes RED on a hot-zone row carrying neither.
   "Enforced everywhere" = "every hot zone either measures or explains itself."
7. **Determinism firewall (P3 law).** All timing/PMU/energy/derived-footprint/linkage values live
   on the P3 forensic plane — excluded from every hash, signature, idempotency, replay, and
   gate-verdict surface. A grep proof that no telemetry value feeds a decision is required
   (item 27 §4.5 precedent). Any telemetry value reaching a decision surface is a violation of
   the highest class.
8. **Zero new dependency; hand-rolled macro grammar only.** No `tracing`, no `#[instrument]`
   proc-macro, no `perf-event`/`libc` crate — consistent with the items-4+29 FDR rewrite and the
   empty `ZERO-DEP-ALLOWLIST.txt` gate. New instrumentation is `macro_rules!` or a `/proc`//`/sys`
   std read, or it does not land.
9. **wasm leg required.** Any surface reachable from `wasm.rs`'s pub fns states its wasm-safe
   clock (`performance.now()` import) or its named absence (`NonLinuxHost`-class reason). The
   FDR plan may not structurally exclude the wasm surface silently (audit G4); `Instant::now()`
   panics on `wasm32` — an unguarded stamp there is a shipping-cdylib break, not a style issue.
10. **Name the reopening trigger + prove the absence path.** State the concrete future event that
    changes the telemetry decision ("a RAPL-capable deploy lights up Tier E automatically with
    zero schema change"; "operator supplies a regional carbon constant → the carbon view
    un-masks"), and ship the red→green test: on a RAPL-less/paranoid host, assert the emitted
    record contains the **literal** `unavailable` reason string — greppable, never a missing key.

### Cost-oracle extension (steps 11–13, thread 5)

11. **Every hot-path action carries a cost-oracle bucket + evidence.** `ORACLE-EXACT` (domain
    enumerated or cost provably input-independent; evidence = the enumeration record / CT proof),
    `ORACLE-BOUNDED` (fixed operation schedule; evidence = the analytic `[min,max]` derivation),
    or `MEASURED-ONLY` (genuinely dynamic/I/O/probabilistic; evidence = p50/p99/CI +
    methodology). *Unclassified* is the one forbidden state — the gate treats it as a missing
    key. (Roadmap item 67 backfills the existing rows.)
12. **ORACLE-EXACT cost is captured as a byproduct of the exhaustive correctness pass** — never
    a separate harness (the same enumeration that proves correctness records the cycle counts;
    roadmap item 68). MEASURED-ONLY reports p50/p99/CI, never a fabricated single number.
    CT-proven functions inherit ORACLE-EXACT from their dudect proof (input-independent timing IS
    input-independent cost). Honest caveat, always carried: even ORACLE-EXACT yields
    measured-with-host-noise cycles — the exact claim is "input-dependence fully characterized,"
    absolute cycles remain a per-host interval.
13. **Aggregate/relational cost uses the existing spectral/Markov machinery — and ONLY at the
    graph level.** ρ(A) of the frequency-weighted call matrix (the existing `classify_drift`
    `Damped/Resonant/Unstable`) decides bounded-vs-unbounded propagated cost; Laplacian diffusion
    locates concentration; `markov::analyze` over discretized cost tiers detects regime drift.
    **Forced-metaphor guard (Anu/Ananke, binding):** no spectral quantity is ever presented as an
    individual function's cost — per-leaf cost comes from enumeration/interval only. No new
    prediction subsystem.

## 2. Terminal states (per blueprint, mirroring item 25's ruling states)

- **(a) COMPLETE** — steps 1–13 walked, enforcement rows (6, 7, 11) green, reopening triggers
  named with their red→green absence proofs.
- **(b) COMPLETE-WITH-LEDGERED-GAPS** — every gap carried as a named `gap:` row (audit G10/G13
  class: out-of-repo/out-of-arc surfaces), never silent.
- **(c) NOT-COMPLETE** — any un-named blind spot, missing key, ratio field, unclassified bucket,
  or telemetry-feeds-decision finding. Blocks the blueprint from "done."

## 3. Standing posture rulings carried by this procedure (from item 57)

- **G9 ruling:** the cheap-path FDR envelope (one relaxed atomic load when disabled) is the
  always-compiled floor; heavy stamps stay behind the `telemetry` feature. The default binary is
  never entirely dark, and never taxed.
- **Cross-tier ban:** efficiency numbers are never compared across tiers (E/C/T); the tier is
  part of the value, enforced structurally — absent counters make a cross-tier ratio
  uncomputable rather than silently wrong. Where tiers C and T are both live, work/cycles vs
  work/ticks must agree within a stated band (a free counter self-test).
- **This procedure governs blueprints; roadmap §L extends the same standard to AI-proposed
  changes** (item 76: a change proposal's lineage + cost classification at the approval seam are
  steps 5 + 11 applied to the proposal itself).
