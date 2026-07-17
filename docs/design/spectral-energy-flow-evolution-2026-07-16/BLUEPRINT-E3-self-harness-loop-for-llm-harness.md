# BLUEPRINT — E3: Self-Harness loop lifted from one Kalman scalar to the LLM harness config

> **Anchors:** cluster 4 (Self-Harness, arXiv:2606.09498), RESEARCH-AND-REASONING §3.3. **Extends:**
> the E2/E3 self-adaptation machinery in `kernel/src/evals.rs` (propose→guard→accept→rollback) — from
> tuning ONE scalar (the Kalman `Q`-scaler) to tuning the actual LLM harness config in `llm-adapters/`.
> **This is a genuinely NEW capability, not a Hermetic-audit-finding fix** — the audit's V-1/V-2/RC-2
> findings are about the hermes-kernel *done-gate* and `FalseClaimMeter`; this blueprint builds a
> harness-tuning loop that does not exist today. It *inherits* RC-2 as a hard constraint and P06's
> `key_V` as a precondition, but it fixes no existing finding.
> **Depends on — phase split, stated up front:**
> - **Phase A (advisory):** NO hard dependency. It reads an existing trace log, re-runs an existing
>   frozen eval suite + regression gate, all in-kernel, offline, zero-dep. Buildable now.
> - **Phase B (auto-apply):** HARD-depends on `BLUEPRINT-P06` `key_V` independent re-execution
>   (today: zero code hits outside docs, cross-repo, itself blocked on Phase 3's C4b). Not designed
>   here beyond naming the precondition.
> Canon: `ARCHITECTURE.md` §0 (SCOPE RULE, M5/M6), `HERMETIC-ARCHITECTURE-PRINCIPLES.md` §P7/RC-2.
>
> **Planning artifact only. No code is written or edited by this document.**

---

## 0. The problem this blueprint solves (one paragraph, no metaphor)

The Self-Harness paper's loop — Weakness Mining → Harness Proposal → Non-Regressive Validation — is
the exact three-stage discipline already implemented in `kernel/src/evals.rs`, but operating on a
single scalar: a Kalman process-noise `Q`-scaler. `SelfAdaptator::propose_step` proposes a change
without mutating kernel state; a Noether stability guard plus `RegressionGate` accept or reject it;
a rejected step rolls back to the last accepted value. Meanwhile the harness this worktree's parent
branch built (`llm-adapters/`: `Dispatcher`, `CachingBackend`, `OllamaAdapter`) exposes a real config
surface — token budget, cache on/off, sampling params, model-per-`TaskClass` routing — that is
tuned by hand and never by that loop, and the execution-trace log (`EvalRow::append_to`) that would
tell the loop *which* knob is weak has zero readers. The extension is to lift the existing
propose/guard/accept/rollback discipline from the one scalar to the enumerable harness knobs. The
single thing that must be gotten right: **if the same session both proposes a harness change and
validates it against the frozen eval suite, "validation" is decorative** — this is RC-2
self-certification (the audit's own finding shape for the hermes done-gate #2 and the unfed
`FalseClaimMeter` #7). The audit's structural fix for that class is the P06 `key_V` independent
re-execution path, which **does not exist** (zero code hits outside docs; cross-repo; blocked on an
open crypto side-channel). **Therefore this loop ships advisory-only — propose + signal, never
auto-apply — until `key_V` exists. Auto-apply requires `key_V`, full stop.**

---

## 1. Current-state evidence (what exists, verified live; this phase reuses, does not rebuild)

**The three-stage embryo — all in `kernel/src/evals.rs`, re-read for this blueprint:**

- **Stage 1 · trace log.** `EvalRow` struct (`:441-451`); `to_jsonl` (`:464`, `run-history.jsonl`
  schema `analyze.mjs` already parses); `append_to(path)` (`:489-498`, fail-closed appender). It has
  **zero in-kernel Rust readers** — only tests and the external Node `analyze.mjs` touch it (confirmed
  by the decorrelated verification pass; the method is `append_to`/`to_jsonl`, **not** `append_jsonl`).
  This is an appender-only half-pendulum: the loop's first job is to give it a reader.
- **Stage 2 · propose-never-mutate.** `SelfAdaptator::propose_step` (`:707-748`) mutates **only the
  adapter's own optimizer internals** (`self.opt` Adam θ, `last_loss`, `steps`, `accepted_theta`) and
  returns a candidate scaler; I confirmed by reading the body that it **never touches a
  `KalmanFilter`**. `apply_step` (`:752-755`) is the **sole** mutator of real kernel state
  (`kf.set_q_scaler(s)`). On guard rejection it rolls the optimizer back to `accepted_theta` (`:741`).
  The propose/guard/apply/rollback separation is genuine, not prose.
- **Stage 3 · non-regressive acceptance.** `RegressionGate` (`:545-623`): `observe` (`:570-597`)
  pushes each sample through an EMA and returns RED only when the monotonic-degradation streak
  `>= window - 1` (`:596`). Tests pin it (`:887-930`): RED on 3 consecutive rises > tol, green on
  oscillation within tol, clears on trend reversal.
- **Leakage gate (the anti-contamination organ).** `MintLog::mint` (`:91-111`) rejects an exact
  duplicate `(kind, payload)` via a 128-bit FNV key; `MetamorphicGenerator::mint_semantic` (`:159-173`)
  adds the Layer-B cosine near-duplicate check. Tests `:805-847` prove duplicates are rejected. This
  is what stops a proposer from contaminating the eval set it is validated against.
- **The red-line the design must not cross.** The self-mod scope comment (`:650-656`, read verbatim):
  `propose_step` NEVER mutates the filter; `apply_step` applies only an ACCEPTED `s`; the optimizers
  are "mutated locally (deterministic, offline, no network) — this is the authorized E3 scope, **not a
  parametric rewrite of unrelated kernel organs.**" Lifting to harness config is a scope *expansion*;
  it stays legal only while it remains data-only, deterministic, and offline (Phase A satisfies this;
  Phase B's live application does not, which is a second reason it is gated).

**The actual harness-config surface (verified in `llm-adapters/` + `kernel/src/ports/llm.rs`):**

- `StackBuilder` (`compose.rs:76-145`): `workers` (default 2, ≤ Ollama parallelism cap), `capacity`
  (token-bucket burst, 64), `refill_rate` (tokens/s, 8.0), `cache` (bool, on), `base` URL.
- `ChatRequest` (`ports/llm.rs:43-75`): `temperature` (0.0), `top_p` (1.0), `max_tokens` (1024),
  `seed: Option<u64>`, `task_class` (`TaskClass::{Code,General,Embedding}`, `:28-33`), `cache_policy`
  (`CachePolicy::{Exact,SemanticOk,NoCache}`, `:79-87`), `options` (Ollama `num_ctx`/`keep_alive`/`think`).
- Model routing: `OllamaAdapter::route_model` (`ollama.rs:36-45`) maps `TaskClass` → concrete
  `model_id` (Code→`qwen2.5-coder:7b`, General→`llama3.1:8b`, Embedding→`nomic-embed-text`),
  overridable by an explicit request `model_id`.
- `Dispatcher` (`dispatch.rs:66-70`, `:135`) already writes a per-call `track_record.jsonl` row
  (`model`, `task`, `success`, `tokens`, `ms`, `cost`) on **every real (non-test) call** — a live
  weakness-signal that flows today, unlike `EvalRow`.
- **Correction to the research sketch (do not propagate):** there is **no retry/backoff policy**
  anywhere in the harness (grep over `llm-adapters/src` + `kernel/src/ports` = clean; `dispatch.rs` is
  degrade-closed with a typed `BudgetExceeded`, no retry). `quirks.rs` (`:11-28`) holds only
  wire-correctness deltas. So "retry policy in `quirks.rs`" is not a knob that exists; the lattice
  below does not invent one, and `Quirks` is excluded (changing it breaks the wire protocol, it is
  not tuning).

---

## 2. Target-state design

### Phase A — advisory, buildable now (propose + signal, human decides to apply)

Three pieces, each mapped to named code, all offline/deterministic/zero-dep.

**(i) Weakness-mining reducer.** A pure fold that gives the appender-only trace its first reader.
Primary input: `EvalRow` `run-history.jsonl` (`checks[]` outcomes); it groups failures by
`(check.name, category, model, config_version)`, counts fail-rate per bucket, ranks by fail-count.
Secondary input available immediately: the `Dispatcher`'s `track_record.jsonl` (`success`/`ms`/`tokens`
per `(model, task)`), which already flows from live calls — so the reducer can bootstrap on real data
before eval runs are wired to emit `EvalRow`. Output: a ranked list of weakness buckets. Pure function,
no fs writes inside the kernel (the caller supplies the bytes), matching the `RegressionGate`/`EmaTracker`
purity discipline.

**(ii) Harness-proposal lattice — proposals are DATA, single-axis, enumerable.** A proposal is a
`HarnessConfig` value plus a diff descriptor `(axis, from, to)` — never a free-form or LLM-authored
config/scaffold diff (the research doc's own open question flags free-form as re-introducing
self-certification at maximum strength, and it would violate the `:650-656` red-line). **Lattice shape:
a Cartesian space of small discrete axes, but each proposal perturbs exactly ONE axis by one step from
the last accepted config** — the direct analogue of `propose_step`'s single Adam step on one θ. This
keeps the candidate neighbourhood tiny and validation near-exhaustive. The axes and their *fixed,
bounded* domains:

| Axis | Domain (discrete, bounded) | Source knob |
|------|----------------------------|-------------|
| `cache` | {on, off} | `StackBuilder.cache` |
| `capacity` | {32, 64, 128} | `StackBuilder.capacity` |
| `refill_rate` | {4.0, 8.0, 16.0} | `StackBuilder.refill_rate` |
| `workers` | {1, 2} (≤ Ollama cap) | `StackBuilder.workers` |
| `max_tokens` | {512, 1024, 2048} | `ChatRequest.max_tokens` |
| `top_p` | {0.9, 1.0} | `ChatRequest.top_p` |
| `temperature` | {0.0, 0.2} — any value > 0 MUST carry a pinned `seed` | `ChatRequest.temperature`/`seed` |
| `cache_policy` per class | {Exact, NoCache} — **SemanticOk excluded** | `ChatRequest.cache_policy` |
| model per `TaskClass` | a per-class **allowlist** only (e.g. Code ∈ {qwen2.5-coder:7b, llama3.1:8b}) | `route_model` / request `model_id` |

**Justification of scope.** (a) Single-axis moves keep the loop's proposal identical in shape to the
proven scalar loop and make each validation a clean A/B. (b) `SemanticOk` is excluded because the
`CachePolicy` type already forbids it for gate-critical callers (`ports/llm.rs:52-55`); the lattice
respects that structural boundary rather than tunnelling around it. (c) `temperature > 0` is admitted
only with a pinned `seed` so validation stays byte-reproducible (P6 determinism) — an un-seeded
temperature move is rejected at proposal time. (d) Model routing is an **allowlist**, never an
arbitrary `model_id` string (unbounded input = unsafe). (e) `Quirks` and retry are out of scope
(correctness-critical / nonexistent). A deterministic mapping table turns each weakness bucket into a
candidate axis (budget/`BudgetExceeded` fails → raise `capacity`/`refill_rate`; low-quality Code
outputs → the alternate Code model; stale-answer suspicion → `cache` off / `NoCache` for that class).
The table is fixed code, not an LLM.

**(iii) Non-regressive validation + report (never auto-apply).** For each single-axis candidate, re-run
the **frozen, mint-log-pinned** `MetamorphicGenerator` suite (kernel-primitive oracles — no LLM judge,
offline) under the candidate config, feed the pass-rate into `RegressionGate`, and tag the proposal
`recommend` iff pass-rate ≥ the last accepted config's AND the gate stays green; else `flag-regression`.
The output is a **report**: the ranked weakness buckets, each candidate with its measured pass-rate
delta and its tag. A human/operator reads it and decides what (if anything) to apply by hand. The
"last accepted config" is a persisted record the loop **diffs against but never writes** — the
`accepted_theta` analogue, held by the human, not the loop. This is the Markov-attractor loop-signals
precedent exactly: *signals inform, guardrails (here, the human) decide; advisory; fail-open;
zero-dep* (`tools/loop-signals/`, reflection 2026-07-13). Because nothing is auto-applied, the
self-certification failure (a self-run measurement silently becoming applied state) structurally
cannot occur in Phase A.

### Phase B — auto-apply (explicitly gated on `key_V`; NOT designed here)

Phase B would let an accepted proposal be applied automatically, closing the loop the paper describes.
It is **not** a minor upgrade of Phase A: auto-acceptance means a self-run validation authorizes a
state mutation, which is RC-2 self-certification unless the validation is re-executed by an independent
identity. The precondition is `BLUEPRINT-P06`'s `key_V` independent re-execution path (fresh worktree,
`key_K ≠ key_V`, signed verdict). Per P06's own current status that path has **zero code hits outside
docs**, reuses cross-repo bebop2 PQ substrate, and is blocked on Phase 3 closing the C4b `mod_l`
side-channel before any signing runs. Until `key_V` lands and the harness validation is routed through
it, Phase B stays unbuilt. Full design deferred to a future blueprint once `key_V` exists.

---

## 3. Migration steps (Phase A only)

1. **Give the trace log a reader.** Add a pure weakness-mining reducer (new fn in `evals.rs`, edit-not-
   create per CLAUDE.md) that folds `EvalRow` rows (and, for bootstrap, `track_record.jsonl`) into
   ranked weakness buckets. No new dependency; mirror `RegressionGate` purity (caller owns fs).
2. **Emit real `EvalRow` rows.** Wire the harness eval run to call `append_to` with genuine rows (today
   only tests do), so the reducer has live input. This is observation only — no kernel-param mutation.
3. **Define the `HarnessConfig` value + single-axis lattice** as data (the table in §2ii), with the
   deterministic weakness-bucket → axis mapping and the seed/allowlist guards enforced at proposal time.
4. **Wire validation over the frozen suite.** Re-run the mint-log-pinned `MetamorphicGenerator` under a
   candidate config, feed pass-rate into `RegressionGate`, tag `recommend`/`flag-regression`.
5. **Emit the advisory report** (ranked buckets + tagged candidates + deltas) for a human to review.
   No apply path is built. Stamp the module header: *advisory-only; auto-apply gated on P06 `key_V`;
   canonical-repo DEV-TIME scope (M5/M6).*

---

## 4. Acceptance criteria (Phase A — numbered, falsifiable)

1. **A regressing proposal is flagged, never silently accepted.** A single-axis candidate that lowers
   the frozen suite's pass-rate below the accepted config's (or trips `RegressionGate`) is tagged
   `flag-regression` in the report, and NO apply occurs (there is no apply path). Falsifies "claim
   replaces check."
2. **A genuinely improving proposal is surfaced.** A candidate that raises the frozen-suite pass-rate
   with the gate green is tagged `recommend` and appears in the report with its measured delta.
3. **The leakage gate prevents proposer contamination.** An attempt to validate against a suite that
   includes an item the proposer minted post-freeze is rejected by `MintLog`/`mint_semantic`
   duplicate-rejection (`:91-111`, `:159-173`); the validated suite is provably the frozen one.
4. **Single-axis discipline holds.** Every emitted proposal differs from the last accepted config on
   exactly one lattice axis, within its bounded domain; a multi-axis or out-of-domain candidate is
   rejected at proposal time.
5. **Determinism preserved.** Any `temperature > 0` candidate without a pinned `seed` is rejected;
   validation runs are byte-reproducible across two processes.
6. **Zero mutation in Phase A.** No harness config and no kernel parameter is written by the loop; the
   only output is the advisory report. (A grep for an apply-call from the loop returns nothing.)
7. **Trace log has a reader.** The weakness-mining reducer consumes `EvalRow`/`track_record.jsonl` and
   produces a non-empty ranked bucket list on seeded fixture input.

---

## 5. Residual limitation (honest — identity of the validator, not just its verdict)

Phase A does **not** achieve real independent validation. The metamorphic suite has hard kernel-
primitive oracles (not an LLM judge), so its PASS/FAIL is not self-graded in the loose sense — but the
suite is still *selected, frozen, and re-executed by the same session that proposed the change*.
Independence of the **runner** is what is missing — the same gap the audit found in the hermes
done-gate (V-1: "both of its inputs are supplied by the same session it gates"). Phase A's honest
mitigations are three and no more: a mint-log-frozen suite the proposer cannot contaminate, an
advisory posture so a self-run measurement never becomes applied state, and a human reviewing every
flag — a weak but real second party. What closes the gap is already named: P06's `key_V` re-execution
path re-runs the frozen suite under a different identity and signs the verdict, so the *measurement*
itself is produced by a party the proposer cannot be. Until `key_V` exists (zero code hits outside
docs; cross-repo; blocked on C4b), auto-apply stays unbuilt and this blueprint refuses to pretend the
loop's own GREEN authorizes anything — the cheapest honest step from *hand-tuned* toward
*loop-proposed, independently-verified*, named exactly for how far short of real independence it stands.

---

*Blueprint E3 complete. Scope: cluster 4 (Self-Harness) at harness-config scale. Reuses the
`evals.rs` propose/guard/accept/rollback machinery (`:545-764`) and the `llm-adapters/` config surface
(`compose.rs`/`ports/llm.rs`/`ollama.rs`) verified live on `feat/spectral-energy-flow-evolution`
2026-07-16. Phase A is advisory-only and buildable now; Phase B (auto-apply) hard-depends on P06
`key_V` and is not designed here. No code written by this document.*
