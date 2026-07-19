# RESEARCH — Resource-Footprint, Zero-Blind-Spot & Relational Telemetry (2026-07-19)

**Kind:** RESEARCH pass (academic + industry sources + codebase grounding). NOT a synthesis,
blueprint, or execution artifact — this feeds a Fable pass that drafts the binding
`PROCEDURE-TELEMETRY-COMPLETENESS-STANDARD` doc, analogous to how
`PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md` (item 25) became binding this session.

**Convention (this session's):** every claim tagged **GROUNDED** (verified against live source /
cited publication) or **PROPOSED** (a design suggestion for Fable to weigh, not a fact). Web
sources carry URLs; code carries `file:line`.

**Operator directive being researched (verbatim):**
> for the telemetry, not just joules, but also atoms/molecules consumption or at least water & air,
> and execution time — all the functions, calls, processes — so 100% of the code is actually
> covered with zero blind/dark spots … and also relations to other nodes/processes/functions/files
> in telemetry as well — maybe more optimized it will be if any file/function/process has it inside.

**Deployment reality that bounds every answer (GROUNDED, confirmed this session):** the target is a
**local / offline-first, build-once-deploy deterministic Rust kernel** (also compiled to
`wasm32-unknown-unknown`), NOT a datacenter tenant. On the actual probe host: `/sys/class/powercap`
is empty → no RAPL/joules; `perf_event_paranoid = 4` → every hardware PMU counter is
`permission_denied` (item 27 blueprint §5, live-probed 2026-07-19). The kernel's default build is
**zero external dependencies**, and this session **removed** `tracing` + its `#[instrument]`
proc-macros in favour of a hand-rolled `macro_rules!` FDR module (blueprint items 4+29). Any new
telemetry proposed here must live inside those constraints or it is dead on arrival.

---

## Thread 1 — Resource footprint beyond joules (water, air/carbon, "atoms/molecules")

### 1.1 What the published methodologies actually measure

