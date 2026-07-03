# Reinforced Swarm — closing the outcome-feedback loop on the dowiz harness

> **Lane:** research + plan (design-time). No code, no commit. This doc specifies a *buildable*
> increment; it does not build it.
> **Date:** 2026-07-02
> **Bound by:** CLAUDE.md Self-improvement loop §1–§7 · memory-corpus **pattern #4** (advisory
> signals inform, deterministic artifacts decide) · pattern #12 (authority-bearing state rots —
> measure release/armament) · `docs/governance/model-calibration.md` §3–§4 standing constraints
> (the calibration ledger is *forever a mirror, never a stick*) · `docs/security/security-loop.md` §4.
> **Scope fence (inherited, model-calibration §4):** the subject here is **the agent's own
> dispatch behavior** — governance-plane only. Reusing any of this on the product/courier/client
> plane (subjects = non-consenting humans) is a separate 🔴 Triadic-Council decision, never a copy-paste.

---

## 0. The operator's ask, stated precisely

> "A feedback, reinforced swarm agentic system in staging — the multi-agent harness should LEARN
> from the outcomes of its own runs."

Decoded against what exists: the harness already **records** outcomes richly and **reflects** on
them, but **nothing reads stored outcomes back to change the next dispatch.** The learning arc today
is: run → telemetry → (sometimes) reflection → (rarely) a guardrail. What is missing is the arc:
**run → telemetry → aggregate by task-signature → advise the *next* selection of loop / agent-type /
model / effort, and retrieve the relevant failure-corpus *before* the task starts.**

That read-back is the whole of the "reinforced" part. Everything below is plumbing for it, kept
inside the advisory-vs-authority boundary that already governs this repo.

---

## 1. Map of the existing feedback substrate (what dowiz ALREADY has)

The repo is unusually rich here — the substrate is ~90% present. Grouped by the four literature
primitives the operator named:

### 1.1 Outcome / trajectory records (the reward signal source)
| Store | What it holds | Written by | Read-back today |
|---|---|---|---|
| `loops/runs/metrics.jsonl` (`MetricsLine`, `tools/loop-harness/src/types.ts:145`) | one compact line per loop run: `outcome ∈ {green,stall,abort,natural_stop}`, `iters`, `fail_start/end`, `per_resolved`, `cost_usd`, `slop_min`, `recurring_flags`, `edits` | `finalize` (§7) | **history comparison only** (`HistoryComparison`, run-over-run avg/best) — never feeds *selection* |
| `RunRecord` / `IterationTelemetry` (`types.ts:33`,`:91`) | per-iteration `agents{}`, `skills{}`, `tokens.by_model{}`, `code.tests_fail_*`, `fake_green_caught`, `slop_score` | harness | not aggregated across runs by signature |
| `collect.ts` (`collectSessionTelemetry`, `:163`) | tokens **by model**, **agents dispatched**, skills used, cost — parsed from the session JSONL | collectors | folded into the run record; not indexed |
| `telemetry/plane` branch (`plane-events-*.jsonl`) via `plane-telemetry.mjs emit` | governance-plane events: `kind/outcome/severity/duration_ms/metrics/refs`, durable append-only git store | plane-maintainer, plane-guard, loops | `digest`/`query`/`inbox` — **human-facing views only** |

### 1.2 Calibration (predict-before / resolve-after)
- `scripts/plane-telemetry.mjs predict|resolve` → `loops/runs/predictions.jsonl` (schema v1):
  `confidence ∈ [0,1]`, `method` (primary|fallback), `gap ∈ {hit,miss,partial}`, M1 backdating
  friction. Read as **calibration (reliability), not a score** — Brier-style, anti-Goodhart
  (`model-calibration.md` §3). **Standing constraint: forever advisory, never wired to a gate.**
- Gap → mandatory WHY-reflection (§4) → `result-vs-expectation` doubt trigger (CLAUDE.md §6).

### 1.3 Experience / reflection memory (Reflexion-class, already present)
- `docs/reflections/INBOX|ARCHIVE|RETRO/` — causal WHY reflections after qualified fixes.
- `docs/lessons/` (+ INDEX) — distilled, `TRIGGER`-keyed lessons.
- `pre-edit-lessons` PreToolUse hook injects the relevant lesson **by keyword TRIGGER** before an edit.
- `librarian` agent: distill → challenge → promote (lesson → guardrail, red→green) → prune.
- `docs/regressions/REGRESSION-LEDGER.md` — the deterministic ratchet ledger.

### 1.4 Verifier / decorrelated-critic selection (already present)
- Council critics **cause-critic / pattern-critic / ratchet-critic** — fresh-model, read-only,
  decorrelated adversarial verification of claimed causal roots.
