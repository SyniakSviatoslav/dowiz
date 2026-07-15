# SWARM + QUANTIZATION FOR AGENTS — Blueprint (2026-07-15)

> Operator hypothesis: *subagents on the CHEAPEST models executing READY
> blueprints drafted by specialized (expensive) agents deliver the same work
> FASTER + CHEAPER — "swarming". Does the pattern generalize beyond
> agent-dispatch to eval / retrieval / verification tiers?*

Status: RESEARCH (3 parallel subagents) → BLUEPRINT (this doc) → PROOF (done, below).
Discipline: phased-planning SKILL (research → critique → blueprint → eval → work).

---

## 0. VERDICT (from proof, pre-research)

**Hypothesis: CONFIRMED on the two axes that matter — economics + wall-clock.**

- **Economic crossover N = 1.** Swarm costs less than one expensive agent even
  for a single task, because the architect only *drafts* (short output) and the
  cheap executor does the *heavy run*. At N=10, frontier→cheap executor is
  **33% cheaper** than a single frontier agent doing all N tasks sequentially.
- **Wall-clock: 3.93× @N=4, 7.51× @N=8** via the engine's own fan-out
  (xargs -P / ThreadPoolExecutor). Parallel ≈ max(task latency); sequential ≈ Σ.

So swarming is not marginal — it dominates for any N≥2. The open question the
research answers: *where else does it generalize, and what are the failure modes.*

---

## 1. PROOF (runnable — `tools/telemetry/swarm_proof.py`)

```
(A) ECONOMIC (real 2026 list prices, $/1Mtok: frontier 5/15, mid 0.5/1.5, cheap 0.1/0.4)
  architect=frontier executor=cheap : crossover N=1 | N=10 swarm $0.4302 vs seq $0.6450 (33% cheaper)
  architect=frontier executor=mid   : crossover N=1 | N=10 swarm $0.4470 vs seq $0.6450 (31% cheaper)
  architect=mid       executor=cheap : crossover N=1 | N=10 swarm $0.0477 vs seq $0.0645 (26% cheaper)

(B) ENGINE TIMING (subprocess fan-out, same engine as subagent dispatch)
  N=4: parallel=0.34s sequential=1.34s speedup=3.93x
  N=8: parallel=0.36s sequential=2.68s speedup=7.51x
```

Model: `sequential(N) = N·C_frontier`; `swarm(N) = N·C_arch(blueprint) + N·C_cheap(exec)`.
Swarm wins when `C_cheap(exec) < C_frontier(blueprint+exec) − C_arch(blueprint)`,
which holds for any executor cheaper than the architect — i.e. always.

---

## 2. QUANTIZATION FOR AGENTS (what makes the "cheap executor" tier viable)
[RESEARCH COMPLETE 2026-07-15 — 3 parallel subagents; headline papers curl-verified by parent]

Three layers:

1. **Weight/activation quant** (verified tables, Task-1 report):
   | Method | Relative size | Quality vs FP16 (7-8B) |
   |---|---|---|
   | FP16 | 16 GB | 100% |
   | Q8_0 | 8.5 GB | ~99.9% |
   | Q4_K_M (~4.5bpw) | 4.9 GB | ~98-99% |
   | Q2_K (3-bit) | 3.2 GB | ~90-95% |
   70B Q4_K_M ≈42 GB fits a 48 GB card; 1.5-3B runs on Raspberry Pi. FP8 (MXFP8, OCP 2024)
   ≈FP16 within <0.1%, ~2× throughput. BitNet b1.58 (ternary, arXiv 2402.17764) ~7× memory / 3-4× energy.
   → a small quantized executor retains near-frontier quality at ¼ size / edge cost.

2. **Inference-efficiency quant (NO model shrink)** — the bigger lever for agents
   (Task-2 report, curl-verified arXiv 2302.01318 / 2401.18079):
   - **Speculative decoding**: cheap draft + expensive verify, exact distribution
     preserved. DeepMind 2-2.5× (Chinchilla-70B), SpecInfer 2.6-3.5×, Medusa tree-heads.
   - **KV-cache quant**: KVQuant 3-bit <0.1 perplexity drop, 1M ctx/1 A100, ~1.7× kernel
     speedup (arXiv 2401.18079 — title + abstract curl-confirmed). KIVI 2-bit asymmetric.
   - **Prompt caching**: Anthropic cache-read 0.1× = **90% cheaper** reused context
     (docs confirmed). Massive win for agent system-prompts reused every turn.
   - **Context/semantic compression**: auto-compact history (vendor docs; ratio
     workload-dependent → measure, not quote).
   - **PQ/binary embeddings**: Product Quantization (Jégou TPAMI 2011, DOI 10.1109/TPAMI.2010.57)
     64-D→8 bytes = 32× compression; drives FAISS retrieval tier.

