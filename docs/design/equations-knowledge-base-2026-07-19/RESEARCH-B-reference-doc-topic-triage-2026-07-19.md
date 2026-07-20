# Reference-Doc Topic-Fit Triage (Groups A/B/D/E/F/G/H) — 2026-07-19

> **Purpose.** The operator pasted a ~53-topic reference corpus (embedded/control/math/AI-ML/backend/
> stats/decision-frameworks/neuroscience) and asked to "research the codebase & existing roadmap …
> connect all the found gaps." This file does the honest **topic-fit triage** for groups **A, B, D, E,
> F, G, H** (C = Navier-Stokes/BVP/Klein-Bottle/TDA and the §1–3/§7 physics/AI-self-improvement/
> neural-field groups are triaged by the parallel agent). Companion to `EQUATIONS-LIBRARY-2026-07-19.md`
> (equation-level) and `TOPICS-INDEX-2026-07-19.md` (enumeration).
>
> **Method.** Every verdict below is grounded in a live `grep`/`Read` against the working tree of
> worktree `research/equations-thermo-eigenvector-2026-07-19` (`/root/dowiz-wt-eq-thermo-gpu`), not the
> Repowise index. **This branch is the sovereign-Rust-kernel architecture: there is no TypeScript
> `apps/api` here — the whole delivery surface lives in `kernel/src/*.rs`** (the CLAUDE.md `apps/api`
> entry-points are stale for this branch). `apps/` contains only `courier/` (a Rust render/surface crate).
>
> **Standing constraint honored.** The same-day `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md` already
> concluded "the roadmap does not have a blueprint-coverage problem." This triage is topic-fit against an
> unrelated reference corpus, **not** a re-litigation of roadmap coverage. Gaps are not manufactured;
> the genuine-gap list at the end is (as expected) empty of new-blueprint work.
>
> **Verdict vocabulary:** **ALREADY IMPLEMENTED** (with file:line) · **PARTIAL / DIFFERENT MODEL**
> (exists but not in the reference's exact shape) · **GENUINE GAP** (worth a blueprint — rare, justified
> hard) · **OUT OF SCOPE** (decorative reference material; why).

---

## Group A — Embedded systems / microcontrollers (A1–A6)

**Verdict for the whole group: OUT OF SCOPE (100% decorative).** dowiz has no firmware/MCU/hardware-
control surface. Exhaustive confirmation:

- **A1 Microcontroller overview / A2 MCU architecture (CPU core, bus, memory map)** — **OUT OF SCOPE.**
  No `no_std` embedded target, no `cortex`/`stm32`/register-map code anywhere in `kernel/src`, `apps`, or
  `engine`.
- **A3 GPIO (MODER/OTYPER/PUPDR/IDR/ODR/BSRR registers)** — **OUT OF SCOPE.** Zero GPIO/register code.
- **A4 ADC `Digital Output=(VIN/VREF)×(2ⁿ−1)`** — **OUT OF SCOPE.** No ADC/analog surface.
- **A5 PWM `V_avg=(D/100)·Vcc`** — **OUT OF SCOPE.** No PWM/duty-cycle surface (already flagged in
  EQUATIONS-LIBRARY §4).
- **A6 Interrupt (IRQ/ISR/vector table)** — **OUT OF SCOPE.** No hardware-interrupt surface.

**Every candidate "device" surface was checked and is NOT firmware:**
- `apps/courier/src/battery.rs` — a battery-**drain measurement gate** (P71/P63 SP-5) that reads a %/h
  number off a *real Android device via a test rig* and `#[ignore]`s until a real verdict lands; it does
  not control battery hardware (`battery.rs:1-11`). `HwClass::Emulator` is explicitly **rejected**.
- `apps/courier/src/dispatch.rs` — a wire-frame contract mirror (30 s accept-timeout Law); "the surface
  renders `deadline_ts`, it owns no timer" (`dispatch.rs:1-8`). No hardware.
