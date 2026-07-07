# Token-economy comparison — native vs the full reduction stack (measured, 2026-07-05)

Operator ask: "run probes to actually analyze and compare token reduction with all my hacks…
a comparison table of native token usage vs all my layers/rules, for different scenarios,
especially agentic coding."

**Method.** Two classes of measurement, one ruler each:
- **Live A/B lanes** — identical task prompt dispatched twice: NATIVE (general-purpose agent,
  file-by-file Read/Grep only, all token-economy tooling explicitly forbidden) vs OPTIMIZED
  (Explore agent + graph-first codebase-memory CLI + `repowise distill` + distilled-return rule).
  Numbers are **real harness tokens for the whole subagent dispatch** (the truest cost), from the
  task-completion usage report. Deliverables were checked for equivalence before comparing cost.
- **Local deterministic benches** — js-tiktoken cl100k on payload files (`tools/vsa/cli.mjs`),
  image cost = w·h/750. Same-ruler ratios hold across tokenizers.

Total probe spend: ~403K subagent tokens across 7 dispatches (incl. one corrupted lane, see §6).

## 1. Agentic coding — live A/B (fresh, real harness tokens)

| scenario | native (general-purpose, file-by-file) | optimized (Explore + graph-first + distill) | Δ tokens | Δ wall-clock | quality |
|---|---|---|---|---|---|
| **Trace + patch plan** (order→PICKED_UP chain: route→state-machine→DB write→WS; then audit-log patch plan) | 72,588 tok · 24 tools · 262s | 58,422 tok · 17 tools · 174s | **−19.5%** | −33% | both correct+complete; native found the slightly sharper minimal patch (reuse `order_status_history`, thread `actor`) — optimized noted the same reuse as a recommendation |
| **Audit sweep** (every writer of `orders.status` in apps/api/src) | 136,892 tok · 51 tools · 424s | 64,813 tok · 16 tools · 157s | **−52.7%** | −63% | same canonical writer, same 16 funnel callers, same bypass sites, same near-miss list; native's extra 72K bought 3 bonus observations (see §7) |
| **Aggregate (both pairs)** | 209,480 | 123,235 | **−41.2%** | −52% | equivalent core deliverables |

**Reading:** the stack's win **scales with search breadth**. A narrow trace (one chain, few files)
saves ~20%; a whole-surface sweep saves ~53% because the graph replaces dozens of speculative
file reads. Prior structural-question A/B (−54.9%, docs/research/codebase-memory-mcp-eval-2026-07-05.md)
sits at the sweep end of the same curve. Honest headline for agentic coding: **−20% (narrow task)
to −55% (broad sweep)**, not a flat −55%.

## 2. Dispatch floor — the narrow-tool-grant rule (fresh)

Trivial "reply OK" probe, zero tool uses, measures pure per-lane overhead (system prompt + tool schemas):

| lane type | tokens for "OK" | Δ |
|---|---|---|
| general-purpose (broad grant) | 35,753 | — |
| Explore (narrow grant) | 16,960 | **−52.6% (−18.8K per lane)** |

Every map-reduce fan-out lane that can be read-only should be Explore — the saving lands before
any work happens. (The ~42K/lane figure in agentic-map-reduce.md included MCP-heavy grants.)

## 3. Data-payload + state layers (deterministic, re-run today)

| scenario | native | with layer | Δ | layer |
|---|---|---|---|---|
| 93KB products JSON into a prompt | 33,928 | 21,068 (+90 spec) | **−37.6%** | VSA1 frame |
| 4-fixture aggregate (products/menu/flags/location) | 54,780 | 36,012 | **−34.3%** | VSA1 frame (lossless, round-trip pinned) |
| small irregular payload (location-info) | 419 | 384 | −8.4% | frame ≈ floor; router decides |
| whole framed dispatch, live A/B (prior, b5c9c532) | 72,147 | 46,725 | **−35.2%** | dispatch.mjs framed attachment |
| lane/session state (rebuild h_t) | 968 | 844 | −12.8% | h_t frame |
| dispatch state, 200 orders | 11,006 | 1,399 | **−87%** | VSA-VIZ 1024² image |
| dispatch state incl. 5×D=512 VSA vectors | 16,386 | 1,399 | **−91.5%** | VSA-VIZ image |
| dispatch state, 5 orders (BELOW crossover) | 400 | 1,399 | **+250% — image LOSES** | crossover ≈25-30 entities; route.mjs picks |
| steady-state dispatch tick | 1,399 (image) | 88 | **−93.7%** (−99%+ vs JSON at scale) | macro 256² + LEGEND_MIN (113 tok once) |
| tick with 4 changed drivers of 50 | 88 | 11 | −87.5% | delta frame |
| cold dispatch of 120 orders (fresh re-run) | 9,622 | 1,652 (+65 cached judge ×38 calls) | **−82.2%**, 68.3% auto-resolved at $0 | blind orchestration (rule −1) |
| steady-state ticks (prior sim) | every tick pays | 0/100 ticks called the LLM | **≈−100%** | blind orchestration |
| recall/shortlist over 106-memory corpus | 1 LLM call | 0 tokens (local hv match) | −100% for the recall step | vsa match |

