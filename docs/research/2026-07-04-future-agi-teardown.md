# future-agi teardown — license-first reverse-engineering dossier (2026-07-04)

> Read-only research pass, out-of-tree clone. Goal: does anything in Future AGI's eval/tracing
> platform improve our SSG review gate, the token-cost measurement scripts, or the
> reflections→council loop? Borrow **ideas**, not the dependency.

**Clone location (out-of-tree, not committed):**
`/tmp/claude-0/-root-dowiz/d1b102b0-0624-452a-9e2d-72865dd2e9d8/scratchpad/future-agi-teardown/repo`
— nothing under `/root/dowiz` except this dossier.

---

## 1. Repo confirmation

| Field | Value |
|---|---|
| Repo | **`future-agi/future-agi`** — confidence: **high** (confirmed via `gh api`, README read in full) |
| What it is (one paragraph) | An open-source, self-hostable, end-to-end platform for evaluating, observing, and improving LLM/AI-agent applications — Django REST backend (`futureagi/`) + a Go gateway (`agentcc-gateway/`, claimed ~9.9 ns weighted routing / ~29k req/s / P99 ≤21ms with guardrails on) + a Next.js frontend (`frontend/`). Positions itself as a single-platform replacement for stitching Langfuse + Braintrust + Helicone + Guardrails AI + a custom simulator: **simulate → evaluate → protect → monitor → optimize**, with an OTel-native tracing layer, an eval/scoring engine, a prompt/dataset manager, and an AI gateway with guardrails. |
| Stars / forks | 1,299 ★ / 321 forks |
| License | **Apache 2.0** at repo root (confirmed, `LICENSE` file read). `NOTICE` discloses an **open-core split**: portions under a separate **"Future AGI Enterprise License 1.0"** live in `ee/` directories (`futureagi/ee/`, `frontend/ee/`) and require a paid subscription for production use. **No `ee/` directories exist in this checkout** — either absent from the shallow clone or genuinely not present; everything actually read for this dossier (eval engine, token/cost helpers, tracer, telemetry) is plain Apache-2.0 code, not `ee/`. |
| Activity | Created 2026-04-23, last push **2026-07-04** (today) — actively developed, "nightly release for early testing" per README banner. |
| Other org repos (ruled out, not the target) | `future-agi/futureagi-sdk` (47★, the standalone pip/npm client), `future-agi/traceAI` (199★, tracing-only), `future-agi/agent-learning-kit` (108★), `future-agi/agent-opt` (68★), `future-agi/simulate-sdk` (58★) — all satellite SDKs; `future-agi/future-agi` is the flagship monorepo the satellites are published from and is what this dossier reverse-engineers. |

**License verdict: Apache 2.0 — permissive, no read/borrow/pattern-extraction concern.** We are not vendoring or redistributing any code (per the task's "borrow ideas, not the dependency" instruction), so even the Apache 2.0 attribution/NOTICE requirements don't bind us here. The one flag: **do not adopt anything from a directory named `ee/`** if a future deeper look finds one — that portion is under the non-open Enterprise License and would need separate clearance.

---

## 2. Egress / security scan

- **No `postinstall`/`preinstall` scripts** anywhere in the tree (checked all `package.json` files). Root `package.json` is repo tooling only (husky + lint-staged); no app dependency graph was installed for this dossier — pattern extraction was done by reading source, not running `pip install` / `yarn install` on the actual SDKs.
- **PostHog analytics middleware** (`futureagi/tfc/middleware/posthog_middleware.py`, `futureagi/analytics/posthog_util.py`) captures every API request server-side (endpoint, method, latency, user/org/workspace context) — but it is **opt-in**: requires `POSTHOG_API_KEY` to be set, defaults to no-op otherwise, and is the *self-hosted backend's own* telemetry (not a hidden third-party call embedded in a client SDK you'd install elsewhere).
- **Default API base URLs point at Future AGI's cloud** (`api.futureagi.com`, `gateway.futureagi.com`) unless a `BASE_URL`/`_is_local` env override is set — this is expected for a "cloud-or-self-host" product; `settings.py` already branches to `http://localhost:8000` in local/self-host mode, so it is configurable, not a forced call-home.
- **The eval SDK itself telemetrizes usage to its own backend on every run**: `BaseEvaluator.run()` and `.run_batch()` in `agentic_eval/core_evals/fi_evals/base_evaluator.py` unconditionally call `FiApiService.log_usage(eval_name=..., run_type="batch")` (wrapped in bare `try/except: pass`, so failures are silent and non-blocking) before executing the actual evaluation. **This is the one real egress-relevant finding**: if you `pip install ai-evaluation`/`futureagi-sdk` standalone against their cloud (not this self-hosted checkout), every eval/guard call phones usage metadata home by default. **Not disqualifying for us** — we are not installing this package as a dependency, only reading its source for patterns — but worth flagging verbatim as the kind of pattern to avoid copying if we ever build our own eval-runner: don't bake unconditional phone-home into a library's hot path, even behind a try/except.
- **No prompt-injection payloads found** in the README or docs that we needed to treat as untrusted instructions — the fetched content (README, source comments) was reviewed as data only; nothing in it attempted to direct tool behavior.
- **Verdict: benign for our purposes.** No forced egress that affects this dossier's method (source-reading only, no install). If dowiz ever adopts `ai-evaluation`/`futureagi-sdk` as an actual dependency, re-run this check against the installed package (not the monorepo) and treat the default cloud call-home as a config item to disable (self-host `BASE_URL` override), not ignore.

