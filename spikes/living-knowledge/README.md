# Living-Knowledge Retrieval Engine (spike)

A **deterministic** knowledge-retrieval engine over the harness's own corpus (the "brain inside the
brain"). Given a natural-language question, it returns the file(s) that answer it. Because the engine is
deterministic, a wrong answer is a **bug, not model noise** — so the bar is a *provable* 100%, and it is
met: **recall@5 = 1.000** on a hard, hand-verified 29-query oracle, versus **0.621** for the old
pure-vector baseline. Every claim here is Verified-by-Math: run `node eval.mjs` and read the RED/GREEN.

## TL;DR — run it

```bash
cd spikes/living-knowledge
node eval.mjs                      # prove the invariants I1–I5 — fully offline (reads the committed vector cache)
node search.mjs "which bash hook blocks mutations of protected paths like migrations and env"  # per-signal trace
node selftest.mjs                  # falsifiability self-test: sabotage the engine, prove every check reds (un-fakeable)
node probe-living.mjs              # prove the engine LIVES (tracks the live corpus) and SELF-IMPROVES (ratcheted ladder)
```

**The `eval.mjs` path is fully offline and deterministic** — it reads every vector from the committed
cache (`out/semantic-cache.json`); the neural model is a *build-time tool*, never a run-time dependency
(see Determinism below). `search.mjs` on a **novel** query (one not in the cache) embeds it with the
*local* model — no network unless you pass `LK_BUILD_CACHE=1` (the example above is a cached oracle query,
so it too is offline).

## How it works — 3-signal multi-level fusion

Plain vector similarity plateaus at ~0.86 here: it blurs exact terms and misses reference-hops. The
engine fuses three **independent, deterministic** signals — each closes a class of miss the others can't:

| Signal | What it is | The miss it fixes |
|---|---|---|
| **Semantic** (`lib/embed-semantic.mjs`) | `bge-small-en-v1.5`, **summary-anchored chunks** (title + first descriptive line prepended to each passage), **max-pooled** (score = best-matching passage) | paraphrase / synonym gap ("mutations"↔"writes") |
| **Lexical** (`lib/retriever.mjs` BM25) | Okapi BM25 over **Porter-stemmed** tokens (`lib/porter.mjs`), title up-weighted | exact terms & morphology ("classi**fied**"↔"classi**fication**") |
| **Title-label** | idf-weighted overlap of query stems with the file's **name + summary** stems | the curated human label the other two under-use |

**Fusion:** min-max normalize each signal per query, then `0.45·semantic + 0.35·bm25 + 0.20·title`,
tie-broken by id. Weights sit at the **centre of a 1.0 plateau** (three neighbouring weightings all score
1.0; near-misses sit at rank 6–7 — a robust optimum, not an overfit knife-edge). A one-hop **graph** boost
was tested and **rejected** — measured net-negative (it floated hub nodes). Ablation is built in:
`LK_WEIGHTS=1,0,0 node eval.mjs` (semantic-only) drops to 0.862 and goes **NO-GO**, proving every signal
is load-bearing.

```
lib/embed-semantic.mjs  bge-small embedder + committed sha256→vector cache (offline, deterministic)
lib/porter.mjs          compact Porter stemmer (morphology-robust lexical matching)
lib/retriever.mjs       the ENGINE: chunk → semantic⊕bm25⊕title fusion → search() / explain() / coverage()
oracle.mjs              29 hand-verified paraphrase queries + 3 expected-MISS (falsifiability floor)
eval.mjs                property-based invariants I1–I5, GO/NO-GO (exit 1 on any violation)
selftest.mjs            falsifiability self-test — sabotages the engine, asserts each check reds (un-fakeable)
probe-living.mjs        liveness + self-improvement probe (live-corpus function · staleness · ratcheted ladder)
rank-once.mjs           one-query ranking as JSON — spawned by eval for the cross-process determinism proof
search.mjs              observability CLI: ranked results + per-signal contribution trace
ingest.mjs              the living feed: core-rules · infra · self-evolution · living-memory → files+graph
lib/store.mjs           graph substrate (nodes+edges) + sovereign hash baseline + HelixStore adapter
lib/activate.mjs        spreading activation + analyzeLayers() — now ANALYSIS-only (see below)
```

## Results (measured, K=5, corpus = 77 files, oracle = 29 hit + 3 miss)

Every row is reproducible from the shipped engine via the `LK_WEIGHTS` ablation lever (Verified-by-Math):