## 4. Reading/output layers (fresh)

| scenario | native | with layer | Δ | layer |
|---|---|---|---|---|
| `git log --stat -30` output | 10,229 | 803 | **−92.1%** | `repowise distill` (omitted lines recoverable via `repowise expand`) |
| small clean test output (562 tok) | 562 | 562 | 0% — honest no-op | distill below crossover |
| understand orders.ts (986-line hotspot) | 11,629 (full Read) | 1,108 skeleton, verified | **−90.5%** (9.5% of full; beats the documented ~37% claim on this file) | repowise get_context skeleton |
| structural question, whole-subagent (prior A/B) | 90,976 | 41,047 | −54.9% | codebase-memory graph |

## 5. The stack, in rule order (what to reach for first)

| rule | measured effect | crossover / when it loses |
|---|---|---|
| **−1 blind orchestration** — don't send state; deterministic resolver, LLM only for irreducible tradeoffs | −82% cold, ≈−100% steady | resolver must be correct; calibration-dependent |
| **0 measure, don't assume** — route.mjs picks cheapest encoding | prevents negative "savings" | — (this rule exists because of the losses below) |
| **narrow tool grants** (map-reduce) | −18.8K per lane (−52.6% of floor) | lane genuinely needs write/broad tools |
| **graph-first** (codebase-memory) | −20% narrow → −55% broad agentic tasks | tiny tasks in 1-2 known files: overhead ≈ win |
| **skeleton/distill** (repowise) | −90% file understanding, −92% noisy output | small clean files/outputs: no-op |
| **VSA1 frames** (data payloads) | −34% aggregate, −35% whole-dispatch | <~1KB or irregular payloads (−8% or worse) |
| **viz/macro/delta** (dispatch state) | −87…−99% at scale | <25-30 entities: image LOSES (+250% measured) |
| **h_t state frames** | −13% | small states — modest, but replaces transcript dumps (the real win) |

## 6. Reliability + honesty notes

- **1 of 7 lanes corrupted at dispatch** (Explore lane returned garbage after 17,405 tok, 0 tools;
  retry succeeded). Budget ~15% retry overhead into fan-out plans; a corrupted lane costs ≈ one floor.
- The optimized lanes' savings come from BOTH the narrow grant (§2) and the method; with matched
  agent types the method-only delta would be smaller on narrow tasks. The stack as-practiced bundles
  them, so the bundled number is the operationally honest one.
- Native lanes were forbidden the tooling by prompt; both complied (2N explicitly ignored the
  PostToolUse repowise nudges).
- Quality is NOT free: on the audit sweep the native lane's extra 72K bought 3 extra observations
  (below). For red-line audits, consider optimized sweep + one cheap "what did we miss" critic lane
  rather than paying full native.

## 7. APPLIED (2026-07-05, operator directive "always use the most token-optimized approach")