- `apps/courier/src/voice.rs` — an input-profile selector (`CourierInMotion`→voice-biased); `AudioParams`
  is a UI signal, not a codec/driver (`voice.rs:1-8`).
- `kernel/src/pq/codesign.rs` — the only "firmware" string in the repo: it **code-signs an OTA update
  blob treated as opaque bytes** (`codesign.rs:1-3`); it authors no firmware and touches no MCU.

Nothing to build. Group A is confirmed reference-only.

---

## Group B — Signals & control theory (B1–B5)

The kernel's real control-theory content is **state estimation (Kalman)** and **queueing-theoretic
backpressure**, *not* frequency-domain transfer functions. Precise per-topic:

- **B1 LTI system `y=x*h`, Laplace/Z-transform** — **OUT OF SCOPE.** No convolution/transfer-function/
  s-plane surface. A delivery kernel has no frequency-domain signal path.
- **B2 Impulse response `h(t)`** — **PARTIAL / DIFFERENT MODEL.** No literal impulse-response object, but
  `kernel/src/kalman.rs` is a full n-D predict/correct Kalman filter (`x←F·x; P←F·P·Fᵀ+Q; K←P·Hᵀ·S⁻¹`,
  `kalman.rs:1-16`) and `geo.rs::ema_next` is its scalar steady-state special case. That is the kernel's
  actual "linear system response" surface — time-domain state-space, not `h(t)`.