- **GSF Software Carbon Intensity (SCI)** — now **ISO/IEC 21031:2024**. Formula
  `SCI = ((E × I) + M) / R`: `E` = energy (kWh), `I` = grid carbon intensity (gCO₂e/kWh), `M` =
  embodied hardware carbon, `R` = a functional unit (per request / user / API call). SCI is a
  **rate**, deliberately not a total, so design choices are comparable. **GROUNDED.**
  (https://sci.greensoftware.foundation/ , https://greensoftware.foundation/standards/sci/ ,
  ISO 21031:2024). An **SCI-for-AI** profile exists (per-inference framing)
  (https://greensoftware.foundation/standards/sci-ai/).
- **Carbon is *derived from* energy, not independently measured.** The survey *"Calculating
  Software's Energy Use and Carbon Emissions"* (arXiv:2506.09683) and *FaaSMeter* (SoCC'24) both
  compute per-function/per-invocation carbon as **runtime hardware activity (RAPL joules or a
  TDP model) × regional grid carbon-intensity**, with intensity pulled from a published dataset
  (ElectricityMaps / national averages). **GROUNDED.**
  (https://arxiv.org/html/2506.09683v1 , https://homes.luddy.indiana.edu/prateeks/papers/faasmeter-socc24.pdf)
- **Water is a facility metric, and NOT attributable to a workload from the standard ratio.**
  The Green Grid **WUE** metric (L/kWh) is defined at the *site* boundary
  (https://www.thegreengrid.org/system/files/store/WUE_v1.pdf). The 2025 review *"The water use of
  data center workloads"* (ScienceDirect S0921344925001892) states plainly that **WUE-site /
  WUE-source cannot be used to compare or attribute water across workloads** — it has no hardware-
  efficiency or performance term. **GROUNDED.** Li et al., *"Making AI Less Thirsty"*
  (arXiv:2304.03271), splits water into **on-site** (cooling-tower evaporation — a physical
  property of the *building*) and **off-site / scope-2** (water consumed generating the
  electricity). Only the off-site component is a function of joules; on-site depends on cooling
  design the software can never observe. **GROUNDED.** *"Not All Water Consumption Is Equal"*
  (arXiv:2506.22773) adds that raw litres are misleading without a regional **water-stress**
  weight. **GROUNDED.**

### 1.2 Honest mapping to a local/offline device

The chain that is physically real at the kernel/software level is **exactly one primitive**:
**energy (joules)**. Everything the operator named beyond joules is either (a) a *constant-multiplied
derivation of joules*, or (b) a *facility property the software cannot see*:

| Operator's word | Honest kernel-level meaning | Measurable here? |
|---|---|---|
| joules | RAPL `energy_uj` counter | **GROUNDED**: mechanism exists (`fdr/schema.rs` `read_joules_uj`); on THIS host → named absence `no_rapl_interface`. Lights up on RAPL-capable silicon with zero schema change. |
| "atoms/molecules consumption" | closest honest physical primitive = **silicon-level power draw** = joules again. There is no second, finer physical observable a userspace Rust fn can read. | Same as joules. **No new mechanism to invent** — item 27's RAPL/PMU work already IS this. |
| air / carbon (CO₂e) | `joules × regional grid carbon-intensity (gCO₂e/kWh)` — a **consumer-side** derivation from the same raw counter | **PROPOSED**: derivable *iff* joules present AND operator supplies a regional intensity constant; else named absence. |
| water | **off-site** water = `joules × regional WUE-source (L/kWh)`; **on-site** water = facility cooling, **unobservable** by a local device | **PROPOSED** for off-site (same joules × constant shape); on-site MUST be a permanent named absence, never a fabricated number. For an offline device with no cooling tower, on-site water ≈ 0/undefined. |

**Key honest conclusion (PROPOSED for the standard):** the kernel needs **no new *measured* field
beyond `joules_uj`.** Carbon and water are **downstream, constant-multiplied VIEWS** of the joules
counter — which is precisely the schema's already-landed *"emit raw monotone counters only;
rates/deltas/derived quantities are a consumer concern"* rule (`fdr/schema.rs` module doc;
`metrics.rs`'s "CPU-% is a derived consumer concern"). **GROUNDED** that this rule exists. So the
correct move is **not** to add `water_ml` / `co2e_ug` fields to `HwStamp`, but to:
1. keep `joules_uj: Reading<u64>` as the single measured energy field, and
2. define carbon/water as a **consumer-side conversion table** keyed on a
   `(region, deployment-class)` constant the operator supplies once — degrading to
   `Unavailable(Absence)` when joules is absent OR the regional constant is unsupplied. This
   reuses `Reading<T>` verbatim and invents nothing.

This directly answers the operator's ambition without over-promising: **"water & air" become
honest, first-class DERIVED readings gated on a joules measurement and a stated regional
constant — never a fabricated per-op litre count, and on-site water is a permanent, greppable
`unavailable` reason on a device that has no cooling loop to measure.**

---

## Thread 2 — Zero-blind-spot execution-time coverage for "100% of functions/calls/processes"

### 2.1 The published tradeoff (the operator's "100%, zero cost" collides with physics here)

- **DTrace's "zero disabled-probe effect"** (Cantrill et al., USENIX '04) is the gold standard for
  *"leave probes everywhere, pay only when enabled."* But it achieves this via **dynamic kernel-
  level instrumentation** (machine-dependent trap/patch of live text) — a whole-OS tracing
  framework, not something a zero-dependency userspace Rust kernel that also targets
  `wasm32-unknown-unknown` can or should embed. **GROUNDED.**
  (https://www.usenix.org/legacy/publications/library/proceedings/usenix04/tech/general/full_papers/cantrill/cantrill_html/index.html)
- **Sampling vs. full instrumentation, measured:** eBPF sampling profilers run ~**1–5 % CPU**
  overhead; Intel Processor Trace ~**7 %** for full-coverage tracing
  (https://docs.base14.io/blog/ebpf-instrumentation-go/). *Coverage-guided tracing / Full-speed
  Fuzzing* (arXiv:1812.11875) shows the canonical trick to reach near-zero *steady-state* overhead:
  **instrument once, discover the site, then REMOVE the instrumentation** — i.e. you do not keep a
  live probe on every call site in the hot loop. **GROUNDED.**
- **"Dark/blind spots" is a real, named concept.** Auto-instrumentation has a documented blind
  spot: it sees HTTP/DB calls but **not business logic** ("calculating pricing", "validating
  inventory") — the **streetlight effect** in observability. Microservice literature calls
  un-instrumented services **"blind spots"** explicitly (TORAI, arXiv:2604.13522; SQLI
  "Observability Blind Spots"). **GROUNDED.**
  (https://www.sqli.com/int-en/observability-blind-spots)

### 2.2 The honest limit for THIS kernel

There is a genuine, statable impossibility triangle — you cannot simultaneously have all three:
1. a runtime timing **stamp on 100 % of call sites**,
2. **zero overhead / no hot-path tax**, and
3. **byte-deterministic replay** (the property this arc protects hardest).

Why (1)+(3) conflict specifically here: an FDR span stamp reads `std::time::Instant::now()`, which
is **nondeterministic** and lives on the **P3 forensic plane, categorically excluded from every
hash/gate/replay surface** (item 27 §4.4, GROUNDED). Putting a wall-clock read inside *every*
function does not corrupt replay (it is P3), but it **perturbs the very timings it measures** (the
classic observer effect) and taxes hot paths that this arc has measured to the microsecond
(item 26: FDR encode 3.9 µs, event-log append 637 µs). And on `wasm32`, `Instant::now()` **panics**
— today's macros are inert on wasm only because no sink is installed; a naive always-stamp guard
would break the shipping cdylib (blueprint items 4+29 §4.1, GROUNDED). So literal "100 % of
functions carry a runtime timer" is **not free and not universally safe.**

### 2.3 The closest honest achievable version (PROPOSED)

Redefine "zero blind spots" from **"a stamp on every call site"** to **"zero *un-named* blind
spots"** — the named-absence doctrine applied to *instrumentation itself*. Every function is
**classified**, and the classification is mechanically enforced; the coverage that reaches 100 % is
coverage of the *accounting*, not of the runtime stamps:

- `INSTRUMENTED` — carries a span/ledger record (Full `StampPolicy`).
- `CHEAP` — deliberately un-stamped hot inner loop (e.g. `import_unit` at 0.9 µs / ~1M-per-sec,
  item 26) — records `Unavailable(SamplingDisabled)`, a **first-class truthful absence**, not a
  silent omission (`StampPolicy::{Full,Cheap}` already exists, GROUNDED).
- `EXCLUDED(reason)` — structurally can't be stamped (wasm `Instant`, a pure `const fn`) with the
  reason named.

Enforcement mechanism **already proposed and grounded** in this session's telemetry audit: extend
`docs/audits/hardening/HOT-PATHS.tsv` with an `eff` column — **every hot-zone row must either name
its workload-kind/span or carry a ledgered `gap:` reason** (the item-6 hardening-gate mechanism).
"Enforced everywhere" becomes **"every hot zone either measures or explains itself"**
(`AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` §1.3, GROUNDED). This is the honest,
CI-checkable form of "100 % coverage" that survives contact with determinism and `StampPolicy`
economics.

**One structural gap the standard must confront (GROUNDED):** all span telemetry today is behind
the **non-default `telemetry` feature** — every shipping default binary carries **zero** runtime
span instrumentation (audit G9). "Enforced everywhere" is currently "enforced nowhere at runtime by
default." The standard must rule on whether the cheap-path FDR envelope (the `LEVEL` atomic-load
disabled cost, matching tracing's dispatch check) is always-compiled while heavy stamps stay
feature-gated. **PROPOSED:** yes — the disabled-path cost is one relaxed atomic load, which is the
honest floor.

---

## Thread 3 — Relational/graph telemetry between nodes/processes/functions/files

### 3.1 Prior art

This is **distributed tracing's core concept**. Relationships are captured by IDs, not per-node
metrics:
- **OpenTelemetry span model:** every span carries `traceId` + `spanId` + **`parentSpanId`**,
  forming a call tree; **Links** associate causally-related-but-not-parent-child work (e.g. a batch
  fed by many producers). **GROUNDED.** (https://opentelemetry.io/docs/specs/otel/overview/)
- **Causality primitives:** Lamport **happened-before** / logical & vector clocks are the formal
  basis for cross-process ordering; **Coz causal profiling** (arXiv:1608.03676) quantifies
  *which* code causally gates throughput/latency (not just where time is spent). **GROUNDED.**
- **Call-graph-aware profiling:** `perf record --call-graph` and data-provenance / causal-
  dependency tracking (forensics) capture *who called whom / whose data flowed where*. **GROUNDED.**

### 3.2 Grounding against dowiz's actual FDR schema — the decisive finding

**The current FDR schema is FLAT and UNLINKED.** Verified against the live exec branch
(`/root/dowiz-wt-space-grade-exec/kernel/src/fdr/schema.rs:212-226`, branch
`exec/space-grade-tier0-2026-07-19`):

```
pub struct FdrEvent { seq, ts_unix_ns, mono_ns, level, kind, name, hw, pmu: Option<PmuStamp> }
```

`grep -niE "parent|trace_id|span_id|caller|correlation"` over `schema.rs` → **zero hits.**
**GROUNDED.** The only cross-record identifier is `seq` — a **monotonic per-process counter**
(recovery ordering key), which conveys *temporal succession* but **not causal parentage**: from two
records you can tell which came first, never which *caused* the other or which function called
which. `SpanGuard` measures each span's own duration and (by owning its own `t0`) even fixes the old
layer's nested-span mis-measurement — but it records **no parent-span identifier** (blueprint items
4+29 §4.1, GROUNDED). So today: **every FDR event is a flat, unlinked record.** This is exactly the
"blind spot" thread 2's literature names — the causal edges between events are the dark matter.

### 3.3 The natural implementation path (PROPOSED, consistent with the reuse rule)

**Extend, don't replace** — the session's standing "reuse existing primitives" rule. Add a minimal
causal-linkage pair to the existing envelope, mirroring OTel's `spanId`/`parentSpanId` but with
zero new dependency and living on the **P3 forensic plane** (excluded from all hash/gate/replay
surfaces, like `ts_unix_ns` already is):
- `span_id: u64` (cheap: a per-process counter or hash of `(seq, name)`), and
- `parent_span_id: Reading<u64>` — **`Unavailable(NoParent)` at a root**, so the named-absence
  doctrine covers "this is a root event" instead of a magic 0 or a missing key.

This makes the FDR a **call/causality tree** rather than a flat log — the operator's "relations to
other functions/processes/files" — using the SAME `Reading<T>`/`Absence` machinery already built.
Cross-**process** edges (the `sh`/`node` subprocess spawns at `living_knowledge.rs:88,185`, audit
G6; the agent-loop↔LLM boundary, G1) get the same `parent_span_id` seeded across the boundary — the
Lamport/OTel-propagation idea reduced to passing one `u64`. **PROPOSED.** Honest cost note: a
`u64` pair per record is ~16 bytes and one counter increment — negligible vs. the existing `hw`
stamp, and it is P3 so it never touches determinism.

**On the operator's "maybe more optimized if any file/function/process has it inside":** the honest
reading (PROPOSED) is that the *linkage identifier* (not a full metric block) is what belongs
"inside" every unit — a function/process that carries its `parent_span_id` lets the recorder
reconstruct the whole graph cheaply, which is far cheaper and more deterministic than embedding a
full resource-stamp in every function (thread 2's impossibility triangle). Relational linkage is the
*cheap* part of the ask; per-function resource stamps are the expensive part.

---

## Thread 4 — Proposed "Telemetry Completeness Standard" (input to Fable, NOT the final doc)

Modeled on item 25's 10-step dependency procedure: a **numbered, CI-checkable checklist every future
blueprint in this arc must walk.** All PROPOSED — Fable to shape into the binding procedure.

1. **Name the workload unit.** Every new hot-path function/boundary states its countable *work*
   (the closed workload-kind enum: tokens, frames, eigensolves, signatures, events, decision-units
   — audit §1.3). No unit named → not blueprint-complete.
2. **Emit pairs, never ratios.** Records carry `(work Δcount, cost = HwStamp-delta [⊕ PmuStamp-
   delta])` as raw `u64`; efficiency ratios (per-joule / per-cycle / per-tick / per-CO₂e / per-L)
   are **consumer-side**. Enforces the landed losslessness rule.
3. **Resource fields degrade via `Reading<T>`, never omission.** Every cost/footprint field
   (`joules_uj`, PMU counters, and any derived carbon/water view) is `Value | Unavailable(reason)`
   with a **closed** reason enum; `write_field` always emits the key. A missing key is a standard
   violation.
4. **No new *measured* footprint field beyond energy.** Carbon and water are **derived views**
   (`joules × regional constant`), gated on a joules measurement AND an operator-supplied
   `(region, deployment-class)` constant; **on-site water is a permanent named absence** on a local
   device. Adding a raw `water_ml`/`co2e` field to the stamp is a violation (fabrication risk).
5. **Every cross-module / cross-process call carries a relational identifier.** New boundaries emit
   `span_id` + `parent_span_id: Reading<u64>` (root → `Unavailable(NoParent)`), extending — not
   replacing — the flat envelope. A boundary that drops linkage is a blind spot and fails review.
6. **Zero *un-named* blind spots.** Every function in a HOT-PATHS zone is classified
   `INSTRUMENTED | CHEAP(SamplingDisabled) | EXCLUDED(reason)`; the `HOT-PATHS.tsv` `eff` column
   must name a workload/span or a ledgered `gap:` reason. 100 % *accounting* coverage, CI-checked;
   NOT 100 % runtime stamps (state the impossibility triangle explicitly so no blueprint promises
   free universal timing).
7. **Determinism firewall.** All timing/PMU/energy/derived-footprint values live on the **P3
   forensic plane** — excluded from every hash, signature, idempotency, and gate-verdict surface;
   a grep proof that no telemetry value feeds a decision is required (item 27 §4.5 precedent).
8. **Zero new dependency; hand-rolled macro grammar only.** No `tracing`, no `#[instrument]`
   proc-macro, no `perf-event`/`libc` crate — consistent with the FDR rewrite. New instrumentation
   is `macro_rules!` or a `/proc`/`/sys` std read, or it doesn't land.
9. **wasm leg required.** Any surface reachable from `wasm.rs`'s pub fns states its wasm-safe clock
   (`performance.now()` import) or its named absence (`NonLinuxHost`-class reason) — the FDR plan
   currently *excludes* wasm (audit G4); the standard closes that.
10. **Name the reopening trigger + prove it.** State the concrete future event that would change the
    telemetry decision (e.g. "a RAPL-capable deploy lights up Tier E automatically"; "operator
    supplies a regional carbon constant → carbon view un-masks"), and ship the red→green test:
    on a RAPL-less/paranoid host, assert the emitted record contains the literal `unavailable`
    reason string — greppable, not a missing key (the §G.9 named-absence proof).

**Done =** a blueprint that walks 1–10 with the enforcement rows (6, 7) green.

---

## Thread 5 — Predictive resource-consumption oracle (cpu/gpu cost, gathered AND predicted)

**Operator directive (verbatim, layered this session):** "for the cpu/gpu resources consumption it
should be both gathered & predicted … an internal 100% correct oracle for the internal system
needs"; refined to "internal physics/math simulation can help"; sharpened to "100% prediction model
for any possible system action with full traceability."

### 5.1 The theoretical limit, stated first and precisely (GROUNDED)

Exact cost prediction for **arbitrary** code is **undecidable** — it reduces to the halting problem.
This is the founding result of the **Worst-Case Execution Time (WCET)** field (Wilhelm et al., *"The
Worst-Case Execution-Time Problem — Overview of Methods and Survey of Tools,"* ACM TECS 2008,
https://www.cs.fsu.edu/~whalley/papers/tecs07.pdf , https://dl.acm.org/doi/10.1145/1347375.1347389).
WCET is **decidable only** under **bounded loops, no recursion, no dynamic dispatch**; even then,
cache/pipeline/branch-predictor state makes exact micro-architectural timing itself undecidable in
general, so real WCET tools compute a **safe over-approximation** (an upper bound, deliberately
erring high), never an exact number, for anything above straight-line code. **A literal "100 %
correct oracle for any code" cannot be promised and must not be.** This is the hard boundary every
part of Thread 5 respects.

### 5.2 The reframing that makes the operator's "100 %" honest and achievable (PROPOSED — the named principle)

**Named principle: COVERAGE-COMPLETE, PRECISION-HONEST.** "100 % prediction for any possible system
action" is achievable **only** as **100 % *coverage* of classification, never 100 % *precision*
everywhere.** Concretely: **every** system action (function / call / process / file-op) — zero
exceptions, zero unclassified blind spots — is assigned to exactly one of three buckets, and the
assignment is itself **traceable** (it carries the evidence that justifies it, not a bare label):

- **ORACLE-EXACT** — bounded input domain whose cost is *fully enumerated* (or provably
  input-independent). Evidence = the enumeration record / constant-time proof.
- **ORACLE-BOUNDED** — structurally bounded (fixed operation schedule); cost is a `[min,max]`
  interval *derived analytically* from the fixed op-count. Evidence = the interval derivation.
- **MEASURED-ONLY** — genuinely unpredictable (dynamic dispatch, unbounded loop, I/O-dependent, or
  the probabilistic AI/inference subsystem, items 33–44). Cost = **measured p50/p99/CI from real
  runs**, never a fabricated single number. Evidence = the measurement methodology + the CI.

"Full traceability" = you can always ask *"why is this action classified this way, and what is the
evidence?"* and get a real, checkable answer — the same **checkable-reasoning-path** idea this
session already built for AI-output validity (item 50's `Validity`/`admit()` design), applied to
resource cost instead of logical correctness. **A classification may be honestly uncertain (with
stated bounds); it may never be silently absent.** This is the single principle that resolves the
apparent contradiction between "100 % for any action" and "some things are genuinely unpredictable,"
and it is the quality-gate rule the Fable synthesis should formalize. It reuses `Reading<T>` exactly:
`ORACLE-EXACT → Value`, `ORACLE-BOUNDED → Value(interval)`, `MEASURED-ONLY → Value(distribution)`,
and *unclassified* is the one state the gate forbids — the analog of a missing key.

### 5.3 Cost oracle as a *byproduct* of the correctness proofs already being built (GROUNDED reuse)

The decisive, non-obvious finding: this session's Kani-feasibility pass
(`RESEARCH-NATIVE-KANI-REPLACEMENT-FEASIBILITY-2026-07-19.md`) **already classified 22 kernel
functions** into buckets that map **almost isomorphically** onto the cost-oracle buckets —
because *the same structural property that makes a function's correctness exhaustively provable
also makes its cost exactly knowable*:

- **Kani Bucket B (exhaustible, 16/22) → ORACLE-EXACT.** When a correctness test is **already
  iterating a function's entire input domain** — `order_machine.rs` 144-transition FSM (12 states),
  `reduce32` over the full `i32` domain (~4.3e9), `caddq` (~1.67e7 residues), `power2round`/
  `decompose` (~8.38e6), the `csr.rs:1296` 1099-graph Laplacian parity sweep — you can **record a
  real cycle-count for every iteration in the SAME pass**, producing a *genuine, complete* cost
  table for that function's whole domain, not a sample and not a prediction-with-uncertainty. For
  the (common) case where control flow is **input-independent** (all the straight-line crypto
  reductions), that table collapses to a **single constant** (or a tight noise interval). **GROUNDED
  feasibility** — the exhaustive harnesses exist or are specified; adding a cycle counter to the
  loop body is cheap and reuses the Tier-A `rdtsc` reader from `fdr/pmu.rs`.
- **Kani Bucket C (interval/algebraic, 6/22) → ORACLE-BOUNDED.** `ntt`/`invntt` run a **fixed
  8-layer / 1024-butterfly schedule** with no data-dependent control flow; `montgomery_reduce`,
  `keccak_f` (24 fixed rounds) likewise. A `[min,max]` cycle interval is derivable **analytically
  from the fixed operation count** (the WCET-decidable straight-line subclass), *not* measured —
  exactly the "butterfly lemma + documented induction" the Kani doc already proposes for
  correctness, reused for cost.
- **Kani Bucket A (needs SAT, 0/22) → would be MEASURED-ONLY.** The Kani pass found **zero** such
  targets in the current kernel — so today *nothing* in the crypto/FSM/arithmetic hot core is
  intrinsically un-oracle-able; the MEASURED-ONLY bucket is populated by the *I/O and dynamic*
  surfaces below, not the math.

**The strongest single reuse (GROUNDED):** the kernel's **constant-time (CT) requirement already
proves cost is input-independent** for the crypto hot paths. `ct_gate.rs` is gated by a **dudect**
timing self-test (HOT-PATHS.tsv row: `ct_gate … dudect … dudect-harness+planted-leak-selftest`).
Dudect proving "no data-dependent timing" **is** proving "cost is a constant independent of input" —
i.e. the security property *is* the ORACLE-EXACT property for that function, already tested. CT crypto
gets its cost oracle **for free** from a proof it already ships.

**Honest caveat (kept, not hidden):** even ORACLE-EXACT enumeration yields *measured* cycle counts
carrying host noise (cache, frequency scaling, context switches). The honest ORACLE-EXACT claim is
"**the input-dependence of cost is fully characterized** (every input's cost class is known)," while
absolute cycles remain a per-host interval. This is why ORACLE-EXACT still reports a value-with-noise,
not a platonic integer — precision-honest even at the exact end.

### 5.4 Physics/math reuse — genuine fit vs. forced metaphor (applying the Anu/Ananke rigor)

The operator asked whether the existing spectral/Markov/Laplacian machinery could model resource
flow instead of a new prediction subsystem. Applying this session's anti-forced-metaphor discipline
honestly — **it is a genuine fit for one layer and a category error for another:**

- **GENUINE (aggregate / relational layer).** A frequency-weighted **call matrix** `A` (`A[i][j]` =
  how often function *i* calls *j*) is a real graph, and the *total* cost fixed-point of a recursive
  call graph is `c = c_self + A·c ⇒ c = (I−A)⁻¹·c_self`, whose convergence is governed **exactly** by
  the **spectral radius ρ(A) vs 1** — which `spectral.rs::spectral_radius` (`:566`) and
  `classify_drift` (`:704`, ρ vs unit circle, `DRIFT_BAND=1e-6`) **already compute**. So
  `classify_drift` applied to the call matrix is a *correct* oracle for the graph-level question
  "**does total propagated cost converge to a bounded value (Damped) or diverge (Unstable)?**" —
  i.e. it decides ORACLE-BOUNDED-vs-MEASURED **at the whole-graph level**, reusing the exact enum
  (`Damped/Resonant/Unstable`) with zero new machinery. Likewise `algebraic_connectivity` (`:660`)
  and Laplacian diffusion (`csr.rs::laplacian_spmv`, `spectral.rs::laplacian`) are a *real* model of
  where cost/load **concentrates and flows** across the call graph (bottleneck identification). And
  `markov::analyze` over a stream of *discretized cost-tier tokens* is a real detector of
  **resource-usage-pattern drift over time** (Healthy / LimitCycle / StrangeAttractor) — reusing the
  live `Verdict` machinery for "is resource usage regime-shifting," a monitoring question.
- **FORCED (per-leaf layer).** Using eigenvalues to "predict" an **individual** function's exact
  cycle count is a category error: a leaf arithmetic function's cost is set by its **instruction
  schedule** (Bucket-C interval analysis / Bucket-B enumeration), which no spectral property of the
  call graph can supply. The graph spectrum answers *graph-level* questions (convergence, flow,
  bottleneck, drift); it says nothing about a butterfly's cycle count. Selling spectral analysis as
  the per-leaf cost predictor would be exactly the forced metaphor the Anu/Ananke standing rule
  forbids.

**Verdict (PROPOSED):** reuse spectral/Markov/Laplacian for the **relational/aggregate** oracle
(ties directly to Thread 3 — the call-graph *is* the relational telemetry graph, and ρ(A) is the
bounded-vs-unbounded-total-cost classifier); use **enumeration (ORACLE-EXACT) and interval derivation
(ORACLE-BOUNDED)** for **per-function** cost. Two layers, two tools, no new subsystem, no forced
metaphor.

### 5.5 Retroactive inventory — representative classification (GROUNDED sample; methodology stated)

The operator requires this apply to **all existing code**, not just future blueprints — a real
retroactive audit-and-backfill. Methodology: for each HOT-PATHS.tsv zone, read its existing test
idiom + control-flow shape and assign a bucket; extend by the same rule to the long tail. Sample:

| Hot-path function/zone | Bucket | Justification (the traceable evidence) |
|---|---|---|
| `order_machine.rs` 12-state FSM (144 transitions) | **ORACLE-EXACT** | control flow input-independent; the existing 25-test exhaustive sweep already iterates all 144 pairs — add a cycle counter → complete cost LUT. |
| `ct_gate.rs` `ct_eq` | **ORACLE-EXACT** | constant-time BY REQUIREMENT, already dudect-proven input-independent ⇒ single constant cost, free from the CT proof. |
| `pq/dsa` `reduce32`/`caddq`/`decompose` | **ORACLE-EXACT** | straight-line, input-independent branch count; full `i32`/residue domains enumerable (Kani Bucket B). |
| `pq/keccak.rs` `keccak_f` | **ORACLE-EXACT** | 24 fixed rounds, no data-dependent branch ⇒ constant cost. |
| `pq/kem.rs` `red`/`poly_addsub`/`compress` | **ORACLE-EXACT** | coeff domains ~1.1e7 enumerable; input-independent flow. |
| `pq/dsa` `ntt`/`invntt`, `montgomery_reduce` | **ORACLE-BOUNDED** | fixed 8-layer/1024-butterfly schedule ⇒ analytic `[min,max]` cycle interval (Kani Bucket C). |
| `householder.rs` tridiagonalization | **ORACLE-BOUNDED** | fixed O(n³) schedule for fixed n ⇒ interval parameterized by n. |
| `token_bucket.rs` GCRA `try_acquire` | **ORACLE-EXACT/BOUNDED** | straight-line integer arithmetic; input-independent (Kani Bucket C for correctness, but cost is straight-line). |
| `fdr/json.rs` escape | **ORACLE-BOUNDED** | cost linear in input length ⇒ interval parameterized by `len`. |
| `retrieval/pattern.rs` matcher | **ORACLE-BOUNDED** | cost a function of input length (bounded), post regex-removal; interval by `|input|`. |
| `spectral.rs` `eigh`/`spectral_radius` (iterative QR) | **MEASURED-ONLY** | eigenvalue iteration count is **data-dependent** (convergence depends on the matrix) ⇒ no analytic exact bound; report p50/p99 (unless a hard max-iteration cap makes it ORACLE-BOUNDED). |
| `event_log.rs`/`hydra.rs` `append`/`insert` | **MEASURED-ONLY** | I/O + fsync latency ⇒ genuinely measured; item 26 already reports **637 µs p50** with a real distribution — the MEASURED-ONLY exemplar. |
| subprocess spawns (`living_knowledge.rs:88,185`), agent-loop turn (audit G1), AI/inference (items 33–44) | **MEASURED-ONLY** | dynamic/external/probabilistic ⇒ p50/p99/CI, never a point estimate. |

**Finding:** the kernel's hot core is **dominated by ORACLE-EXACT/BOUNDED** (the CT-crypto + FSM +
fixed-schedule arithmetic), with MEASURED-ONLY confined to the **I/O, subprocess, and AI** surfaces —
which is the honest and expected shape, and means the retroactive backfill is *tractable*, not a
boil-the-ocean effort. The backfill task = tag every HOT-PATHS.tsv row with its bucket + evidence
(reusing the `eff`/`gap` column mechanism from the telemetry audit §1.3), and require every new
hot-path row to carry a bucket before it can go green.

### 5.6 Thread-5 additions to the quality-gate checklist (PROPOSED)

Fold into the Thread-4 standard as steps 11–13:
- **11. Every hot-path action carries a cost-oracle bucket + evidence** (ORACLE-EXACT / -BOUNDED /
  MEASURED-ONLY); *unclassified* fails the gate (COVERAGE-COMPLETE, PRECISION-HONEST).
- **12. ORACLE-EXACT cost is captured as a byproduct of the exhaustive correctness pass**, not a
  separate harness; MEASURED-ONLY reports p50/p99/CI, never a fabricated single number; CT-proven
  functions inherit ORACLE-EXACT from their dudect proof.
- **13. Aggregate/relational cost uses the existing spectral/Markov machinery** (ρ(A) of the call
  matrix for bounded-vs-unbounded total cost; drift classification for resource-pattern regime
  change) — NOT a new prediction subsystem, and NOT applied to per-leaf cost (forced-metaphor guard).

### 5.7 The "digital twin" ambition, split into two honestly-scoped halves

Operator (verbatim): "real math/physics simulation allowing to achieve this & have always shorter/
faster/more optimized version of any action or prediction — basically a digital twin of the
os/system itself." This bundles **two very different asks**; conflating them is where over-promising
would happen. Kept separate:

**(A) State-mirroring predictive digital twin — REAL, NEAR-TERM, roadmap-ready (PROPOSED).**
This is nothing more than **everything in Thread 5 made runnable as one model**, not a new subsystem:
- the per-function cost oracle (§5.3: ORACLE-EXACT tables + ORACLE-BOUNDED intervals + MEASURED-ONLY
  distributions), plus
- the aggregate call-graph dynamics layer (§5.4: ρ(A) convergence + Laplacian flow + Markov drift,
  reusing `spectral.rs`/`markov.rs`/`csr.rs` **as-is**), plus
- a **genuine existing precedent that "the system's real behavior is mirrored by real math":**
  `tools/eqc-rs` — a **real, zero-dep equation→Rust compiler** (`src/lib.rs:1` "ONE source-of-truth
  math expression → Rust code"; `Equation::new(name, args, Expr-tree)`; **dual emission** — `emit_rust`
  codegen **plus** `emit_proof_program` that asserts the generated code ≡ the `Expr::eval` reference
  at sample points, `README.md:37`). **GROUNDED.** eqc-rs already embodies the twin's core idea *for
  the math organs*: the equation *is* the source of truth and the running code is a proven-faithful
  mirror of it. A state-mirroring twin is the composition — given a system action + inputs, return its
  bucket + cost value/interval/distribution and (via ρ(A)) the propagated aggregate — built from these
  three already-real pieces. **This is buildable, tractable, and the honest long-form of the operator's
  "gathered & predicted" oracle.** Scope it as real work.

**(B) Auto-optimizing twin that "always finds a shorter/faster version of any action" — LONG-TERM
ASPIRATION, NOT near-term buildable (GROUNDED honesty).**
Taken literally this is **automated program optimization / superoptimization** — a real, hard, active
CS research field, NOT a free byproduct. The published state of the art:
- **STOKE** (stochastic search over instruction sequences) and **Souper** (an SMT/synthesis-based
  LLVM superoptimizer) search a space of candidate programs for a cheaper one equivalent to the
  original. **GROUNDED.**
- **Equality saturation** — **egg** (*"egg: Fast and extensible equality saturation,"* POPL 2021,
  https://dl.acm.org/doi/10.1145/3434304) and **egglog** (PLDI 2023,
  https://dl.acm.org/doi/10.1145/3591239) — represents *many* semantically-equivalent variants in an
  e-graph and **defers the choice to a final extraction phase that picks the cheapest under a cost
  model** (also applied to tensor-graph superoptimization, arXiv:2101.01332). **GROUNDED.**
This is exactly "search a space of equivalent programs for the cheapest," and it is **expensive**
(exponential search spaces, an e-graph/SMT engine — antithetical to a zero-dep deterministic kernel).
**Do not promise a general auto-optimizer as buildable.** Name it as the direction the operator is
pointing toward, with eyes open about its cost.

**(B′) One small, concrete, honestly-scoped FIRST STEP toward (B), grounded in what exists (PROPOSED).**
eqc-rs already holds an **`Expr`-tree IR** and generates Rust from it, and its README already lists a
future *"machine-readable equation IR so every kernel math organ is generated."* The minimal, bounded
first step is to give eqc-rs's generation a **cost-aware extraction over a small, hand-curated, finite
set of provably-equivalent algebraic rewrites** — strength reduction / re-association picked by lower
op-count: `a*2 → a+a`, `a*b + a*c → a*(b+c)`, constant folding — choosing the cheaper form **at codegen
time** and re-using the **existing `emit_proof_program`** to prove the chosen form still equals the
reference. That is the equality-saturation "extraction picks cheapest equivalent" idea at **toy scale
over a finite rule set** — **no e-graph, no SMT solver, no SAT, zero new dependency** — and it is
honestly "constant-folding-plus-strength-reduction-with-a-proof," NOT a general superoptimizer. It is a
real first rung on (B)'s ladder that costs almost nothing and over-promises nothing. Whether even this
is worth building is an operator call; it is offered as the *smallest grounded step*, not a commitment.

---

## Honest bottom line

At genuine space-grade rigor for a **local/offline-first** kernel:
- **Joules** is the one real physical footprint primitive; **"atoms/molecules"** honestly *is*
  silicon power draw = joules — item 27's RAPL/PMU work already is that mechanism, and it correctly
  reports **named absence** on this host. No parallel mechanism to invent.
- **Water & air/carbon** are achievable as **derived, constant-multiplied views of joules**, gated
  on an operator-supplied regional constant and degrading to named absence otherwise; **on-site
  water is physically unobservable by software and must stay a permanent named absence** — the
  operator's ambition is honored by making the derivation first-class, not by fabricating litres.
- **"100 % coverage, zero blind spots"** is achievable as **zero *un-named* blind spots**
  (every function classified, CI-enforced) — NOT as a free runtime timer on every call site, which
  the impossibility triangle (100 % ∧ zero-cost ∧ deterministic) and the wasm `Instant` panic
  forbid. State the limit; ship the honest version.
- **Relations** are the *cheapest* and highest-leverage part of the whole ask: the FDR is flat
  today, and a `span_id`/`parent_span_id: Reading<u64>` pair (OTel's model, reduced to two `u64`s
  on the P3 plane, reusing `Reading<T>`) turns the flat log into a causal call/data graph with
  negligible, determinism-safe cost.
- **Prediction** (Thread 5): a literal "100 % correct cost oracle for any code" is **undecidable**
  (WCET = halting problem). The honest, achievable form is **COVERAGE-COMPLETE, PRECISION-HONEST** —
  every action classified ORACLE-EXACT / ORACLE-BOUNDED / MEASURED-ONLY with traceable evidence, zero
  unclassified, uncertainty stated-not-hidden. The cost oracle falls out **as a byproduct of the
  correctness proofs already being built** (Kani Bucket B → exact enumeration, Bucket C → analytic
  interval; CT-crypto inherits an exact constant from its dudect proof). Aggregate cost reuses the
  **real** spectral/Markov call-graph machinery (ρ(A) bounded-vs-unbounded); per-leaf cost does not
  (forced-metaphor guard). A **state-mirroring digital twin (A)** composing these is real near-term
  work with `eqc-rs` (equation→proven-Rust) as an existing precedent; a **general auto-optimizing
  twin (B)** is superoptimization (STOKE/Souper/egg-egglog) — named as long-term aspiration, not
  promised — with a cost-aware `eqc-rs` rewrite-extraction as the one small, honestly-scoped first step.