- **AGENTS.md "RULE: TOKEN ROUTER"** — the binding task-shape→route table + quality floors,
  for every agent and subagent; leads embed the one-line router in every dispatch prompt
  (subagents don't reliably load AGENTS.md).
- **docs/operating-model/agentic-map-reduce.md** — floor figure refreshed (35,753 GP vs 16,960
  Explore, −18.8K/read-only lane) + the ~15% lane-retry budget.
- **CLAUDE.md** — protected zone (hook-gated governance); the equivalent bullet is PROPOSED for
  manual operator apply: `docs/operating-model/proposed-claude-md/token-router-rule.md`.
- Memory hub updated ([[vsa-token-economy-2026-07-05]]) with the honest −20%…−55% range so future
  sessions don't over-quote.

**Quality floors (why audits/reasoning don't degrade):** red-line sweeps run optimized + one
independent "what-did-we-miss" critic lane (≈105K, still under the 137K native, recovers the
native-depth findings — §6); reasoning/design/council stays plain-prose on the full model;
byte-reads remain mandatory before edits and for load-bearing audit claims; doubt/council ladders
are never skipped for cost.

## 8. Long-run savings estimate (grounded in measured anchors)

**Dev-agent side.** Modeling a heavy orchestration day as 8 task lanes (5 broad + 3 narrow, the
measured per-lane costs): native = 5×136.9K + 3×72.6K ≈ **902K**; routed = 5×64.8K + 3×58.4K ≈
**499K** (−45%); add ~15% retry overhead and two red-line critic lanes (+80K) → **~654K = −27%
with full quality floors**, −36…−45% on days without red-line audits. Solo/narrow days: −20…−25%.

> **Blended honest estimate: ~30–40% of all dev-agent tokens.** At the recent cadence
> (~1–3M subagent tokens on a heavy day — this probe session alone spent 403K on 7 dispatches),
> that is **~0.5–1M tokens/day ≈ 15–25M tokens/month** at sustained heavy usage — roughly one
> day in three free.

**Stacking levers not yet shipped** (from `2026-07-04-harness-token-audit.md`, hook/governance-
gated, additive on top): router-nudge narrowing ~15–25K/session; REGRESSION-LEDGER index
~20K/librarian-run (grows weekly); CLAUDE.md CORE/REFERENCE split ~2–2.5K × every lane; unused
MCP connector trim ~2–4K/lane. Together roughly another **5–10% of session tokens**.

**Product runtime side (the largest absolute save).** Once blind orchestration + macro/delta is
live-wired into the Rust dispatch tick (banked bench, 86.4K calls/mo at Sonnet rates): naive rich
JSON ≈ $12,283/mo → macro ≈ $78/mo → delta+blind ≈ **$58/mo (−99.5%, ~160×)** — steady-state
ticks measured at 0/100 LLM calls. This is a product-cost line, independent of dev tokens.

## 9. Incidental findings from probe 2N (escalate-class, NOT fixed — flag only)

1. **Owner pickup proxy WS/DB divergence** — `routes/owner/dashboard.ts:379-429`: flips the
   assignment to `picked_up` and broadcasts `status:'PICKED_UP'` on the dashboard WS channel (:428)
   but never persists `orders.status` (row stays IN_DELIVERY). UI/DB state divergence; contract-class.
2. `SCHEDULED` exists in `OrderStatusEnum` but has no writer anywhere in apps/api/src.
3. `PENDING` is insert-only (never an UPDATE target) — consistent with the timeout sweep design.

## 10. B0 baseline — measured TOKEN ROUTER / MODEL ROUTING violation rate (2026-07-06)

STRUCTURE-UPGRADE.md Part B, step **B0 · Baseline before enforcing**. Source: `scripts/audit-token-router.mjs`
(deterministic, $0, read-only) over **95 session transcripts** (`~/.claude/projects/-root-dowiz/*.jsonl`).
Re-run: `node scripts/audit-token-router.mjs`. Measured, not assumed (VSA rule 0).

| rule (auditor metric) | count | rate | tier |
|---|---|---|---|
| **Agent dispatches (total)** | **1027** | — | — |
| (a) no explicit `model:` | **885** | **86%** of dispatches | EXIT-1 |
| (e) `model: fable` | 43 | 4% (across 5 sessions) | EXIT-1 |
| (d) unstamped >1KB raw JSON in prompt | 8 | 0.8% | advisory |
| (c) batching-miss candidates (≥3 single-tool turns) | 86 | — | advisory, human-judged |
| peak lead-session context | 999,580 | ⚠ ran to the full window (≫300K recycle) | datapoint |
| (b) per-lane 80K crossings | — | **not measurable from lead transcript** (sub-agent sidechains never appear: `isSidechain` False on all 2464 marked lines) | honest gap |

**Model-less dispatches (885) by `subagent_type`** — the surface B1's `model:`-required check would DENY:

| subagent_type | model-less count |
|---|---|
| general-purpose | 340 |
| system-architect / system-breaker / counsel (triad council) | 110 / 105 / 93 |
| Explore (read-only, inherits parent by convention) | 106 |
| _(no subagent_type → default agent)_ | 54 |
| security-sentinel / invariant-guardian / test-scout / others | 78 (combined) |

**Compliant tail:** only ~99 of 1027 dispatches (**≈10%**) carry an explicit non-Fable `model:`
(46 gp=sonnet, 20 default=opus, 9 invariant-guardian=opus, 9 security-sentinel=opus, others).

### Load-bearing finding for B1 (drives a rollout decision, recorded in PROGRESS.md)

A **blind hard-DENY** on "`model:` absent" would have blocked **~90% of the current dispatch
pattern on day one** — including 106 legitimate `Explore` read-only lanes that inherit the parent
model by convention, and the triad-council agents. This is precisely the #47 over-block failure
mode the plan warns about, now quantified rather than feared. **B1 must not ship as a blind
hard-deny.** The data supports a **warn-then-ratchet rollout** (PostToolUse/soft nudge that logs
`_hev` WARN + names the fix, promoted to PreToolUse DENY only after the stamp habit is measurable in
the log) OR a hard-deny gated on a grace flag — a genuine design fork surfaced for the operator.
The 43 `fable` dispatches are NOT all violations: the operator sanctioned Fable for *plan authoring*
this arc — B1's human-only expiring `fable-override` is the correct discriminator, so the raw count
overstates the true violation rate (Fable on 2 `Explore` lanes is off-policy regardless).

### Post-B1 live signal (2026-07-06) — the gate is ARMED, not just registered

Part B exit-gate criterion "`_hev` shows real ALLOW/DENY traffic." First session with the gate live
produced real decisions in `.claude/logs/harness-events.jsonl`:
`agent-dispatch-gate deny` ×1 (a `model: fable` dispatch REFUSED), `agent-dispatch-gate warn` ×1
(a model-less Explore lane nudged), `distill-nudge warn` ×1 (an ~8.3K undistilled Bash dump).
The gate caught the lead's own behavior and changed it (operator's "use cheaper models" now enforced).
**The full B0-vs-post-B1 violation DELTA needs session accumulation** — re-run `audit-token-router`
over the next N sessions; the model-less rate trend (baseline 86%) is the ratchet trigger for
promoting Check 1 warn→deny. Tracked in PROGRESS.md NEXT SEQUENCE.

## 11. Re-measure 2026-07-07 — the ratchet DELTA + first $-by-model audit (answers the operator's audit question)

Operator ask: "reduce as much as you can without losing quality; evaluate current token-reduction
tools, find patterns/cross-patterns; comparison in % to the last report; and — which model takes the
biggest share of the token-budget in **money**, reasoning-heavy Opus or huge-context Haiku/Sonnet?"
Method: two deterministic $0 auditors re-run over the now-**107** transcripts (was 95 at §10),
both with hermetic self-tests incl. a RED case (VbM). Re-run: `node scripts/audit-token-router.mjs`
and `node scripts/audit-model-spend.mjs`.

### 11a. TOKEN ROUTER ratchet — % vs the §10 baseline

| metric | §10 baseline (95 sess) | now (107 sess, agg) | **newest 12 by mtime** |
|---|---|---|---|
| Agent dispatches | 1027 | 1042 | 13 |
| model-less rate | **86%** | **85%** (lagging, history-dominated) | **0%** ⟵ leading indicator |
| `model: fable` | 43 | 46 | 2 (both sanctioned one-shot audits) |

**Reading:** the aggregate barely moved (86→85%) **by construction** — 95 pre-gate sessions dominate
the mean. The signal is the leading edge: the 12 newest sessions carry **0 model-less dispatches**
(vs the ~90% day-one rate §10 predicted a blind deny would block). The B1 warn-then-ratchet gate
LANDED; the aggregate will decay toward compliance only as pre-gate history ages out. This is the
`_hev` DELTA §10 left open, now measured — Check-1 warn→deny is defensible on the leading trend.

### 11b. $-by-model — the audit question, answered (lead-loop spend, 107 transcripts)

| model | share of $ | dollars | note |
|---|---|---|---|
| **opus** | **89.3%** | **$14,585** | the reasoning lead loop (pinned Opus 4.8, 1M ctx @ standard price) |
| fable | 10.6% | $1,738 | sanctioned one-shot audits |
| haiku | 0.0% | $0.46 | doer lanes are ~free per token |
| sonnet | — | $0 | not used in the lead loop |
| **TOTAL (visible)** | | **$16,323** | over 21.1B billed tokens |

**Answer: it's Opus, ~89%** — decisively the reasoning lead loop, NOT huge-context Haiku/Sonnet
(Haiku is $0.46). Pricing from the authoritative claude-api catalog (Opus 4.8 = $5/$25 per MTok, **not**
$15/$75; 1M context at standard price, no long-context premium). Scope caveat (same as §10's honest
gap): sub-agent sidechains never appear in lead transcripts, so this is **lead-loop only** — the
largest single visible line, and precisely the one the "which model dominates" question is about.

### 11c. $-by-LEVER — why the operator's 6 methods have near-zero headroom left (the cross-pattern)

Decomposing the same spend by *what the tokens are* (Opus-priced approximation of the mix):

| lever | share of $ | what cuts it | status |
|---|---|---|---|
| **cache-read** (re-reading grown context every turn) | **62.8%** ($10,249) | shorter sessions / context-recycle / compaction | ⟵ **the real lever; NONE of the 6 methods touch it** |
| cache-write (establishing cache) | 18.7% ($3,059) | stable frozen prefixes (CLAUDE.md CORE split) | partially shipped |
| output | 12.7% ($2,078) | `effort` tuning · output-constraining (method #4) · route to Haiku (methods #2/#6) | real headroom |
| **fresh input** | **0.4%** ($71) | distill · graph-first · VSA1 frames · state-delta (methods #1,#3,#5) | **saturated — near-zero ROI** |

**The cross-pattern, measured.** The operator's advice maps onto the spend as follows:
- **#1 Context Distillation, #3 Prompt Caching, #5 State-Delta** → already shipped AND target the
  **0.4% fresh-input line**. Prompt caching in particular is already doing its job — 62.8% of $ is
  cache-*read* at 0.1×, i.e. caching is already saving ~90% on that volume. Pushing these harder is
  ground-truth-negative ROI (per §0·GP: measure before cutting/building — this measurement says stop).
- **#4 Output Constraining** → real, hits the 12.7% output line; low-risk for **subagent returns**
  (schema-forced) and lead-loop `effort` on routine turns.
- **#2 Speculative Execution + #6 Complexity-Threshold Routing** → the one genuinely-unspent lever:
  route routine turns to Haiku doers instead of the Opus lead loop. Opus output is $25/M vs Haiku
  $5/M (5×); Opus is 89% of $. Constraint: the lead loop can't downshift mid-session (model switch
  invalidates the cache — see shared/prompt-caching.md), so this is a *dispatch-more-to-Haiku*
  pattern, not a lead-model swap. MODEL ROUTING already ships the mechanism; the data justifies leaning on it.
- **The single biggest lever — cache-read at 62.8% — is addressed by NONE of the six.** It is a
  **session-length-discipline** problem: peak lead context hit **999,580** (§10), i.e. the 300K
  recycle rule ([[token-lifecycle-thresholds-2026-07-05]]) was breached. A ~1M-token session pays
  ~$0.50/turn just to re-read its own cached prefix, every turn. Enforcing recycle + adopting
  server-side **context-editing/compaction** (clear stale tool results/thinking; beta) attacks the
  largest cost line no frame or codec can reach.

**Net honest conclusion.** The reduction stack (frames/distill/graph/blind-orch) is *already
excellent* and has driven fresh input to 0.4% of $. Further token cuts do **not** come from more of
that tooling — they come from (1) **session-recycle enforcement** (biggest, red-line-adjacent:
touches settings/hooks → operator-gated) and (2) **more Haiku dispatch + `effort`/output-constraint
on the lead loop** (method #2/#4/#6, mechanism already shipped). Both are discipline/routing changes,
not new codecs. `scripts/audit-model-spend.mjs` is the new durable instrument to track them
(the per-lane `subagent_tokens` follow-up from `2026-07-04-token-reduction-synthesis.md` §5 —
partially discharged for the lead loop; sub-agent lane $ still needs telemetry at emit time).
