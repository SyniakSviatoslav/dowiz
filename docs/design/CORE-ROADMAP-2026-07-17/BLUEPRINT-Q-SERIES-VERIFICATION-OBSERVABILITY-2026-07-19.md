# Q-SERIES — Roadmap Verification & Observability: the governance layer over the P75–P96 build wave (2026-07-19)

> **Planning document — writes ZERO product code, touches no branches, pushes nothing.**
>
> **Why a Q namespace, not another P-number.** The P-series numbers *features* (a kernel fix, a
> mesh fast-path, an ETA wire-in). This is not a feature — it is the **governance/tracking layer
> that rides ABOVE the P-series** and asks, for each P-item as it actually gets built: *was the
> blueprint's own stated Definition-of-Done actually met, is the built thing observable in
> production, did a second pair of eyes look at the diff, and — for the few items with a screen —
> does the interface still work?* A distinct letter keeps that altitude legible: a Q-item is never
> "the next thing to build," it is "the check applied to everything built." Q is orthogonal to the
> Layer A–I altitude axis and the P01–P96 numeric axis — a fourth lens, like the ecosystem-component
> axis in `CORE-ROADMAP-INDEX.md §0`.
>
> **Proportionality rule, stated up front (this doc obeys its own honesty bar).** Three of the four
> sub-items are **mostly already built** — this document's job is to *cite and wire the seams*, not
> to invent parallel machinery. Only **Q1** is a genuine new (lightweight, process-only) gap. Q2
> extends one real closed enum + two existing blueprints. Q3 is "the gates already exist — here is
> the list, and here is the one honest thin spot." Q4 is "almost the entire P75–P96 wave has no
> interface, so this is a four-line section." Where a thing exists, this doc says so and stops.
> Inflating a "yes, cite it" into invented process would violate the very standard
> (`CORE-ROADMAP-STANDARD-2026-07-17.md`) the P-series is measured against.

---

## 0. Ground truth — what already exists (re-verified live this pass, 2026-07-19)

Everything below is cited, not re-derived. The Q-series **owns none of these** — it wires them
together into a done-time checkpoint.