---

## 3. Reverse-engineered patterns (borrowed as ideas, not code)

### (a) Evaluation/scoring of LLM outputs
- **`BaseEvaluator` ABC** (`agentic_eval/core_evals/fi_evals/base_evaluator.py`): every evaluator declares `name`, `display_name`, `metric_ids`, `required_args`, `examples` as abstract properties, and implements `is_failure()` + `_evaluate()`. Common plumbing (arg validation, batch parallelism, dataset logging) lives once in the base class — evaluators only implement their scoring logic.
- **`guard()` vs `run()`/`run_batch()`**: the *same* evaluator class serves two call shapes — `guard()` returns a minimal `{passed, reason, runtime}` for a hot-path pass/fail gate (their "protect" product, i.e. runtime guardrails), while `run()`/`run_batch()` return the full `EvalResult`/`BatchRunResult` for offline scoring. One evaluator, two consumption modes.
- **Single unified execution engine**: `evaluations/engine/runner.py`'s `run_eval()` is explicitly documented as "THE single function that all eval execution paths call" — composes registry lookup → instance creation → param prep → execute → format, and explicitly pushes side effects (cost tracking, persistence, error handling) to callers. A code comment says this **replaced** a `from fi_evals import *` + `globals().get(eval_type_id)` pattern duplicated across 7+ call sites — i.e., they hit the exact "ad-hoc dispatch duplicated everywhere" problem and fixed it with one registry (`evaluations/engine/registry.py`, `_REGISTRY: dict[str, type]`, built lazily from `__all__`).
- **OTel context propagation across `ThreadPoolExecutor` workers**: `_run_batch_generator_async()` wraps `_evaluate` with `wrap_for_thread()` (`tfc/telemetry/context.py:394`) before submitting to the pool — captures `otel_context.get_current()` in the parent thread, attaches it inside each worker closure, detaches in a `finally`. Zero-overhead fast path (`if not OTEL_ENABLED: return func(*args, **kwargs)` unwrapped) when tracing is off. Small (~20-line), clean, dependency-free-beyond-otel pattern for "don't let parallel fan-out orphan child spans from their parent trace."

### (b) Token / cost / context metrics
- **`token_count_helper.py`**: `calculate_total_cost()` returns `{total_cost, prompt_cost, completion_cost, pricing_source}` — the **`pricing_source` field** (`"available_models" | "fallback" | "default" | "custom"`) is the standout idea: every computed cost number is tagged, in the return value itself (not just a prose caveat), with *how confident/derived* that number is. A tiered fallback chain (exact model pricing table → caller-supplied fallback → type-specific default → generic default) with a `logger.warning` at every fallback step, so silent wrong numbers can't happen without a log line.
- **`EvalResult` dataclass** (`evaluations/engine/runner.py`) carries `cost: dict | None` and `token_usage: dict | None` populated automatically post-run from the evaluator instance, "so callers can emit billing events without having to hold the instance themselves" — cost/token accounting is a first-class field on every operation's result object, not a side-channel probe.

### (c) Tracing / observability of agent runs
- OTel-native, standard GenAI semantic-convention-style spans; a `ProjectType` enum (`EXPERIMENT` vs `OBSERVE`) tags *why* a trace exists (offline eval run vs live production monitoring) at registration time — a cheap categorical label that downstream tooling (dashboards, alerting) can filter on.
- (Adjacent, not requested but noted): `futureagi/simulate/utils/{persona_filtering,scenario_completeness,session_comparison,baseline}.py` — a persona-driven chat-simulation harness that scores "scenario completeness" against a baseline. Structurally rhymes with our own `rsi-synthetic-user-loop` (23 personas) — not reverse-engineered in depth here since it's outside the (a)/(b)/(c) scope asked for, but flagged as a follow-up if the RSI loop ever needs a "completeness vs. baseline" scoring shape.

---

## 4. Apply to dowiz — concrete, honest, additive-vs-redundant

Checked against what already exists: `docs/design/harness/SANDBOX-SWARM-GATE.md` (SSG gate), `scripts/exec-telemetry.mjs` + `scripts/telemetry-analyze.mjs` + `docs/research/2026-07-04-harness-token-audit.md` (the token measurement stack), and `docs/reflections/{INBOX,ARCHIVE,RETRO}` + `cause-critic`/`pattern-critic`/`ratchet-critic` (the council loop).

