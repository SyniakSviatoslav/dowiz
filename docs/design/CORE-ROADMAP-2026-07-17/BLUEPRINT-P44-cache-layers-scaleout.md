# BLUEPRINT P44 — Cache Layers (EC-05) + Own-RAG / Own-Inference Scale-Out (2026-07-18)

> **Priority banner (read this before anything else):** P44 is marked **LOW PRIORITY /
> FAR-FUTURE** in the master roadmap (§10.5.5) and this blueprint agrees with that verdict rather
> than fighting it. Its real deliverable is NOT five cache designs — it is the **measurement
> methodology** that must exist before any layer may be built. Everything else here is
> deliberately thin, and that thinness is the point.

Phase: **P44** (ECOSYSTEM/OPS) · Status: **PLANNED — 0 of 5 layers built, correctly so**
Absorbs: EC-05 + the own-inference/own-RAG/chunking/gossip units of EC-03/04/06/08/12–15
Source arc: `/root/.claude/projects/-root-dowiz/memory/ecosystem-strategy-arc-2026-07-13.md`
Master entry: `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.5 (P44)

---

## 1. Ground truth — every cite re-verified live this pass (standard §2 item 1)

1. **Exactly one cache exists, unchanged since Wave 1(b).** `llm-adapters/src/cache.rs`
   (273 lines) is still the single exact-match sha3-keyed response cache:
   `CachingBackend<B, S>` wraps any `LlmBackend` with a `dowiz_kernel::backup::BlockStore`;
   key = `sha3_256` of a BTreeMap-canonical request (`cache.rs:57-81` — model_id, messages,
   temperature, top_p, max_tokens, seed, task_class, options); `MemStore::put` idempotence IS
   the hit semantics; `NoCache` store compile-disables it (`cache.rs:155-168`). Three green
   tests (`exact_hit_makes_zero_upstream_calls`, `changed_param_is_a_miss`,
   `no_cache_policy_bypasses`). §10.5.5's "exactly one basic exact-match sha3-keyed cache"
   claim is **confirmed still true** — the file has not grown.
2. **`embed()` and `rerank()` pass through UNCACHED** (`cache.rs:108-113` — straight
   delegation to `inner`). The future "embedding cache" layer's insertion point is already
   visible in the code; nothing occupies it. Recorded as fact, not built here.
3. **Zero cache observability exists.** `llm-adapters/src/telemetry.rs` tracks per-model
   success rates and token means from a ledger (`ModelStats`, `Telemetry::ingest`,
   `from_ledger`) — there is **no hit/miss counter, no cache-outcome event, no latency
   histogram anywhere**. A cache hit today is invisible: the P44 baseline cannot be measured
   with what exists. This gap — not any missing layer — is P44's first real build item.
4. **No own-RAG, no chunking, no HNSW in this repo.** The retrieval/HNSW machinery the arc
   references lives in the repowise tooling, not in `dowiz` product code. Confirmed by
   inspection of `llm-adapters/src/` (8 files: cache, compose, dispatch, lib, ollama, quirks,
   telemetry, transport — nothing retrieval-shaped).
5. **The traffic source does not exist yet.** AGENT P40's tool-loop
   (`BLUEPRINT-P40-agent-loop-tool-wiring.md`) and P41's three-mode operation
   (`BLUEPRINT-P41-three-mode-ai-operation.md`) are the phases that will produce real
   inference traffic; DELIVERY P37 produces real product load. Per §10.2, P37 is 0% (no HTTP
   server) — so there is no workload to measure, and therefore nothing a cache layer could
   honestly claim to improve.

## 2. Scope & anti-scope — a blueprint that argues for its own low priority

**Why this document is deliberately light.** Two phases in the entire roadmap are explicitly
marked lowest-priority (this one and P46), and the operator's standing instruction — "нічого
не добавляти що не критично" (add nothing that isn't critical) — applies to planning effort as
much as to code. This session's own 6-week assessment already diagnosed "planning outpaces
building" as a live risk: P34/P37's wiring work sits unbuilt while planning documents multiply.
A 500-line cache-architecture spec written against imagined load would be exactly that failure
mode wearing a blueprint's clothes. So this file records the *shape* of the work for future
swarm pickup, formalizes the one rule that matters (baseline before layers), and stops.

**In scope (when unblocked):**
- The baseline measurement methodology (§4.1) — the only item with near-term buildability.
- The five EC-05 layers, **cited from the arc, not re-derived here**: embedding cache
  (content-hash), Merkle incremental re-index (re-embed only changed), prompt/prefix disk
  cache (content-address-by-prefix-hash), retrieval-pipeline cache (ancestor-signature DAG
  memoizer over decide/fold, invalidation = refold-not-timer), semantic cache. The arc doc is
  the design source; this blueprint adds only the shipping gate each must pass.
- Own-inference-beyond-Ollama and own-RAG (pgvector-on-pgrust HNSW + chunking + RRF/rerank)
  scale-out — even further out than the cache layers; gated identically.

**Anti-scope (binding):**
- **Do NOT build any cache layer before AGENT P40/P41 produces real inference traffic.**
  Cache design against imagined workloads is the definition of premature optimization
  (§10.5.5, verbatim). This includes "just prototyping" a layer.
- **Do NOT design cache algorithms in this document or its successors** beyond the arc
  citations above. When a layer becomes buildable, its detailed design happens *then*,
  against measured traffic shapes — not now, against guesses.
- **Do NOT stand up own-inference infra while the existing Ollama port is unsaturated.**
  Saturation is a measured claim (dispatcher queue depth / latency under real load), never
  an assertion.
- **Do NOT weaken the `CachePolicy` gate**: gate-critical callers must remain unable to opt
  into semantic caching (`cache.rs` module doc, lines 6-7). Any future semantic layer is
  advisory-call-sites-only. This is the one safety invariant future workers must carry.

**Depends on / blocks:** depends on AGENT P40/P41 (traffic) + DELIVERY P37 (load). Blocks
nothing; nothing waits on P44. If a swarm agent finds this file while those are unbuilt, the
correct action is to go build *those* instead.

## 3. Predefined types & constants (standard §2 item 4 — minimal, instrumentation-only)

Only the measurement layer gets types now; naming layer-implementation types today would be
premature-design smuggled in through the type system.

```rust
/// Outcome of one cache-mediated call. Emitted per request by CachingBackend once
/// instrumentation lands (§4.1). Lives in llm-adapters (NOT kernel — zero-dep invariant).
pub enum CacheOutcome { Hit, Miss, Bypass /* CachePolicy::NoCache */ }