- **B3 Poles & zeros `H(s)=K·Π(s−zᵢ)/Π(s−pⱼ)`** — **OUT OF SCOPE** as transfer-function algebra. (Note:
  `spectral.rs::charpoly`/`dominant_period` compute a *graph* characteristic polynomial and eigenvalue-
  modulus stability — a different, genuinely-implemented notion of "poles" that the kernel already uses as
  its `ρ(A)>1 ⇒ Unstable` drift gate. Not the reference's meaning, so not counted as a hit for B3.)
- **B4 Closed-loop `T=L/(1+L)`, `S=1/(1+L)`, `T+S=1`** — **PARTIAL / DIFFERENT MODEL (EQ-LIB §4 claim
  CONFIRMED).** No literal transfer-function feedback math exists (`grep` for `PID|transfer.?function|
  closed.?loop` returns only unrelated `pid`=process-id fields). The structural analogues are real
  feedback-regulated capacity controllers: `TokenBucket::refill_locked` (saturating P-controller on a
  bounded resource, `token_bucket.rs`), `WorkerSlots` degrade-closed semaphore, and — the closest — 
  `kernel/src/impedance.rs`, which **is** a control-loop by design: it computes a reflection coefficient
  from load ratio `ρ=λ/μ` and gates backpressure on `ρ < 1−margin` (`impedance.rs:1-13`), explicitly
  choosing a stability-margin control law over "impedance matching." These implement the *spirit* of
  closed-loop stability but are not derived from or checked against `T+S=1`. EQUATIONS-LIBRARY §4's exact
  wording ("structurally similar … but not derived from control theory") is verified correct.
- **B5 Gain/Phase margin `GM`, `PM`** — **OUT OF SCOPE.** No Bode/Nyquist/frequency-response surface. The
  `impedance.rs` `ρ<1−margin` gate is the kernel's stand-in "stability margin," but it is a queueing
  margin, not a gain/phase margin.

**B-group net: nothing to build.** Kalman (B2-adjacent) and impedance/TokenBucket backpressure (B4-
adjacent) already cover the only control notions this system needs. Frequency-domain control theory
(B1/B3/B5) has no target here.

---

## Group D — AI & ML (D1–D7)

(§3 Document-1 AI-self-improvement and §7 neural-field are the parallel agent's; D5's ML formulas overlap
that boundary and are triaged here since they are concrete kernel code.)

- **D1 10 AI-Engineering Design Principles (Sagare)** — **PARTIAL / process-lens.** No single "principles"
  file, but the repo *practices* several verbatim: local-first/degrade-closed, deterministic replay, seams
  over vendor runtimes (`ports/llm.rs` `AiMode`), advisory-vs-deterministic split. It is a checklist lens,
  not code to wire. No gap.
- **D2 Microsoft Foundry (Retrieval-as-Subagent, Eval-and-Optimizer loop)** — **ALREADY IMPLEMENTED (as a
  loop, under a different name).** The Eval-and-Optimizer shape is this repo's own HARNESS council
  (`librarian`/`ratchet-critic`/`cause-critic`, advisory store → deterministic promotion, non-regressive)
  and its `eval-layer/`. Retrieval-as-subagent = the `.claude/agents/Explore` + `retrieval/` split.
  Same shape, already running. No gap.
- **D3 Tokenization + Transformer (embeddings, self-attention)** — **ALREADY IMPLEMENTED (attention).**
  `kernel/src/attention.rs` implements numerically-stable `softmax(QKᵀ/√d)·V` as a single diffusion step
  over a learned affinity matrix (`attention.rs:4-23`), with a SIMD batch path in `simd.rs`. Tokenization
  proper is `kernel/src/chunker.rs`/`trigram.rs`. The transformer *primitive* exists; a full LLM does not
  (by design — LLM is an out-of-process port). No gap.
- **D4 31 Claude Skills for Small Business** — **OUT OF SCOPE (product-marketing content).** No code
  mapping; the repo's own skills live in `.claude/skills` and `AGENTS.md`. Decorative for this codebase.
- **D5 ML Formulas (Linear/Logistic Regression, Gradient Descent, MSE, Cross-Entropy, Entropy, Info Gain,
  Euclidean Distance, Bayes, Softmax)** — **MOSTLY ALREADY IMPLEMENTED — and this CORRECTS a stale gap
  note.** Point by point:
    - **Softmax** — **IMPLEMENTED** at `attention.rs:23` (row-max-stable) + `simd.rs:36,164` (batch SIMD).
    - **Cross-Entropy / log-loss** — **IMPLEMENTED** as the loss of `online.rs`'s `NaturalLogistic`
      learner: sigmoid `σ(t)=1/(1+e⁻ᵗ)` (`online.rs:144`), "gradient of the log-loss is the prediction
      error `(y−p)`" (`online.rs:167-177`).
    - **Logistic Regression** — **IMPLEMENTED** (`online.rs` `NaturalLogistic`, natural-gradient/Fisher).
    - **Linear Regression + Gradient Descent + MSE** — **IMPLEMENTED** (`online.rs` `LinearSGD`, online
      ridge-regularized least-squares, one SGD step per local sample; built on the `micrograd.rs` reverse-
      mode autodiff tape, `micrograd.rs:1-16`).
    - **Entropy `H=−Σp·log₂p`** — **IMPLEMENTED** (`markov.rs:179-189`, per EQ-LIB §3).
    - **Bayes' theorem** — **IMPLEMENTED** as the causal back-door adjustment / do-operator in
      `causal.rs` (`causal.rs:1-14`), which is conditional-probability inference proper.
    - **Euclidean distance** — **still ABSENT as a named primitive, and correctly so.** Retrieval is
      graph-diffusion, not vector-L2: relatedness decays with *hop-distance* (`retrieval/diffusion.rs:274`),
      and the index deliberately forbids Bloom/compression for bit-reproducibility (`retrieval/index.rs:80`).
      PPR-diffusion has no softmax-over-L2 step to add. This resolves EQUATIONS-LIBRARY §8's open
      "gap-or-intentionally-absent" question to **(b) intentionally absent**.
    - **Information Gain** — not a standalone fn, but its two ingredients (entropy in `markov.rs`,
      conditional distributions in `causal.rs`) are present; trivially composable, no gap.
  > **★ Documentation correction to carry forward:** EQUATIONS-LIBRARY §8's "Softmax … Cross-Entropy …
  > **not found anywhere in the current kernel**" is **wrong for 2 of its 3 flagged items** — that pass
  > grepped only `retrieval/*.rs`; softmax lives in `attention.rs`/`simd.rs` and cross-entropy(log-loss)
  > lives in `online.rs`. Only Euclidean-distance is genuinely absent (and intentionally). §8 should be
  > amended, not acted on as a build gap.
- **D6 9 Feature-Engineering Techniques** — **OUT OF SCOPE (deferred by policy).** dowiz does not train
  tabular models; the roadmap's P54 fine-tuning is explicitly DEFERRED (gap-audit §3). No feature pipeline
  to build.
- **D7 9 Hyperparameter-Optimization Libraries (Optuna/Ray Tune/…)** — **OUT OF SCOPE.** No training loop
  with tunable hyperparameters; `online.rs` uses a fixed LR by determinism requirement. Importing an HPO
  library would violate the zero-vendor-ML-runtime house rule. Decorative.

**D-group net: no gap.** The one actionable item is a *doc correction* to EQ-LIB §8 (softmax/cross-entropy
are present), not new code.

---

## Group E — Backend / software / data infrastructure (E1–E7)

The single richest group. Most E2/E7 patterns are genuinely present in the kernel.

### E1 AWS Networking (Subnet, Route Table, IGW, NAT)
**OUT OF SCOPE.** dowiz deploys on Hetzner + Cloudflare (Fly.io retired 2026-07-18); no AWS/VPC/subnet/
NAT anywhere (`grep` hits are false positives on "draws"/entropy). Decorative.

### E2 Advanced Backend Concepts (the big list)
- **CAP / Eventual Consistency** — **DELIBERATELY DECIDED (documented CP choice).** `wallet/mod.rs:18` and
  `wallet/draft.rs:9`: "**NO CRDT — single-writer LWW is strictly correct** (R4 §3.1). NO tombstones."
  dowiz consciously chooses single-writer consistency over AP/CRDT convergence for money. Not a gap — a
  ruled position.
- **Idempotency** — **ALREADY IMPLEMENTED (strong).** `event_log.rs` is content-addressed: a duplicate
  content-id is a structural `Duplicate` no-op with no TTL (`event_log.rs:7,257-258,301,349-351`). This is
  the textbook idempotency-key pattern, done at the log layer so every downstream `decide` inherits it.
- **Message Queues / Backpressure** — **ALREADY IMPLEMENTED.** `bounded_drainer.rs` (`BoundedDrainer`
  consumes AT MOST `k` units, each debiting one token, `bounded_drainer.rs:1-24`) + `impedance.rs`
  backpressure gate + `bounded_drainer`/`spool.rs`. Real bounded-queue + backpressure discipline.
- **Consistent Hashing** — **OUT OF SCOPE / not needed.** Single-node-authoritative event logs; no
  distributed key-partitioning ring. Absent by architecture, not by omission.
- **Sharding** — **PARTIAL (storage, not DB).** `backup.rs:107` uses a 65536-way sharded fan-out for the
  content-addressed block store. Sharding-of-blobs exists; DB sharding does not (no relational DB in
  kernel). Correct for the architecture.
- **Replication** — **PARTIAL (mesh gossip, below).** No primary/replica DB; instead append-only event +
  revocation gossip (below) provides the anti-entropy replication.
- **Caching Strategies** — **ALREADY IMPLEMENTED (content-addressed invalidation).** `spectral_cache.rs`
  `DecompCache` keys an expensive eigensolve by content-address and only recomputes on a genuine root
  change, proving it "is NOT serving a stale stuck cache" (`spectral_cache.rs:1-15,34-41`). Arena/slot
  caches (`arena.rs`, `slot_arena.rs`) cover allocation reuse.
- **Rate Limiting** — **ALREADY IMPLEMENTED.** `token_bucket.rs` (saturating token bucket) — already the
  canonical EQ-LIB §4 citation. Genuine Token-Bucket, not a fixed-window hack.
- **Circuit Breaker** — **PARTIAL / DIFFERENT MODEL (honest).** There is **no classic 3-state Closed/Open/
  Half-Open breaker** with timeout-probe recovery (`grep HalfOpen|CircuitState|Tripped` = empty). The
  kernel instead uses **fail-closed + degrade-closed** everywhere (`WorkerSlots`→`Busy`; enumerated
  fail-closed `ProvisioningError`, `hub_provisioning.rs:177`; `impedance.rs` backpressure verdict). This
  is a deliberate stance: a delivery kernel prefers deterministic fail-closed over probe-retry half-open
  (which reintroduces nondeterministic timing). Not a gap worth a blueprint — but the honest note is that
  the reference's specific 3-state machine is absent by design, not present.
- **Observability** — **ALREADY IMPLEMENTED.** `span_metrics/mod.rs` (P83, `tracing`+`tracing-subscriber`
  spans over verified functions, `span_metrics/mod.rs:1-40`), `telemetry.rs`, `metrics.rs`,
  `typed_metrics.rs`. Feature-gated so shipping binary is perf-neutral. Real production observability.
- **CQRS** — **PARTIAL (event-sourced read/write split).** `decide`/`fold` separation across `wallet/`,
  `order_machine.rs`, `event_log.rs` is command-vs-projection by construction, though not labeled CQRS.
- **Saga** — **ALREADY IMPLEMENTED (strong).** `wallet/draft.rs:7` "event-sourced saga"; the N-leg payment
  atomicity saga at `ports/payment_provider.rs:396` ("the hardest correctness item," two-phase leg
  capture/void). This is the reference's Saga pattern, done for money.
- **2PC / 3PC** — **PARTIAL.** `ports/payment_provider.rs:223` "Two-phase leg controls (capture/void,
  idempotent + provider-side)" + `impedance.rs:8` "two-phase write→verify→drop." Two-phase commit for
  payment legs is real; a generic distributed-transaction coordinator is not (not needed).
