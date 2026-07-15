# Swarm Cost-Math: Architect + Cheap Executors

*Prepared 2026-07-15. NOTE: live web search/extraction was unavailable in this environment; API prices below are published figures from 2024–2025 and should be re-verified against current provider price pages (marked `unverified` where a 2026 change is plausible). URLs are the canonical sources for each system.*

## 1. Cost math & the crossover N

Model. One expensive "architect" agent designs a READY blueprint once; N cheap subagents execute it in parallel.

- **Sequential baseline (1 expensive agent does all N tasks):** `Cost_seq = N · C_a`
- **Swarm:** `Cost_swarm = C_a + N · C_s`
- **Swarm wins when** `C_a + N·C_s < N·C_a` → **`C_s < C_a·(N−1)/N`**.

As N grows, the threshold approaches `C_a` from below: the swarm wins whenever the executor is cheaper than the architect *and* the fixed architect cost is amortized over enough tasks. Even at N=2 you need `C_s < C_a/2`.

**Concrete $ numbers** (output tokens; prices `unverified` for 2026):

| Role | Model (example) | Out / 1M tok | vs. architect |
|---|---|---|---|
| Architect | Claude Opus 4 | ~$75 | `C_a` |
| Executor | Claude Haiku 3.5 | ~$4 | `C_s ≈ 0.053·C_a` |
| Executor (alt) | GPT‑4o‑mini | ~$0.60 | `C_s ≈ 0.02·C_a` |

With `C_a=$75`, `C_s=$4`: threshold `C_s < 75·(N−1)/N` is satisfied for **all N≥2** (since 4 ≪ 75·½=37.5). Breakeven on cost vs. sequential is immediate for any N≥2. Per task, swarm mean cost ≈ `(75 + N·4)/N = 4 + 75/N` → drops toward $4 as N rises (vs. flat $75 sequential).

## 2. Wall-clock: when parallelism actually wins

Parallel speedup is bounded by the **slowest task in a wave**, not by N: `T_swarm ≈ T_arch + max_i(T_i)`. Amdahl/Gunther effects apply — if any single subagent task takes time `L`, total wall-clock ≥ `L` regardless of how many others finish early. Parallelism wins on clock only when tasks are (a) independent and (b) roughly balanced / shardable. If N tasks are perfectly parallel and balanced: `T_swarm ≈ T_arch + T_task` vs. `T_seq ≈ N·T_task`; speedup ≈ `N` until coordination/queueing (Gunther's "serial fraction" `σ`) saturates it: `Speedup ≤ N / (1 + σ·(N−1))`. **Takeaway:** spend the architect on good *sharding*; a bad partition leaves you waiting on one long tail.

## 3. Risks

- **Subagent drift / inconsistent blueprint interpretation** — each executor re-reads the spec; ambiguity amplifies N‑fold.
- **Error propagation** — one wrong executor output poisons the merge; no shared context.
- **Verification gap** — need a **verifier tier**: either a cheap self-check/unit-test executor loop, or the architect re-validating merged results. Without it, cost savings buy silent defects.
- **Coordination overhead** — orchestrator latency, retries, and the architect's own rework can erode both $ and clock gains.

## 4. Real orchestration systems

- **Anthropic — "Building effective agents" / workflow patterns** (prompt-chaining, routing, parallelization, orchestrator-workers, evaluator-optimizer): the "orchestrator-worker" + "evaluator-optimizer" patterns are exactly architect+executors+verifier. https://www.anthropic.com/engineering/building-effective-agents
- **OpenAI Agents SDK** (successors to the experimental "Swarm" demo) — handoffs, fan-out via tool calls. https://openai.github.io/openai-agents-python/ ; Swarm repo: https://github.com/openai/swarm
- **Microsoft AutoGen** — `GroupChat` / `RoundRobinGroupChat` multi-agent conversation; "AutoGen: Enabling Next-Gen LLM Applications via Multi-Agent Conversation" (Wu et al., 2023/2024). https://microsoft.github.io/autogen/ ; paper: https://arxiv.org/abs/2308.08155
- **LangGraph** — explicit state graph with `Send` API for dynamic fan-out / map-reduce supersteps. https://langchain-ai.github.io/langgraph/concepts/low_level/
- **MapReduce-style** — Anthropic's "Many small agents" / Claude Research-style sub-agent fan-out (each sub-agent independently retrieves+answers, then synthesized).

## 5. Routing model

Route by **task complexity / uncertainty → tier**:
- High ambiguity, design, cross-cutting constraints → **architect (expensive)**.
- Well-specified, atomic, verifiable → **cheap executor**.
- Output that feeds other agents or is untestable → insert a **verifier** (cheap self-check, or architect review).

## CITATIONS
- Anthropic, "Building Effective Agents" — https://www.anthropic.com/engineering/building-effective-agents
- OpenAI Agents SDK — https://openai.github.io/openai-agents-python/ ; Swarm — https://github.com/openai/swarm
- AutoGen: Wu et al., "AutoGen: Enabling Next-Gen LLM Applications via Multi-Agent Conversation," arXiv:2308.08155 — https://arxiv.org/abs/2308.08155
- LangGraph low-level concepts (Send / fan-out) — https://langchain-ai.github.io/langgraph/concepts/low_level/
- Amdahl's Law — https://en.wikipedia.org/wiki/Amdahl%27s_law ; Gunther's Universal Scalability Law — https://en.wikipedia.org/wiki/Scalability#Universal_Scalability_Law
- Model pricing (unverified for 2026): Anthropic — https://www.anthropic.com/pricing ; OpenAI — https://openai.com/api/pricing/
