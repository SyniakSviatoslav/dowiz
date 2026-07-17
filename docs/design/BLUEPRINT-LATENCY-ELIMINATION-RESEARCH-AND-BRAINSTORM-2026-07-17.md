# BLUEPRINT — Latency Elimination: Research + Brainstorm (2026-07-17)

> **Research/blueprint artifact. No product code written by this document.** Branch:
> `feat/harness-llm-backend`. Built per the Detailed Planning Protocol (`AGENTS.md`): live-measured
> ground truth first, explicit dependencies, inline DECART, falsifiable done-checks, 2-question
> doubt audit, Anu/Ananke check. **Epistemics discipline:** every section is labeled either
> **[MEASURED]** (probe run on this host this session), **[GROUNDED]** (external source, cited,
> with the researching subagent's confidence marker preserved), or **[SPECULATIVE]** (brainstorm,
> §5 — not decided, not DECART'd, honestly assessed for infeasibility).
>
> **Problem (operator, verbatim intent):** agent-dispatch work is bottlenecked by ~10-second
> round-trips to remote LLM APIs; the operator's leading hypothesis was "local AI should fix this,"
> with an explicit invitation to research any alternative, grounded or wild.
> **Operator mid-task ruling (2026-07-17):** *"llm deciding in nanoseconds in core is better idea
> > more caching"* — the LLM-as-one-time-compiler idea is promoted from brainstorm item to the
> **primary recommendation** of this blueprint (§2), designed in depth with its own DECART.

---

## 0. Executive summary

**[MEASURED]** Today's two sessions made **1,000 remote API calls**: latency from user-side entry
to first assistant chunk **p50 = 4.9 s, mean = 10.6 s, p90 = 26.2 s** (the operator's "~10
seconds" is the *mean*, exactly). Prompt caching is **already near-optimal**: 99.3% of all prompt
tokens were served as cache reads (474.7M read vs 3.45M written vs 30K uncached). Average output
per call: **1,232 tokens**. At Claude-class API decode speed (50–90 tok/s, §1.2) that output volume
*is* the round-trip — **the bottleneck is decode volume (thinking + generation), not network, not
prefill, not connection overhead**.

**The local-AI hypothesis inverts under this arithmetic.** This host's local decode is 4.8–10.5
tok/s (measured, LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md §1.2) — 5–15× *slower* than the
remote API. Generating today's average 1,232-token response locally would take **2–4 minutes**, not
10 seconds. Local inference helps only where output is tiny and schema-constrained (the G3 router's
existing territory), or where the network is absent. "More local AI" does not fix this latency;
**"fewer generated tokens" does — and the limit of that is zero tokens: a compiled native decision.**

**Primary recommendation (operator-ruled, §2): the Decision Compiler.** Recurring
*question shapes* — not question instances — get answered by the LLM **once**, as generated,
tested, provenance-stamped native Rust; every later instance of that shape is decided in
nanoseconds in-process with **zero network and zero tokens**. This repo has already proven the
pattern four separate times without naming it (§2.2): `is_redline()`, `Scope::touches_red_line()`,
the hermes `gov_route` EV table, and the `skillspector-rs` build.rs rules-generation pipeline.
Invalidation rides the GapWire event pipeline (§2.4), not a new staleness mechanism.

**Secondary real levers, ranked by measured leverage (§3):** output-token discipline on dispatch
prompts (attacks the dominant 85–90% decode share directly); three prompt-cache deltas that remain
despite the 99.3% rate (wave-dispatch cache-write race, 1h-TTL for inter-wave gaps, doc-edit/
dispatch-wave separation); `effort: low` on doer subagents; task-level draft-local/verify-remote
(the speculative-decoding *pattern* one level up — viable precisely because verification is
prefill-heavy/decode-light for the remote model); Batch API time-shifting for latency-insensitive
lanes. Connection-level optimization is measured to be a ≤1–2 s ceiling out of 10.6 s — real but
minor. Repo-targeted distillation is deferred with a named trigger (weakest evidence per dollar,
§3.5).

---

## 1. Ground truth — measured this session

### 1.1 The latency measurement [MEASURED]

Method: parsed today's two session transcripts
(`~/.claude/projects/-root-dowiz/{a096e731…,72e1c065…}.jsonl`), grouped assistant entries by unique
`message.id` (= one API call), summed `usage`, and took the wall-clock delta from the preceding
user-side entry to the first assistant chunk of each new message id (deltas >300 s discarded as
human-idle gaps). Script: session scratchpad `lat.py` / `hist.py` (throwaway, not repo code).

| Metric | Value |
|---|---|
| Unique API calls (both sessions) | 1,000 |
| Latency, user-entry → first assistant chunk | **p50 4.9 s · mean 10.6 s · p90 26.2 s** |
| Prompt tokens: cache-read / cache-write / uncached | 474,732,391 / 3,450,666 / 30,266 (**99.3% read**) |
| Output tokens, total / avg per call | 1,232,884 / **1,232** |

Output-size distribution (the load-bearing table for §2.6):

| Output bucket | Calls | % calls | Out-tokens | % decode |
|---|---|---|---|---|
| ≤64 tok | 40 | 4% | 1,537 | 0% |
| 65–256 | 283 | 28% | 46,398 | 4% |
| 257–1,024 | 364 | 36% | 194,851 | 16% |
| 1,025–4,096 | 257 | 26% | 524,183 | 42% |
| >4,096 | 59 | 6% | 467,428 | 38% |

Honest metric caveat: the transcript timestamp granularity is "first content block written," not
first token on the wire, so per-call figures overstate true TTFT and understate pure decode time.
The *decomposition* conclusion (decode dominates) is robust to this; the exact split is not.

### 1.2 Where the ~10 seconds actually goes [GROUNDED + MEASURED]

Third-party live benchmark (artificialanalysis.ai, fetched 2026-07-17 — *secondary*, point-in-time):
Claude Haiku 4.5 TTFT ≈ 1.0 s @ 87–90 tok/s; Sonnet 5 TTFT ≈ 1.4 s @ 61–84 tok/s; Opus-class
50–57 tok/s. (No Fable 5 row was captured; Fable runs adaptive thinking always-on, so its effective
visible-output rate is lower still — unmeasured, flagged in §6.) TTFT bundles network + queueing +
prefill; with a 99.3%-cached prefix, prefill is largely already paid.

Decomposition of a representative call (1,232 output tokens @ 60–90 tok/s): **TTFT ≈ 1–1.5 s
(10–15%), decode ≈ 8.5–20 s (85–90%)**. This matches the measured p50/mean/p90 spread — short
turns land near TTFT-only (~2–5 s), long thinking/synthesis turns dominate the tail. Anthropic's
own prompt-caching claim — "up to 90% cost and up to 85% latency reduction for long prompts";
worked example 100K-token prompt 11.5 s → 2.4 s (*official*, claude.com/blog/prompt-caching) — is
a **TTFT** lever, and this session has already banked it.

### 1.3 What is already built (don't re-recommend it)

- **Exact-match response cache** — `llm-adapters/src/cache.rs` (`CachingBackend`, sha3-keyed,
  `CachePolicy` type-enforced). Solves byte-identical repeats; this document is about the
  non-repeat case. (Its unbounded-store defect is owned by P26; not re-litigated here.)
- **Model routing v3.4** — Haiku-lead doer tier, red-line escalation to Opus, explicit `model:`
  per dispatch (memory: model-routing-policy-2026-07-03). This blueprint *extends* that policy
  (§2.7 compiles its dispatch decision; §3.7 adds an effort dial); it does not replace it.
- **Local Ollama plane** — `LlmBackend`/`OllamaAdapter`/dispatch/telemetry (HARNESS-LLM-BACKEND.md),
  measured 4.8–10.5 tok/s decode / ~636 tok/s prefill, prefill:decode ≈ 130:1.
- **Headroom compression proxy** — `ANTHROPIC_BASE_URL` → 127.0.0.1:8787 (memory 2026-07-07);
  connection reuse to the local proxy is free.
- **Wave scheduling / admission** — P25 (BLUEPRINT-WAVE-SCHEDULING…): parallelism across dispatches
  is that phase's territory; this document only adds the cache-write stagger (§3.1) to it.

### 1.4 The inversion, stated plainly

For the workload measured in §1.1, replacing the remote API with local Ollama **increases**
latency ~5–15× on the dominant call class. The operator's instinct — *"llm deciding in nanoseconds
in core"* — is correct precisely because it is **not** "local AI" in the inference sense: a
compiled decision procedure generates **zero tokens**. The rest of this document is organized
around that ordering: eliminate the call (§2) > shrink the call (§3.7 output discipline) > overlap
the call (§3.1/3.4) > relocate the call (§3.5/3.6, mostly rejected).

---

## 2. PRIMARY — The Decision Compiler: LLM as one-time compiler for native decision procedures

> **[GROUNDED in this repo's own shipped code + operator-ruled primary].** Mechanism in one line:
> for a **recurring question shape**, query the LLM **once** (or a small bounded number of times,
> under review) to *author* a fast native decision procedure — generated Rust, compiled, tested,
> provenance-stamped — and answer every subsequent instance of that shape in-process in
> nanoseconds, with a typed escalation path back to the LLM for instances the procedure cannot
> decide. The LLM moves from the request path to the *build* path.

### 2.1 What "compile" means mechanically

A **question shape** is a parameterized judgment, not a sentence: *"does path P touch a red-line
surface?"* is one shape with millions of instances. A **DecisionUnit** is the compiled artifact
for one shape:

```rust
/// Provenance header (doc comment, machine-readable):
/// compiled-by: <model id> on <date>, from N harvested instances (sha3 of the instance set)
/// watched-inputs: [<paths/policies/event kinds whose change invalidates this unit>]
/// falsifier: <the property test that must stay green>
pub enum Decision<T> { Answer(T), Escalate(EscalateReason) }   // never a silent guess
pub fn decide(input: &ShapeInput) -> Decision<ShapeOutput>;     // pure, no I/O, no alloc in hot path
```

Three properties are non-negotiable, and all three follow existing repo law:
1. **Typed input, closed output** — plain Rust structs/enums, mirroring `ports/llm.rs`'s
   zero-serde convention. No free-text input in regime 1 (§2.4).
2. **`Escalate` is a first-class outcome** — an instance outside the unit's competence degrades to
   the slow LLM path, never to a guess (the same degrade-closed rule as `TokenBucket` and
   `CachePolicy`).
3. **Falsifiable before live** — RED→GREEN property tests authored *with* the unit, plus
   independent review (P7/RC-2: the compiling LLM's own "looks right" is not verification —
   the `deliberate()` Mirror seam or a deterministic oracle checks it).

**Why generated Rust and not a generic ML artifact:** this repo's established pattern is
hand-rolled native primitives for everything (crypto, spectral math, telemetry, TOTP) with
from-scratch falsifiers. An LLM-authored *readable Rust function* can be reviewed line-by-line
against VERIFIED-BY-MATH; a fitted classifier cannot be read, only measured — and there is no
labeled training set on this host anyway. Full DECART in §2.5.

### 2.2 This repo has already proven the pattern — four times, plus two memory-canon precedents

| Precedent | Where (verified this session) | The judgment it compiled | What it replaced |
|---|---|---|---|
| `is_redline(path)` | `tools/ci-truth/src/main.rs:237-245`, test `:717` | "is this path a money/auth/order/event-log surface?" | a grep in a bash script — and, upstream of that, per-diff human/LLM judgment. Consumed live at `:385-388` to compute `redline_hits` for the v5c gate, and recomputed independently in `v1.rs:325-374` (`redline_touch` honesty check) |
| `Scope::touches_red_line()` | mesh worktree `kernel/src/ports/agent/scope.rs:244`, `RedLinePolicy::check` `:264` | "may this agent scope be granted without operator sign-off?" | per-request reasoning about capability grants — now a closed-enum match, structurally refusing red-line scopes |
| hermes `gov_route` EV table | `tools/telemetry/hermes-kernel` binary (probed live 2026-07-17 → `{"route":"ESCALATE"}`) | "which model tier for this task class?" | per-dispatch model-choice deliberation; the EV table was *derived from harvested track-record data*, then serves in microseconds |
| `skillspector-rs` rules pipeline | `tools/skillspector-rs/build.rs` + `gen_rules.py` | "which static patterns flag a skill?" | the Python analyzer at runtime — regenerated into `src/rules.rs` **automatically when the source of truth changes** (`cargo:rerun-if-changed`), byte-identical by AST walk |
| Metacognition transfer | memory: metacognition-transfer-2026-07-06 | "compile reasoning into cheap-doer checklists" | the canon rule this section is the native-code endpoint of |
| Knowledge-as-circuits | memory: knowledge-as-circuits-and-eye-2026-07-05 | lessons → `registry.json` circuits | the data-table variant of the same move |

The Decision Compiler is not a new architecture; it is **naming, systematizing, and closing the
loop on a pattern the repo already converges to by hand**. What is missing is (a) the harvest step
that *notices* a recurring shape, (b) the compile-with-verification protocol, and (c) event-driven
invalidation — §2.4.

### 2.3 Candidate shapes from TODAY'S own session (concrete, harvested by hand)

Each row is a judgment that was answered fresh by LLM calls today — across the ~15+ dispatched
agents plus lead-session turns — whose *shape* is stable and compilable:

| # | Question shape (today's recurrence) | Compiled form | Verifier / oracle |
|---|---|---|---|
| C1 | "Which model tier for this dispatch?" (every Agent call carries `model:` — ~15+ fresh judgments today) | native port of hermes `classify_complexity` + policy-v3.4 table: `fn route(task: &TaskDescriptor) -> ModelTier` | 30-case fixture from today's actual dispatches; must reproduce policy v3.4 incl. red-line escalation rail |
| C2 | "What is the next free phase number in roadmap §8?" (several agents re-derived it today — this task's own brief warns about collisions) | not LLM work at all once named: a tiny native registry (max(phase)+1) maintained by a GapWire consumer watching the roadmap file | grep-derived table equals registry on every commit |
| C3 | "Does this dispatch need `isolation: worktree`?" (AGENTS.md shared-tree rule — re-reasoned per dispatch) | `fn needs_worktree(writes_code: bool, concurrent_writers_possible: bool) -> bool` — the rule is already deterministic prose | truth-table test straight from AGENTS.md §shared-working-tree |
| C4 | "Does this diff touch red-line surfaces?" | **already compiled** (`is_redline` — the existence proof) | `main.rs:717` tests |
| C5 | "Does commit exclusion apply?" (F_max=30 bulk-commit rule, realtime-change-intelligence §4.1) | `fn excluded(files_touched: u32) -> bool { files_touched > F_MAX }` + the per-node top-32 prune — already specified as arithmetic in the proposal | proposal §4.1 pair-count arithmetic as property test |
| C6 | "Which `CachePolicy` for this call?" (Exact/SemanticOk/NoCache per task class — currently per-call-site judgment) | task-class → policy decision table (the §3.3 hard boundary of the harness doc, as a match) | type-system already refuses SemanticOk on gate-critical; table test for the rest |
| C7 | "Does adding X require a DECART report?" (the harness doc's §5 'at a glance' table is literally this decision table in prose) | native table keyed on {new external dep?, trust surface?, internal primitive?} | the harness doc's own table as fixture |
| C8 | "Which loop/skill applies to this request?" (loop-orchestrator's 4-condition classification) | 4-predicate match over request features | loop registry fixtures |

Not every row is equal: C2/C3/C5/C7 are *already deterministic in prose* — compiling them is
transcription, near-zero risk, immediate call elimination. C1 is the pilot (§2.7): genuinely
judgment-shaped, high recurrence, existing harvested data (`TrackRecord` ledger), and an existing
pattern to port (hermes routing.rs). C6/C8 follow. **The class this table deliberately excludes:**
open-ended synthesis (blueprint writing, research, code authoring) — see the honest ceiling, §2.6.

### 2.4 Mechanism — the four hard parts, designed

**(a) Shape recognition — the genuinely hard sub-problem, split into two regimes, honestly.**

- **Regime 1 — closed-world, caller-typed (works now, cost ≈ 0).** The caller *knows* it is asking
  a compiled shape, because the call site is migrated to call the DecisionUnit directly (exactly as
  `v5c-reexec` calls `is_redline` rather than prompting anything). Recognition is done at
  *migration time by a human/agent editing the call site*, not at runtime by a classifier. All
  eight candidates in §2.3 are regime-1: they occur at known choke points (dispatch gate, commit
  gate, roadmap registration, cache construction). **This regime is the recommendation.**
- **Regime 2 — open-world free-text matching ("this incoming natural-language question has the
  same shape as compiled unit U") is NOT solved and must not be pretended solved.** It requires a
  fast local matcher — the existing retrieval stack (trigram/BM25 + `nomic-embed-text` cosine) can
  serve as one — but a near-match is exactly the Layer-B semantic-cache problem, and inherits its
  hard boundary verbatim: **advisory shapes only, threshold-gated, never gate-critical, miss ⇒
  LLM**. A wrong shape-match silently answering the wrong question is the proxy-over-ground-truth
  failure canon forbids. Regime 2 is deferred until regime 1 has ≥5 live units generating match
  telemetry to tune against.

**(b) When to compile — the harvest trigger.** A shape earns compilation when it *recurs*: the
`Dispatcher`'s `TrackRecord` ledger (`{task, model, tokens, ms}` rows — already harvested per call)
plus the session transcripts are the corpus. Proposed threshold, tunable: ≥10 instances of one
shape in 7 days with stable inputs/outputs → candidate; a human/agent (the librarian-curation role
already exists for lessons) names the shape and its typed schema. No automatic compilation without
a named schema — naming is where a human catches a shape that only *looks* stable.

**(c) The compile-with-verification protocol (RED→GREEN, no self-certification).**
1. Assemble the instance set (harvested Q/A pairs for the shape) + the governing policy documents.
2. One LLM call (frontier tier — this is exactly what the expensive model is *for*) emits: the
   Rust `decide()` fn, its property tests including at least one **falsifier** ("an input on which
   a naive implementation would differ"), and the provenance header with `watched-inputs`.
3. Tests run RED first (against a stub), then GREEN against the emitted unit.
4. **Independent check** — a *different* model lineage or a deterministic oracle replays the
   harvested instances through the unit; disagreement ⇒ unit rejected, shape flagged
   not-actually-stable. (RC-2/P7: the author's own green is never the certificate.)
5. Red-line-adjacent shapes (anything C4-like) additionally require the operator gate — a wrong
   compiled rule is *worse* than a slow LLM, because it is fast, confident, and invisible.
6. Unit registered: `decision-units/` registry (a `mod` per shape; a standalone crate only if the
   count outgrows the consumer crates — follow the `is_redline`-lives-near-its-consumer precedent
   until then).

**(d) Invalidation — ride GapWire, not a new staleness mechanism.** Every unit's provenance header
declares `watched-inputs` (paths, policy docs, event kinds). The GapWire pipeline
(BLUEPRINT-EVENT-DRIVEN-ORCHESTRATOR-2026-07-17.md §3.1 `GapEvent`, `triage(ev, policy) -> Route`
at `:204`) already turns "something changed" into typed events; a `GapEvent` whose subject matches
a unit's watched-inputs routes to a new `Route::RecompileDecisionUnit(shape_id)`, which flips the
unit to `Stale`. **A Stale unit answers `Escalate` unconditionally** — the system degrades to the
slow-but-correct LLM path until recompilation (steps c.1–c.6) completes. This is the
`skillspector-rs` `build.rs` `rerun-if-changed` semantics promoted from build-time to run-time, and
it means staleness handling costs zero new architecture: no cron, no TTLs, no second source of
truth about "what changed."

### 2.5 DECART — Decision-procedure representation (filed before adoption, per the Integration Decart Rule)

| Criterion | **(A) LLM-generated native Rust fn (CHOSEN)** | (B) Data decision table, interpreted | (C) Small local classifier (fitted) | (D) Status quo: LLM per query |
|---|---|---|---|---|
| Fit to sovereign core | Exactly the repo's idiom — from-scratch native primitive, reviewed like any other code; zero deps | Fits (a table is just Rust `static` data) but adds an interpreter layer with its own bugs | Foreign idiom: opaque weights in a repo whose canon is VERIFIED-BY-MATH readability | The thing being escaped |
| Correctness — falsifiable | Line-readable + property tests + independent replay (c.4); a wrong branch is *visible* | Same testability; table/interpreter drift is a new failure class | Only statistically testable; no line-level review possible; needs labeled data that does not exist here | Correct-ish but 10⁹× slower and non-deterministic |
| Performance | ns–µs, zero alloc, in-core (operator's stated bar) | ns–µs (equal) | µs–ms + a model artifact to load | 4.9–26 s + tokens |
| Supply chain | Zero new deps | Zero new deps | Training stack (even linfa-class) = new surface + DECART of its own | n/a |
| Maintainability | Each unit is ~30–100 lines of reviewed Rust + tests; GapWire recompile path | Central interpreter to maintain forever | Retraining pipeline to maintain; drift silent | n/a |
| Reversibility | Delete the unit ⇒ call sites fall back to `Escalate` ⇒ LLM path; fully reversible | Same | Same mechanically, but sunk training cost | — |

**DECISION: (A), with (B) as its degenerate case** — when the emitted `decide()` is naturally a
lookup, the "generated Rust" *is* a `match`/`static` table, no interpreter needed; A subsumes B
without the extra layer. **(C) rejected for now** with a falsifiable re-open trigger: if ≥3 shapes
are found where the decision is genuinely statistical (no readable rule reproduces the harvested
instances at ≥95%), a classifier DECART is owed at that time — today zero such shapes are known
(all of §2.3 is rule-shaped). **Probe (honest case against A):** LLM-generated code can encode a
subtly wrong rule *confidently*; mitigations are structural (c.3–c.5 falsifier + independent
replay + operator gate on red-line shapes), and §6.Q2 names the residual risk rather than waving
it off. Banned-reason check: nothing above is decided by "industry standard"; every cell traces to
a repo convention or a measured number.

### 2.6 The honest ceiling — what the compiler can and cannot win [MEASURED]

From §1.1's histogram: calls emitting ≤256 output tokens are **32% of calls but only 4% of decode
tokens**. The judgment-shaped class the compiler targets lives almost entirely in that bucket
(plus some of 257–1,024). Therefore:

- **What it eliminates:** up to ~1/3 of round-trips — each a full fixed cost (p50 ≈ 4.9 s of TTFT
  + short decode + orchestration overhead). Eliminating a *call* also eliminates its context
  assembly, its cache-write risk, and its slot in the dispatch queue — wins that don't show up in
  token accounting.
- **What it cannot touch:** the 80% of decode tokens in >1K-token calls — blueprint synthesis,
  research, code authoring. Those are not question *shapes*; they are the actual work. No honest
  version of this design claims otherwise.
- **The compounding property (why it is still the right primary):** unlike every cache, a compiled
  shape **permanently exits the latency economy** — it never expires, never misses, costs zero
  marginal tokens, and its count ratchets upward. Caching bounds latency; compilation *deletes*
  the asymptote for its class. Over months, the recurring-judgment fraction of the workload trends
  toward zero LLM involvement, which is the operator's stated direction.
- **The named risk:** a wrong compiled rule is worse than a slow LLM (fast, confident, invisible).
  Test-integrity rules (10 banned classes) and the c.4 independent replay are load-bearing, not
  ceremony; red-line shapes stay operator-gated (never-bypass-human-gates canon).

### 2.7 Pilot — one shape end-to-end (falsifiable)

**Shape C1: model-tier routing for Agent dispatches.** Chosen because it is genuinely
judgment-shaped (not mere prose transcription), recurs ~15×/session, has harvested data
(`TrackRecord` + transcripts + policy v3.4), and has a pattern to port (hermes
`classify_complexity`/`rank_models_for_bucket`, reimplemented ~200 lines per the LOCAL-AI doc's own
DECART — not a cross-repo dependency).

Done-checks (all RED before code):
1. `fn route(task: &TaskDescriptor) -> ModelTier` reproduces policy v3.4 on a 30-case fixture
   drawn from today's real dispatches, **including** the red-line escalation rail (money/auth/RLS/
   migrations ⇒ never below the escalation tier).
2. Decision latency < 1 µs (bench), zero alloc.
3. Independent replay (c.4) by a different model lineage agrees on ≥29/30; the disagreement case,
   if any, is adjudicated by the operator, not averaged.
4. A `GapEvent` on the routing-policy memory file flips the unit `Stale`; while Stale, `route()`
   returns `Escalate` (test asserts the degrade path, not just the happy path).
5. The dispatch gate consumes it (advisory first; enforcement flip is a separate, later decision).

---

## 3. Established techniques, evaluated against THIS host [GROUNDED]

### 3.1 Prompt caching — already banked; three real deltas remain

**Status quo [MEASURED]: 99.3% cache-read share.** The dispatch pattern (shared CLAUDE.md/AGENTS.md
system prefix across all of today's agents) is *already* structured to benefit, and does. Anthropic
mechanics (authoritative, via the claude-api reference loaded this session): reads ≈ 0.1× input
price; writes 1.25× (5-min TTL) or 2× (1-h TTL); prefix-match — any byte change invalidates
everything after it; max 4 breakpoints; min cacheable prefix is model-dependent (2,048 tokens on
Fable 5; 4,096 on Opus 4.8/Haiku 4.5). Remaining deltas:

1. **The wave-dispatch cache-write race.** A cache entry becomes readable only after the first
   response *begins streaming*; N parallel dispatches with an identical new prefix all pay full
   uncached price. Today's fan-outs fire waves simultaneously. Fix (documented pattern): dispatch
   1, await its first streamed token, then fire the remaining N−1 — they read the cache the first
   just wrote. Belongs in P25's admission logic (one rule, no new machinery).
2. **TTL vs wave gaps.** Default TTL is 5 min; gaps between waves (operator review pauses) exceed
   it, forcing re-writes. 1-h TTL costs 2× write vs 1.25× and breaks even at 3 reads — trivially
   met by any multi-wave session. Candidate default for dispatch-heavy sessions.
3. **Doc-edit/dispatch interleaving.** Editing CLAUDE.md/AGENTS.md/memory mid-session invalidates
   the shared prefix for *every subsequent dispatch*. Discipline rule: batch doctrine edits at
   session boundaries or between (not inside) waves. Also available: `max_tokens: 0` pre-warm
   after any doctrine edit, and mid-conversation `role:"system"` messages (Opus 4.8 only) which
   append instructions *without* invalidating the prefix.

Other providers, for completeness (one line each — this stack is Anthropic-primary): OpenAI prompt
caching is automatic ≥1,024-token prefixes at a 50% input discount; Gemini has implicit + explicit
context caching. *(Confidence: training-knowledge, not re-verified this session — the dedicated
web lane exhausted its search budget; flagged in §6, immaterial to any decision here since no
routing to those providers is wired — free-tier data-governance gate stands, memory v3.4.)*

### 3.2 Speculative decoding — real at token level (not our lever); the *pattern* transfers one level up

The founding papers, confirmed *(official, arXiv abstracts fetched)*: Leviathan/Kalman/Matias,
**arXiv:2211.17192** (ICML 2023) — draft model proposes k tokens, target verifies all in one
parallel forward pass, modified rejection sampling keeps the output distribution *identical*,
**2–3×** speedup; Chen et al. (DeepMind), **arXiv:2302.01318** — same idea concurrently, **2–2.5×**
on Chinchilla-70B. Evaluation here:

- **Token-level, remote:** happens (or not) inside the provider's serving stack; not a knob we hold.
- **Token-level, local CPU:** llama.cpp supports it (`-md` draft model), but the one CPU-only
  community benchmark found ~18% (0.44→0.52 tok/s) *(secondary)* — and the draft model taxes the
  same cores and RAM. Not worth it on this host; matches the LOCAL-AI doc's ⚠ prior.
- **Task-level analogue (the real transfer): draft-local, verify-remote.** Local model drafts a
  full candidate answer; the *remote* frontier model receives the question + draft and emits a
  schema-constrained ~1-token verdict (accept/revise). The economics work for exactly one reason:
  **verification is prefill-heavy and decode-light** — the remote model reads the draft at
  (cached/prefill) speed and pays almost no decode, while the local host's weakness (decode) is
  only spent where local decode is short. Viability window, from the measured numbers: local draft
  must be ≤~100 tokens (≤10–20 s at 4.8–10.5 tok/s) — i.e., **short, schema-constrained outputs
  only**, which is precisely the G3 router's escalate-on-doubt territory plus the §2.3 shapes not
  yet compiled. Acceptance-rate reality check from the token-level literature: constrained
  code-gen drafts hit 75%+ acceptance, free-form ~35% *(secondary)* — same directional expectation
  here: draft structured verdicts, not prose. **Relationship to §2:** this is the *transitional*
  form — a shape drafted locally and verified remotely 10 times is a shape ready to be compiled
  once. Anthropic's server-side **Advisor tool** (cheap executor + strong advisor consulted
  mid-generation) is the same asymmetry productized, worth a trial on dispatch lanes where Haiku
  executes and Opus advises.

### 3.3 Connection-level optimization — measured to be a minor lever, do the free parts only

From §1.2's decomposition: everything network-shaped lives inside the ~1–1.5 s TTFT slice of a
10.6 s mean — the **ceiling of all connection fixes combined is ~10–15%**. Standard budget
figures (TCP 1 RTT + TLS 1.3 1 RTT ⇒ a cold HTTPS connection ≈ 2 RTT ≈ 200–300 ms at ~100–150 ms
Europe↔US RTT; 0-RTT resumption and HTTP/2 multiplexing reduce repeats) — *(confidence:
training-knowledge for the RTT arithmetic; the dedicated verification lane did not complete —
§6)*. The SDK already keeps connections warm and retries; the headroom proxy is local. **Do:**
nothing new. **Don't:** chase edge/geo relocation of a research host for ≤1 s on a 10.6 s mean.

### 3.4 Streaming + downstream overlap — honest: mostly not available to today's orchestration

The research finding is itself the finding *(lane-verified)*: outside user-facing TTFT and voice
(LLM→TTS sentence-boundary pipelining, the one mature instance), **no published controlled number
isolates the gain of piping partial LLM output into a next agent stage**. Today's orchestration is
report-at-end: a subagent's result lands whole; downstream work cannot legally start earlier
without a contract for *typed partial results*. The realistic path is not "stream text between
agents" but **GapWire blackboard events**: an agent that emits typed intermediate events
(finding-N, decision-D) onto the event log lets a dependent lane subscribe and start on D before
the full report exists. That is an orchestrator-design option for the GapWire arc — filed there,
not oversold here. One overlap that *is* free today: the §3.1 stagger already overlaps wave-mate
startup with the first agent's stream.

### 3.5 Repo-targeted distillation/fine-tuning — deferred with a named trigger

Lane findings *(confidence markers preserved)*: LoRA **arXiv:2106.09685** (~10,000× fewer
trainable params) and QLoRA **arXiv:2305.14314** (65B on one 48GB GPU; 24-h single-GPU fine-tunes)
confirmed *(official)*. **CPU-only fine-tuning of a 7–8B model: not realistic** *(secondary but
uncontested)*. Hosted/rented path exists: ~$20–36 per 8 GPU-hours (Modal/RunPod, *official
pricing*), LoRA→GGUF→local. Two honest blockers before spending: (1) the lane found **no
ADAPTER-import support in Ollama** *(low confidence — contradicts this author's training-era
recollection of a Modelfile `ADAPTER` directive; unresolved, needs a 5-minute local probe before
any plan depends on it)*; (2) **no published eval supports "narrow-domain 7–8B ≈ frontier"** —
Self-Instruct-lineage results still show a ~5% gap on expert tasks at 52K examples *(official)*,
and dowiz has at best a few thousand harvestable instances. Verdict: **weakest evidence per dollar
of all six techniques**. Trigger to re-open: after the Decision Compiler (§2) and G3 router are
live and measured, *if* a recurring shape class resists both compilation and draft-verify (§3.2)
at ≥100 instances/week, a fine-tune DECART is owed then — with the Ollama-import question probed
first.

### 3.6 Mesh/peer sharing of reasoning results — right shape, wrong day

Distributed-cache precedent is real (CDN origin-offload, epidemic/gossip dissemination — Demers et
al. 1987; SWIM) *(training-knowledge; the verification lane did not complete)*. The dowiz-correct
form is visible from the mesh arc: a hub publishes **signed decision records** — (question-shape
id, input hash, answer, provenance, model, date) — which is a WorkReceipt-shaped object (B2), so
peer trust rides capability signatures, never reputation (SOVEREIGN-EVENT-EXCHANGE stance). And
note the composition with §2: **the highest-value thing to gossip is not cached answers but
compiled DecisionUnits** — one hub pays the compile, every hub gets nanosecond decisions —
subject to the same independent-verification-at-import rule as any foreign code. **Rejected for
now:** this is a single-host session; B2 is unbuilt; building peer cache before a second live hub
exists is capability theater. Filed as a mesh-arc consumer, not a phase.

### 3.7 Levers the six-item brief didn't name (found via the authoritative API reference)

1. **Output-token discipline — the largest *shrink* lever.** Decode is 85–90% of the round trip;
   every dispatch prompt that demands a "≤15 sentences" report is a latency policy. Standing rule
   candidate: doer-lane reports ≤300 tokens unless the task is synthesis; schema-constrained
   verdicts where a verdict suffices.
2. **`effort: low` for doer subagents** — documented to produce fewer, more-consolidated tool
   calls and terser outputs; fewer round-trips per subtask, not just fewer tokens per round-trip.
   Extends routing v3.4 (which pins *models*) with a per-dispatch *depth* dial.
3. **Fast mode** — Opus 4.8 at up to 2.5× output tok/s (premium price, own rate-limit pool, beta).
   Real decode acceleration for Opus-tier lanes; not available for Fable (the lead session), so it
   helps dispatched Opus reviewers, not the lead loop.
4. **Message Batches API** — 50% cost, ~1-h typical completion: a *time-shifting* lever, not a
   latency one. Overnight lanes (S4, §5) ride it; nothing interactive should.

---

## 4. Adopt-now register (each item: dependency + done-check; DECART only where owed)

| # | Item | Depends on | DECART? | Done-check |
|---|---|---|---|---|
| A1 | Decision Compiler pilot — shape C1 (§2.7) | GapWire for the Stale path (advisory unit can land before it) | **Yes — filed §2.5** | §2.7's five checks |
| A2 | Wave-dispatch cache-write stagger (§3.1.1) | P25 admission point | No (dispatch discipline) | one wave's usage shows N−1 dispatches with `cache_read>0` on the shared prefix |
| A3 | 1-h TTL + post-doc-edit pre-warm for dispatch sessions (§3.1.2–3) | none | No (config) | inter-wave dispatches stop paying `cache_creation` for the stable prefix |
| A4 | Doc-edit/dispatch-wave separation rule (§3.1.3) | AGENTS.md standing-rule merge (operator) | No | proposed text staged; operator merges |
| A5 | Output-discipline + `effort: low` defaults for doer lanes (§3.7.1–2) | none | No | mean output-tokens/doer-call falls ≥30% at equal task success (Telemetry ledger) |
| A6 | Draft-local/verify-remote trial on one schema-constrained lane (§3.2) | G3 router precondition (P-2 small-model pull, operator go) | Covered by LOCAL-AI doc's DECART | verdict-call decode ≤5 tokens remote; end-to-end lane latency < remote-only baseline on a 20-case fixture |

Everything in §5 is explicitly **not** in this table.

---

## 5. SPECULATIVE / FUTURE — the brainstorm the operator invited [SPECULATIVE — none decided, none DECART'd]

Labeled honestly; one-line mechanism, one-line why-it-doesn't-work-today (or the real fragment).

- **S1 — Question-futures prefetch.** *Mechanism:* during idle wall-clock, predict the operator's
  next questions (Markov attractor detector over session history + roadmap open items) and
  pre-generate answers into the exact-match cache. *Why not today:* phrasing entropy means
  exact-match almost never hits a pre-generated answer, semantic matching is advisory-only by
  canon, and hit-rate must exceed the pre-generation spend — no evidence it would. *Real
  fragment:* prefetching **context packs** (file reads, grep results) for predicted questions is
  cheap and always valid — worth folding into the orchestrator arc.
- **S2 — Hedged local/remote race.** *Mechanism:* fire Ollama and the remote API simultaneously,
  first answer above a confidence bar wins (Dean & Barroso, "The Tail at Scale," CACM 2013 —
  tied/hedged requests cut p99.9 ~40% for ~5% extra load *(secondary via Colyer's review; primary
  PDF not retrieved)*). *Why not today:* local decode at 4.8–10.5 tok/s loses every race longer
  than ~50 output tokens, so the race window collapses onto exactly the class §2 compiles away
  more cheaply. *Real fragment:* hedging **remote-vs-remote** (same request, second fire at the
  p95 mark) is the honest tail-latency tool for p90=26 s stalls — cost +5%, zero architecture.
- **S3 — Promise-pipelining dispatch.** *Mechanism:* downstream agents start against a *promise*
  of the upstream report (E-lang/Cap'n Proto promise pipelining), doing all non-dependent work
  while upstream decodes; typed partial results flow over GapWire blackboard events. *Why not
  today:* the Agent surface is report-at-end; no typed-partial contract exists. This is §3.4's
  design option pushed to its logical end — filed to the GapWire arc.
- **S4 — Nightly batch pre-reasoning.** *Mechanism:* every roadmap open question / named unknown
  becomes an overnight Message-Batches job (50% cost, latency-free time slot); answers land as
  dated advisory docs before the operator wakes. *Why "not" today:* it's not a latency lever for
  novel questions and risks a pile of unread LLM prose. *Real fragment:* this is genuinely
  buildable this week for the **known** open-decision list (O-items table) — the only speculative
  part is whether pre-answered questions are the ones that get asked.
- **S5 — Enumerate the question space.** *Mechanism:* treat "questions about this codebase" as a
  finite grammar (k symbols × m relations), batch-generate the full answer table, and make Q&A a
  lookup. *Why not today:* combinatorial explosion × staleness-per-commit makes the full version
  economically absurd. *Real fragment:* the bounded version already exists and works — the
  Repowise index; §2's DecisionUnits are the *judgment* rows of this same table, built lazily on
  recurrence instead of eagerly by enumeration (lazy beats eager here by the same YAGNI law as
  everywhere).
- **S6 — Diffusion/parallel-block decode.** *Mechanism:* text-diffusion LLMs decode token blocks
  in parallel — Inception Labs' Mercury reports ~737–1,109 tok/s *(official paper claim,
  arXiv:2506.17298 — on H100 only)*. *Why not today:* closed weights, GPU-only, nothing to run on
  this host; and the remote providers will productize parallel decoding without us doing anything.
  Watch, don't build.
- **S7 — Gossip of compiled decisions across hubs.** *Mechanism:* §3.6 taken to its endpoint —
  hubs exchange signed DecisionUnits, so the mesh's total compiled-shape count grows
  super-linearly in hubs. *Why not today:* one host, B2 unbuilt, and importing foreign executable
  decision logic is the highest-trust operation imaginable (needs the full AgentBridge admission +
  independent-replay machinery). The one brainstorm idea that is *more* real than it sounds —
  because every ingredient (canonical-TLV signing, admission caging, replay verification) is
  already designed in the mesh arc for other cargo.
- **S8 — Answer-by-construction remote calls.** *Mechanism:* invert the token asymmetry — the
  remote model is only ever asked for ultra-short schema-constrained verdicts (`max_tokens`
  double-digit), all prose is assembled locally from templates + the verdict. *Why it's only
  half-speculative:* it is §3.2's verify-remote leg generalized; the speculative half is prose
  assembly without decode, which for reports would read like a form letter. Verdict lanes: real;
  prose lanes: no.

---

## 6. Doubt audit (2 questions, mandatory)

**Q1 — least confident (ranked):**
1. **The latency metric is a proxy.** Transcript entry timestamps measure "user-entry → first
   assistant chunk *written*," not wire TTFT; chunk-write granularity in the harness is unverified.
   The decode-dominates conclusion is corroborated independently (output-token arithmetic ×
   third-party tok/s), so the *decision* stands even if the split shifts.
2. **No Fable-specific decode/TTFT figure** — the benchmark snapshot had Haiku/Sonnet/Opus rows
   only; Fable's always-on thinking makes its effective rate worse, direction known, magnitude not.
3. **Ollama `ADAPTER` import support is contested** (lane found none, low confidence; author's
   prior says it exists). Cheap to resolve; nothing in §4 depends on it — only §3.5's deferred path.
4. **The Tail-at-Scale numbers are from a review, not the primary PDF** (fetch 403'd).
5. **§3.1's other-provider one-liners and §3.3's RTT arithmetic are training-knowledge**, not
   re-verified this session (the third research lane exhausted the session's web-search budget and
   its WebFetch retry had not returned by writing time). No adopted item depends on either.
6. **The compilable-fraction estimate (§2.6) conflates "short call" with "judgment call"** — some
   ≤256-token calls are mid-loop acknowledgments, not compilable shapes; 1/3-of-calls is an upper
   bound, not a forecast. The pilot's telemetry, not this estimate, is the number that matters.

**Q2 — the biggest thing possibly missed:** the largest wall-clock line item may be
**orchestration serialization, not per-call latency** — the p90=26 s calls are the *lead's own*
long thinking/synthesis turns, which no cache, compiler, or router shortens, and which serialize
everything behind them. The honest implication: P25's wave parallelism and lead-loop output
discipline may buy more wall-clock than every technique in §3 combined, and the Decision
Compiler's true yield is *strategic* (permanently shrinking the recurring-judgment class, freeing
the lead loop for the synthesis work only it can do) rather than a big immediate mean-latency
drop. Second miss, named because it is the design's own failure mode: a compiled-decision registry
whose units were authored *and* reviewed by the same model lineage is RC-2 self-certification with
extra steps — the independent-replay leg (§2.4.c.4) is the load-bearing safeguard and must not be
value-engineered away when implementation gets tedious.

## 7. Anu / Ananke check

- **Anu (logic):** every quantitative claim above traces to a probe run this session (§1 tables),
  a cited source with preserved confidence marker, or an explicit training-knowledge flag (§6.5).
  The central inference — decode dominates ⇒ eliminate/shrink calls beats relocating them — is
  arithmetic on measured numbers, and the local-AI inversion (§1.4) follows from the same two
  measured rates; no step relies on authority or vibes.
- **Ananke (structural necessity):** every adopted item names its real dependency (A1→GapWire for
  Stale-handling, A2→P25's admission point, A6→P-2 operator go); the compiler's invalidation
  deliberately *reuses* GapWire instead of adding a staleness mechanism, and its registry follows
  the lives-near-consumer precedent instead of inventing a platform. Nothing in §5 leaks into §4;
  the one operator-gated surface (red-line shapes, A4's AGENTS.md merge) is named, not assumed.

---

*Provenance: written 2026-07-17 on `feat/harness-llm-backend`. Local probes this session:
transcript latency/usage/histogram analysis over today's two session JSONLs (scratchpad `lat.py`/
`hist.py`); greps + reads: `tools/ci-truth/src/main.rs` (`is_redline` :237, test :717; consumers
:385-388), `tools/ci-truth/src/v1.rs` (:325-374), mesh `kernel/src/ports/agent/scope.rs` (:244,
:264), `tools/skillspector-rs/{build.rs,gen_rules.py}`, `llm-adapters/src/cache.rs`,
BLUEPRINT-EVENT-DRIVEN-ORCHESTRATOR §3.1/:204, realtime-change-intelligence proposal §4.1 (F_max),
roadmap §8 (phase registry re-read fresh). Authoritative Anthropic API facts via the claude-api
reference loaded this session (caching economics/TTL/pre-warm/concurrent-write race, fast mode,
Batches, Advisor tool, effort). Web research via three delegated lanes with per-claim confidence
preserved (spec-decoding + API latency: complete; distillation: complete; connection/other-provider
caching: incomplete — budget-exhausted, flagged §6.5). Companion docs: HARNESS-LLM-BACKEND.md,
LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md, BLUEPRINT-WAVE-SCHEDULING (P25),
BLUEPRINT-EVENT-DRIVEN-ORCHESTRATOR (GapWire), model-routing-policy v3.4 (memory). No code
written.*