- **Bloom Filter** — **DELIBERATELY REJECTED.** `retrieval/index.rs:80`: "**No Bloom filter**, no
  compression ⇒ bitwise reproducibility (blueprint §3)." Consciously declined for determinism. Not a gap.
- **Gossip** — **ALREADY IMPLEMENTED.** `decision/mod.rs:373` join-semilattice "gossip convergence is
  order-independent"; owner-signed revocation blobs gossiped via `RevocationSet::merge`
  (`capability_cert.rs:9,574-576,762`); events gossiped/synced (`event_log.rs:280`). Real anti-entropy
  gossip for revocation/CRDT-free merge.

**E2 net: the vast majority are already implemented, often more rigorously than the reference sketch.**
Two are deliberate *rejections* (CRDT, Bloom filter) and one (Circuit Breaker) is answered by a different
fail-closed philosophy. No E2 gap worth a blueprint.

### E3 API Authentication (API Key, JWT, Session, OAuth2, Basic, mTLS)
**ALREADY IMPLEMENTED — but as a capability-certificate model, not the reference's bearer-token taxonomy.**
The operator ruling (2026-07-18) is explicit: "identity = **per-order capability grant**"
(`ports/customer.rs:3-12,55`). Inbound auth is a hybrid-signed, algorithm-agile **capability-cert chain**
(`capability_cert.rs:1-4`) with single-use nonces (`capability_cert.rs:385-386`), TTL, revocation sets,
and downgrade-resistant suite negotiation with transcript-hash binding (`capability_cert.rs:514,1442-1474`)
— which is *closer to mTLS* (mutual signed-cert auth) than to JWT/session/OAuth. There is **no** JWT /
session-cookie / OAuth2 / bearer-token surface, by deliberate design (`grep bearer|refresh.?token` in
`ports/` = empty; the only `api_key` is the *outbound* LLM-provider key `DOWIZ_LLM_API_KEY_FILE`,
`ports/llm.rs:219-263`, not inbound auth). **Verdict: implemented-differently, not a gap** — the reference's
6 token schemes are the thing dowiz consciously replaced with capability certs.