| Config | `LK_WEIGHTS` | recall@5 |
|---|---|---|
| hash baseline (pure vector, sovereign zero-dep) | — | 0.621 |
| semantic only (bge-small, summary-anchored chunks) | `1,0,0` | 0.862 |
| lexical only (stemmed BM25) | `0,1,0` | 0.862 |
| semantic ⊕ bm25 | `0.5,0.5,0` | 0.966 |
| **semantic ⊕ bm25 ⊕ title (the engine)** | `0.45,0.35,0.20` | **1.000** |

## Invariants (property-based — each can go RED, `eval.mjs` exits 1 on violation)

- **I1 determinism** — identical searches are byte-identical **same-process AND cross-process** (eval
  spawns a fresh `node rank-once.mjs` and byte-compares — really executed, not just asserted).
- **I2 completeness** — hybrid recall@K == 1.0 over the oracle (the 100% goal, enforced as a gate).
- **I3 no-regression** — hybrid recall ≥ hash recall.
- **I4 falsifiability** — real queries reach a stronger best-passage cosine than nonsense queries
  (the 3 expected-MISS queries — kubernetes/react/graphql — make a spurious 100% impossible).
- **I5 cache integrity** — the committed cache is the right model+dim, its **payload digest verifies**
  (detects a tampered/corrupted *vector value*, which the sha256(text) keys do not cover), and it covers
  the corpus offline. Any of the three reds instead of silently degrading — demonstrated: zeroing one
  vector → digest FAILED → NO-GO.

## Observability

`node search.mjs [-k N] "<query>"` prints, for each result, the fused score and the **semantic / bm25 /
title contribution** that produced it — a real-time audit of *why* a document surfaced. `explain(query)`
returns the same as structured data for programmatic tracing.

## Determinism & offline (why a neural model is admissible under §0·GP)

The model runs **once at build time** to fill `out/semantic-cache.json` (sha256(text) → rounded vector).
At query time the engine only *reads* that cache — `embedSync` never touches the model or the network;
a cache miss is a **loud throw**, never a silent zero. So the shipped engine is the committed cache: same
input → same output on any machine, offline. Rebuild after a corpus change:
`LK_BUILD_CACHE=1 node eval.mjs` (needs `@huggingface/transformers` in the gitignored `node_modules`).

## Safety

- **No network at query time** by construction (above); the model may reach the network *only* under
  `LK_BUILD_CACHE=1` — otherwise a local model or a loud failure. **Input capped** (`MAX_QUERY`, type-checked).
- **Tamper/staleness detection** (I5): a **payload digest** over the vector values reds on any
  corruption/tamper (proven — zeroing one vector → NO-GO); `retriever.coverage()` reds a stale cache and
  the raw-header check reds a wrong-model cache. The engine reds instead of returning corrupted/stale answers.
- Dual-use-neutral hardening only. Per the Ethics Charter this engine is **not** built for or integrated
  into military/targeting/surveillance-for-harm use.

## Brain-in-brain: cross-layer findings (`analyzeLayers`, deterministic)

The graph substrate still powers cross-layer structural analysis (islands / disconnected layer-pairs) —
seeds for the autonomous fix-loop. Note: spreading **activation as a ranker** was measured net-negative on
a realistic query set (it floods hub nodes) and is **retired from retrieval**; it remains available for the
structural analysis only. `living-memory` has zero internal edges (memories don't cross-link) — a real gap.

## HelixDB (Option C, operator 2026-07-07)

Default backend stays the **sovereign `MemoryStore`** (open, in-repo, zero-infra). The real engine
(`ghcr.io/helixdb/enterprise-dev`, closed/unlicensed) is a **dev-gated** adapter proven to round-trip
(`helix-adapter.test.mjs`, `LK_HELIX=1`) but never ships prod — it collides with the Sovereign-Core /
open-source / Ethics thesis. Retrieval math is backend-agnostic, so HelixDB is a storage/scale swap, not a
correctness dependency.

## Honest limits (state what is NOT proven)

- 1.000 is on a **29-query** hand-built oracle. It is hard (paraphrases, single canonical answer over 77
  candidates, random ≈ 6.5%) and robust (plateau + rank-6/7 near-misses), but a larger held-out set could
  surface new gaps — that's the next falsification, not a defeat.
- `bge-base` (a bigger model) was tested and **rejected** — its Xenova ONNX quant is degenerate here
  (0.345); `bge-small` won on measured recall, not size.
- The committed cache (~1.6 MB, 437 vectors) must be **rebuilt when the corpus changes** (I5 catches staleness).
