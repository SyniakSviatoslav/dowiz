# Reflection — measure-before-enforce turned a would-be over-block into a warn-then-ratchet gate; and a guardrail outside the enforced path rots red

**Date:** 2026-07-06 · **Slug:** token-gate-warn-then-ratchet
**Qualified because:** red-line surface (`.claude/hooks/` + `.claude/settings.json` governance) + ≥3 harness files.

## CONTEXT
Continuing STRUCTURE-UPGRADE.md Part B (make the TOKEN ROUTER / MODEL ROUTING stack mechanically
enforced). Prior run did B0 (baseline): `scripts/audit-token-router.mjs` measured that **86% of 1027
real Agent dispatches carry no explicit `model:`** and only ~10% are compliant. This run shipped B1 —
the dispatch gate — after the operator picked rollout **A (warn-then-ratchet)** off that data.

## DECISIONS
1. **B0 before B1 was load-bearing, not ceremony.** The 86% number is what killed the naive plan (a
   blind hard-DENY on missing `model:`). We shipped `agent-dispatch-gate.sh` in WARN mode (logs `_hev`
   + nudges, never blocks), with the DENY path armed (`TOKEN_GATE_MODE=deny`) so promotion is a config
   flip gated on the `_hev` habit trend — not a rewrite, not a guess.
2. **ONE check first** (missing `model:`), not all 7. The stamp-based checks (LANE-CLASS/router) have 0%
   current adoption — warning on them now is pure nudge-spam that trains people to ignore the gate.
3. **Live smoke test, not just hermetic armament.** Dispatched a real model-less Explore agent and
   confirmed the hook fired (`_hev` warn, non-blocking) — because "armament green" only proves the
   script, not that the harness binds it to the real Agent PreToolUse surface.
4. **Fixed a stale governance test I found en route** (`guardrail-gate-armament.mjs`) rather than route
   around it.

## WHERE
`.claude/hooks/agent-dispatch-gate.sh` (new), `.claude/settings.json`, `scripts/guardrail-token-gates.mjs`
(new armament), `scripts/guardrail-hook-matchers.mjs` (anti-unregister pin), `scripts/guardrail-gate-armament.mjs`
(stale-case fix), `docs/design/sovereign-core-mvp/PROGRESS.md`, `docs/research/token-economy-comparison-2026-07-05.md`.

## WHY (causal, not just where)
- **Enforcement tuned on assumption over-blocks — the #47 root.** The plan's own instinct ("model
  required → DENY") would have blocked ~90% of live dispatches on day one. What prevented that was
  *measuring first*: B0 converted a plausible policy into a demonstrably wrong one, and the operator's
  warn-then-ratchet choice fell straight out of the data. The lesson isn't "warn is nicer" — it's that a
  gate's threshold must be set on the real violation distribution, never on the rule's aspiration.
- **A registered gate with a silent parse bug is worse than no gate** (false security). The armament's
  simulated-DENY caught a real defect: fields were emitted tab-separated, but tab is IFS-whitespace, so
  an EMPTY `model` field collapsed under `read` and the slug landed in `MODEL` — the missing-model check
  could never fire. Registration + "looks wired" would have shipped a dead gate. Only the hermetic
  DENY-simulation surfaced it (exactly docs/lessons/2026-07-02-gate-state-file-expiry.md #47).
- **A guardrail outside the enforced path rots red, unseen.** `guardrail-gate-armament.mjs` had been
  failing since `340a8c3a` (the unlock narrowed guard-bash to make `.claude/hooks/` agent-editable but
  didn't update the "sed into hooks blocked" assertion) — invisible because gate-armament is NOT in
  `.husky/pre-commit`. This is error-class-7 ("only hook-enforced artifacts survive") applied to the
  guardrails themselves: a test that nothing runs is decoration that decays into a lie.

## CONFIDENCE
High on the gate (live-verified fired + armament red→green + registration pinned). Medium on the ratchet
timing — "when the habit took" needs a concrete `_hev` threshold, not a vibe (see NEXT-TIME).

## NEXT-TIME
- Put `guardrail-gate-armament.mjs` + `guardrail-token-gates.mjs` into an *enforced* path (pre-commit or
  weekly curation) so they can't rot red the way gate-armament did. A guardrail not on a runner isn't one.
- Define the ratchet trigger as data: e.g. promote model-check warn→deny when `audit-token-router` shows
  model-less dispatches <10% over a trailing window. Encode it, don't eyeball it.
- Sandbox note: hook `_hev` writes lag the Bash-tool view of the log by ~2 min (fs-sync) — read-back of
  live hook effects needs a beat, or a re-read, before concluding "didn't fire."

## LINK
docs/regressions/REGRESSION-LEDGER.md ; docs/lessons/2026-07-02-gate-state-file-expiry.md #47 ;
docs/design/sovereign-core-mvp/STRUCTURE-UPGRADE.md (Part B) ; docs/design/sovereign-core-mvp/PROGRESS.md