### E4 Database Normalisation (1NF–5NF/BCNF)
**OUT OF SCOPE for the kernel (event-sourced, not relational).** The kernel stores an append-only hash
chain, not normalized tables, so normal-form theory doesn't apply to it. A historical relational layer
exists only as `docs/audit/2026-06-18/proposed-migrations.sql` + the (off-branch) pgrust rebuild. It is a
DB-design lens, not kernel work. No gap on this branch.

### E5 12 Data Architecture Patterns (Medallion/Lambda/Kappa/Lake/Warehouse/Lakehouse/Mesh/Fabric/
Hub-and-Spoke/Data-Vault/Event-Driven/Modern-Streaming)
**Mostly OUT OF SCOPE (no Spark/warehouse), with ONE real mapping:**
- **Event-Driven / Event-Sourcing** — **ALREADY IMPLEMENTED (this IS the architecture).** `event_log.rs`
  is a content-addressed append-only hash chain (`event_log.rs:129,300,805`); the whole kernel is
  event-sourced (`decide`/`fold`, sagas). The reference's "Event-Driven" pattern is dowiz's spine, just
  named "event log / sovereign core."
- **Hub-and-Spoke** — **PARTIAL (topology, not data-warehouse).** `hub_provisioning.rs`/`hub_supervisor.rs`
  + the README "decentralized mesh-hub" framing is a hub-and-spoke *mesh* topology, not a data-integration
  hub. Analogous name, different layer.