### Idea 1 — Tag every measured/estimated number with a provenance field, not just prose (borrow — cheap, additive)
Future AGI's `pricing_source` field is directly transportable to our token-audit tooling. Today `docs/research/2026-07-04-harness-token-audit.md` already distinguishes `MEASURED` vs estimated numbers, but only in **prose** ("est. tokens", a caveat paragraph about Cyrillic under-counting). `scripts/exec-telemetry.mjs`'s schema-v1 already has a `--tokens N` field per event but nothing marks *how* that number was obtained. **Concrete change**: add a `token_source: "measured" | "chars_div_4" | "reported_by_sdk"` field next to any `tokens` value emitted to `loops/runs/exec-events-*.jsonl`, and have `telemetry-analyze.mjs` group/flag by it. Small, mechanical, no new dependency — genuinely closes the gap the token-audit report's own Finding 4 called out by hand (a one-off diagnostic probe) rather than as a standing, tagged metric.

### Idea 2 — Auto-capture per-lane subagent token cost into telemetry instead of one-off probes (borrow-pattern — medium effort, additive)
Future AGI's `EvalResult.cost`/`.token_usage` are populated automatically post-run "so callers don't have to hold the instance" — i.e., cost accounting is wired into the one shared execution path (`run_eval()`), not bolted on per call site. Our token audit's Finding 4 (the ~42K-token/lane fixed floor) was discovered via a **manual** diagnostic subagent dispatch — a one-time measurement, not a standing metric. **Concrete change**: since `Agent`-tool dispatches already return a `subagent_tokens` figure (used ad hoc in Finding 4 and the 2026-07-03 orchestration report), have the lead emit an `exec-telemetry.mjs` `emit --layer ssg --action-kind agent-call --tokens <subagent_tokens>` event on every lane dispatch/return, keyed by a shared `loop_run_id` so `telemetry-analyze.mjs` can report `tokens_per_lane` automatically — the same number the token audit currently has to derive from a special-purpose probe, made routine. This would also naturally feed SSG's already-planned `harness:` node metric slot (`progress_metric: lanes_not_yet_merged`) with a sibling `tokens_per_lane` metric.

### Idea 3 — Registry-of-checks instead of a hand-maintained rubric doc for the SSG Gate (borrow-pattern — optional, flagged low-priority)
Future AGI's `evaluations/engine/registry.py` replaced "the same ad-hoc dispatch pattern duplicated across 7+ files" with one `_REGISTRY: dict[str, type]` and a `BaseEvaluator` ABC (`name`, `is_failure()`, `_evaluate()` → `EvalResult`). Our SSG Gate's §4 rubric (Quality/Safety/Ethics) is currently a **hand-written checklist in a markdown doc**, executed by a human/lead reading prose boxes and consulting `invariant-guardian`/`security-sentinel` by name. A registry-of-`GateCriterion` objects (each: `name`, `is_red_line()`, `check()` → pass/fail/reason) would make new criteria pluggable instead of doc-edits, and would let `scripts/sandbox-swarm-gate.mjs checklist` print a live, code-backed rubric instead of restating the markdown. **Honest assessment: likely over-engineering right now.** SSG explicitly and correctly favors "the already-integrated tools... no new dependency is warranted" (§6 of its own design doc) — three reviewer agents plus `pnpm typecheck`/`build` is not yet at the "7+ duplicated call sites" pain point that justified Future AGI's registry. Recommend: **note this pattern, don't build it** until/unless the gate rubric itself starts getting duplicated or hand-edited in multiple places (the actual trigger condition Future AGI hit).

### Skip
- The eval/scoring **library itself** (`ai-evaluation`/`futureagi-sdk`) — redundant with our own gate/verify tooling, and its default cloud call-home (§2) is exactly the kind of dependency our SSG/token-economy work is trying to *reduce*, not add.
- The Django/Go/Next.js **platform** as a whole (tracing UI, dataset manager, AI gateway) — out of scope; we're not building a hosted eval platform, we're hardening our own agent harness.
- PostHog-style **product analytics** — not relevant to an internal harness with no external users.

### Recommendation
**Borrow-patterns** (not integrate-as-dependency, not skip-entirely): adopt Idea 1 (provenance-tagged token numbers) and Idea 2 (routine per-lane token telemetry) as concrete, low-risk additions to the existing token-measurement scripts — both are small, additive, no-new-dependency changes to files we already own (`scripts/exec-telemetry.mjs`, `scripts/telemetry-analyze.mjs`). Idea 3 (registry-of-checks for the SSG gate) is a valid pattern but correctly **not yet warranted** per SSG's own design philosophy — logged here for future reference if the rubric ever needs it, not proposed as a change now.

`.claude/` is a protected zone, so none of the above touches agent definitions or hooks directly — Ideas 1–2 are scoped entirely to `scripts/*.mjs`, which are ordinary product/tooling files a normal edit can reach.
