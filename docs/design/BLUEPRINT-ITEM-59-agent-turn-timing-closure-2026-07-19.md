# BLUEPRINT — Item 59: Agent-Turn Timing Closure (gaps G1 + G2 + G12)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


- **Date:** 2026-07-19 · **Tier:** code (roadmap §K, item 59) · **Status:** BLUEPRINT (planning
  artifact, no code).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §K item 59
  (lines 1023–1035); `AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` (gaps G1/G2/G12);
  `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md` (binding once item 57 ratifies); item 58
  blueprint (`BLUEPRINT-ITEMS-57-58-…`); ground-truth code: `kernel/src/ports/llm.rs`,
  `llm-adapters/src/dispatch.rs`, `agent-loop/src/main.rs`, `kernel/src/agent/loop.rs`.
- **Prerequisites:** **item 58** (the work/cost pair schema + `WorkloadKind::TokensGenerated`). This
  is the highest-leverage single gap: tokens are already counted; wall-clock is one `Instant` pair
  away.

---

## 1. Scope & goal

**Goal.** Close the agent-turn timing blind spot. Tokens are already the pervasive currency
(`Usage::total_tokens`), but **wall-clock latency is dropped on the floor** on the one path the
production agent loop actually drives. Add a `(work: TokensGenerated, cost: Δwall ⊕ Δticks)` pair to
each turn so `tokens/sec` is a consumer-derivable ratio — never fabricated, never a bare `0` on an
LLM-absent turn.

**Non-goals.**
- NOT a change to the deterministic kernel core (the LLM port is a seam; latency is P3 data).
- NOT a new dependency (std `Instant` on native; procedure step 9's wasm leg named for any wasm
  surface — the agent loop is a host binary, not wasm, so wasm is a stated N/A here).
- NOT a ratio field in the record (item 58 law: raw pair, ratio is consumer-side).

## 2. Current-state grounding

### 2a. The port cannot transport latency (gap G1)

- `kernel/src/ports/llm.rs:110–119` — `ChatResponse { content, usage, tool_calls }`. There is **no
  duration/TTFT field**. `:374` `fn chat(&self, req) -> Result<ChatResponse, LlmError>`. Even where
  an adapter *measures* latency, the port contract cannot carry it back to the kernel.

### 2b. The production loop bypasses the one timed path (gap G2 — the headline)

- `llm-adapters/src/dispatch.rs:37,44–47` — `TrackRecord` **already has** a `ms: u64` field
  ("Wall-clock latency of the (blocking) call in ms"), and `:148–150` the `Dispatcher` times the call
  with `Instant::now()` / `elapsed()`. So exactly **one** path in the repo measures per-call latency.
- `agent-loop/src/main.rs:23` — the production agent loop constructs `OllamaAdapter::new(&base_url)`
  and drives it **directly**, *bypassing* the `Dispatcher`. `:39,88–89` it folds only `total_tokens`;
  `:63` `emit("agent_loop", success, total_tokens, total_tokens as f64)` writes a `TrackRecord`
  **without a `ms`** (the field defaults / is not populated on this path). Result: the production
  agent turn records tokens but **no duration** — the one timed path is the one it does not use.
- The shared ledger is `track_record.jsonl` (`dispatch.rs:10,189–194`) — both the `Dispatcher` and
  `agent-loop`'s `append_harvest` write the *same row schema* ("one channel, no schema drift").

### 2c. The kernel executor has no per-iteration timing (gap G12)

- `kernel/src/agent/loop.rs:40` — `MAX_AGENT_ITERATIONS = 8` (bounded). `:44–60` `LoopLogEntry {
  iteration, event }` with `LoopEventKind::ModelReply { content, total_tokens }` — **tokens, no
  timing**. The executor logs the *sequence* of steps but not their durations.

## 3. Implementation plan (numbered)

1. **(a) Port surface — additive latency field.** Extend `ChatResponse` (`ports/llm.rs:112`) with an
   **additive typed timing field** — either a `duration_us: Option<u64>` (total call time) and/or a
   TTFT companion, or a small `Timing { total_us, ttft_us: Option<u64> }` sub-struct. Additive: every
   existing `ChatResponse` constructor keeps compiling (the field defaults to `None` / an absence).
   The adapter that *does* measure (e.g. Ollama) populates it; adapters that cannot leave it `None`
   (a named absence, never a fabricated `0`). **Plane note:** this is P3 forensic data — it must not
   feed `Usage::cost` or any budget/decision (grep-firewall proof, procedure step 7).
2. **(b) `agent-loop` host binary times each turn.** In `agent-loop/src/main.rs`, bracket each turn
   with `Instant::now()`/`elapsed()` (parity with `dispatch.rs:148–150`) and fold per-turn **Δwall +
   Δticks** alongside the existing `total_tokens` into the harvest row. Populate the `TrackRecord.ms`
   field that already exists (`dispatch.rs:47`) on the direct-adapter path — closing the exact "one
   channel, no schema drift" gap. Optionally also record `Δticks` (CPU ticks via the `typed_metrics`
   reader) for a Tier-T/C pair beyond wall-clock.