- **Medallion / Lambda / Kappa / Data-Lake/Warehouse/Lakehouse / Data-Mesh / Data-Fabric / Data-Vault /
  Modern-Streaming** — **OUT OF SCOPE.** These presuppose a batch/stream analytics warehouse dowiz does
  not run. Decorative reference material.

### E6 9 CI/CD Concepts
**ALREADY IMPLEMENTED.** `.github/workflows/` = `ci.yml`, `safety-floor.yml`, `skill-security.yml`,
`heartbeat-monitor.yml`, plus `.husky/` pre-commit and the deterministic-gate machinery. Real CI/CD. The
reference is a checklist the repo already satisfies. No gap.

### E7 DSA Pattern Recognition (Prefix Sum, Monotonic Stack, Trie, Union Find, Topological Sort, Bit Manip)
- **Union Find** — **ALREADY IMPLEMENTED (textbook).** `kernel/src/dsu.rs` = Disjoint-Set Union with path
  compression + union-by-rank + Kruskal MST (`dsu.rs:1,17,48-73,152`).
- **Topological Sort** — **ALREADY IMPLEMENTED (textbook).** `order_machine.rs:234-258` = Kahn's algorithm
  over the order-lifecycle FSM (`Some` iff acyclic), companion to `has_cycle`/`cyclomatic_number`.
- **Bit Manipulation** — **ALREADY IMPLEMENTED.** `order_machine.rs:298,395` uses a `u16` bitmask (one bit
  per FSM state) for reachability; PQ/crypto layers use bit ops throughout.
- **Trie** — **PARTIAL / not-as-such.** No labeled trie; prefix search is served by `trigram.rs`
  (trigram index) and `blocklist.rs`. Functionally covers prefix lookup without a trie structure. Not a gap.
- **Prefix Sum** — **OUT OF SCOPE / trivially inlined.** No named prefix-sum primitive; where cumulative
  sums are needed (e.g. `stats.rs` reductions) they are inlined. No standalone artifact warranted.
