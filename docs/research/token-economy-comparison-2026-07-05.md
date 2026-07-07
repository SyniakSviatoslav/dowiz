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

## 12. ENFORCEMENT APPLIED — the §11 levers wired to teeth (2026-07-07, operator "allow and enforce")

§11 *measured* where the $ goes; §12 is the *enforcement delta* — the mechanisms shipped THIS session so
the reduction is armed, not advisory. Ground truth unchanged (108 transcripts now, $16,340 lead-loop
visible; cache-read still 62.8%, Opus still 89.4%). What changed is what the harness now *forces*.
Each row ships with a falsifiable RED (VbM). Re-run the same two auditors to re-check.

| lever | §11 state (measured) | §12 enforcement shipped | falsifiable re-check |
|---|---|---|---|
| **A1** cache-read 62.8% — session length | single 300K directive; peak hit **452,886** | `context-budget-guard.sh` → **two-tier** WARN@300K (soft, start handoff) / **HARD@400K** (mandatory wrap+/clear) + hermetic `--self-test` (RED: WARN≠HARD; 452,886 parsed) | `bash .claude/hooks/context-budget-guard.sh --self-test` (9 asserts, incl. 2 RED) |
| **B0** Opus 89% $-share | lead loop pinned **Opus 4.8** | Opus→**Haiku 4.5** pin (operator "push further" 2026-07-07 late; supersedes the earlier same-day Sonnet pin — ~5× cheaper than Opus), AND **fixed the apply path**: `proposed-settings/settings.json` was STALE (would have *deleted* the A1+B1+subagent-return hooks on `cp`) → regenerated so `proposed − model-key ≡ live`. **Safety rail:** red-line reasoning (money/auth/RLS/migrations — 0b-5/1.2/2.2/2.3) escalates to `opus` via `doubt-escalation`. | `diff <(grep -v '"model"' proposed-settings/settings.json) .claude/settings.json` → identical |
| **B5** free/external LLMs (OpenRouter/Gemini/Groq/…) | AGENTS.md: bridge DEAD 2026-06-22 | **GATED policy** (not wired): free tiers only for non-proprietary/non-red-line; ground-truth privacy scan below | AGENTS.md MODEL ROUTING "Free/external LLM APIs" bullet |
| **B1** model-less dispatch | warn-only (86% agg / **0% newest-12**) | `agent-dispatch-gate.sh` Check-1 **warn→DENY by default** + Explore/fork inherit-parent carve-out + `TOKEN_GATE_MODE=warn` escape hatch | `node scripts/guardrail-token-gates.mjs` (deny-default, carve-out, escape hatch — 9 Check-1 asserts) |
| **B2** Haiku dispatch (output line) | mechanism shipped, under-used | AGENTS.md MODEL ROUTING: read-only/analytical lanes **default to explicit `model: haiku`**; opus = escalation-only | audit-token-router model-mix on future sessions |
| **A3** context-editing/compaction | not evaluated | **verdict below** — harness-owned, not a repo change | n/a |

**The apply-path bug was the load-bearing find.** The one manual step the prior handoff prescribed —
`cp docs/operating-model/proposed-settings/settings.json .claude/settings.json` — would have silently
**regressed the entire enforcement stack**: the staged file predated the live hooks and carried an older
`hooks` block (no `agent-dispatch-gate`, no `context-budget-guard`, no `subagent-return-guard`; it re-added
`route-request`/`loop-detector`/`pre-edit-lessons`). Applying the Sonnet pin would have deleted A1 and B1.
Fixed: the staged file is now byte-for-byte the live settings + the single `model` key (proven by the diff
check above). This is the §0·GP move — a deterministic check (diff) catching what a proxy read (“looks like
just a model pin”) would have shipped.

