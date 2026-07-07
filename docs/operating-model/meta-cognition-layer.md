# The Meta-Cognition Layer — a model-agnostic "brain inside the brain"

> Operator 2026-07-07: *"finish brain inside the brain like a meta-cognition layer, prepare your own
> replacement, and make all models use the same unified approach — model-agnostic, so it does not matter
> which one is used."*

## The problem it solves

Every LLM that can drive this harness — Opus, Sonnet, Haiku, Fable, whatever comes next — carries
**different parametric knowledge** and **different failure modes**. If the harness's correctness depends
on *which* model is reasoning, quality is a lottery. The fix is not a better model; it is to stop trusting
any model's memory for facts the harness already knows.

So the harness externalizes its own knowledge into a **deterministic retrieval engine** that any model
**consults before acting**. The model brings reasoning; the layer brings grounded, verified, *invariant*
knowledge. Same knowledge for every model → the same floor of quality regardless of who is driving. This
is the retrieval dual of **§0·GP (ground truth over proxy reasoning)** and **Verified-by-Math**: a model's
recall of a rule is *proxy*; the deterministic layer is *ground truth*. Prefer the layer.

## What it is (built, proven)

`spikes/living-knowledge/` — a deterministic engine over the harness's OWN corpus (its rules, hooks,
guardrails, loops, reflections — the "brain inside the brain"). Ask a natural-language question, get the
file(s) that answer it.

- **recall@5 = 1.000** on a hard, hand-verified 29-query oracle (vs 0.621 for pure-vector). A wrong answer
  is a *bug*, not model noise — because the engine is deterministic — so the bar is a *provable* 100%.
- **Three fused signals** (multi-level indexing, not plain similarity): semantic (bge-small, summary-
  anchored chunks, max-pool) ⊕ stemmed BM25 (lexical) ⊕ title-label. Each is load-bearing (ablation reds).
- **Deterministic + offline**: vectors come from a committed, digest-sealed cache; the query path never
  touches a model or the network. Same input → byte-identical output on any machine, cross-process proven.

Full architecture + how-to: `spikes/living-knowledge/README.md`.

## The unified consult protocol (every model, same steps)

Before any non-trivial action (a rule-governed edit, a "which guardrail…", a "how do we…", a "where is the
canonical…"), the driving model does NOT answer from parametric memory. It consults the layer:

1. **Ask** — a natural-language question. `node spikes/living-knowledge/search.mjs "<question>"`.
2. **Ground** — read the returned canonical file(s); `explain()` shows *why* each surfaced (per-signal
   contribution), so the grounding is auditable, not blind trust.
3. **Act** — reason and act on the retrieved ground truth, not on a remembered approximation.

Because retrieval is **deterministic**, two different models asking the same question get the **same**
files. The knowledge substrate is invariant across models — that is what "model-agnostic" means here.

## The contract: invariants, not vibes (I1–I5)

The layer proves *itself* (`eval.mjs`, each can go RED, exits 1 on violation):

- **I1 determinism** — byte-identical rankings, same-process AND cross-process.
- **I2 completeness** — recall@5 == 1.0 over the oracle.
- **I3 no-regression** — hybrid ≥ the pure-vector baseline.
- **I4 falsifiability** — real queries out-score nonsense (expected-MISS floor forbids a spurious 100%).
- **I5 cache integrity** — right model + payload digest verified + covers the corpus offline (tamper/
  staleness reds instead of silently degrading).

These guarantees hold for whichever model is driving. The layer's quality is a property of the layer, not
of the model — which is the whole point.

## "Prepare your own replacement" — the handoff IS the layer

The knowledge does not live in any model's weights; it lives in the **deterministic layer + its committed
cache + its invariants**. That is what makes the current model replaceable:

- A fresh session (any model) rebuilds **identical** state (determinism) — no warm-up, no lore in a
  chat history. `node eval.mjs` re-proves the invariants from the committed artifact, offline.
- The layer is **self-describing**: README (supportability) + `explain()`/`search.mjs` (observability) +
  I1–I5 (the contract). A successor reads these and continues — it does not need to have *been* here.
- The corpus is the harness itself, so the layer **grows as the harness grows** (rebuild the cache when
  the corpus changes; I5 reds on staleness so drift can't hide).

The replacement is not a person or a model — it is the invariant layer that any model plugs into.

## Scope & limits (honest)

- The 1.0 is on a 29-query hand-built oracle — hard and robust (plateau, rank-6/7 near-misses), but a
  larger held-out set is the next falsification, not a defeat.
- Spreading **activation as a ranker** was measured net-negative (hub flooding) and retired from
  retrieval; the graph substrate remains for cross-layer structural analysis (`analyzeLayers`).
- This layer is knowledge *retrieval*, not the business kernel. Per the **Ethics Charter** it is
  dual-use-neutral tooling and is **not** built for or integrated into military/targeting/surveillance use.

See also: [model-agnostic-playbook](model-agnostic-playbook.md) · [verified-by-math](verified-by-math.md)
· [living-knowledge-helixdb-arc](living-knowledge-helixdb-arc.md).