- `autoupgrade` oracle + **decorrelated lenses** (security · reversibility · perf) gate Class-A keeps.
- `tools/loop-harness/src/proven-upgrades.ts` — **the closest thing to a reinforcement asset today**:
  a frequency-weighted (`count++`), append-only "gene" registry of oracle-*kept* upgrades. But it is
  scoped to the autoupgrade oracle, is a *ledger not an apply/select path*, and is keyed by patch-id,
  not by task-signature.
- `security-redblue` loop — advisory findings → guardrail + ledger; red-line → council.

### 1.5 The router (the one place selection happens — and where learning is absent)
- `tools/loop-harness/src/router.ts` `route()`: on every command decides
  `DIRECT | RUN <loop> | BUILD | BOUNCE` by **static tag/scope score** over `registry.json`
  (derived from `loops/registry.md`). Emits `routing.jsonl` (append-only intent log).
- **It never reads `metrics.jsonl`, `predictions.jsonl`, reflections, or findings.** `routing.jsonl`
  is write-only — no consumer closes the loop. The router's confidence is a hand-tuned formula
  (`0.6 + score*0.1`), not calibrated against observed outcomes.
- It selects a **loop**, never an **agent-type × model × effort** — that dispatch choice is made
  ad-hoc by the lead agent with no memory.

**Summary of the gap:** the substrate stores outcomes (1.1), calibration (1.2), experience (1.3),
and has verifiers (1.4) — but the **selection surface (1.5) is memoryless.** No component reads the
outcome stores to weight the next choice, and no pre-task retrieval pulls the relevant
failure-corpus by *task-signature* (the `pre-edit-lessons` hook matches by keyword TRIGGER, not by
the outcome-weighted similarity that "learn from your runs" implies).

---

## 2. What the literature says (and how it maps onto this substrate)

The current best patterns for outcome-feedback in agent swarms, and their dowiz mapping:

| Pattern (source) | Idea | dowiz has? | The delta to build |
|---|---|---|---|
| **Reflexion** — verbal RL, no weight updates ([Shinn et al. 2023](https://www.semanticscholar.org/paper/Reflexion:-language-agents-with-verbal-learning-Shinn-Cassano/0671fd553dd670a4e820553a974bc48040ba0819)) | attempt → textual outcome → NL reflection → reuse as context next trial | ✅ reflections + pre-edit hook | reflections aren't **retrieved by task-signature** nor **aggregated** across runs |
| **ExpeL** — experiential learner ([Zhao et al. 2024](https://arxiv.org/pdf/2308.10144)) | collect trajectories → abstract insights → retrieve top-k relevant at test time | partial (lessons by keyword) | outcome-weighted **top-k retrieval before the task** |
| **ReasoningBank** ([2509.25140](https://arxiv.org/abs/2509.25140)) | distill strategy items from **both success and failure** trajectories; retrieve→apply→re-integrate | ⚠️ only failures (reflections); successes uncaptured | index **green runs** as positive strategy items too, not just misses |
| **Agent Workflow Memory** ([AWM](https://arxiv.org/html/2603.10600v1)) | induce reusable workflows from past traces, inject selectively | ⚠️ loops ARE workflows, but selection is static | outcome-weighted loop selection |
| **Contextual bandit / learned router** ([RouteLLM/MetaLLM; BaRP 2510.07429](https://arxiv.org/html/2510.07429v1); [PILOT]; [MasRouter 2511.02200](https://arxiv.org/pdf/2511.02200)) | model routing as a contextual multi-armed bandit; learn from partial/online feedback; MasRouter also routes **role + model** | ❌ static tag score only | a **bandit over agent-type × model × effort**, keyed by task-signature, reward = observed outcome |
| **Outcome/Process reward + verifier-guided selection** ([ORM/PRM](https://www.emergentmind.com/topics/outcome-supervised-reward-model-orm); [AgentRM 2502.18407](https://arxiv.org/pdf/2502.18407)) | score complete trajectories by verifiable outcome; rerank/select | ✅ decorrelated critics = the verifier | add a **calibration-weighted advisory verifier score** |
| **Best-of-N via self-certainty** ([2502.18581](https://arxiv.org/html/2502.18581v1)) | select among candidates by the model's own certainty | ✅ calibration ledger = self-certainty | keep certainty **advisory** (standing constraint); use it only to *discount* an over-confident verifier |
| **Escape the self-confirmation trap — Execute-Distill-Verify** ([2606.24428](https://arxiv.org/html/2606.24428v1)) | a self-graded memory can reinforce its own errors; verify against execution, not self-report | this is exactly pattern #4 + M1 friction | **reward on the deterministic outcome, never the self-graded confidence** |

**The load-bearing lesson from the 2025/26 literature that dovetails with dowiz's own #4:** a memory
that reinforces on **self-judged** success (Reflexion's known failure mode; ReasoningBank and
Execute-Distill-Verify both flag it) drifts into self-confirmation. dowiz's calibration ledger is
*already* fenced against this ("mirror, never a stick"). So the reinforced router must take its
**reward from the deterministic outcome** (`metrics.jsonl.outcome==green`, `fail_start→fail_end`
delta, `fake_green_caught==0`, tests green) — **not** from the self-reported `confidence`. The
calibration ledger stays a human-facing mirror; it may only *discount* a verifier that is
systematically over-confident, and even that stays advisory.

---

## 3. The missing link — an advisory "experience index" + reinforced router

One new component, three read paths, zero new authority. Stdlib + the existing telemetry only
(no new deps — the plane-telemetry / loop-harness precedent).

### 3.1 The task-signature (deterministic, cheap)
A pure function `signature(task) → key`, reusing what the router already computes:
```
sig = {
  tags:      sorted specific trigger-tags that hit (router.scoreMatches),
  scope:     Class A | B  (from registry / classification),
  surface:   api | ui | infra | governance | test   (path/keyword heuristic),
  redline:   bool  (auth|money|RLS|PII|migrations|bulk — CLAUDE.md red-line globs),
}
key = stable hash of the above (bucket, not identity — many tasks share a signature)
```
This is the *context* of the contextual bandit. It is coarse on purpose: buckets must be big enough
to accumulate ≥ a handful of outcomes before they carry weight.

### 3.2 The arms (what the router will learn to weight)
An **arm** = `{ loop-or-DIRECT × agent-type × model × effort }`, e.g.
`RUN:error-fix-convergence × system-breaker × sonnet × medium`. The dispatch dimensions already
exist in telemetry: `agents{}` and `tokens.by_model{}` (`collect.ts`), loop id (`metrics.jsonl`).
Only **effort** and an explicit **arm stamp** on the run record are new (additive fields — §5 Stage 0).

### 3.3 The three read paths (all advisory, all report-first)
1. **Reinforced selection (bandit).** `experience-index` reads `metrics.jsonl` (+ plane-events),
   groups by `signature`, and for each arm computes a **quality-gated win-rate with a Bayesian prior**
   — Beta(1,1) start, `win` = `outcome==green ∧ fake_green_caught==0`, `loss` = `stall|abort|
   green-with-fake-green`. Rank arms by a simple Thompson sample or the Wilson lower bound (both
   stdlib-computable; no ML). Cost/eco is a **secondary tiebreak only** (never the primary reward —
   guards against the "always pick cheapest" degenerate, per Execute-Distill-Verify). Output: a
   ranked, **advisory** suggestion the router prints alongside its existing static decision.
2. **Pre-task failure-corpus retrieval (ExpeL/ReasoningBank).** Before the task, retrieve top-k by
   signature-overlap from: reflection WHYs (`docs/reflections/`), lessons (`docs/lessons/INDEX`),
   calibration **misses** (`predictions.jsonl` `gap!=hit` for this signature), and security-redblue
   findings. Return a compact "**what bit us here before**" pack — successes *and* failures
   (ReasoningBank's both-signals insight). This generalizes the keyword `pre-edit-lessons` hook to
   outcome-weighted, signature-keyed retrieval.
3. **Calibration-weighted advisory verifier.** When the decorrelated critics verify a result, surface
   the arm's historical calibration (from `predictions.jsonl`) as an **advisory discount**: "this
   arm's 0.9-confidence claims hit 62% here — read its confidence down." Never gates; mirrors §3 of
   model-calibration.

### 3.4 Why this is genuinely "reinforced" and not just reporting
The bandit's suggested order **changes** with each resolved run (append-only update to the
experience index), so the *next* dispatch for a signature reflects the *last* outcome on it — a
closed loop. But the change is confined to the **advisory suggestion**; the authority to act stays
where it is (§4). This is the RouteLLM/BaRP contextual-bandit pattern with the reward hardened to a
deterministic outcome and the actuator down-graded to advice.

---

## 4. What stays human / deterministic authority (non-negotiable)

Per pattern #4 and the model-calibration standing constraints — the reinforced router **informs**;
it never **decides**:

- **Gates are untouched.** `plane-guard.mjs`, `verify:all --ci`, `serious-gate`, `guard-bash`,
  ESLint Tier-1, the Stop-gate reflection pulse — all unchanged. The router's output is a printed
  suggestion + rationale; it emits **exit 0** always (like `router.ts` today) and writes only
  advisory records.
- **The calibration ledger stays a mirror.** The bandit reward is the **deterministic outcome**, not
  the self-graded `confidence`. Calibration only *discounts* a verifier, advisorily. Wiring
  `predictions.jsonl` into any gate remains forbidden (`model-calibration.md` §3, plane-guard
  advisory-forever HARD check).
- **Red-line topology overrides the bandit.** If `signature.redline` is set (auth/money/RLS/PII/
  migrations/bulk), the router's suggestion is forced to `escalate → Triadic Council + human`,
  regardless of any historical win-rate. History never buys a shortcut through a red-line.
- **Promotion advisory→default is a human/council act** (§5 Stage 3), never automatic — the same
  bar the librarian uses to promote a lesson to a guardrail.
- **Scope fence.** Governance-plane only (agent's own dispatch). Any product-plane reuse = red-line.
- **Anti-cheat on the reward.** A run with `fake_green_caught>0` or a skipped-no-env is a **loss**,
  not a win — the index cannot be gamed by cheap green.

---

## 5. Staging-first rollout (advisory-only first, gated promotion)

Four stages, each shippable dark (pattern #3), each with its own DoD. Runs against **staging
telemetry** first — the operator's "in staging" — before any influence on live dispatch.

- **Stage 0 — Instrument (additive, no behavior change).** Add `arm` (loop×agent×model×effort) and
  `signature` as **optional additive fields** to `MetricsLine` / the run record and to
  `plane-telemetry emit --metrics` (the plane-telemetry precedent: additive v1 fields, old readers
  unaffected). Backfill nothing; start accumulating. **DoD:** new runs carry `arm`+`signature`;
  `verify:all` still green; existing digests unchanged.
- **Stage 1 — Read-back digest (report-only, the smallest increment — see §7).**
  `scripts/experience-index.mjs` reads `metrics.jsonl` + `predictions.jsonl` (+ plane-events), groups
  by signature, prints per-arm quality-gated win-rate + retrieved failure-pack for a queried task.
  **No dispatch change.** **DoD:** `experience-index digest` and `experience-index --suggest "<task>"`
  produce ranked advice; a hermetic unit test feeds a fixture history and asserts the known-better
  arm ranks first (red→green); zero writes outside `loops/runs/`.
- **Stage 2 — Advisory router augment (shadow mode).** `router.ts` calls the index and **prints** the
  suggested arm + failure-pack next to its static decision, tagged `advisory`. Log
  `{suggested_arm, actual_arm, outcome}` to a shadow log. Nothing is forced. **DoD:** every routed
  loop-worthy command shows a suggestion; shadow log accumulates ≥ N paired rows on staging; router
  still exit-0 and never blocks.
- **Stage 3 — Gated promotion (human/council decision).** *Only after* the Stage-2 A/B (§6) shows a
  real lift, the suggested arm becomes the **default order** the lead agent sees first — still fully
  overridable, still never a gate. Promotion is logged like a librarian promotion + a REGRESSION-
  LEDGER row. Red-line signatures never auto-promote. **DoD:** promotion PR carries the A/B numbers;
  the default-order change is reversible in one line; a guardrail asserts the router still exits 0 and
  still ignores the index on red-line signatures.

---

## 6. Proof / DoD — does it actually improve outcomes?

The Mandatory Proof Rule applies: the claim "the swarm learns" must have an assertion that fails when
it doesn't. Two layers:

### 6.1 Hermetic (red→green, gates the build)
- **Aggregator correctness:** feed `experience-index` a fixture `metrics.jsonl` where arm A wins 8/10
  and arm B wins 2/10 on signature S → assert A ranks above B; flip the fixture → assert the ranking
  flips. (Proves the loop is actually closed — output changes with recorded outcome.)
- **Anti-cheat:** a fixture where the cheapest arm has `fake_green_caught>0` → assert it is **not**
  ranked first (reward is quality-gated).
- **Authority invariants:** assert `experience-index` and the augmented `router.ts` exit 0 on all
  inputs; assert red-line signature → suggestion == `escalate`; assert no write touches gate state or
  `predictions.jsonl`.

### 6.2 Live A/B on staging (proves lift, not just plumbing)
- **Design:** Stage-2 shadow mode records `{signature, suggested_arm, actual_arm, outcome, per_resolved, cost_usd}`
  per run. Split into **followed-the-suggestion** vs **diverged** cohorts (a natural A/B — the lead
  agent doesn't always take the advice).
- **Primary metric:** green-rate (`outcome==green ∧ fake_green_caught==0`) of the suggested arm vs the
  realized arm, **on a held-out set of signatures** not used to build the index (guard against
  overfitting a tiny history).
- **Secondary:** `per_resolved` and `cost_usd` at equal quality; and a **calibration guardrail** —
  the Brier score of `predictions.jsonl` must **not worsen** after promotion (if the router is
  steering toward arms whose confidence is miscalibrated, that shows up here).
- **Real data sources:** `loops/runs/metrics.jsonl` (outcome/per_resolved/cost), `predictions.jsonl`
  (Brier), `plane-events-*.jsonl` (fail counts), the Stage-2 shadow log.
- **Success bar:** a meaningful, sustained lift in green-rate **or** a reduction in per_resolved/cost
  at equal green-rate on the held-out signatures, with Brier not worse. Below the bar → the index
  stays advisory-only (Stage 2 forever is an acceptable terminal state — advice with no promotion).
- **Honest limit (state it, per the repo's honesty norm):** `metrics.jsonl` is currently sparse (7
  rows, mostly degenerate autoupgrade zeros — meta-loop-audit-2026-07-02) and most-used loops
  (demo-builder ~60 runs, acquisition) write **bespoke JSON that bypasses `finalize`**. So the index
  is **data-starved on day one.** The precondition for any real A/B is the audit's own backlog item:
  route demo-builder/acquisition through `finalize` and keep `metrics.jsonl` git-tracked. Measurement
  is the precondition for the ratchet (pattern #12). This is a dependency, not a blocker for Stage 0/1.

---

## 7. The smallest first buildable increment

**`scripts/experience-index.mjs` — a report-only read-back over the existing outcome stores (Stage 1),
plus the additive `arm`+`signature` stamp it needs (Stage 0).** Node stdlib only, no new deps, no gate,
exit 0 always.

Concretely, the one increment delivers:
1. `signature(task)` — the deterministic bucket function (reuses `router.scoreMatches` tags + red-line globs).
2. `experience-index digest` — reads `loops/runs/metrics.jsonl` + `predictions.jsonl`, groups by
   signature, prints per-arm **quality-gated win-rate (Beta prior + Wilson lower bound)** and the
   calibration gap. Report-only.
3. `experience-index --suggest "<task>"` — ranks arms for that signature and retrieves the top-k
   failure/lesson/finding pack. Advisory text; the router does not yet consume it.
4. A hermetic fixture test (§6.1 correctness + anti-cheat + exit-0) — red→green.

This is the missing read-back path in its minimal honest form: it makes stored outcomes *visible as a
next-choice recommendation* without touching any authority. Stage 2 (wire it into `router.ts` in
shadow mode) and Stage 3 (gated promotion) build on it only after it has data and a passing A/B.

---

## Sources
- Reflexion — [Shinn & Cassano et al., 2023](https://www.semanticscholar.org/paper/Reflexion:-language-agents-with-verbal-learning-Shinn-Cassano/0671fd553dd670a4e820553a974bc48040ba0819)
- ExpeL: LLM Agents Are Experiential Learners — [arXiv 2308.10144](https://arxiv.org/pdf/2308.10144)
- ReasoningBank: Scaling Agent Self-Evolving with Reasoning Memory — [arXiv 2509.25140](https://arxiv.org/abs/2509.25140)
- Agent Workflow Memory / Trajectory-Informed Memory Generation — [arXiv 2603.10600](https://arxiv.org/html/2603.10600v1)
- Contextual Experience Replay for Self-Improvement of Language Agents — [arXiv 2506.06698](https://arxiv.org/pdf/2506.06698)
- Learning to Route LLMs from Bandit Feedback (BaRP) — [arXiv 2510.07429](https://arxiv.org/html/2510.07429v1)
- Optimal-Agent-Selection / MasRouter (state-aware multi-agent routing) — [arXiv 2511.02200](https://arxiv.org/pdf/2511.02200)
- AgentRM: Enhancing Agent Generalization with Reward Modeling — [arXiv 2502.18407](https://arxiv.org/pdf/2502.18407)
- Agentic Reward Modeling (verifiable correctness + preferences) — [arXiv 2502.19328](https://arxiv.org/pdf/2502.19328)
- Outcome-Supervised Reward Models (ORM) — [EmergentMind](https://www.emergentmind.com/topics/outcome-supervised-reward-model-orm)
- Scalable Best-of-N via Self-Certainty — [arXiv 2502.18581](https://arxiv.org/html/2502.18581v1)
- Execute-Distill-Verify (escaping the self-confirmation trap) — [arXiv 2606.24428](https://arxiv.org/html/2606.24428v1)
</content>