/// The baseline artifact every future layer is benchmarked against. Serialized into the
/// telemetry ledger; the DoD's "number a layer must improve" is a field of this struct.
pub struct CacheBaseline {
    pub window_requests: u64,     // total requests in the measurement window (≥ MIN below)
    pub hit_rate: f64,            // hits / (hits + misses), bypasses excluded
    pub p50_hit_us: u64,          // latency, cache-served
    pub p50_miss_us: u64,         // latency, upstream-served
    pub upstream_tokens_total: u64, // spend that a better layer could reduce
}

/// A baseline measured on fewer real requests than this is not a baseline.
pub const BASELINE_MIN_REQUESTS: u64 = 1_000;
```

## 4. Build items — spec → RED test → code (standard items 3, 5)

**Honest statement: there is almost nothing to specify yet.** Neither the five layers nor the
scale-out units are buildable now (no traffic, no load), and inventing RED-test names for code
that must not be written yet would be fake rigor. Exactly one item is genuinely specifiable:

### 4.1 The baseline instrumentation (the phase's only near-term item)

- **What to instrument:** `CachingBackend::chat` emits one `CacheOutcome` + wall-clock
  duration per call; the telemetry ledger (extend `telemetry.rs`'s existing ledger pattern —
  reuse-first, standard item 19) aggregates into a `CacheBaseline`.
- **When:** only after P40's tool-loop produces real traffic. Landing the counters a little
  earlier is acceptable (they're cheap and passive); *publishing a baseline* from synthetic
  traffic is not — a baseline over developer smoke-tests is a lie with a decimal point.
- **How the baseline is established:** first window of `BASELINE_MIN_REQUESTS` real
  AGENT-loop requests → serialize `CacheBaseline` → commit it under `docs/regressions/` as
  the pinned number. Adversarial case (item 5): a window mixing `Bypass` outcomes into
  `hit_rate` must fail the aggregation test — bypasses are excluded by definition.
- **RED test (the one honest one):** `baseline_excludes_bypass_outcomes` — constructible
  today against the `CountingBackend` double already in `cache.rs` tests. Everything beyond
  this single test belongs to the future pass that builds it.

### 4.2 Layers 1–5 and scale-out — NOT specified here, by design

Each layer, when its time comes, gets its own mini-spec *in this file* (append-only), and
must arrive with: (a) the pinned `CacheBaseline` it targets, (b) a benchmark showing net win
over the existing sha3 exact-match cache, (c) a delete-clause — see §6. Writing those specs
now would contradict §2. This section is intentionally four sentences long.

## 5. Cross-cutting design obligations (standard items 6, 8, 11–16 — proportionate)

- **Hazard safety (item 6):** the only reachable hazard today is a semantic-cache layer
  serving a stale/approximate answer to a gate-critical caller. Unreachable by construction:
  `CachePolicy` on `ChatRequest` is typed so gate-critical call sites cannot opt in
  (`cache.rs:6-7`). Future layers must preserve that unreachability argument, not re-prove it.
- **Scaling axis (item 8):** `CacheBaseline` scales per-node (requests/sec on one hub);
  cross-node/gossip cache sharing is an arc idea that would change the schema — that is the
  stated breaking point, and it is far past any current horizon.
- **Isolation (item 11):** cache failure must degrade to miss, never to error — already the
  live pattern (`decode_response` returns `None` → miss, `cache.rs:135-136`). Binding on all
  future layers.
- **Living memory (item 15):** the embedding/Merkle layers have temporal-topological access
  patterns — cross-reference `internal-retrieval-living-memory-arc-2026-07-14` when (and only
  when) those layers start.
- Items 12/13/16 (mesh, rollback-as-math, spectral): **not applicable at this phase's current
  depth**; stating pro-forma compliance would be padding. Revisit per-layer at build time.

## 6. DoD — falsifiable, and the centerpiece of this blueprint (standard item 2)

Formalizing §10.5.5's two-line DoD into the shipping gate:

1. **BASELINE-FIRST (hard gate):** a `CacheBaseline` measured on ≥ `BASELINE_MIN_REQUESTS`
   real AGENT-loop requests exists and is pinned in `docs/regressions/` **before any layer
   PR opens**. Falsifier: a layer PR whose description cannot cite the pinned baseline file
   + the specific field it improves is rejected mechanically, no judgment call.
2. **NET-WIN-OR-DELETE:** each layer lands only with a before/after benchmark (standard
   item 10 — real measured output, not an estimate) showing net win over the existing sha3
   exact-match cache on the pinned baseline's field. A layer that doesn't beat it is
   **deleted, not kept** (§10.5.5 verbatim). Falsifier: the layer's code present in-tree
   without its benchmark artifact.
3. **SATURATION-FIRST for scale-out:** own-inference/own-RAG work may not start until a
   measured saturation number on the existing Ollama port exists. Falsifier: any
   inference-infra commit predating that number.

No further DoD items — a far-future phase with a long DoD is a phase pretending to be near.

## 7. Benchmark plan (standard item 10)

The phase IS a benchmark plan — §4.1 and §6 are it. One addition: the baseline measurement
itself must cost < 1% overhead on the request path (counter increment + one ledger append),
asserted by a microbenchmark when the instrumentation lands. Nothing else to measure until
there is traffic.

## 8. Links to docs & memory (standard item 7)

- Master entry: `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.5 P44 block.
- Source arc (the five layers' design provenance — cite, don't re-derive):
  `/root/.claude/projects/-root-dowiz/memory/ecosystem-strategy-arc-2026-07-13.md`.