| # | Existing asset | Fresh evidence (this pass) | What Q reuses it for |
|---|---|---|---|
| 1 | **The master ledger** — the tracking artifact, with a fixed status vocabulary (`FULLY-BLUEPRINTED-NEEDS-REGISTRATION` · `SKETCH-ONLY-NEEDS-FULL-BLUEPRINT` · `DONE-LOCAL-UNPUSHED-CODE` · `NEEDS-OPERATOR-DECISION` · two `CLOSED-*` kinds), every row citing its source doc | `MASTER-STATUS-LEDGER-2026-07-19.md §1`; **no `verified-by` / `DONE-VERIFIED` string exists in it** (grep = 0 this pass) | **Q1** adds one status value + one field to this schema — nothing else |
| 2 | **The P54/P55/P56 verification trio** (2026-07-18) — the LLM/agent + protocol/ecosystem + shared-harness verification stack. P56 §4f is a **meta-verification layer** (`FlakyProbe`/`InstrumentTooNoisy`/`StaleGround`/`DeadProbe` canaries) whose entire purpose is catching a test that reports GREEN against a referent that silently moved | `BLUEPRINT-P54/P55/P56-*.md`, read in full this pass; META-GAP-AUDIT §2 confirms "agent/model-output verification … owned by the pre-existing P54/P55/P56 trio" | **Q1** reuses P56 §4f.3 `StaleGround` as its automated backstop; Q **does NOT re-propose agent/model verification** — that is P54's, cited not duplicated |
| 3 | **Typed local-observability core** — `kernel/src/metrics.rs`: a **CLOSED** `LogEvent` enum `{ Metric(MetricSample), ClaimLatency(ClaimLatencyRecord), ClaimLatencyAnomaly(AnomalyFlag), Bench(BenchRecord) }`, parse-or-reject, deterministic fixed-field line format (`to_line`/`from_line`), pure-std, GPU typed-absent | `kernel/src/metrics.rs:103-159`; sibling `kernel/src/typed_metrics.rs` (`MetricLine`, `ProcCpuSample::from_proc_self`) | **Q2** extends this closed enum by variants — the real logging system, not a new one |
| 4 | **Kernel span telemetry — already blueprinted** — P83 designs `SpanMetricsLayer` in a new `kernel/src/span_metrics.rs` folding span durations → `metric.jsonl` (Layer 1) + breach-triggered `perf record` (Layer 2), integrated with the `tools/telemetry` monitor loop | `BLUEPRINT-P83-kernel-span-metrics-2026-07-19.md §0.1/§6` | **Q2** rides P83's layer for per-feature spans; **Q2 closes G14** (P83 Layer-1 rows are emit-only — no consumer) |
| 5 | **The live telemetry lane** — `tools/telemetry/lib.sh` (`log_event`, `tg_send`, `tg_deliver`, `bench_run`) with JSONL sinks already in `tools/telemetry/logs/` incl. `metric.jsonl`, `bench.jsonl`, **`blueprint.jsonl`, `phase.jsonl`, `plan.jsonl`, `plan_step.jsonl`** | `ls tools/telemetry/logs/` this pass | **Q1** appends its checkpoint verdict to the existing `blueprint.jsonl`/`phase.jsonl` lane; **Q2** ships feature metrics through `log_event`; no new sender |
| 6 | **The CI quality gate** — `.github/workflows/ci.yml`: `cargo-test` (kernel+engine, unconditional, offline), `bench-regression` (`bench_track.py --threshold 10`), `gitleaks`, `dco-check` (Signed-off-by), `supply-chain` (audit+deny+zero-oci), `decart-dep-lint`, `no-courier-scoring` (E58), `fence-check` (P45 `fences.toml`), `regression-digest` drift, `firewall-agent-loop` (P40), `no-pub-raw-matrix-hash` (P-B invariant), `v5c-reexec` (independent re-execution of any red-line diff range) | `.github/workflows/ci.yml`, read this pass | **Q3** routes P-items through these; they already exist |
| 7 | **The human-owned floor** — `.github/workflows/safety-floor.yml` runs `verify-safety-floor.sh`; lives in `.github/`, a red-line path the agent cannot modify | `.github/workflows/safety-floor.yml` | **Q3** — the money/auth/RLS/migration backstop already exists as an external check |
| 8 | **The per-blueprint independent-review row** — `D-REVIEW` is a real DoD row in every crypto/mesh blueprint (attestation filed under `docs/reflections/`, blueprint RED on FAIL); P85 restores `three-model-review.sh` | `BLUEPRINT-P92-*.md:698` `D-REVIEW`; META-GAP-AUDIT §2 "code-review/quality checks" | **Q3** — the code-review checkpoint the heavy items already carry |
| 9 | **The 20-point quality contract** — every blueprint carries a compliance map against it | `CORE-ROADMAP-STANDARD-2026-07-17.md §2` | **Q1/Q3** — the design-review bar; Q1 checks the *result* against it |
| 10 | **Playwright E2E + the wgpu-canvas UI shift** — CLAUDE.md keeps Playwright E2E against the live Hetzner/Cloudflare target (`/s/:slug`, `/admin/*`); but the JS/TS DOM frontend was DROPPED (ci.yml "Post 'drop js' 2026-07-15") and the product surface is now **wgpu-canvas, zero-DOM** (`CORE-ROADMAP-INDEX.md §2` Layer-G superseded note) | ci.yml header; CLAUDE.md; index §2 | **Q4** — cite/reuse; the canvas shift changes *how* interface verification applies |
| 11 | **The meta-gap-audit itself already found the DoD-integrity misses** — G1 (P95/P96 orphaned from the ledger), G3 (P81/P82 gate DoDs unsatisfiable as written), G4 (P91 conformance vectors don't exist), G11 (P79 alloc-harness never committed), G13 (P92 threshold symbolic), G15 (P96 no field telemetry) | `META-GAP-AUDIT-2026-07-19.md §1` | **Q1** is the *standing* version of the one-off review that caught these — it formalizes "check the DoD is real *at done-time*, every time" |

**The one-sentence thesis:** the day's dominant result (ledger §6) is that dowiz is "short on landed
wiring, not on sound design" — so the risk now shifts from *is the design right* to *did what got
built actually satisfy the specific numbers the design promised*. Q1–Q4 are the four checks that
close that shift, each built almost entirely from assets already on disk.

---

## Q1 — Claim-verification checkpoint (the one genuine new item; process-only, no new tool)

**The gap, precisely.** A blueprint's DoD is a set of **specific, falsifiable claims**: a benchmark
number (P90 "budget CAS 2.0×"), a property-test equivalence (P94 "the exhaustive 408-pair
equivalence RED test"; P95 "incremental ≡ full-rebuild"), a falsifiable-bet threshold (P89's
three-path head-to-head verdict table; P92's `FASTPATH_BENEFIT_THRESHOLD`). Today, an item flips to
"done" in the ledger when **code lands** — there is no step that asserts *the blueprint's own claimed
numbers were the ones actually measured*. The meta-gap-audit had to catch G3 (two blueprints' "the
P75 gate goes RED" DoDs are unmeetable because no gate runs their crates) and G4 (P91's FIPS-203
conformance spine rests on vectors that don't exist) **by hand, one time**. Q1 makes that a standing,
cheap, per-item gate. **It is a process + one ledger-schema field. It is NOT a new binary, harness,
or CI job** (the CI jobs already exist — §0 row 6; Q1 references their outputs as evidence).

**Q1-a — the ledger schema extension (the whole mechanism).** Add ONE status value and ONE field:

- New terminal status **`DONE-VERIFIED`**, distinct from the existing `DONE-LOCAL-UNPUSHED-CODE`.
  `DONE-LOCAL-UNPUSHED-CODE` means *code exists*; `DONE-VERIFIED` means *code exists AND every DoD
  claim was checked off against evidence*. An item may sit at `DONE-LOCAL-UNPUSHED-CODE` with a
  half-filled checklist; it may not reach `DONE-VERIFIED` with an unchecked claim.
- New ledger column **`verified-by`**: a list of `{ claim, evidence }` pairs, one per DoD line in
  the blueprint's own §DoD table, where `evidence` is a **pointer, never prose**:
  - a **commit SHA** (the landing commit), and/or
  - a **`bench.jsonl` / `metric.jsonl` line hash** (the measured number, from §0 row 5's live lane),
    and/or
  - a **test name** (`cargo test` node that encodes the property/equivalence claim), and/or
  - a **`docs/reflections/` attestation path** (the D-REVIEW artifact — §0 row 8), and/or
  - an **explicit `NOT-MET: <reason>`** — an honest unchecked claim is a legal, visible value; a
    *silently absent* claim is the failure Q1 exists to prevent.

**Q1-b — the checklist procedure (per P-item, at the moment it would be marked done).** For each
row in the item's blueprint DoD table, the person/agent closing it records the evidence pointer.
Worked against the four claim-shapes the wave actually contains:

| Claim shape | Example (blueprint) | Evidence that discharges it |
|---|---|---|
| **Benchmark number** | P90 "budget CAS 2.0× @8t"; P77 "spool.rs O(N²)→O(N) drain" | a `bench.jsonl` line at the claimed id showing the number, gated by the existing `bench-regression` CI job (§0 row 6) — the number is *measured*, not asserted |
| **Property-test / equivalence** | P94 "408-pair equivalence"; P95 "incremental ≡ full-rebuild (P1–P5)" | the named `cargo test` node, green under the unconditional `cargo-test` CI job |
| **Falsifiable-bet threshold** | P89 "three-path verdict table"; P92 D-BENCH `FASTPATH_BENEFIT_THRESHOLD` | the pre-committed threshold (G13: P92 must pick a concrete N *before* running D-BENCH) + the measured value + the GO/NO-GO recorded as a ledger row |
| **Conformance / provenance** | P91 "real NIST ACVP ML-KEM-768 vectors"; P85 "port source pinned to `986646a`" | the pinned artifact path + provenance (G4: the vectors must *exist and be pinned* before the DoD can be checked) |

**Q1-c — the automated backstop (reuse P56 §4f.3, do not rebuild).** The manual checklist is
backed by a machine check that already exists in design: **P56's `StaleGround` detector**. Each
`verified-by` evidence pointer that is a test/bench/fixture is registered as a P56 `Ground`
(content-addressed hash of the referent). When the referent silently moves — the exact G3/G4 class,
and the exact P34/P36/P06 "reported GREEN against a moved referent" class P56 §4f.3 was built for —
the item's `DONE-VERIFIED` demotes automatically. **Q1 writes no detector; it declares that a
`verified-by` evidence pointer IS a P56 Ground**, so the meta-layer P56 already specifies polices
Q1's checklist for free. This is the single most important reuse in the Q-series: it is why Q1 can
be process-only.

**Q1-d — scope of application (honest triage, not blanket ceremony).** The checklist is
**proportional to the item's own DoD**: a small non-red-line item (P96) has ~3 DoD lines and a
3-pointer checklist; a crypto item (P91) has a conformance-vector line that is itself a
prerequisite. Q1 adds **no** new DoD content — it only requires the *existing* DoD lines be checked
off with evidence. An item whose blueprint DoD is already fully falsifiable (P94, P89) needs zero
new thought; Q1 is a filing discipline over that DoD, applied uniformly.

**How Q1 would have caught the real misses (falsifiable value claim):** G1 (P95/P96 absent from the
ledger) → cannot reach `DONE-VERIFIED` because there is no ledger row to hold the `verified-by`
field, forcing registration. G3 (P81/P82 gate DoD unmeetable) → the "injected slowdown trips the
P75 gate" claim has **no dischargeable evidence pointer** (no gate runs those crates), so the
checklist row is stuck at `NOT-MET`, visible, before the item is called done. G4 (P91 vectors don't
exist) → the conformance claim's evidence pointer is a non-existent file, an immediate `StaleGround`.
The checkpoint's success metric is exactly this: **no future item reaches `DONE-VERIFIED` with a
claim that the meta-gap-audit would later have to catch by hand.**

**Q1 DoD (falsifiable):** (i) the ledger schema carries `DONE-VERIFIED` + `verified-by`; (ii) a
scratch item marked `DONE-VERIFIED` with a fabricated evidence pointer (a test name that does not
exist / a bench id with no line) is caught — either by the P56 `StaleGround` backstop or by a
reviewer following the pointer — RED until the pointer resolves; (iii) every P75–P96 item that
lands carries a filled `verified-by` (or explicit `NOT-MET`) before its status flips. RED = an item
at `DONE-VERIFIED` with an unresolvable or absent evidence pointer.

---

## Q2 — Logging/telemetry for newly-built features (extend the real system; invent nothing)

**Rule (from §0 rows 3–5): there is already a logging system — the closed `LogEvent` enum in
`kernel/src/metrics.rs`, drained through `tools/telemetry/lib.sh log_event` to JSONL, plus P83's
span layer.** Q2 says *what each newly-built P-item should emit so its behavior is observable
post-deployment*, and routes all of it through those seams. **A new logging framework is
review-rejectable.**

**Q2-a — the extension mechanism (one closed enum, by variant).** New feature-behavior signals
become new `LogEvent` variants (the enum is closed and parse-or-reject by design — §0 row 3 —
so a new variant is the *only* correct way to add a signal; a stringly-typed side-channel is the
anti-pattern). Each variant keeps the deterministic fixed-field `to_line`/`from_line` contract.
Continuous per-span timing rides **P83's `SpanMetricsLayer` → `metric.jsonl`** (§0 row 4), not a
second path.

**Q2-b — what each build-bearing P-item should log (only the items that actually get built).**

| P-item (when built) | Signal to log | Vehicle | Why it must be observable |
|---|---|---|---|
| **P92 mesh hot-stream fast-path** | per-frame **fast-path-taken vs full-verify** counter; channel-bound MAC-verify latency; **fast-path reject reason** (the security-relevant one) | new `LogEvent` variant + P83 span on `hybrid_gate::check` | the whole item is a measure-first GO/NO-GO (D-BENCH); after deploy, the *actual* fast-path hit-rate and reject distribution are the only proof the D-BENCH bet held in the field |
| **P96 ETA adaptive speed** | **adaptive-vs-static-fallback path counter** per order (which branch fired); adaptive Δ vs the static baseline | `LogEvent` variant via `log_event` | **closes META-GAP G15** — P96's value prop is accuracy, but it currently *asserts* the win with no field telemetry and no per-order path counter. This is the minimal instrument that confirms the adaptive path is actually chosen and actually more accurate |
| **P77/P79/P90 kernel hot-path fixes** | span durations on `spool.rs` drain, `spine.rs` dedup, contended-lock sections | **P83 `metric.jsonl`** (no new code beyond a span annotation) | a landed O(N²)→O(N) fix that silently regresses later is invisible without a continuous span trend |
| **P83 itself** | (already the telemetry blueprint) | — | **Q2 closes META-GAP G14**: P83 Layer-1 span rows are emit-only. Q2 requires a minimal **consumer** for `kind=kernel_span` rows — a p99-drift check wired into the existing `tools/ops-alert` pattern, so a rising span p99 pages like a bench regression instead of sitting unread |

**Q2-c — what NOT to log (bounded, degrade-closed).** No PII, no order contents, no money figures
beyond the typed money-law's own audit trail (red-line, memory `test-integrity-rules`). Volume is
bounded: feature counters are aggregate, span rows are already P83-bounded. Logging failure is
fail-open for the *product* (a dropped metric never blocks an order) and fail-closed for the
*signal* (a dropped metric is a visible gap, per the `metrics.rs` parse-or-reject contract), exactly
P83's posture — Q2 inherits it, adds no new policy.

**Q2 DoD (falsifiable):** (i) each built P-item from the table above emits its named signal as a
`LogEvent` variant or a P83 span — verified by a test asserting the line appears in the JSONL sink
after the feature runs; (ii) the P83 `kernel_span` consumer (G14) exists and a planted span-p99
inflation trips it; (iii) the P96 path counter (G15) distinguishes adaptive from fallback in the
log. RED = a built feature with zero post-deploy observability, or a new bespoke logging path
outside `LogEvent`/P83/`log_event`.

---

## Q3 — Code-review / quality gate (mostly ALREADY EXISTS — cite, don't invent)

**Honest verdict: dowiz has a substantial, deterministic code-review/quality gate already. The one
missing artifact (CODEOWNERS/PR-template) is *appropriately* missing for this repo's operating
model, and inventing a heavyweight PR process would be process for its own sake.**

**Q3-a — what exists (the real gate, §0 rows 6–9), enumerated so P-series work routes through it:**

- **Deterministic CI (`ci.yml`)** — every push/PR runs: unconditional `cargo-test` (kernel+engine),
  `bench-regression` (the P75-schema gate), `gitleaks`, `dco-check`, `supply-chain`
  (audit+deny+zero-oci), `decart-dep-lint`, and the red-line grep/invariant gates
  (`no-courier-scoring` E58, `fence-check`, `no-pub-raw-matrix-hash`, `firewall-agent-loop`), plus
  **`v5c-reexec`** which *independently re-executes* any diff touching money/order-machine/event-log/
  auth. That is a genuine automated reviewer on the red-line surface.
- **The human-owned floor** (`safety-floor.yml`) — lives in `.github/`, unmodifiable by the agent,
  fails CI RED if any self-mod removes a product red-line.
- **The per-blueprint `D-REVIEW` row** — crypto/mesh items (P85/P91/P92/P93) already *mandate* an
  independent adversarial-review attestation filed under `docs/reflections/`, blueprint RED on FAIL;
  P85 restores `three-model-review.sh`. This is the code-review checkpoint for exactly the items
  that need a second reviewer.
- **The 20-point standard** — the design-review bar every blueprint maps against.
- **Read-only review subagents** available in-harness (`invariant-guardian`, `security-sentinel`,
  the `/code-review` skill) — a second-pass reviewer for any diff, no new infrastructure.

**Q3-b — the honest thin spot, and why it stays thin.** There is **no CODEOWNERS and no PR
template** (verified: `.github/` holds only `workflows/`). For a repo where CLAUDE.md has SUSPENDED
ship-gates and authorized full self-management on `main` (solo-BDFL-through-agents operating model,
memory `agent-operating-discipline-2026-07-18`), a mandatory multi-approver PR flow would be
**invented ceremony** — the deterministic gates + `v5c-reexec` + `D-REVIEW` + the review subagents
already deliver the *substance* (red-line protection, second-eyes on hard diffs) that CODEOWNERS
delivers by convention elsewhere. Q3 therefore proposes **no new review process**. Its single,
zero-cost addition is a **`reviewed-by` evidence pointer folded into Q1's `verified-by` field**: for
a red-line P-item, the pointer is the `D-REVIEW` attestation path; for a routine item, it is the
`/code-review` subagent pass or the reviewing commit — filed, not ceremonial.

**Q3 DoD (falsifiable):** (i) every red-line P-item (P85/P91/P92/P93) carries a `reviewed-by`
pointer to a real `docs/reflections/` D-REVIEW attestation before `DONE-VERIFIED`; (ii) a red-line
item marked done with no review pointer is RED (caught by the same Q1 checklist). No new CI job, no
new file, no CODEOWNERS mandate. RED = a red-line diff reaching `DONE-VERIFIED` with no review
evidence.

---

## Q4 — Interface verification (almost the whole wave has no interface; four honest lines)

**Verdict (confirmed against META-GAP-AUDIT §2 "Interface/UI checks" + a grep of the wave): no
P75–P96 item is a UI-surface blueprint.** The wave is kernel/mesh/crypto/bench internals. Only two
narrow surfaces touch an interface, and both are **already owned by an existing DoD** — Q4 cites
them, it does not invent tooling.

- **P86/P87/P88 (physics/GPU render path).** Their interface concern is **META-GAP G2**: they must
  carry P38 §12.3's standing **"renders correctly on the WebGL2 and CPU floors" DoD line**, and
  P88's atomics policy must state its WebGL2-fallback scope. Interface verification for these rides
  **P38's WebGL2/CPU-floor render DoD**, not a new tool — and all three are build-gated on the
  P38 §4.2 operator GPU decision (OD-11) anyway. Q4 = "carry the G2 DoD line," already tracked.
- **P96 (ETA).** P96 makes **no UI change** — it changes a value (`TrackingView.eta_seconds`,
  `ports/customer.rs:180`) that an *existing* courier/customer view already renders. Its interface
  concern is **META-GAP G15** (already folded into Q2-b): assert the improved value reaches and
  renders unchanged. When the courier/customer surface (P71/P52/P70 — *outside* this wave) is built,
  its render check runs then.

**The one honest nuance worth recording:** the product UI is now **wgpu-canvas, zero-DOM** (§0 row
10). Classic **Playwright DOM E2E** (per CLAUDE.md, against `/s/:slug` and `/admin/*`) applies to
whatever DOM surfaces remain (admin/public shells), **not** to the canvas render path — canvas
correctness is verified by P38's WebGL2/CPU-floor render DoD (pixel/floor conformance), not by DOM
selectors. So the interface-verification *story* for this wave is: **reuse P38's render-floor DoD
for the two render-path items; reuse Playwright for residual DOM surfaces; invent neither.**

**Q4 DoD (falsifiable):** P86/P87/P88 each carry the P38 WebGL2/CPU-floor DoD line before
`DONE-VERIFIED` (this is G2's remediation, tracked); P96's render-unchanged assertion is discharged
by its Q2-b path counter + (when the surface exists) a render check. RED = a render-path item at
`DONE-VERIFIED` with no floor-DoD evidence.

---

## 5. Q-series DoD (the governance layer's own falsifiable bar)

| Q | Item | Falsifier (RED unless proven) |
|---|---|---|
| **Q1** | claim-verification checkpoint | ledger carries `DONE-VERIFIED` + `verified-by`; an item at `DONE-VERIFIED` with an unresolvable/absent evidence pointer is caught (P56 `StaleGround` or reviewer) → RED; every landed P75–P96 item carries a filled `verified-by` |
| **Q2** | feature telemetry | each built P-item emits its named signal as a `LogEvent` variant / P83 span (asserted present in the JSONL sink); G14 `kernel_span` consumer exists and trips on planted p99 inflation; G15 P96 path counter present; no bespoke logging path |
| **Q3** | review checkpoint | every red-line P-item (P85/P91/P92/P93) carries a `reviewed-by` pointer to a real `docs/reflections/` D-REVIEW attestation before `DONE-VERIFIED`; no new CI job/file invented |
| **Q4** | interface verification | P86/P87/P88 carry the P38 WebGL2/CPU-floor DoD line (G2); P96 render-unchanged discharged via Q2-b; no new UI tooling |

---

## 6. Anti-scope (each review-rejectable)

1. **NOT a new verification tool/harness.** Q1 is a ledger field + a process; the automated backstop
   is P56 §4f.3, already specified. Building a "Q-runner" binary is the failure this blocks.
2. **NOT re-proposing agent/model-output verification.** That surface is P54/P55/P56's (§0 row 2;
   META-GAP §2/§3 confirm it is a non-gap for this wave). Q references it; it does not redesign it.
3. **NOT a new logging system.** Q2 extends the closed `LogEvent` enum + P83 spans + `log_event`.
   A parallel metrics/tracing stack (OTLP/Jaeger/Prometheus) is rejected — a `metric.jsonl` line is
   the whole need (P83's own ruling).
4. **NOT re-proposing the perf/metrics benchmark infra.** P75/P80/P81/P82/P83 own bench/metrics
   infrastructure for performance; Q1 *consumes their outputs as evidence*, it does not re-specify
   them.
5. **NOT a new code-review process / CODEOWNERS mandate.** Q3 is honest that the gate largely exists;
   its only addition is a `reviewed-by` pointer folded into Q1. Inventing a multi-approver PR flow on
   a self-managed `main` is ceremony (CLAUDE.md ship-gates SUSPENDED).
6. **NOT new UI tooling.** Q4 reuses P38's render-floor DoD and the existing Playwright E2E; it
   invents no framework, and it does not manufacture an interface gap where the wave has none.
7. **NOT gating the deterministic core.** Every Q check is a *tracking/filing* discipline over
   already-deterministic gates and advisory signals (GROUND-TRUTH-over-PROXY). Q adds no runtime
   gate to the kernel.

---

## 7. Links (item 7)

- Governs: `MASTER-STATUS-LEDGER-2026-07-19.md` (the ledger Q1 extends) ·
  `META-GAP-AUDIT-2026-07-19.md` (the one-off review Q1 makes standing; source of G1/G3/G4/G13/G14/G15).
- Consumes, does not rebuild: `BLUEPRINT-P54/P55/P56-*.md` (verification trio; P56 §4f meta-layer is
  Q1's backstop) · `BLUEPRINT-P83-kernel-span-metrics-2026-07-19.md` (Q2's span vehicle) ·
  `BLUEPRINT-P75-ci-bench-gate-rearchitecture-2026-07-19.md` (Q1's benchmark-evidence gate) ·
  `BLUEPRINT-P38-webgpu-render-engine.md §12.3` (Q4's render-floor DoD).
- Reuses: `kernel/src/metrics.rs` (closed `LogEvent`), `kernel/src/typed_metrics.rs`,
  `tools/telemetry/lib.sh` + `tools/telemetry/logs/{metric,bench,blueprint,phase}.jsonl`,
  `.github/workflows/{ci,safety-floor}.yml`, `docs/reflections/` (D-REVIEW attestations),
  `CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point bar).
- Memory: `agent-operating-discipline-2026-07-18.md` (proven-first / no-guessing — why Q reuses),
  `test-integrity-rules-2026-06-27.md` (red-line logging bounds), `verified-by-math-2026-07-07.md`
  (RED→GREEN as the checkpoint's spine), `ground-truth-over-proxy-2026-07-07.md` (advisory, never a
  core gate).

---

*Cross-reference maintenance: add the Q-series row to `CORE-ROADMAP-INDEX.md §10`. This doc is a
governance layer, not a P01–P96 feature phase — it carries no numeric-phase entry in SOVEREIGN §10.2,
by design (the same "letters/axes are a lens, not a renumbering" principle as Layer I).*