**A3 — context-editing / compaction (`context_management.edits`) verdict.** *Direction is right, ownership
is wrong for a repo change.* The beta (`clear_tool_uses_20250919` / `clear_thinking_20251015`) prunes stale
tool-results/thinking from the re-read prefix — it attacks cache-read (62.8%, the dominant line) **at the
source**, the one lever no frame/codec reaches. BUT it is an **API-request parameter set by the Claude Code
harness**, not configurable from `settings.json` or a hook — the repo cannot wire it. The repo-controllable
equivalents are already in hand: **A1 session-recycle** (shorter sessions ⇒ smaller re-read prefix) + Claude
Code's built-in auto-compaction / `/compact`. Recommendation: rely on A1 + `/compact` at the WARN tier; treat
native `context_management` as an upstream-CLI ask, not a repo task. No code shipped (correctly — §0·GP: don't
build machinery you can't control).

**B5 — free / external LLM APIs (OpenRouter, OpenCode, `free-llm-api-resources`) ground-truth verdict.**
Operator "use free APIs for other tasks" (2026-07-07). §0·GP requires measuring before adopting (the bridge
was dead 2026-06-22). Measured the current catalog (WebFetch of `cheahjs/free-llm-api-resources`):

| provider | free limit | trains on / retains inputs? |
|---|---|---|
| OpenRouter (`:free`) | 20 req/min, **50 req/day** (1k with $10 topup) | unspecified ⇒ treat as retained |
| Google AI Studio (Gemini) | ~20 req/day/model | **yes** — "data used for training outside UK/CH/EEA/EU" |
| Mistral La Plateforme | 1 req/s, 1B tok/mo | **yes** — free tier = **opt-in to training** |
| Groq / Cerebras | ~1k req/day / 1M tok/day | unspecified ⇒ treat as retained |
| GitHub Models / Cohere / CF Workers AI | very low | unspecified ⇒ treat as retained |
| OpenCode Zen (free models) | unspecified | **yes** — "may use data for improvement" |

**Verdict: adopt ONLY for non-proprietary, non-red-line tasks; NEVER route dowiz code/PII/secrets out.** Two
hard facts kill the "free doer tier" idea for this codebase: (1) **data governance** — every provider that
*states* a policy trains on or retains free-tier inputs, and the rest are silent (⇒ retained by default); a
codebase with money/auth/RLS/PII cannot egress source to a training-on-input free tier. (2) **rate limits** —
20–1000 req/day is far below one agentic doer session's call volume, so even for non-sensitive work it can't
be the primary tier. Legit uses: public-info research, decorrelated second opinions on PUBLIC facts, prose
drafting from non-sensitive inputs — always *advisory*, never red-line authority. For in-Anthropic doer
volume, **Haiku ($1/$5, no data egress, no rate cap)** is the tier — which is exactly what the B0 Haiku pin
now makes the default. Wiring the OpenRouter bridge is a **staged, opt-in tool gated on an operator
data-governance sign-off + keys** (BLOCKER), not a default switch.

**What is NOT yet re-measurable (honest gap).** The Haiku pin is *staged*, not applied (operator `cp` +
next session). So the $-by-model table is UNCHANGED — Opus is still 89% because every session in the corpus
predates the pin. The falsifiable claim is deferred by construction: **re-run `audit-model-spend.mjs` after
N Haiku sessions; the Haiku share must rise and Opus fall below 80% (except red-line escalations), or the pin
did not take** (§B0). B1's deny is live *now* (this session's own dispatches are gated); its aggregate signal
likewise decays in only as pre-gate history ages out (§11a).

## 13. MVP token/$ budget — how much to ship the Sovereign Core MVP, and the account question

Operator ask (2026-07-07): "estimate how much token will be needed to ship MVP and how much is left on my
Claude account." Answered from the same ground-truth transcripts — **empirical per-step cost × steps
remaining**, not a guess.

**Empirical per-step anchor (lead-loop $, measured).** The newest 15 sessions — all the recent
Sovereign-Core / token-economy work, the right analog for the remaining MVP steps — cost **$792.84 total,
mean $52.86/session, median $54.86** (range $5.52–$210.08). This is the *post*-context-budget-guard regime
(shorter sessions); the all-history mean is $151/session, inflated by the pre-recycle marathons (one hit
999,580 ctx). A Sovereign-Core "step" ≈ one focused session; red-line / large-"L" steps run 1.5–2.5×.

**Steps remaining to the MVP exit gate** (from `docs/design/sovereign-core-mvp/PROGRESS.md` + GRAND-PLAN;
DONE = Part A/B, 0b-1/0b-2/0b-3, Validation 1–6):

