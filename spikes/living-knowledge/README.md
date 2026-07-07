# Living Knowledge + multi-band activation (spike, 2026-07-07)

Executes the arc `docs/operating-model/living-knowledge-helixdb-arc.md` steps 1→5, with the operator's
2026-07-07 refinements: **integrate HelixDB (Option C: sovereign default + dev-gated real-engine
adapter), wire it with the memory layers, reverse-engineer its patterns, and prove it works with math.**
All claims here are Verified-by-Math (falsifiable; run the commands below).

## What this is

A deterministic, zero-dependency **living-knowledge store** (graph + vectors) behind a thin **port**,
with a **spreading-activation** retrieval layer that multiple **bands** trace over the SAME store.
The corpus is the harness's own layers — this is the "brain inside the brain."

```
lib/embed.mjs     deterministic TF-IDF hash embedding + cosine (no model, no network, reproducible)
lib/store.mjs     PORT + MemoryStore (sovereign default) + HelixStore (dev-gated real-engine adapter)
lib/activate.mjs  spreading activation a(n,t+1)=clamp(a·retain + Σ a(m)·w(type,band)·decay/√(deg·deg)),
                  BANDS (code-structure · why · data · temporal), trace(), analyzeLayers()
ingest.mjs        the living feed: core-rules · infra · self-evolution · living-memory → graph+vectors
eval.mjs          oracle Q&A: precision/recall vs baseline + determinism + GO/NO-GO (falsifiable)
helix-adapter.test.mjs   proves the store round-trips on the REAL HelixDB engine (LK_HELIX=1)
```

Run: `node spikes/living-knowledge/eval.mjs` · report: `node scripts/probe-system-comparison.mjs`

## Results (measured, K=5, corpus = 76 files / 165 reference edges)

| Metric | Baseline (pure vector) | Activation (bands) |
|---|---|---|
| recall@5 | 0.813 | **0.875** (+0.062) |
| precision@5 | 0.225 | **0.25** |
| real vs nonsense top-1 confidence | — | 0.454 vs 0.356 (separable) |
| determinism (bit-identical reruns) | — | **true** |

The lift comes exactly where designed: the "certified loop must have its report" query goes 0.50→1.00
because activation reaches `guardrail-loop-registry-parity.mjs` through its **reference edges** — the
referenced-neighbour signal a pure-vector store cannot follow. The eval includes expected-MISS queries
(kubernetes, react) so a spurious 100% is impossible; the GO verdict ASSERTS activation ≥ baseline,
determinism, and real>nonsense — each can go RED (it did, twice, during development — see git history:
first cut flooded high-degree hubs; fixed by fan-out normalization + TF-IDF).

## HelixDB (Option C, operator 2026-07-07)

Ground truth (real build+run, see `../../../tmp/.../helix-recon.md` / handoff): the `helix-db` OSS repo
is only CLI+SDKs (Apache-2.0); the graph-vector **engine is a closed, no-visible-license Docker image**
(`ghcr.io/helixdb/enterprise-dev` v3.0.8). It runs here — proven. So:

- **Default backend = sovereign `MemoryStore`** (open, in-repo, zero-infra) — the closed/unlicensed
  engine collides with the Sovereign-Core thesis + open-source goal + Ethics Charter, so it never ships prod.
- **`HelixStore` adapter is dev-gated** and PROVEN against the live engine (`helix-adapter.test.mjs`,
  `LK_HELIX=1`): readiness 200, AddN 3/3 writes, count 0→3 round-trip. The wire contract was
  reverse-engineered from the engine's OWN validation errors — `returns` nests inside `query` (not
  root), properties are `[key,{Value:{String}}]` pairs — not from the (wrong) README/CLI shape.
- **Patterns reverse-engineered and applied** to the sovereign layer: deterministic scan-row budget
  (not wall-clock) on traversals; AST-composed hybrid retrieval (seed→spread→rerank); index-families
  thinking (activation edges as an overlay). The activation function is backend-agnostic, so retrieval
  math is identical on either backend — HelixDB is a storage/scale swap, not a correctness dependency.

## Brain-in-brain: cross-layer findings (deterministic, `analyzeLayers`)

Layers: core-rules · infra · living-memory · self-evolution. Cross-layer edge matrix + islands surface
the "not connected edges / useful findings" for the autonomous phase:

- **`living-memory` has ZERO internal edges** — reflections/memories never cross-reference each other
  (they cite rules/loops but not siblings). A real gap: memory is a bag, not a graph.
- **13 infra guardrails are islands** (no cross-layer edge) — referenced by no rule/registry, e.g.
  `guardrail-no-set-cookie`, `guardrail-license`, `guardrail-legacy-freeze`, `guardrail-deliver-v2`.
  Candidate: some are orphaned (unwired), some just undocumented. `guardrail-no-set-cookie` has no
  reachable failure path (flagged separately) → prime autonomous-fix target.
- **`task-exit-rule.md`** (a core rule) is an island — not referenced by sibling rule docs.

These are seeds for Task: "cross-layer pattern discovery (brain-in-brain autonomous phase)" — the loop
that ingests → multi-band traces → proposes fixes for disconnected edges, gated by the deterministic
armaments. NOT auto-fixed here (some islands are legitimately standalone); surfaced as falsifiable findings.

## Honest limits (Verified-by-Math: state what is NOT proven)

- The embedding is a deterministic **TF-IDF hash BoW**, not a semantic model — one query
  ("which bash hook blocks mutations") misses for BOTH baseline and activation (no hidden failure).
  A real offline embedding is a drop-in swap behind `embed()` and would raise the absolute numbers.
- The HelixDB adapter proves the **graph** path (AddN/NWhere/Count); its **vector KNN** (HNSW) AST was
  not reverse-engineered — activation runs in our layer, so this doesn't affect retrieval quality, only
  where vectors could live at scale.
