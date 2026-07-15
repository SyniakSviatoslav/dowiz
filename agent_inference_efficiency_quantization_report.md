# Agent-Level & Inference-Efficiency "Quantization" — Research Report

**Scope:** techniques that make an agent swarm *cheaper/faster without shrinking the model weights*. All claims below preserve target-model quality by construction (verification / exact sampling / lossless caching) unless noted.

## 1. Speculative decoding (draft + verify)
A cheap draft model proposes K tokens; the expensive target verifies them in one parallel pass with a rejection-sampling scheme that **preserves the target distribution exactly** (no quality loss).
- Leviathan & Matias, *Fast Inference from Transformers via Speculative Decoding* (NeurIPS 2023, **arXiv 2211.17192**): exact sampling, no retraining.
- DeepMind, *Accelerating LLM Decoding with Speculative Sampling* (**arXiv 2302.01318**, 2023): **2–2.5×** decoding speedup on Chinchilla-70B, no quality compromise.
- **SpecInfer** (Serving, **arXiv 2305.09781**, 2023): token-tree draft + tree verifier; **1.5–2.8×** distributed, **2.6–3.5×** offloading, same generative quality.
- **Medusa** (arXiv 2401.10774, 2024): multiple MLP decoding heads + tree attention; drops the separate-draft-model requirement.

## 2. KV-cache quantization
The KV cache (not weights) dominates memory at long context; quantizing it is the bottleneck fix for long-context agents.
- **KVQuant** (arXiv 2401.18079, 2024): <0.1 perplexity degradation at **3-bit**; serves LLaMA-7B at **1M context on one A100-80GB** and **10M on 8 GPUs**; custom kernels give **~1.7×** mat-vec speedup over fp16.
- **KIVI** (arXiv 2402.02750, 2024): tuning-free **2-bit asymmetric** (per-channel keys, per-token values) KV quantization, large memory cut with minimal accuracy loss.

*Claim "INT2 keeps 99% accuracy" is unverified per exact figure; KVQuant reports <0.1 perplexity drop at 3-bit, KIVI at 2-bit — verify against each paper's exact table before quoting a %.*

## 3. Semantic compression / context distillation
Compress or summarize stale conversation/memory before re-feeding; the live model never sees the raw token history.
- Anthropic **prompt caching** (docs, 2024) lets you pin long, reused context (system prompt, agent memory, tool schemas) so it is computed once. Cache-read tokens cost **0.1× base input = ~90% cheaper**; cache-write 1.25× (**docs.anthropic.com/en/docs/build-with-claude/prompt-caching**).
- Claude/Gemini **auto-compact**: providers summarize overflowing context automatically (vendor docs; exact token savings not published → *unverified* as a fixed ratio).
- Distilled/"memory-rank" compaction (summarize + retrieve) yields token savings proportional to history length; no single canonical number — treat as *unverified* unless measured on your corpus.

## 4. Distilled router / Mixture-of-Depths / early-exit
Route easy tokens/layers to cheap compute instead of full depth everywhere.
- **Mixture-of-Depths** (arXiv 2404.02258, 2024, Google): top-k token routing per layer under a fixed FLOP budget; matches baseline quality but is **>50% faster to step** during sampling.
- Early-exit / mixture-of-experts routers: route by confidence; reported speedups are model- and task-specific (*unverified* as a single number here).

## 5. Tool-call & prompt caching
- **Anthropic prompt caching**: **0.1× (90% cheaper)** cached input; 5-min (now longer) TTL; ideal for agent system prompts + tool definitions reused every turn.
- **OpenAI prompt caching** (platform.openai.com/docs/guides/prompt-caching): automatic on ≥1024-token prefixes; discounted cache reads, cache writes **1.25×** input on newer families. (OpenAI does not publish a flat % — *unverified* as exactly 90%.)

## 6. Quantized / binary embeddings for retrieval
Lossy vector compression for agent memory/Retrieval-Augmented agents.
- **Product Quantization (PQ)** — Jégou, Douze, Schmid, *Product Quantization for Nearest Neighbor Search*, IEEE TPAMI 2011 (DOI **10.1109/TPAMI.2010.57**): splits a d-dim vector into m subvectors, each coded to a centroid; **64-D SIFT → 8 bytes (m=8) = 32× compression** at ~1% recall loss. Drives **FAISS** PQ/IVF-PQ (github.com/facebookresearch/faiss).
- **ScaleANN / binary (Hamming)** codes give further (e.g. 32×–64×) compression with faster scan; accuracy-savings tradeoff is corpus-dependent (*unverified* as fixed %).

## Bottom line for an agent architect
Three levers give the biggest, *quality-preserving* wins with published numbers: **speculative decoding (2–3.5×)**, **KV quant (3-bit, 10M-context capable)**, and **prompt caching (90% cheaper reused context)**. Context-compaction and PQ-retrieval compress agent *state/memory* but lack a single canonical ratio — measure on your own workload.

## CITATIONS
- Leviathan & Matias 2023 — https://arxiv.org/abs/2211.17192
- DeepMind speculative sampling 2023 — https://arxiv.org/abs/2302.01318
- SpecInfer 2023 — https://arxiv.org/abs/2305.09781
- Medusa 2024 — https://arxiv.org/abs/2401.10774
- KVQuant 2024 — https://arxiv.org/abs/2401.18079
- KIVI 2024 — https://arxiv.org/abs/2402.02750
- Mixture-of-Depths 2024 — https://arxiv.org/abs/2404.02258
- Anthropic prompt caching (docs) — https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching
- OpenAI prompt caching (docs) — https://platform.openai.com/docs/guides/prompt-caching
- Jégou et al. PQ 2011 — https://doi.org/10.1109/TPAMI.2010.57 (FAISS: https://github.com/facebookresearch/faiss)