- Traffic source phases: `BLUEPRINT-P40-agent-loop-tool-wiring.md`,
  `BLUEPRINT-P41-three-mode-ai-operation.md` (same directory); load source: P37
  (`BLUEPRINT-P37-order-http-surface.md`).
- Live code: `llm-adapters/src/cache.rs`, `llm-adapters/src/telemetry.rs`.
- Living-memory arc (future layers only): memory
  `internal-retrieval-living-memory-arc-2026-07-14.md`.

## 9. Standard-compliance map (all 20 points — honest N/A where true)

| Item | Status | Item | Status |
|---|---|---|---|
| 1 ground truth | §1, live cites | 11 isolation | §5 degrade-to-miss |
| 2 DoD | §6, falsifiable | 12 mesh | N/A at this depth (§5) |
| 3 TDD plan | §4.1 (one honest test) | 13 rollback-as-math | N/A at this depth (§5) |
| 4 types first | §3 | 14 error-propagation gate | §6-1 mechanical PR rejection |
| 5 adversarial case | §4.1 bypass-exclusion | 15 living memory | §5 cross-ref |
| 6 hazard-as-math | §5 CachePolicy unreachability | 16 tensor/spectral | N/A at this depth |
| 7 links | §8 | 17 regression | baseline pinned in docs/regressions/ |
| 8 scaling axis | §5 | 18 agent instructions | §10 |
| 9 Linux discipline | inherits P-A verdicts, nothing new | 19 reuse-first | telemetry-ledger extension, no new store |
| 10 benchmarks | §6-2, §7 | 20 Hermetic | measurement-before-mechanism = the Verum/causality principle; no new surfaces |

## 10. Clear instructions for other agentic workers (standard item 18)

You have zero session context. Read this before touching P44:

1. **Check the gate first:** does real AGENT-loop traffic exist (P40 live, requests flowing)?
   If NO — stop; P44 is not startable; go build P34/P37/P40 instead. That redirect is this
   blueprint working as intended, not a failure to find work.
2. If traffic exists: build §4.1 exactly (instrument, measure ≥1000 real requests, pin the
   `CacheBaseline`). Do not start any layer in the same PR.
3. Only then: pick ONE layer from the arc's five, append its mini-spec to §4.2 of this file,
   cite the pinned baseline field it targets, build it behind the net-win-or-delete clause.
4. Never let any layer weaken the `CachePolicy` gate (§2 anti-scope, §5 hazard argument).
