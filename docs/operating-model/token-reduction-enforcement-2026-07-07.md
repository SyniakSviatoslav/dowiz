# Token-reduction ENFORCEMENT spec (operator-authorized 2026-07-07 "allow and enforce")

Ground truth: `docs/research/token-economy-comparison-2026-07-05.md` §11 (measured, falsifiable).
Lead-loop $: **Opus 89.3%**; by-lever **cache-read 62.8% · cache-write 18.7% · output 12.7% ·
fresh-input 0.4%**. Instruments (both $0, self-test + RED case): `scripts/audit-model-spend.mjs`,
`scripts/audit-token-router.mjs`. This doc is the apply-ready enforcement — execute in a FRESH
session (cheap cache); doing it at ~310K live context is the exact anti-pattern §11 measures.

## Lever A — cache-read 62.8% (the biggest line) → SESSION-RECYCLE + PREFIX-SHRINK

**A1. Session-recycle is ALREADY partly enforced.** The context-budget `UserPromptSubmit` hook fired
this session at ~310K (30% of 1M) with the mandatory finish/persist/handoff directive. The historical
peak 999,580 (§10) was PRE-hook. Action: verify the hook lives in `settings.json`, and TIGHTEN — the
first WARN should land at the [[token-lifecycle-thresholds-2026-07-05]] 300K recycle line, and a
second HARD directive by ~400K. Falsifiable check: a synthetic ≥300K transcript must trip it.

**A2. Prefix-shrink (method #5, HIGH ROI).** Every cache-read is priced on prefix SIZE; a smaller
frozen prefix makes all 62.8% cheaper linearly. Continue the CLAUDE.md CORE/REFERENCE split
(already underway, header says "operator-approved slim"). Measure with `scripts/context-tax.mjs`
before/after. This is the one method that cuts the dominant line without any behavior change.

**A3. Context-editing / compaction (beta, not yet wired).** `context_management.edits`
(`clear_tool_uses_20250919` / `clear_thinking_20251015`) prunes stale tool results from the
re-read prefix — attacks cache-read at the source. Evaluate wiring into the lead loop; measure.

## Lever B — output 12.7% + Opus 89% share → ROUTE-DOWN + CONSTRAIN

**B0. REASONING → SONNET, not Opus (operator directive 2026-07-07: "my system is harnessed enough").**
The biggest single move: the lead/reasoning tier drops from Opus 4.8 ($5/$25) to **Sonnet 5**
($3/$15 — ~40% cheaper both ways, near-Opus coding/agentic quality). Justified because the harness
(deterministic gates, RED-proven guards, `kernel::decide`) catches errors the model would otherwise
have to — reasoning can lean on the cheaper tier. This directly deflates the 89% Opus share §11b.
- Apply: flip the `settings.json` `model` key `opus`→`sonnet` (per the operator-cp gate,
  [[model-routing-policy-2026-07-03]] — stage the proposed-settings change, operator applies via `cp`).
- Update the MODEL ROUTING table: **reasoning/lead lane default = Sonnet 5** (was Opus 4.8); reserve
  Opus only for explicitly-escalated red-line reasoning (money/auth/RLS/migrations) via `doubt-escalation`.
- Haiku stays the doer tier (B2). Fable stays OFF-for-lanes (sanctioned one-shot only).
- Falsifiable: re-run `scripts/audit-model-spend.mjs` after N sessions — the Sonnet share should rise
  and total $/session should fall; if Opus share stays ≥80%, the pin didn't take.

**B1. Promote `audit-token-router` Check-1 warn→DENY for model-less dispatch.** Now justified: the
newest-12 sessions carry **0% model-less** (§11a) — the #47 over-block fear is empirically retired on
the leading trend. Keep the `Explore`-inherits-parent and human `fable-override` carve-outs.

**B2. Default analytical/read-only subagent lanes to `model: haiku` explicitly.** Opus output $25/M
vs Haiku $5/M (5×). Mechanism ships in MODEL ROUTING; the data says lean on it. (This is operator
method #6 complexity-routing / #2 speculative execution — realised as *dispatch-to-Haiku*, NOT a
lead-model swap: the lead loop can't downshift mid-session, a model switch invalidates the cache.)

**B3. Schema-forced structured returns from subagents (method #4 output-constraining).** Workflow
`schema:` / structured-output already available; make it the default for data-returning lanes —
kills prose padding on the output line.

**B4. Lead-loop `effort` guidance.** Routine/mechanical turns at lower `effort`; reserve `high`/`xhigh`
for reasoning/red-line. Output-token line responds directly.

## Operator's new cache-read methods — GROUND-TRUTH verdict (§0·GP: measure before adopting)

| method | verdict | why (measured) |
|---|---|---|
| #1 Modular cache partitioning | ✅ ADOPT | = CORE/REFERENCE layering (A2); cache only the frozen shell |
| #2 Canonical normalization (sort keys, fixed JSON) | ✅ already-rule | prompt-cache silent-invalidator rule; frozen deterministic prefix |
| #3 **Aggressive TTL eviction** | ❌ **NET-NEGATIVE for the dev loop** | reading an EMPTY cache is **not $0** — it pays FULL input $5/M to re-establish, vs $0.50/M cache-read. An active agentic session reuses its prefix *every* turn, so short TTL forces 10× re-writes. Only helps when you were NOT going to reuse (one-shot/rare). Do not adopt. |
| #4 Anchor tokens (volatile vars outside the block) | ✅ already-rule | = "keep volatile content after the last cache breakpoint" |
| #5 System-prompt minimization | ✅ HIGH ROI | = A2; directly shrinks the 62.8% line |
| #6 Crossover math (drop cache when P·C_read → C_gen) | ✅ principle-correct, rarely-triggers | for THIS workload C_read $0.50/M ≪ C_gen-with-small-ctx, so an active session almost never crosses. True only for rare/one-shot reads. |

**"Zero-cache steady-state / exception-only inference" — this is the PRODUCT runtime, and it is
ALREADY the design.** That vision (FSM kernel; LLM as event-driven override only on Exception State;
$0 in steady-state) is exactly the Sovereign Core + blind-orchestration rule −1: `kernel::decide`
is a pure deterministic function, the LLM makes **zero** runtime dispatch decisions, and the bench
measured **0/100 steady-state ticks calling the LLM** (§8, ≈−99.5%). The "Graph of Exceptions →
promote repeats to kernel rules" IS the Validation Layer arc (steps 1–6: each RED-proven guard
converts a would-be-judgment case into a deterministic invariant). So: yes, repeated logic is
already being moved INTO the kernel — that is the current Sovereign Core work, not a new idea to start.
The dev-agent loop ($16K measured here) is a DIFFERENT surface: it has no "steady state" (every turn
is active work), so its levers are A (session-length) + B (routing), not zero-cache-in-idle.