| step | size | red-line | est. sessions |
|---|---|---|---|
| 0b-4 Hard-Truth L1–2 (mostly satisfied by 0b-3 proptests — review/close) | light | no | 0.5 |
| 0b-5 shell flip to `kernel::decide` (keystone; staging deploy + shadow-diff) | medium | **yes** | 2.0 |
| 0b-6 CI + cargo-deny (`.github` operator-gated) | light | no | 1.0 |
| 1.1 hub events-in | medium | **yes** | 1.5 |
| **1.2 persistent event log** ("L") | large | **yes** | 2.5 |
| 1.3 / 1.5 hub reads + placement | light×2 | no | 2.0 |
| 2.1 MVP surface wiring | medium | no | 1.0 |
| **2.2 checkout** ("L", money) | large | **yes** | 2.5 |
| 2.3 MVP exit hardening | medium | **yes** | 1.5 |
| A3 orders-split (unblocks at 0b-5) + Validation tail | medium | mixed | 2.0 |
| **subtotal** | | | **~16.5 sessions** |
| +15% fan-out/retry + red-line verification overhead (measured ops-note) | | | **~19 sessions** |

**The number, four ways (lead-loop visible $; add ~20–40% for sub-agent sidechains on fan-out steps).**
The Haiku pin is ~5× cheaper than Opus on *every* line — input, output, AND the dominant 62.8% cache-read
(Haiku input×0.1 = $0.10/M vs Opus $0.50/M). But red-line steps (0b-5/1.1/1.2/2.2/2.3) escalate their
*reasoning/verification turns* to opus, so those sessions are ~0.45× an all-Opus session, not 0.20×; the
blend of ~11 routine + ~8 red-line-ish sessions is what the Haiku rows price:

| basis | per-session (blended) | × ~19 | note |
|---|---|---|---|
| recent **Opus** median (today's regime) | ~$55 | **≈ $1,045** | if NO pin applied |
| **Sonnet 5** pin (superseded) | ~$34 | ≈ $650 | earlier same-day directive |
| **Haiku 4.5** pin + opus red-line escalation (the "push further" target) | ~$18–25 | **≈ $340–475** | routine ≈$11, red-line ≈$25 |
| Haiku pin + A1 shorter-session recycle | ~$14–20 | **≈ $270–380** | if recycle discipline holds |

→ **Ship-MVP estimate with the Haiku pin: ≈ $300–500 lead-loop** (most-likely **~$350–425**), ≈ **$450–800
all-in** including sub-agent lanes — down from ~$1,045 all-Opus. In billed-token terms ≈ **0.8–1.4 B billed
tokens** (mostly cheap cache-read at 0.1×); the unit that matters is $. Caveat: the Haiku-blend assumes the
opus-on-red-line rail holds — if a red-line step is (wrongly) left on Haiku and ships a money/parity bug, the
rework cost dwarfs the savings, so the rail is load-bearing, not optional.

**"How much is left on my Claude account" — the honest answer.** *This environment cannot read the
Anthropic billing balance* (no billing API here) — check **console.anthropic.com → Billing / Usage** for the
authoritative number. Two cases change the meaning of "left":
- **Pay-as-you-go API credits:** the $16,340 above is *real depletion*; remaining-MVP ≈ $550–1,050 more. If
  the console shows **≥ ~$1,200 credit**, the MVP is covered with headroom; the Sonnet pin roughly halves the
  remaining burn.
- **Claude subscription (Max/Team, which Claude Code can run on):** the $16,340 is **list-price-equivalent
  usage, NOT out-of-pocket** — you are bounded by *rate limits / usage windows*, not a draining credit
  balance. At ~19 sessions over the ~2–3 weeks of remaining work, MVP is comfortably within a normal plan;
  "how much is left" = your rolling usage allowance, which resets, not a finite pot.

Either way the actionable lever is the same: **apply the Haiku pin** (`cp` the fixed proposed-settings),
**hold the A1 recycle discipline**, and **keep opus on red-line steps** — together they take the
remaining-MVP cost from ~$1,045 all-Opus toward **~$350–425**, a ~2.5–3× reduction, without betting the
money/parity-critical steps on the cheap tier.
