---
name: doubt-escalation
description: In-flight escalation (Doubt model) for the dowiz/DeliveryOS harness. When an OBSERVABLE signal fires mid-task (a loop of N failed attempts, a red-line/irreversible edit, ≥2 surviving interpretations, evidence conflict, novelty, or a result that contradicts a stated expectation), climb the cheapest-sufficient escalation rung within budget K — self-divergence → specialist/research subagent → stronger model → council → human — then RESUME the main task. Friction, not a hard stop, except irreversible/red-line which gate to a human. Routine reversible low-uncertainty work never escalates.
---

# Doubt model — in-flight escalation (HARNESS)

> Calibrated friction. The signal raises doubt; the **resolution** (a green check, specialist evidence with `file:line`, or a human decision) is the **authority**. Signals inform — they never decide.

## Core stance
- **Escalate-and-CONTINUE.** Doubt adds friction (a cheaper-rung check) then you resume the same task with the answer. It is NOT "stop and ask the human" — that is only the top rung, reserved for irreversible / red-line / genuinely ambiguous-spec.
- **Routine never asks.** Reversible + low-uncertainty + matches a known pattern → act, zero escalation. Anti-over-ask is a feature: a question you could have answered yourself is a budget leak.
- **Observable, not introspective.** Trigger on signals you can point at (a counter file, a glob match, a diff, a failed assertion). Never trigger on "am I sure?" vibes.
- **Budgeted.** At most **K escalations per task** (default K=2 discretionary rungs before the loop-detector forces the issue). Budget exceeded → stop guessing, ask one crisp decision-shaped question.

## Observable triggers (any one arms doubt)
1. **Loop** — N consecutive failed attempts on the same target/error-signature (default **N=3**; enforced deterministically by `loop-detector.sh`, not at your discretion). A loop is a **mandatory** escalation — you may not attempt #N+1 the same way.
2. **Irreversibility / blast-radius** — the action touches a red-line glob (auth · money · RLS · `packages/db/migrations/` · bulk/multi-file edit). Surfaced by `red-line-doubt-gate.sh`.
3. **Ambiguity** — ≥2 plausible interpretations of the spec/task survive after you read the relevant code.
4. **Evidence conflict** — two sources (test vs docs, ADR vs code, two files) disagree.
5. **Novelty** — no matching pattern / lesson / ADR exists for what you're about to do.
6. **Expectation breach** — the result contradicts something you explicitly stated would happen ("this should make the test pass" → it didn't).

## The ladder — cheapest sufficient rung first, within budget K

**Rung 0 — Act (no escalation).** Reversible, low-uncertainty, pattern-matched. Default for routine work. Do NOT spend an escalation here.

**Rung 1 — Self-divergence (cost: ~0, in-context).**
Generate **2–3 viable options / interpretations** explicitly. Score each against evidence already in hand (code, tests, ADRs, lessons).
- One **clearly dominates** on evidence → act on it. Doubt resolved, no budget spent beyond the think.
- **Several survive** → the ambiguity is real; spend an escalation, go to Rung 2.

**Rung 2 — Specialist / research subagent (cost: 1 escalation, isolated context).**
Raise the **relevant** critic or a research subagent in a fresh/isolated context so the main thread stays clean:
- security/auth/secret doubt → `security-sentinel`
- invariant / red-line semantics → `invariant-guardian`
- unknown bug / wrong-result → `systematic-debugging` or a `cause-critic`-style research subagent
- domain/architecture → `system-architect`; tests/coverage → `test-scout`
Demand an answer **with evidence (`file:line`)**. Then **RESUME the main task** with that evidence. Do not let the subagent do the main work — it informs, you act.

**Rung 3 — Stronger model (cost: 1 escalation).**
Escalate the **specific sub-decision** (not the whole task) to a stronger model — `/model` switch or an OpenRouter route — phrased as one bounded question. Bring back the answer, resume.

**Rung 4 — Council (cost: 1 escalation, systemic).**
The ambiguity is **systemic** (design-level, contract-shaped, recurring across the codebase) → trigger a council retro (`/council <decision>`). Produces a hardened decision (ADR + threat-model). Resume against the cleared plan.

**Rung 5 — Human (terminal — only when warranted).**
ONLY for: **irreversible** action, **red-line** with no reversible path, or **ambiguous spec** you cannot resolve from evidence. Ask **one crisp decision-shaped question** — present the options, the evidence, the reversibility, and the single choice you need. Never "are you sure?"; always "A or B, because X vs Y — which?".

## Calibration table

| Situation | Reversible? | Uncertainty | Rung |
|-----------|-------------|-------------|------|
| Routine edit, known pattern | yes | low | 0 — act |
| Two readings of the task survive | yes | med | 1 → maybe 2 |
| Security/RLS/auth semantics unclear | yes | med-high | 2 (specialist) |
| Stuck: N failed attempts, same signature | — | high | forced ≥2 (loop-detector) |
| Design ambiguity recurring across files | yes | high | 4 (council) |
| Migration / irreversible / red-line, no safe path | **no** | any | 5 (human gate) |
| Budget K exhausted | any | any | stop → 1 crisp question |

## Budget defaults
- **K = 2** discretionary escalations per task (rungs 1–4 you choose to spend).
- **N = 3** failed attempts on one target/error-signature → mandatory escalation (loop-detector enforced).
- Always pick the **cheapest rung that resolves the doubt**. Skipping straight to "ask the human" on a reversible, answerable doubt is a calibration failure.

## Hooks that arm this skill (deterministic, agent-independent)
- `.claude/hooks/loop-detector.sh` — PostToolUse/Stop; counts same-signature failures; at N emits a STRONG escalation directive (don't retry the same path — fresh context / specialist / stronger model).
- `.claude/hooks/red-line-doubt-gate.sh` — PreToolUse on `Edit|Write|MultiEdit`; on a red-line glob emits a required **doubt-pass** (options considered · why this · reversibility); irreversible set requires explicit human confirmation. Zero-noise on routine reversible edits. **Complementary to `protect-paths.sh`** — it never weakens that hard block.