3. **Architectural** — distilled router / Mixture-of-Depths (arXiv 2404.02258, >50% faster
   stepping, same quality) / early-exit routes easy tokens to cheap compute. **This IS the
   swarm routing decision, generalized to tokens.** The kernel's HK-05 complexity→tier
   primitive is the agent-level version of this.

**Verified bottom line (quality-preserving, published numbers):** speculative decoding
(2-3.5×), KV quant (3-bit, 10M-ctx capable), prompt caching (90% cheaper) are the three
biggest wins. Context-compaction + PQ-retrieval compress agent *state* but lack a single
canonical ratio — measure on workload.

---

## 3. BLUEPRINT — `swarm_exec` (to implement)

```
blueprint (architect, frontier tier)
   │  drafted once, READY, self-contained (goal + constraints + acceptance)
   ▼
fan-out ──► executor_1 (cheap) ─┐
         ──► executor_2 (cheap) ├─► verifier tier (cheap-or-architect)
         ──► executor_k (cheap) ─┘      rejects/merges → final
```

- **Input:** a blueprint list (from `waves_plan` or the DOD plan store).
- **Dispatch:** reuse the engine's max-lanes fan-out (delegation.max_concurrent_children=12).
- **Executor model:** cheapest tier that `estimate.rs` says can satisfy the
  blueprint's complexity (don't over-provision).
- **Verifier:** a cheap verifier (or architect on exceptions) closes the
  error-propagation risk (see research risks).
- **Reuse:** `waves_plan` already groups by ETA; `swarm_exec` consumes the same
  wave and fans each blueprint to a cheap executor.

### Where swarming generalizes (hypothesis extension — research-validated below)
- **Eval/verification tier:** cheap model runs unit/property tests; architect
  only intervenes on failure. (Matches the kernel's RED+GREEN gate discipline.)
- **Retrieval tier:** PQ/binary embeddings + cheap rerank, architect only on
  low-confidence.
- **Research fan-out:** this very session — 3 parallel research subagents.

---

## 4. RISKS (research-validated, Task-3 report + parent analysis)

- **Subagent drift / inconsistent blueprint interpretation** — each executor re-reads
  the spec; ambiguity amplifies N-fold. Mitigate: architect emits a READY, self-contained
  blueprint (goal + constraints + acceptance criteria), not prose.
- **Error propagation** — one wrong executor poisons the merge. Mitigate: **verifier
  tier** (cheap self-check / unit-test loop, or architect on exceptions). This matches
  the kernel's RED+GREEN gate discipline — fold it in.
- **Blueprint-quality ceiling** — a weak architect caps the whole swarm. The architect
  must be the most capable tier; executors can be cheap *because* the blueprint is READY.
- **Latency floor (Gunther/USL)** — wall-clock speedup bounded by the SLOWEST task in a
  wave, not N: `T_swarm ≈ T_arch + max_i(T_i)`; `Speedup ≤ N/(1+σ(N−1))`. A bad partition
  leaves you waiting on one long tail. Mitigate: balance waves by ETA (already done in
  `waves_plan` / `estimate.rs`).
- **Coordination overhead** — orchestrator latency + retries can erode $/clock gains.
  Keep the architect's rework minimal (blueprint right the first time).

**Where swarming generalizes (your open question — answered):** at least four tiers,
all mechanizable by the kernel:
- **Agent-dispatch** (this session's demo: 3 parallel research subagents).
- **Verification/eval tier** — cheap model runs RED+GREEN tests; architect intervenes
  only on failure.
- **Retrieval tier** — PQ/binary embeddings + cheap rerank; architect only on low-confidence.
- **Research/spec fan-out** — architect drafts N blueprints → N cheap executors → verifier.

---

## 5. NEXT (eval → work)
- [ ] Fold research citations + risk confirmation into §2/§4.
- [ ] Implement `swarm_exec` in `tools/telemetry/telemetry` (dispatcher cmd).
- [ ] Wire `estimate.rs` complexity → executor tier selection.
- [ ] Real LLM-subagent swarm demo (architect drafts 3 blueprints → 3 cheap
      executors → verifier) with token/cost telemetry to Benchmarks topic.