3. **(c) Kernel executor per-iteration timing at span granularity.** In `kernel/src/agent/loop.rs`,
   record per-iteration timing via the FDR span machinery (`fdr::info_span!` — the SpanGuard already
   times via `Instant` and is wasm-safe/gated, `fdr/mod.rs:216–251`), so each loop iteration closes a
   span whose `SpanClose` FDR record can carry the item-58 `(work: TokensGenerated, cost: Δwall)`
   pair. The executor stays deterministic and bounded (`MAX_AGENT_ITERATIONS`); timing is P3 data on
   the span record, never a loop-control input.
4. **Workload-kind = `TokensGenerated`** on all three surfaces; the `(work, cost)` pair is item 58's
   schema. `tokens/sec` is derived consumer-side from the raw pair.
5. **LLM-absent turn = named absence.** A turn where the backend is unavailable (`LlmError`) records
   the *reason* (a named absence on the timing/work fields), **never a fabricated `0` duration or `0`
   tokens** (procedure step 3). This is the falsifiable honesty property.

## 4. Required tests / proofs (CHECKLIST.md 5-point standard)

| Checklist item | Disposition for item 59 |
|---|---|
| 1. **Oracle** | A **live loop run** yields track-record entries carrying **both tokens and duration** for the direct-adapter path (parity with the `Dispatcher` path's existing `ms`); `tokens/sec` derivable consumer-side from one record's raw pair. The `TrackRecord` schema round-trip is the oracle (one channel, no drift). |
| 2. **Dudect** | **N/A** — LLM latency is inherently variable and public (network/model time); no secret-dependent branch. The plane-firewall proof (below) is the relevant guard, not a CT gate. |
| 3. **Debug cross-check** | **N/A** — no per-call arithmetic reference; timing is a measured value. |
| 4. **ASM spot-check** | **N/A** — no branch-free hot path. |
| 5. **Kani/formal** | **N/A** — the property is "the pair is recorded, absence is named," an oracle concern. |

**Plane-firewall proof (procedure step 7, mandatory):** grep proof that the new `duration_us`/timing
field never feeds `Usage::cost`, the `TokenBucket` budget, or any decision — latency is recorded,
never a control input. (The budget stays token-priced: `dispatch.rs:142` `cost = max_tokens`.)

**Additive-field proof:** existing golden/track-record consumers unbroken — the `ms` field already
exists in the schema, so populating it on a second path is strictly additive; every existing
`ChatResponse` construction still compiles with the new `Option`-defaulted timing field.

## 5. Falsifiable acceptance criteria

1. A live `agent-loop` run produces a `track_record.jsonl` row with **both** `tokens` and `ms`
   populated on the direct-`OllamaAdapter` path (today `ms` is absent/zero there).
2. `tokens/sec` is computable from one record's raw `(tokens, ms)` pair — no ratio stored.
3. An LLM-absent turn records a **named absence** on timing/work, not a fabricated `0`.
4. The kernel executor (`agent/loop.rs`) emits a per-iteration `SpanClose` FDR record carrying the
   `(TokensGenerated, Δwall)` pair under item 58's schema.
5. Grep-firewall proof green: no timing value reaches `Usage::cost`/`TokenBucket`/any decision.
6. Every existing `ChatResponse` constructor and every existing track-record consumer compiles/passes
   unchanged (additive discipline).

**Falsifier:** a fabricated `0` duration on an LLM-absent turn; a timing value influencing the budget;
a broken existing track-record consumer; the direct-adapter path still dropping latency.

## 6. Dependency gates

- **Upstream:** **item 58** (the `(work, cost)` pair schema + `WorkloadKind::TokensGenerated`) — the
  kernel-executor span record (step 3) writes item 58's pair, so it cannot be *complete* before 58.
  The `agent-loop`/`ports/llm` halves (steps 1–2) can begin against the existing `TrackRecord.ms`
  field, but their `WorkloadKind` tagging aligns to item 58.
- **Transitively:** item 58 → item 57 (procedure binding) → the FDR merge. Item 59 inherits both.
- **Crate-boundary note (build model):** the three surfaces live in **three standalone crates** —
  `kernel/` (`ports/llm.rs`, `agent/loop.rs`), `llm-adapters/` (`dispatch.rs`), `agent-loop/`
  (`main.rs`). Per the repo build model there is no workspace: each is built with its own
  `cd <crate> && cargo test`. The `ChatResponse` field change (step 1) ripples across all three via
  path-deps — land the port change first (kernel), then the adapter/loop consumers, each
  `cd`-verified.
- **Downstream:** none — item 59 is a leaf consumer of item 58.

## 7. Operator-decision points & accepted risks

- **[NOTE, not operator] TTFT vs total-only.** Whether to add time-to-first-token (streaming) or
  total-call latency only is an implementation call — the port surface should be shaped so TTFT can
  be added later without a breaking change (a `Timing` sub-struct with an optional `ttft_us` is the
  extensible shape). Not an operator decision; recorded so the field is not shaped to preclude TTFT.
- **[ACCEPTED] Adapter-measured, port-carried.** The kernel port merely *transports* a value the
  adapter measures; the kernel takes no clock in the deterministic core. This keeps MANIFESTO C2
  (no clock in the decision path) intact — the agent loop is not the decision path. **Owner:** arc
  lead.
- **[ACCEPTED] Cross-crate ripple.** The additive `ChatResponse` field touches three crates; because
  it defaults to a named absence, no existing caller breaks. The cost is three `cd`-scoped builds,
  not a workspace rebuild. **Owner:** executor.
