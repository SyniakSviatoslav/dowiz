# BLUEPRINT P-F — Local AI / MoE Mesh (2026-07-17)

> **Layer F of the canonical CORE roadmap** (`docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §3),
> written against the §2 20-point contract. Wave-2 Fable planning pass.
> **Provenance note:** this file is a same-day reconstruction — the original was lost uncommitted
> to concurrent git activity (a consolidation session merged 20 branches onto `main` while the file
> existed only on disk). The design it records was operator-confirmed before the loss ("great idea,
> ++" — MEMORY `bebop2-mesh-masterwork-2026-07-17.md`, "Operator-confirmed design"); every code
> citation below was **re-verified fresh against current `main`** during reconstruction, drift
> noted where found (§1).
> **Absorbs (does not re-derive):**
> `bebop2-mesh-tensor-hermetic-2026-07-17/21-LOCAL-LLM-MESH-GOSSIP-DISTRIBUTED-INFERENCE.md`
> (the measured rejection this phase is built on),
> `BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md` §2 (**the Decision
> Compiler — design authority for everything a DecisionUnit *is*; this blueprint extends it, never
> re-derives it**), `bebop2-mesh-tensor-hermetic-2026-07-17/15-BATCH6-…-findings.md` §1 (the three
> distributed extensions), and `BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` §C-C (item 128
> ADOPT-AS-REFRAME; items 41/42/46/60 checkpoint-STARK; items 23/45 lineage-in-one-log).
> Rolls up (per `CORE-ROADMAP-INDEX.md` §1 row F): P21 (resident-agent runtime) · P15 E13-cpu ·
> P29 · masterwork doc 21 · `harness-2026-07-16/HARNESS-LLM-BACKEND.md`.

---

## Why this layer exists (context for a reader with zero session history)

Layer F is the mesh's answer to "where does the AI live?" — and the answer, arrived at by
measurement rather than fashion, is deliberately unglamorous: **the LLM is a build-time tool on
one machine, not a runtime service on every node.** The naive vision (each mesh node runs a small
model; the mesh is a distributed mixture-of-experts) dies on this host's physics: local decode is
memory-bandwidth-bound and does not parallelize, layer-sharding across a phone mesh is 3–4 orders
of magnitude too slow, and the transport refuses GB-scale weight blobs (§0 has the numbers). So
this layer inverts the idea: **gossip the compiled OUTPUT of inference — DecisionUnits — not the
inference.** A DecisionUnit is a pure `decide()` function harvested from real recurring judgments
and compiled to native code; it runs in nanoseconds and zero tokens, so the bandwidth wall simply
does not apply to it. A "domain expert" becomes a *family* of these units sharing a `DomainTag`,
authored by one hub-only oracle at compile time, never a model-per-node (§2). The only genuinely
new design ground is the three things distributing that output needs: version ordering across hubs
(epoch max-merge, §4.1), an independent replay gate before trusting a foreign compiled unit (§4.2,
the P06 `key_V` shape), and rollback lineage inside the one event log (§4.3, not a second DAG).
Everything else is reuse. The problem this layer solves, in one line: **get LLM-quality judgment
onto every node at native speed, by moving the compile off the hot path and gossiping the
result.**

---

## §0. The settled physics: raw distributed inference is REJECTED — read this first

This phase does **not** build a distributed LLM inference layer, and the reason is measured, not
argued. Doc 21 ran the probes on this host (8 vCPU / 4 physical EPYC-Milan cores, 30 GiB, no GPU,
`llama3.1:8b` Q4 = 4.9 GB, warm) and the numbers close the question:

- **Concurrency does not add throughput.** TRUE aggregate decode (total tokens ÷ wall clock) is
  **flat at 9.21 / 9.36 / 9.80 tok/s for 1 / 2 / 4 concurrent inferences** (1.00× / 1.02× / 1.06×)
  — the host serializes. Decode is **memory-bandwidth-bound**: every generated token streams all
  ~4.9 GB of weights through the ALUs, and at ~10 tok/s the host is already moving ~49 GB/s, at or
  near this EPYC config's **DDR4 ceiling**. The ceiling is hardware, not scheduler (doc 21 §1.2,
  §1.4).
- **Hedged racing measured a LOSS.** Racing 2 copies of the same request on this host: first-done
  wall 7.04 s vs 6.95 s single — a **0.09 s loss**, because both copies contend for the same
  cores/bus (doc 21 §1.3). Across identical deterministic nodes the race is a tie by construction.
- **Layer-sharding is 3–4 orders of magnitude too slow on this mesh.** Tensor/pipeline parallelism
  crosses the node boundary **once per layer** — ~32 hops/token for an 8B model — and a
  phones-on-WiFi/cellular hop is ~5–50 ms, i.e. **~0.16–1.6 s of pure network per token** against
  the ~0.1 s/token the host already does alone (doc 21 §2.a). It is a datacenter-NVLink technique
  wearing a mesh costume.
- **The transport also refuses the cargo.** Weight-sharding would gossip GB-scale blobs; the sync
  layer's hard bound is **`MAX_SYNC_PAYLOAD = 1 << 20` (1 MiB)** — verified fresh this pass at
  `bebop-repo/bebop2/proto-wire/src/sync_pull.rs:159`, enforced before allocation on decode
  (`:259-261`). Weights are not event facts: no `prev/actor/seq`, nothing to fold, no convergence
  semantics (doc 21 §3).

**This is a measured physics verdict, not re-openable without new hardware physics** (the one
named flip condition, from doc 21's own doubt audit: a µs-interconnect LAN cluster of owner hubs
would be a different regime — no such hardware exists in this deployment). What survives, and what
this phase builds, is the operator's own prior idea taken to its endpoint: **gossip the compiled
OUTPUT of inference — DecisionUnits — not the inference.** A DecisionUnit executes in zero tokens
/ nanoseconds, so the bandwidth wall does not apply to it at all; the existing transport carries
it natively (§1). Distributing the inference fights physics; distributing the compiled output
sidesteps it (doc 21 §4–§5).

---

## §1. Ground truth (verified THIS pass, live — contract item 1)

Every claim about existing code below was re-checked against current `main` during this
reconstruction, not inherited. Drift against the lost original's citations is stated explicitly.

| Fact | Evidence (live this pass) | Drift note |
|---|---|---|
| `TaskClass` — the kernel-side routing enum the oracle rides | `kernel/src/ports/llm.rs:29-33`: `pub enum TaskClass { Code, General, Embedding }`; doc comment `:25-27` states the adapter, never the kernel, maps it to model ids | **None** — cite exact on current `main` |
| `OllamaAdapter` model routing (`TaskClass` → concrete model id) | `llm-adapters/src/ollama.rs:33-45`: `route_model` (fn `:36-45`, doc comment `:33-35`): Code→`qwen2.5-coder:7b`, General→`llama3.1:8b`, Embedding→`nomic-embed-text`; explicit `model_id` passes through verbatim (`:37-39`) | **Minor drift**: the lost original cited `:33-40`; the match arms now end at `:44` — re-cited as `:33-45` |
| Kernel↔adapter compile firewall (kernel has zero HTTP/JSON/serde) | `kernel/src/ports/llm.rs:1-11` module doc: "ZERO network / HTTP / JSON / serde"; `cargo tree -p dowiz-kernel` firewall check named there | None |
| Transport payload cap | `bebop-repo/bebop2/proto-wire/src/sync_pull.rs:159` (`MAX_SYNC_PAYLOAD: usize = 1 << 20`), decode-time enforcement `:259-261` | **None** — cite exact |
| Content-addressing + idempotent dedup on the sync layer | `sync_pull.rs:300-307` (`compute_content_id` = sha3 over `prev‖actor‖seq‖payload`), dedup no-op `:604-610` | None (doc 21 cited `:300-307`, `:606-608` — both still land) |
| Gossip roster agent (the epoch's home) | `proto-wire/src/discovery.rs:148` (`pub struct GossipAgent`) | None |
| Verify-before-persist precedent in the kernel event log | `kernel/src/event_log.rs`: `append_raw` `:321`, `commit_after_decide` `:357`, `commit_after_decide_drift_gate` `:410` | **Drift**: Batch 6 cited the drift-gate at `:389`; it now sits at `:410` on `main` (the exactly-once `append_raw` port landed above it) |
| P06 `key_V` — the independent-re-execution pole this phase's import gate shares its shape with | `docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md:196-204` (RC-2: "the author-hub's own GREEN is never the certificate") | None |
| Compute-admission seam for oracle/prover jobs | `kernel/src/budget.rs` + `kernel/src/bounded_drainer.rs` (degrade-closed admission; the "4-slot" figure is V2 §C-C's) | None |
| Design authority for the DecisionUnit itself | `BLUEPRINT-LATENCY-ELIMINATION…` §2.1 (`Decision<T> { Answer, Escalate }`, pure `decide()`, provenance header, `watched-inputs`), §2.4 (harvest ≥10-in-7-days, compile-with-verification c.1–c.6, GapWire invalidation, Stale⇒Escalate) | Authority unchanged — **nothing in this blueprint redefines it** |
| Doc-21 measurements quoted in §0 | `bebop2-mesh-tensor-hermetic-2026-07-17/21-…INFERENCE.md` §1.2–§1.4, §2, §3 | Reproduced verbatim |

---

## §2. The "domain expert" resolution — a DecisionUnit FAMILY, not a model per node (operator-confirmed: "great idea, ++")

The MoE-mirror idea (dialogue item 128: "different mesh nodes = different domain experts") survives
— but only after one decisive reframe, which the operator explicitly confirmed and which is
**settled; do not revisit** (MEMORY `bebop2-mesh-masterwork-2026-07-17.md`): a "domain expert" is
**NOT a separately running model per node.** Per-node resident models would re-enter the §0
bandwidth wall for every query and add a model-ops surface to every phone. Instead:

### §2.1 Definition — a domain expert is a DecisionUnit family, three parts

1. **A `DomainTag`** — a closed kernel enum naming the functional domain:
   `Dispatch · EtaGeo · Pricing · FraudAuth · MenuInventory · Harness` (§3 for the type). The tag
   is a **declared capability**, exactly like a capability scope — never a measured quality.
2. **A set of compiled ns-native decision procedures sharing that tag** — ordinary Decision-
   Compiler units (P29 §2.1: pure `decide()`, typed input, closed output, `Escalate` first-class),
   grouped by domain. Examples per family:
   - `Dispatch`: *"does order O batch with order B?"* (pickup adjacency × window overlap ×
     capacity — rule-shaped, harvested from real dispatch decisions);
   - `EtaGeo`: *"which ETA band for (distance-band, hour, weather-class)?"* (the `geo.rs::ema_next`
     Kalman neighborhood — closed-form, eqc-rs-compilable);
   - `Pricing`: *"fee tier for (zone, cart-band, hour)"* — output is **integer basis-points**
     (`FeeBps(u32)`, never float money), and the whole family is **explicitly operator-gated as a
     money red-line: a Pricing unit is NEVER auto-adopted**; it registers, replays green, and then
     waits for an operator activation event (§3, §6.3-A6; docket R-4 in SOVEREIGN §8.12 — money-law
     eqc flip + integer basis-points);
   - `FraudAuth`: *"is this auth/order pattern anomalous enough to escalate?"* (escalate-biased:
     the unit may only ever answer `Escalate` or `NotAnomalous`, never auto-block);
   - `MenuInventory`: *"is item X order-safe now?"* (stock/86'd/schedule window);
   - `Harness`: the harness's own recurring judgments — model-tier routing (P29 §2.7's C1 pilot
     IS this family's first unit), worktree-isolation, cache-policy choice (P29 §2.3 C1/C3/C6/C8).
3. **One build-time oracle model per family compile-pass, hub-only.** The LLM appears **only at
   compile time, only on the owner hub**, to author/recompile a family's units through the
   compile-with-verification protocol (P29 §2.4(c).1–6, reused verbatim). It runs through the
   **existing** harness plumbing — the `LlmBackend` port and its adapter routing, verified fresh:
   `TaskClass` (`kernel/src/ports/llm.rs:29-33`) routed by `OllamaAdapter::route_model`
   (`llm-adapters/src/ollama.rs:33-45`; unit authoring is `TaskClass::Code` →
   `qwen2.5-coder:7b` locally, or the remote frontier tier per P29 §2.4(c).2 when quality demands
   — backend choice is consumer configuration, never a kernel recompile, per the port's own M5
   note). **No node other than the hub ever runs the oracle; no node ever needs it to decide.**

The V2 adjudication (§C-C, item 128 **ADOPT-AS-REFRAME**) is the load-bearing argument: the
operator's variant is query-level/domain-level, not token-level MoE — and for its compilable core
(most domain decisions are exactly the judgment-shaped class) it **collapses into DecisionUnit
gossip**, where the bandwidth wall does not apply at all because the runtime is a native `match`,
not a model. The non-compilable tail (open synthesis) stays on the P29 escalation path; the
domain-partitioned-throughput niche stays deferred behind doc 21 §5.3's triple trigger.

### §2.2 The routing law — capability tag, never quality (NO-COURIER-SCORING preserved)

Routing a question to a family is a **lookup on `DomainTag`** — a closed-enum match on a declared
capability. It is **never** "which node/unit answers best": there is no score field, no
success-rate, no ranking anywhere in the types (§3), so a quality-router is *unrepresentable*, not
merely forbidden. This preserves the CI-enforced `NO-COURIER-SCORING` red-line and the
Cheng–Friedman finding (Batch 4 §1) inside this new phase: domain = static capability, safe;
quality-rank = a courier-score, forbidden (V2 §C-C "the one hard constraint"). Any future PR that
adds a numeric ranking input to family routing must fail the no-courier-scoring guard (§7).

---

## §3. Spec first — types & constants (contract items 3, 4; spec precedes test precedes code)

New file `kernel/src/decision/mod.rs` (kernel side — mirrors `ports/llm.rs`'s zero-serde,
zero-network firewall; the oracle lives outside the kernel):

```rust
/// Closed set of decision-procedure families ("domain experts"). Extension = a code change +
/// review, deliberately — an open string tag would be an unreviewed capability grant.
/// NO ordering, NO numeric rank: routing is `match`, never comparison (NO-COURIER-SCORING).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainTag { Dispatch, EtaGeo, Pricing, FraudAuth, MenuInventory, Harness }

/// Identity of a question shape: sha3-256 over (DomainTag discriminant ‖ canonical input/output
/// schema). Content-derived — two hubs naming the same shape get the same id with no registry.
pub struct ShapeId(pub [u8; 32]);

/// Logical epoch for a compiled unit. Lamport-style monotone counter — NEVER wall-clock
/// (Batch 2 §7 HLC rejection stands). Merge law = max (§4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnitEpoch(pub u64);

/// Provenance + lineage record for one compiled unit version. Registered as an event in the
/// EXISTING content-addressed sha3 event log — `content_id`/`prev` ARE the lineage (§4.3);
/// there is no second DAG type anywhere in this design.
pub struct DecisionUnitMeta {
    pub shape: ShapeId,
    pub domain: DomainTag,
    pub epoch: UnitEpoch,
    /// sha3 of the harvested instance set the unit was compiled from and must replay against.
    pub instance_set_hash: [u8; 32],
    /// sha3 content-address of THIS version's artifact (source + tests), in the one log.
    pub content_id: [u8; 32],
    /// Previous version's content-id — rollback lineage inside the same log. None = genesis.
    pub prev_content_id: Option<[u8; 32]>,
    /// Money red-line: true for every Pricing unit (and any unit whose output moves money).
    /// A money-gated unit CANNOT reach Live via import alone — operator activation required.
    pub money_gated: bool,
}

/// Unit lifecycle. The ONLY path to Live is through the import gate (§4.2) — the type is
/// constructed by `import_unit`, nowhere else (module-private constructor).
pub enum UnitState { Live, Stale, Rejected }

/// Reuse P29 §2.1 verbatim — never a silent guess:
/// pub enum Decision<T> { Answer(T), Escalate(EscalateReason) }

/// Integer money — basis points. No float money type exists in this module on purpose.
pub struct FeeBps(pub u32);

/// The artifact bound = the transport bound. Pinned to the same literal as
/// bebop2 `sync_pull.rs:159` MAX_SYNC_PAYLOAD; the cross-repo drift test (§6.2 D6) keeps them
/// equal — a unit that cannot ride the transport may not exist.
pub const MAX_UNIT_ARTIFACT_BYTES: usize = 1 << 20;

/// Harvest threshold, inherited from P29 §2.4(b) — tunable, named, never magic.
pub const HARVEST_MIN_INSTANCES: usize = 10;
```

`kernel/src/decision/import.rs` — the gate (signature is the contract; body per §4.2):

```rust
pub enum ImportReject { ReplayDisagreement { case: usize }, MalformedArtifact,
    OversizeArtifact, EpochNotNewer, LineageParentMissing, MoneyGateRequired }

/// Verify-before-persist: NOTHING is registered until the full harvested instance set has been
/// replayed through the candidate unit AND the local oracle, and agreed on every case.
pub fn import_unit(
    meta: &DecisionUnitMeta,
    artifact: &[u8],                        // ≤ MAX_UNIT_ARTIFACT_BYTES, checked first
    instances: &InstanceSet,                // hash must equal meta.instance_set_hash
    local_oracle: &dyn Fn(&Instance) -> Verdict,
) -> Result<Registered, ImportReject>;
```

The oracle-side compile driver is **not kernel code**: it lives in a `tools/` Rust crate (zero
scripts, per the standing execution-model rule) consuming `LlmBackend` via `llm-adapters`, and
emits (unit source + property tests + provenance header + instance set) for the kernel-side gate
to judge. Where a unit's logic is closed-form (EtaGeo bands, Pricing tier arithmetic), the oracle
authors an **equation and compiles it through `tools/eqc-rs`** (equations-not-primitives rule) —
the emitted Rust is the adapter, the equation is the logic.

---

## §4. The three gossip extensions to the Decision Compiler (this phase's ONLY new design ground)

P29 §2 is a single-host design and remains the authority for what a unit is, when to compile
(harvest trigger), how to verify at compile time (c.1–c.6), and how to invalidate (GapWire,
Stale⇒Escalate). The distributed setting needs exactly three additions — the three pieces Batch 6
§1.2 identified as genuinely uncovered — and nothing else. **Anything not listed here defers to
P29 verbatim.**

### §4.1 Epoch max-merge (Batch 6 §1.2a)

P29's invalidation is binary (Stale/not); it has no version identity across recompiles, so two
hubs that independently recompile the same shape cannot order their units. `UnitEpoch` supplies
it: a **Lamport-style monotone logical counter — never wall-clock** (the Batch 2 §7 HLC rejection
is standing law: wall-clock in a merge law breaks replay determinism). Merge law, total and
deterministic:

```
merge(a, b) = if a.epoch != b.epoch      { the higher epoch }
              else if a.content_id != b.content_id { the lexicographically-lower content_id }  // total tiebreak, no scoring
              else                        { a }                                                // identical — idempotent
```

This is a join-semilattice (commutative, associative, idempotent — property-tested, §6.2 D2), so
gossip convergence is order-independent, riding the same anti-entropy the roster already uses
(`discovery.rs:148` `GossipAgent`; DecisionUnit epoch == gossip epoch, **one** counter — the
compose point Batch 6 named). An arriving unit with `epoch ≤` the local Live unit's is a no-op
(§6.3 A2): max-merge cannot downgrade.

### §4.2 The import-time independent-replay gate (Batch 6 §1.3) — the key_V shape

**What a receiving hub checks before trusting a foreign compiled unit** — the piece P29's local
verification never had to design. The adjudicated form, and the one-artifact/per-transaction
distinction that makes it survive Batch 4's rejections:

- **Gating a compiled artifact ONCE at import is verify-before-persist** — the repo's blessed
  pattern (`event_log.rs:410` `commit_after_decide_drift_gate` on current `main`; Hermetic RC-2),
  **categorically different from the rejected per-transaction patterns**: optimistic fraud-proof
  PoQ (propagate now, challenge later) stays REJECTED as degrade-open (Batch 4 §2.5; R1 §6 — no
  permissionless fraud proof worked on any optimistic rollup for ~3 years), and statistical/vote
  PoQ stays REJECTED as reputation aggregation (NO-COURIER-SCORING + Cheng–Friedman). The cost
  asymmetry is the whole argument: a per-transaction gate pays challenge-window machinery to save
  a sub-ms check; a **per-artifact replay is a one-time import cost amortized over the artifact's
  entire service life** — exactly what importing foreign *executable decision logic* (the
  highest-trust operation on the mesh, P29 S7) should pay.
- **Mechanics:** the receiving hub replays the unit's full harvested instance set (hash-pinned by
  `meta.instance_set_hash`) through the candidate artifact **and** through its own local oracle;
  **any disagreement ⇒ `ImportReject::ReplayDisagreement`, nothing persisted** (§6.3 A1). The
  author-hub's own GREEN is never the certificate — this is **explicitly the SAME shape as P06
  key_V** (`HERMETIC-ARCHITECTURE-PRINCIPLES.md:196-204`): author ≠ verifier, independent
  re-execution before trust.
- **P06 dependency, staged honestly** (SOVEREIGN §8.12): the **unsigned local-replay gate builds
  now** (it needs no signer — the replay itself is the evidence). The **signed import-verdict
  form** — a hub attesting "I replayed this and it passed" so third hubs can skip re-replay —
  plugs into P06's `Signer` slot and is **blocked until key_V lands** (P06 remains the standing
  multi-arc blocker; this phase adds its 4th consumer, it does not fork a signer).
- **Red-line overlay:** `money_gated` units (all of Pricing) additionally require the operator
  activation event even after a green replay (§6.3 A6) — a wrong compiled money rule is worse
  than a slow LLM (fast, confident, invisible — P29 §2.6's named risk, doubled for money).

### §4.3 Rollback lineage inside the ONE content-addressed sha3 log — NOT a second Merkle-DAG (Batch 6 §1.2c, V2 items 23/45)

A hub that discovers its newest unit is wrong must revert to the **last-known-good compiled
version** — cheaper than falling all the way to the LLM path. The lineage that enables this is
`DecisionUnitMeta.prev_content_id`: each version is an event in the **existing** sha3
content-addressed log (`sync_pull.rs:300-307` addressing; kernel `event_log` registration), and
the version chain is walked through content-ids **in that same log**.

The adjudicated nuance, reproduced exactly because it was corrected twice: **DAG-shaped lineage
for DecisionUnits specifically is ADOPTED** (V2 item 23: **ADOPT-AS-SINGLE-AUTHORITY**, flipping
v1's rejection — "DAG-shaped lineage of *new* facts is fine") — it is a *different domain* than
the rejected general case. What stays rejected is a **second Merkle-DAG authority**: a parallel
hash-DAG holding the history would recreate the exact dual-authority hazard the RCI Triadic
Council overturned (`docs/adr/ADR-realtime-change-intelligence.md`; Batch 6 §1.2c "the rollback
concept is sound; a second hash-DAG to hold it is not"; V2 item 45 merges here — the existing
prev-link chain + genesis anchor already prevent empty-state forgery). Concretely: **no new
Merkle module, no new root, no new store** — lineage is rows in the one log, and a lineage entry
whose parent content-id is absent from that log is rejected (§6.3 A5 / D4).

Rollback itself is **Snapshot Re-entry** in the operator's three-way synthesis (contract item 13):
revert-to-last-valid-version is a regenerative recovery from a known-good epoch, not a supervisor
decision — the reverting hub re-runs the import replay on the old version before re-activating it
(same gate, no trust shortcut).

---

## §5. Core-resident periodic checkpoint-STARK — designed, DEFERRED with named triggers (V2 §C-C, items 41/42/46/60)

The proof layer for the epoch machinery above, recorded so the deferral is a design, not an
omission:

- **PQ-STARK only — never a pairing-SNARK.** Hash-based STARKs are PQ-consistent with the mesh's
  hybrid Ed25519⊕ML-DSA stance; Pedersen/pairing constructions are discrete-log and already
  REJECT-ON-PHYSICS (V2 item 51).
- **The corrected cost basis** (doc 20 #3 — the old 10⁶× figure was stale by 1–2 orders): prover
  overhead is **~10⁵× (Jolt CPU frontier, ~100 kHz effective)**, with small-space proving <2×
  memory overhead (eprint 2025/611) and small-field turns promising 5–10× further (Binius,
  eprint 2026/1371). **Per-message proving stays REJECTED** — tens of ms of prover work against a
  ~71 µs verify it would replace.
- **What it proves, and where it runs:** a periodic checkpoint — "**the event-log fold from epoch
  boundary N to M was applied correctly against the FSM**" (~10⁶ fold steps ≈ **seconds** of
  CPU-bound proving). That burst runs on the **owner hub's spare core-budget, no GPU**, queued
  through the same degrade-closed compute admission as any heavy kernel job (`kernel/src/budget.rs`
  / `bounded_drainer.rs`; V2 §C-C's 4-slot figure) — GPUs are for real-time per-block proving,
  not an hourly delivery-mesh checkpoint. **Couriers verify in milliseconds.** This maps the
  mesh's heterogeneity exactly (item 46: the one node with spare cores proves; phones verify) and
  gates **no** delivery decision — strictly off the hot path.
- **NOT built now. Triggers (both required):** (i) a real periodic FSM-replay-audit need exists
  (an actual dispute/audit event that a replay-proof would settle), **and** (ii) a second live
  node exists to verify a proof. Until both, the checkpoint-STARK is a deferral register entry;
  building it today would be proving to nobody. The DoD (§6.2 D8) asserts the *absence* of prover
  code as part of done — deferral is checkable too.

---

## §6. Spec/event-driven TDD plan (contract items 2, 3, 5, 17)

### §6.1 RED state (before any implementation)

`kernel/src/decision/` does not exist on `main` (verified this pass — `ls kernel/src/` shows no
`decision*`). Every DoD item below starts RED by construction; D2/D3/D5 additionally get explicit
failing-first tests against a stub unit before the real gate lands (P29 §2.4(c).3's RED-first
rule, inherited).

### §6.2 DoD — D1–D8 (falsifiable, machine-checkable)

| # | Done-check | RED→GREEN evidence |
|---|---|---|
| **D1** | §3's types compile in `dowiz-kernel` with **zero new deps** and zero network (existing `cargo tree` firewall check extended to the `decision` module); `DomainTag` matches are exhaustive (no `_` arm — compile-time enforcement) | `cargo build -p dowiz-kernel` + a clippy deny on wildcard match in `decision/` |
| **D2** | Epoch merge is a join-semilattice: commutativity, associativity, idempotence property tests over arbitrary `(epoch, content_id)` pairs; no downgrade path exists | proptest suite RED against a deliberately-wrong `min`-merge stub, GREEN against §4.1 |
| **D3** | `import_unit` rejects on any single-case replay disagreement and persists **nothing** on reject (log length unchanged — asserted, not assumed) | A1 poisoned-unit corpus case (§6.3) is the RED fixture |
| **D4** | Lineage resolves entirely inside the one log: an entry with `prev_content_id` absent from the log ⇒ `LineageParentMissing`; **no second Merkle/DAG module exists** (CI grep-gate: no new `merkle`/`dag` mod under `decision/`) | A5 fixture + the CI text gate |
| **D5** | A `Stale` unit answers `Escalate` **unconditionally** — the state machine has no `Stale→Answer` transition to test because none compiles; the event-sequence test asserts `[GapEvent(watched-input) → UnitStale → Escalate*]`, events not just end-state | A3 fixture (§6.3) |
| **D6** | Transport fit: a maximum-size unit artifact (exactly `1 << 20` bytes) round-trips over `SyncFrame` with idempotent dedup; `1 << 20 + 1` is rejected **before allocation**; the cross-repo constant-drift test pins `MAX_UNIT_ARTIFACT_BYTES == 1 << 20` with a doc-comment pointing at `sync_pull.rs:159` so either side moving breaks CI | A2 boundary fixture |
| **D7** | Oracle leg is hub-only and kernel-firewalled: the compile driver lives in `tools/` (Rust, no scripts), consumes `LlmBackend` with `TaskClass::Code`, and `cargo tree -p dowiz-kernel` still shows zero HTTP after wiring | existing WAVE-0 firewall check, re-run |
| **D8** | Bench + telemetry: compiled-unit `decide()` p50 < 1 µs, zero alloc in the hot path (bench in-tree, number recorded in this doc's ledger row); counters for `units_live{domain}`, `escalation_rate{domain}`, `import_rejects{reason}` emitted through the native telemetry lane (P24); **no STARK prover code exists** (deferral asserted per §5) | `cargo bench` output + telemetry fixture |

### §6.3 Adversarial corpus — six cases, each designed to break an invariant (contract item 5)

| # | Attack | Must-hold invariant | Expected outcome |
|---|---|---|---|
| **A1** | **Poisoned DecisionUnit** — a foreign unit that is (i) malformed (doesn't parse/compile) or (ii) well-formed but malicious: replay-agrees on 9/10 harvested instances and flips the 10th (a targeted wrong rule) | Import gate = independent replay of the **full** instance set, author's green never trusted | (i) `MalformedArtifact`; (ii) `ReplayDisagreement{case:10}` — **rejected, nothing persisted**, shape flagged not-stable |
| **A2** | **Epoch downgrade replay** — re-gossip an older epoch (or the same epoch with a different content-id) after a newer unit is Live | Max-merge join-semilattice; deterministic tiebreak; no scoring input | No-op (older) / deterministic lexicographic winner (tie) — never a downgrade, never a rank |
| **A3** | **Stale-unit answer attempt** — drive a `GapEvent` on a watched-input, then query the unit before recompile completes | Degrade-closed: `Stale ⇒ Escalate` unconditional | Every post-event query returns `Escalate`; test asserts the event sequence |
| **A4** | **Quality-score smuggling** — a router patch that picks between two same-domain hubs' units by historical success-rate | Routing is `DomainTag` match only; no rank field exists; NO-COURIER-SCORING CI guard | Unrepresentable in §3's types; the CI no-courier-scoring guard is the backstop for creative re-introductions |
| **A5** | **Forked/orphan lineage** — a unit whose `prev_content_id` points outside the one log (a would-be second authority smuggling history in) | Lineage lives inside the single sha3 log; single-authority | `LineageParentMissing` — rejected |
| **A6** | **Money auto-adoption** — a Pricing unit that replays 100% green and then attempts activation without the operator event | `money_gated` ⇒ operator activation required regardless of replay result; integer basis-points only | Registers but stays non-Live; `decide()` answers `Escalate`; `MoneyGateRequired` on forced activation. Float money doesn't typecheck |

### §6.4 Telemetry (contract item 10)

Before/after numbers this phase owes when built: (a) the D8 sub-µs decide bench per family; (b)
escalation-rate per family over time (the compounding-property curve P29 §2.6 predicts — the
recurring-judgment fraction trending toward zero LLM involvement); (c) import-gate outcomes.
Regressions in any of the three surface in the P24 native-telemetry lane automatically, not at
review time.

---

## §7. Safety / hazard section — argued from structure, not policy (contract item 6)

- **"Foreign logic decides without local verification" is unreachable**, not discouraged: the only
  constructor of a `Live` `UnitState` is `import_unit`'s success path, which type-consumes the
  replay evidence. There is no `UnitState::Live` literal exported from the module.
- **"Quality-ranked routing" is unrepresentable**: `DomainTag` is a closed enum with no ordering
  and no numeric neighbor field; a rank needs a number, and there is none to read. The CI
  no-courier-scoring guard catches re-introduction attempts that add one (A4).
- **"A stale unit answering" is unrepresentable**: the `Stale→Answer` transition does not exist in
  the state machine; D5's test asserts the event sequence, but the primary defense is that the
  code path doesn't compile.
- **Money**: `FeeBps(u32)` integer basis-points is the only money type in the module; `money_gated`
  is set by domain (`Pricing ⇒ true`) in the constructor, not by the unit author — the finite
  anchored authority here is the operator's activation event (doc 19's "finite anchored authority,
  not zero"), the one deliberate human anchor in an otherwise structural design.
- **The named residual risk (inherited from P29 §2.6, not waved off):** a wrong compiled rule is
  fast, confident, and invisible. The independent replay gate and the money/operator gates are
  load-bearing against it; the honest limit is that a shape whose harvested instance set is itself
  unrepresentative passes replay and is still wrong — which is why `HARVEST_MIN_INSTANCES` is a
  floor not a target, and why every unit ships a falsifier test (P29 §2.4(c).2).

## §8. Scaling, bulkheads, mesh budget, and discipline tags (contract items 8, 9, 11, 12, 13, 15, 16)

- **Scaling axes, stated with break points (item 8):** families = 6 (closed enum; extension is a
  reviewed code change — deliberate, not a limitation); units per family O(10) now, registry
  stays a `mod`-per-shape in-crate until count outgrows consumers (P29 §2.4(c).6's
  `is_redline`-lives-near-its-consumer precedent), break point ~10³ units ⇒ own crate + index;
  artifact ≤ 1 MiB **hard** — an over-cap unit is not chunked, the *shape is split* (chunking
  would re-open the weight-sharding door §0 closed); epochs u64 (no practical bound); instance
  sets 10–10³ rows (beyond that, replay cost at import is still one-time and amortized).
- **Bulkhead (item 11):** the family IS the isolation boundary — a rejected/poisoned/stale family
  degrades **only itself** to the Escalate/LLM path; other families' units keep serving. Oracle
  compile jobs and (future) STARK proving queue through the existing degrade-closed compute
  budget so build-path work can never starve decide-path work.
- **Mesh budget (item 12):** decisions are **node-local, zero network** (the whole point); gossip
  is per *recompile* (rare, epoch-bounded), payload ≤ 1 MiB over the existing `SyncFrame`
  anti-entropy — no new transport machinery, per doc 21 §3's transport-fit table.
- **Rollback/self-healing as math, not metaphor (item 13):** Self-Termination = Stale⇒Escalate
  and the unreachable-Live invariants (§7) — hard boundaries, no supervisor; Self-Healing = the
  always-available escalation path is redundancy by construction; Snapshot Re-entry = lineage
  revert to last-known-good version (§4.3), re-verified through the same gate.
- **Smart index (item 14):** the bug classes this phase could introduce each have a named gate —
  wildcard-match clippy deny (D1), the constant-drift CI test (D6), the no-second-DAG grep gate
  (D4), the no-courier-scoring guard (A4) — compile/CI-time, not runtime surprises.
- **Living memory (item 15):** units, instance sets, and lineage are temporal/topological data in
  the one log — recall of a prior version is a content-id walk, never a flat search
  (cross-ref `internal-retrieval-living-memory-arc-2026-07-14`).
- **Equations/tensor (item 16):** closed-form families (EtaGeo, Pricing arithmetic) are authored
  as equations through `tools/eqc-rs` and emitted as the unit's Rust — the equation is the logic,
  Rust is the adapter (standing execution-model rule). No spectral machinery is needed by this
  phase; none is invented.

## §9. Regression ledger entries (contract item 17 → `docs/regressions/REGRESSION-LEDGER.md`)

On build, three permanent entries: **RF-1** A1 poisoned-unit rejection (the import gate's reason
for existing); **RF-2** D6 constant-drift pin (artifact bound == transport bound); **RF-3** A6
money-gate (Pricing never auto-adopts). Each references this blueprint by path.

## §10. Agent-executable build instructions (contract item 18 — zero session context needed)

1. Read `BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md` §2 first — it
   defines the DecisionUnit; this blueprint only adds §3's types and §4's three extensions.
2. Create `kernel/src/decision/mod.rs` with §3's types verbatim; wire `pub mod decision;` in
   `kernel/src/lib.rs`. D1 + D2 (proptest RED first against a `min`-merge stub).
3. Create `kernel/src/decision/import.rs` with §3's `import_unit` signature; implement §4.2
   (size check → schema/parse → instance-hash check → full replay vs unit AND local oracle →
   epoch check → lineage-parent-present check → money-gate check → register in event log).
   D3/D4/D5 via the A1/A3/A5 fixtures RED first.
4. Add the D6 boundary + constant-drift tests (transport side untouched — bebop2 `SyncFrame`
   already carries opaque ≤1 MiB payloads; do NOT add transport code).
5. Oracle driver: new `tools/decision-forge/` Rust crate (no scripts) consuming `llm-adapters`'
   `OllamaAdapter` with `TaskClass::Code`; emits unit source + tests + provenance + instance set
   into the import gate. D7 firewall re-check.
6. Bench + telemetry counters (D8). Do NOT build the STARK prover (§5 triggers unmet — its
   absence is part of done). Do NOT build the signed import-verdict (P06-blocked).
   Do NOT activate any Pricing unit (operator gate, docket R-4).
   Acceptance = D1–D8 green + A1–A6 green + RF-1..3 filed.

## §11. Hermetic principles honored (contract item 20)

**Degrade-closed** (Stale⇒Escalate; a rejected import changes nothing); **verify-before-persist /
RC-2 independence** (the import gate is the key_V shape — author ≠ verifier, §4.2); **single
authority** (one content-addressed log holds units, lineage, and epochs — no second DAG, §4.3);
**finite anchored authority** (exactly one human anchor: the money-activation event, §7);
**unrepresentable-state safety** (Live-without-replay, quality-ranked routing, and Stale-Answer
do not compile, §7); **physics over bureaucracy** (§0 is a measured verdict, and the design's
wins come from removing tokens, not supervising them).

## §12. Docs & memory cross-links (contract item 7) + reuse-first accounting (item 19)

**Depends on / extends:** `BLUEPRINT-LATENCY-ELIMINATION…-2026-07-17.md` §2 (authority) ·
doc 21 (§0's physics) · `15-BATCH6…` §1 (the three extensions + the PoQ adjudication) ·
`BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` §C-C + items 23/41/42/45/46/51/60/128 ·
`harness-2026-07-16/HARNESS-LLM-BACKEND.md` (oracle plumbing) · SOVEREIGN §8.11 (P29) / §8.12
(P30, dockets R-1..R-4) · `HERMETIC-ARCHITECTURE-PRINCIPLES.md` (RC-2/key_V) ·
`ADR-realtime-change-intelligence.md` (dual-authority precedent). **Memory:**
`bebop2-mesh-masterwork-2026-07-17.md` (the operator confirmation this design is settled under) ·
`sovereign-architecture-19-phase-roadmap-2026-07-17.md` (P06 blocker) ·
`internal-retrieval-living-memory-arc-2026-07-14` · `math-first-architecture-arc-2026-07-14`
(eqc-rs). **Reuse-first accounting:** this phase's new machinery is exactly three things — §3's
types, §4.2's gate, §4.1's epoch field. Everything else is reuse: the unit definition (P29), the
transport (`SyncFrame`, zero changes), the log (`event_log` + sha3 addressing), the oracle
plumbing (`LlmBackend`/`OllamaAdapter`), the compute budget (`budget.rs`), the equation compiler
(`eqc-rs`). Extension of each was shown insufficient only where cited (Batch 6 §1.2's three gaps);
nothing else earned new code.

---

*Reconstructed 2026-07-17 against current `main` (`caba2203c`). Original operator confirmation:
"great idea, ++" on the DecisionUnit-family resolution — that resolution is settled canon; do not
re-open "should nodes run separate small models." Citation drift found during reconstruction:
`ollama.rs` routing 33-40 → 33-45; `event_log.rs` drift-gate 389 → 410; `ports/llm.rs:29-33` and
`sync_pull.rs:159` exact and unchanged.*

---

## §13. Dated status note (2026-07-18) — the Mistral/local-LLM audit CONFIRMS this layer's verdicts

Added after the reconstruction pass as a **status note, not a design change** — §0–§12 stand
exactly as written. Source: `docs/repo-maintenance-2026-07-17/LOCAL-LLM-AGENTIC-INFRA-MISTRAL-AUDIT.md`
(a read-only audit run against the live Ollama daemon this session). It independently re-measured
this layer's foundations and confirms every verdict; nothing here reopens a settled decision.

**The §0 physics verdict is re-confirmed against the live host.** The audit re-ran the probes:
`llama3.1:8b` at **~9.2–10.0 tok/s single-stream, flat across 1/2/4 concurrent** (9.21 / 9.36 /
9.80) — the same memory-bandwidth ceiling §0 cites, re-measured, not inherited. The host is 30 GB
RAM, CPU-only, ~26 GB free, Ollama daemon live (4 models pulled: `qwen2.5-coder:7b`,
`llama3.1:8b`, `nomic-embed-text`, `qwen3-embedding:0.6b` — exactly the `TaskClass` routing targets
§1 cites).

**Mistral/Mixtral: ZERO code, and pulling Mixtral 8×7B is explicitly rejected.** A grep over the
whole tree (`mistral|mixtral` across `*.rs *.ts *.tsx *.js *.json *.toml *.yaml *.sh`) returns
**zero matches** — all 8 mentions anywhere are docs/skill fixtures, never executable wiring. The
audit's recommendation, grounded in this host's measured numbers, matches §0's own reframe
verdict exactly:

- The bottleneck is **memory bandwidth, not FLOPs**, and MoE only saves FLOPs. Mixtral's 2-of-8
  expert trick cuts compute per token but its ~13B active params still stream from RAM → **slower
  per token than the 8B dense model** on this bandwidth-bound host. The architecture the brainstorm
  (item 128) praised does not help the metric that limits this host — which is precisely why §2
  reframes the MoE-mirror into *build-time DecisionUnit families*, not a runtime MoE model.
- **Fit is hostile:** Mixtral 8×7B Q4 ≈ 26–28 GB vs ~26 GB free — loads to essentially all RAM
  (the Ollama service already peaked at 19.5 G), risking OOM, for a model that is architecturally
  worse on this workload.
- **The re-open condition is unchanged and narrow:** revisit *only* if a measured table shows
  Ollama's coarse knobs cost >30% aggregate throughput vs tuned alternatives — and even then the
  fix is tuning / `llama-server`, not an MoE model pull. Keep the dense-model + typed-fallback
  Ollama stack; spend effort on the answer-cache / build-time-compile path (i.e. this layer's
  DecisionUnit compile pipeline, §2–§4), not on a new model.

**The `LlmBackend` / `OllamaAdapter` oracle plumbing this layer depends on is confirmed healthy.**
The audit ran `cargo test --tests` in `llm-adapters/` against the live daemon: **12 unit + 3
integration green** — a real, non-mocked chat roundtrip, a 768-dim embed, and rerank-fail-closed,
all green against `127.0.0.1:11434`. This is the exact plumbing §2.1(3) / §3 route the hub-only
compile oracle through (`TaskClass::Code` → `qwen2.5-coder:7b`). One honest caveat the audit adds,
consistent with this layer's own framing: the stack is library + test code, **not yet a product
runtime call-site** — which is fine here, because §2's oracle is a *build-time* consumer, exactly
the seam the audit found working.

**One housekeeping item surfaced (routed to Layer H/I, noted here for completeness):** the audit's
link-resolution pass found `MEMORY.md` has 1 broken link —
`UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-2026-07-11.md` should repoint to the `-v3-` variant. Not a
Layer-F design item; recorded so it is not lost.

**Net effect:** zero design change. This layer's central bet — LLM at compile time on the hub,
native DecisionUnits at runtime on every node — is the correct response to a bandwidth wall that
has now been measured three independent times (doc 21 originally, synthesis §C-C, and this audit).
The settled-canon marker on §2 stands reinforced, not merely restated.
