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

## Extended Context

Of the seven blueprints raised this session, **this is the one with the sharpest self-certification
risk, and it is not close.** The other six extend a computed object (a Laplacian, a CLT envelope, a
lineage seam) whose correctness is external to the extending agent. This one is an agent proposing
changes to *its own operating parameters* — the token budget, cache policy, and model routing that
govern how the same agent thinks. When the proposer and the validator are one session, the loop's
GREEN is a statement the loop makes about itself, and "validation" collapses into decoration (RC-2, the
audit's exact finding shape for the hermes done-gate #2 and the unfed `FalseClaimMeter` #7). This is
precisely why the phase split is not a convenience but the load-bearing safety structure: **Phase A is
advisory-only** — it proposes and signals, a human applies, so a self-run measurement can never
silently *become* applied state — and **Phase B (auto-apply) is hard-blocked on P06's `key_V`**
independent re-execution path, which today has zero code hits outside docs, lives cross-repo on the
bebop2 PQ substrate, and is itself blocked on Phase 3 closing the C4b `mod_l` side-channel. Auto-apply
is not "later"; it is structurally unbuildable until an identity that the proposer cannot be re-runs the
frozen suite and signs the verdict.

That discipline was reached here **independently of, and before, the agentic-mesh-protocol arc** —
whose entire design rests on the same refusal, expressed in mesh vocabulary as the rejection of *inline
self-auditing* (a node cannot vouch for its own state; trust must be a signed capability re-checked by a
distinct party, never a self-asserted score). The two arcs converged on the identical rule from opposite
starting points — one from a single Kalman scalar's propose/guard/accept loop, the other from mesh
trust topology — and that convergence is worth naming as **evidence the discipline is structural, not
stylistic**: two independent derivations landing on "the validator must be a party the proposer cannot
be" is a much stronger signal than either alone. The dependency now runs in reverse, too — the mesh
arc's `AgentManifest` config-axes design (B1) explicitly follows *this* blueprint's Phase-A
proposal-lattice shape (single-axis, bounded, enumerable, DATA-not-free-form), so the pattern minted
here is already load-bearing outside its own arc.

What breaks without this blueprint is concrete and current: **the LLM harness (`llm-adapters/`) has
zero tuning loop.** Every config choice — `workers`, `capacity`, `refill_rate`, `cache`, `max_tokens`,
`top_p`, `temperature`, `cache_policy`, model-per-`TaskClass` — is a static default set once in
`StackBuilder`/`ChatRequest` at deployment and never revisited against real usage. The
`track_record.jsonl` harvest ledger already streams the exact signal that would say *which* knob is
weak (per-model success-rate, token cost, EV), and the `EvalRow` trace log is a durable record with no
in-kernel reader at all. The data to improve the harness flows today and is thrown away. This blueprint
is the reader that turns it into a proposal — advisory, single-axis, human-gated — instead of a
never-consulted log.

---

## Definition of Done

This DoD is **distinct from and additional to** the §4 acceptance criteria (which test the loop's
behaviour on fixtures). It governs *when the work may be called finished* and, more importantly, the
hard boundary between the two phases.

**The Phase A / Phase B split is a HARD DoD boundary, stated without euphemism:**

1. **Phase A is independently completable.** All seven §4 acceptance criteria pass; the module header is
   stamped *advisory-only; auto-apply gated on P06 `key_V`; canonical-repo DEV-TIME scope (M5/M6)*; a
   grep for any apply-call from the loop returns nothing (criterion #6). When these hold, **Phase A is
   DONE** — it does not wait on Phase B, on `key_V`, or on anything cross-repo. Phase A shipping is a
   real, closeable deliverable.
2. **Phase B cannot be STARTED — not merely "is not yet done" — until `key_V` exists.** This is the
   distinction that matters: there is no partial Phase B, no scaffolding-ahead, no "wire the apply path
   now and gate it behind a flag." An apply path built before `key_V` is RC-2 self-certification the
   moment it can fire, regardless of the flag's default. The DoD for *beginning* Phase B design is a
   single external precondition: `BLUEPRINT-P06`'s `key_V` independent re-execution path exists in code
   (fresh worktree, `key_K ≠ key_V`, signed verdict), which today it does not (zero code hits outside
   docs). Until then, the only legal Phase-B artifact is the named precondition itself. **Attempting
   Phase B before `key_V` is a scope violation, not early progress.**

**Re-verification ledger (live, this pass, against the CURRENT worktree state).** Because the
harness/LLM-backend work merged since this blueprint was first written, the config surface was
re-verified line-by-line. Result:

3. **The proposal-lattice axis list is accurate and needs no correction.** All five named surfaces are
   unchanged from the blueprint's §1/§2 description: `StackBuilder` (`compose.rs:75-82`: `workers` def 2,
   `capacity` def 64, `refill_rate` def 8.0, `cache` def true — `workers` clamps `n.max(1)`, which the
   `{1,2}` domain already respects); `ChatRequest` (`ports/llm.rs:43-75`: `temperature` 0.0, `top_p` 1.0,
   `max_tokens` 1024, `seed: Option<u64>`, `task_class`, `cache_policy`, `options`); `CachePolicy`
   (`:78-87`: `Exact`/`SemanticOk`/`NoCache`, with the `:52-55` doc-contract that gate-critical callers
   MUST NOT use `SemanticOk` — the lattice's exclusion of `SemanticOk` is still structurally justified);
   `TaskClass` (`:28-33`: `Code`/`General`/`Embedding`); and `route_model` (`ollama.rs:36-45`:
   Code→`qwen2.5-coder:7b`, General→`llama3.1:8b`, Embedding→`nomic-embed-text`, request `model_id`
   override). **Every lattice axis and domain in §2's table maps to a live knob.**
4. **The no-retry correction still holds.** `quirks.rs` (`:11-28`) contains only wire-correctness deltas
   (`pass_tag_ids_verbatim`, `strip_sentinel_fingerprint`, `embeddings_path`,
   `native_embeddings_path`, `surface_options`, `extra_headers`) — no retry/backoff field.
   `dispatch.rs` remains degrade-closed with a typed `BudgetExceeded` and no retry anywhere in the file.
   The research sketch's invented "retry policy in `quirks.rs`" is still correctly excluded from the
   lattice; do not re-introduce it.
5. **One additive delta found — recorded, not a contradiction of §1.** The `Dispatcher`'s harvest row
   grew (commit `c7f53c689`, "consumer wiring + EV-loop close"). `track_record.jsonl` now emits the
   gov_route-compatible **superset** `{model, task, success, value, cost, backend, tokens, ms}`
   (`dispatch.rs::append_harvest` `:135-148`) — the §1 field list `(model, task, success, tokens, ms,
   cost)` is still present and correct but **incomplete**: it omits `value` (the EV numerator, default
   0.0, supplied by the caller/agent loop) and `backend`. More importantly, `track_record.jsonl` **now
   has a native in-process Rust reader** it did not have when §1 was written: `decode_track_record`
   (`dispatch.rs:156`) plus the `Telemetry::from_ledger` fold (`telemetry.rs:73`), which already
   computes per-model `dispatches`/`successes`/`success_rate`/`mean_tokens`/`total_value` — exactly the
   weakness signal §2(i)'s reducer needs. **DoD consequence:** the Phase-A weakness-mining reducer MUST
   reuse `decode_track_record`/`Telemetry` rather than re-parse the ledger (one schema, no drift — the
   telemetry module's own stated discipline), and it SHOULD consume the `value` field as a weakness
   signal, not just `success`/`tokens`. This enrichment strengthens §2(i)'s "bootstrap on live data"
   claim; it does not weaken any existing statement, so §1's prose is left intact and the correction
   lives here.
6. **`EvalRow` still has zero in-kernel Rust readers** (confirmed live: only tests `:955`/`:997`/`:1012`
   and the `lib.rs:154` re-export touch it; the external `analyze.mjs` remains the sole non-test
   consumer). The blueprint's framing — that giving `EvalRow` its first reader is Phase A's first job —
   is unchanged and still accurate.

Phase A is DONE when items 1, 3–6 hold and §4 passes. Phase B's DoD does not begin to accrue until
`key_V` lands.

---

## Event-Driven Architecture Treatment

This blueprint is genuinely event-adjacent: `EvalRow::append_to` (`evals.rs:489`) is already a durable,
append-only, fail-closed record, and the harvest ledger is a second append-only stream. The question
is whether a **Phase-A proposal's full lifecycle** should be recorded as a sequence of typed events in
its own durable log, so a future audit can reconstruct *why a config change was proposed and what
evidence backed it* rather than seeing only the final report and having to trust it. **It should.** The
lifecycle is a natural event sequence:

```
ProposalEvent =
  | WeaknessMined      { bucket, fail_rate, source_ledger, rank }
  | ConfigProposed     { axis, from, to, motivating_bucket }
  | ValidatedFrozen    { suite_hash, pass_rate_before, pass_rate_after, gate_verdict }
  | Tagged             { recommend | flag_regression, delta }
  | HumanDisposition   { applied | rejected, operator_note }   // appended out-of-band by the human
```

Recording these as typed rows makes the propose/validate/reject cycle **auditable rather than
ephemeral** — the exact Ananke/DoD goal: someone six months later can ask "why is `cache_policy` set to
`NoCache` for Code?" and replay the `WeaknessMined → ConfigProposed → ValidatedFrozen → Tagged →
HumanDisposition` chain that produced it, with the frozen-suite hash proving *which* suite scored it.
The `HumanDisposition` event closes the loop the report alone cannot: it captures that a human actually
saw and decided, which is the whole point of advisory posture.

**Design call: a simpler local append-log (the `EvalRow` shape), NOT `kernel/src/event_log.rs`'s
primitives directly.** The reasoning is specific, not aesthetic. `MeshEvent`
(`event_log.rs:134`) is content-addressed over `(prev, actor_pubkey, actor_seq, payload)` and
`EventLog::append` is built for a *mesh-shared, multi-actor, adversarial-trust* substrate — its
identity field is a real signing key, and its whole value is that a distinct party can verify a
node's claims. A Phase-A harness-tuning log is the opposite: **local, single-actor, dev-time
(M5/M6 canonical-repo DEV-TIME scope), not mesh-shared state.** Forcing it through `MeshEvent` means
either minting a fake `actor_pubkey` — which is *dishonest*, inventing a signing identity for a log
that has none — or dragging the PQ-signing substrate into dev-tooling that has no trust boundary to
defend. That is exactly the anti-pattern the mesh arc and this blueprint jointly reject. The right
weight is `EvalRow`'s: a typed enum serialized to a local JSONL via the same fail-closed `append_to`
appender, offline, deterministic, zero-dep.

What *should* be borrowed from `event_log.rs` is the **pattern, not the code**: (a) append-only +
fail-closed persistence (already `EvalRow`'s), and (b) optionally a lightweight **prev-line hash chain**
using the kernel's existing SHA3 content-address helper (`event_log.rs:28`, `:146`) so the proposal log
is *tamper-evident* — an auditor can detect a rewritten history — **without** an identity. That is a
zero-cost integrity win that needs no signing key. Crucially, this leaves a clean **forward seam for
Phase B**: when `key_V` eventually exists, the *validated-verdict* event is the one — and the only one —
that graduates to full `MeshEvent`/signed semantics, because at that point an independent identity
really is signing a real claim. In other words, Phase A's log is a local append-log by design, and the
single event that becomes mesh-trust-relevant is exactly the event Phase B introduces. The architecture
does not need to be retrofitted; it needs one event type promoted when its second party arrives.

---

## Long-Term Consequences, Safety, Scalability

**(a) Scalability of the single-axis discipline.** "Perturb exactly one axis by one step from the last
accepted config" is, in optimization terms, **coordinate descent** — and the honest statement is that it
converges to a good configuration *only under assumptions that weaken as the lattice grows*. Today's
nine axes with 2–3 values each give a small neighbourhood where near-exhaustive single-axis A/B is
cheap and the proven scalar-loop shape (`propose_step`'s single Adam step) transfers cleanly. But two
real scaling limits are foreseeable and should be named rather than papered over: **(i) iteration count
grows at least linearly** with axis count — each accepted move re-opens the full axis fan, so a lattice
of N axes needs O(N) validations per improvement round, and each validation re-runs the frozen suite;
**(ii) coordinate descent cannot see axis interactions** — if the *joint* move `(max_tokens↑,
temperature↑)` helps but neither single step does, single-axis search is blind to it and can stall in a
coordinate-wise local optimum that is not a joint optimum. So the answer to "does single-axis-at-a-time
scale indefinitely?" is **no, and this blueprint should not claim it does.** It is the correct *starting*
strategy — minimal, provably-shaped like the scalar loop, near-exhaustively validatable — and the
honest future-scaling question is: *when the axis count crosses roughly a dozen, or when suspected axis
interactions appear in the weakness data, a smarter proposal strategy will be needed* (candidates,
named for the future implementer, not adopted now: grouping correlated axes into joint moves, a
screening/fractional-factorial design to find the interacting subset cheaply, or a small surrogate model
over the lattice). This is flagged as a real future-work trigger, not solved here.

**(b) Safety — the worst case for Phase A specifically.** The dangerous case is not a bug in the
propose/validate loop; it is a proposal that **passes the frozen validation suite and is nonetheless
bad in a way the suite does not cover** — the suite's own blind spot. Example: raising `max_tokens`
lifts the metamorphic pass-rate (longer answers satisfy more oracles) while quietly tripling token cost
and latency in production, which the offline suite never measures. Because Phase A **never auto-applies**,
the last line of defense is the human reviewer — so the safety question reduces to: *what does the human
actually SEE?* If the report surfaces only a `recommend` / `flag-regression` **pass/fail tag, that is a
real gap in the human-in-the-loop safety story** — a human rubber-stamping a green tag catches nothing
the suite missed, and the "human as second party" mitigation (§5) degrades to theatre. The design must
therefore surface the **full evidence trail on every recommendation**, not a verdict: (1) the
weakness-mining data that motivated the axis (which bucket, its fail-rate, its rank, from which ledger);
(2) the before/after **frozen-suite scores** with the measured delta, not just "improved"; (3) the exact
`(axis, from, to)` diff and the deterministic mapping-table rule that selected it; and — tying in E2 —
(4) the **confidence interval** on the pass-rate delta (a +2% move whose CI straddles zero is not the
same evidence as a +2% move with a tight CI). Surfacing (1)–(4) is what lets a human catch a
suite-blind-spot regression *that the suite itself declared green*. **Recommendation: promote "full
evidence trail per proposal" from an implied nicety to an explicit Phase-A acceptance requirement** — it
is the only thing that makes the human a real reviewer rather than a green-light presser, and it is the
concrete fix for the gap this walk-through exposes.

**(c) Ethics / long-term — the complacency failure mode.** §5 already names this blueprint's honest
residual limitation (the validator's *identity*, not just its verdict, is what Phase A cannot supply).
The long-term risk is that the operator **forgets** this and starts treating a running string of
Phase-A `recommend` flags as authoritative — applying them reflexively without reading the evidence
trail from (b), which quietly re-creates the exact self-certification the phase split exists to prevent,
just with a human as the silent conduit. A one-time caveat in a doc header does not defend against this;
memory decays and headers go unread. The structural nudge is to make the warning **impossible to
forget without actively ignoring it**: every single recommendation the loop emits — every row of the
report, not one banner at the top — carries an inline marker, e.g. `⚠ NOT independently verified —
advisory only; auto-trust requires P06 key_V (absent)`. The property that matters is *repetition at the
point of decision*: complacency then requires the operator to ignore the same warning on every proposal
they act on, which is a conscious, visible choice, rather than the passive drift of forgetting a caveat
they read once. This is the same "friction, not a gate" philosophy the repo already applies to advisory
signals (the Markov-attractor loop-signals precedent) — the loop cannot and must not *stop* a human from
applying a change, but it can make sure the human is told, every time, exactly how far short of
independent verification the recommendation stands. When `key_V` lands, that marker is the line that
flips to `✓ independently re-executed (key_V)` — and only then.

---

*Blueprint E3 complete. Scope: cluster 4 (Self-Harness) at harness-config scale. Reuses the
`evals.rs` propose/guard/accept/rollback machinery (`:545-764`) and the `llm-adapters/` config surface
(`compose.rs`/`ports/llm.rs`/`ollama.rs`) verified live on `feat/spectral-energy-flow-evolution`
2026-07-16. Phase A is advisory-only and buildable now; Phase B (auto-apply) hard-depends on P06
`key_V` and is not designed here. No code written by this document.*