- **Monotonic Stack** — **OUT OF SCOPE.** No sliding-window-extremum problem in the delivery domain.
  Decorative.

**E7 net: the load-bearing three (Union-Find, Topological-Sort, Bit-manip) are already implemented as real
domain primitives; the rest are inlined or absent-by-non-need.** No gap.

---

## Group F — Research methodology & statistics (F1–F3)

- **F1 Types of Research Design (Exploratory/Descriptive/Experimental/Correlational/Qualitative/
  Quantitative)** — **PARTIAL / methodology-lens, with one real code anchor.** No "research design" module,
  but the correlation-vs-causation distinction — the sharp end of this topic — is genuinely implemented:
  `causal.rs` does Pearl back-door adjustment / do-operator to get `P(Y|do(X))` from observational data
  (`causal.rs:1-14`), i.e. the kernel already distinguishes *experimental/causal* from *correlational*
  inference. The rest of F1 is a methodology lens for the eval-harness design, not code. No gap.
- **F2 Common Statistical Tests (t-Test/ANOVA/Chi-Square/Pearson/Regression/Mann-Whitney/Kruskal-Wallis/
  Wilcoxon/McNemar/Fisher's-Exact)** — **PARTIAL / correctly-scoped-absence.** `stats.rs` implements the
  *inferential* stats the eval oracle actually needs — CLT envelope, Wilson score interval, Bessel SE, and
  a **seeded percentile-free bootstrap interval** (`stats.rs:137`, a resampling method) — but **no
  hypothesis-test battery**, because there is no A/B-experiment / treatment-comparison surface (`evals.rs`
  has only Fisher–**Yates** shuffle, not Fisher's exact test). Adding t-Test/ANOVA would be building for a
  use case that doesn't exist. **Not a gap** — the correct set of tests for a deterministic-eval oracle is
  already present.
- **F3 MSA — Measurement System Analysis / %GRR (Gage R&R)** — **OUT OF SCOPE / superseded by a stronger
  guarantee.** Gage-R&R quantifies measurement *variance* in a manufacturing context; dowiz's eval harness
  instead asserts **bit-identical determinism** (same inputs ⇒ byte-identical output, enforced across
  `simd.rs`, `spectral.rs`, `online.rs`), which is the zero-variance limit of measurement repeatability. A
  %GRR module would be a weaker statement than the determinism KATs already shipped. No gap.

**F-group net: no gap.** The causal-inference anchor (F1) and the determinism guarantee (F3) are stronger
than the reference framing; the hypothesis-test battery (F2) has no target here.

---

## Group G — Critical thinking & prioritization (G1–G2)

Neither has (or should have) a code equivalent — these are lenses for the *process*, not the product.

- **G1 20 Questions / 5W1H** — **PROCESS-LENS (adopt for the blueprint pass, don't build).** Useful as a
  scoping checklist when writing any downstream blueprint. No code.
- **G2 Clarity Reset (FOCUS→CLARIFY→DISTILL→ALIGN→COMMIT)** — **PROCESS-LENS.** Mirrors the repo's own
  loop-orchestrator / council discipline. No code.

**Cross-reference to the decision frameworks (prompt §5.1 — RICE/MoSCoW/Eisenhower/Kano/Pareto/OKR).** As
EQUATIONS-LIBRARY §6 already ruled, these have no kernel equivalent by nature. Their genuine utility here
is as the **prioritization lens for the roadmap-update pass itself**: given the huge reference corpus, a
**RICE-style score (Reach×Impact×Confidence÷Effort)** is the right instrument to decide what — if anything
— from this whole exercise earns build effort. Applied below, RICE trivially confirms the empty gap list
(every candidate scores near-zero Reach/Impact or fails Confidence because it is decorative/out-of-scope).
This is a lens for *this* triage, not a "MoSCoW module" to implement.

---

## Group H — Neuroscience (H1)

- **H1 Nerve Cells (Motor/Sensory/Pyramidal/Purkinje/Interneuron/Granule/Basket/Chandelier/Stellate;
  retina Bipolar/Amacrine/Ganglion; skin Pacinian/Ruffini/Meissner/Merkel)** — **OUT OF SCOPE for code /
  DECORATIVE, cross-referenced to the neural-field viz.** This is morphology reference material for the
  Document-4 GPU neural-field rendering arc (already ingested 2026-07-16 as
  `living-interface-2026-07-16/EXTERNAL-RESEARCH-gpu-neural-field-sonification.md`, triaged by the parallel
  §7 agent). The key finding from that pass stands: the living-memory visualization needs **no neuron-
  simulation layer** — positions/activity/health come from the existing spectral/PPR primitives
  (`spectral.rs`, `csr.rs::personalized_pagerank`), and the spiking-neuron morphology/ODEs were
  **deliberately not adopted**. H1's cell taxonomy is aesthetic reference for that already-declined layer.
  No code, no gap.

---

## Prioritized genuine-gap list (carry into the blueprint pass)

**Empty of new-blueprint work — consistent with the same-day `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md`
finding ("the roadmap does not have a blueprint-coverage problem").** Across groups A/B/D/E/F/G/H, this
triage found **zero** reference topics that are both in-scope for dowiz AND genuinely unbuilt AND worth a
blueprint. What it did find:

1. **★ One documentation correction (not a build) — HIGH value, trivial effort.** EQUATIONS-LIBRARY §8's
   "Softmax … Cross-Entropy … **not found anywhere in the current kernel**" is **wrong**: Softmax is at
   `attention.rs:23` + `simd.rs:36,164`; Cross-Entropy (log-loss) is at `online.rs:167-177`
   (`NaturalLogistic`). Only Euclidean-distance is genuinely absent — and **intentionally** (retrieval is
   graph-diffusion, not vector-L2, `retrieval/diffusion.rs:274` + `index.rs:80`). §8 should be amended to
   say "softmax + cross-entropy present in `attention`/`simd`/`online`; Euclidean-distance intentionally
   absent (PPR-diffusion needs no L2 step)." **This is the single most important carry-forward** — it
   turns a flagged "possible gap" into a resolved non-gap.

2. **Two honest "different-model" notes (no action, record only):** (a) dowiz has **no classic 3-state
   circuit breaker** — it uses fail-closed/degrade-closed + `impedance.rs` backpressure instead, by
   deliberate determinism preference; (b) dowiz auth is **capability-certificates, not JWT/session/OAuth**
   — the reference's E3 token taxonomy is the thing dowiz consciously replaced. Neither is a gap; both are
   worth stating so a future reader doesn't "add JWT" or "add a circuit breaker" thinking they're missing.

3. **Two deliberate rejections re-confirmed (do NOT re-litigate):** CRDT (`wallet/mod.rs:18`) and Bloom
   filter (`retrieval/index.rs:80`) are consciously declined for money-correctness and bit-reproducibility
   respectively. The gap-audit §3 "do not manufacture blueprints for rejected items" rule applies.

**Prioritization lens applied (RICE):** the only item with non-trivial Impact and near-1.0 Confidence is
#1 (the §8 doc fix); everything else is Reach≈0 (decorative) or fails Confidence (would build for a
non-existent use case). No item clears the bar for a new blueprint.

---

*Triage performed 2026-07-19 in worktree `research/equations-thermo-eigenvector-2026-07-19`
(`/root/dowiz-wt-eq-thermo-gpu`). Read-only against the working tree: no product code, no git operations.
All file:line citations verified live via `grep`/`Read` on this branch (sovereign-Rust-kernel
architecture; no `apps/api` TypeScript surface exists here). Companion files: `EQUATIONS-LIBRARY-2026-07-19.md`,
`TOPICS-INDEX-2026-07-19.md`, `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md`.*
