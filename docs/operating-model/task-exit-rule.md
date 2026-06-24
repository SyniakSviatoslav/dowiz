# dowiz / DeliveryOS — Task-Exit Rule (universal execution invariant)

> **Operational-layer rule** (NOT a product phase). Augments `v4.4`/`v4.5` governance and the
> [Agent Operating Model](agent-operating-model.md). On execution-discipline conflicts, **this wins**.
> **Authority:** complements the existing hooks (`post-edit-gates`, `require-classification`); does
> not change product scope/contracts/red lines.

## What it closes

Context-understanding, research, and execution are already good. What leaks is **verification at the
exit of small, "done" changes**. The heavy gates (Convergence Loop, Frontend Audit Gate, phase
exit-audits) fire only at phase boundaries — so drift accumulates between them, and those gates are
too global to attribute it. The deeper cause: the agent that *built* the change verifies it in the
same pass — attention is spent on construction, and it checks its **intent** ("did I do what I
planned") against the plan, not the **artifact** against the spec.

## Nature: this is a RULE, not a loop

- Runs **inline**, as part of doing the task — not a separate loop, not a deferred post-check, not an
  external watcher.
- **Independent of spec completeness.** Thin spec or no clarifications → the agent **generates** the
  full definition of "done" and the exit criteria itself. It does not wait to be handed them.
- **No size exemption.** The smaller the change, the likelier a missed detail. A one-line fix gets the
  same treatment as a new screen.
- Enforcement is **ordering**: exit criteria are written **BEFORE** touching code. What's written
  up-front can't be silently skipped or back-fitted to the result.

## The order (4 steps, fixed)

1. **ENRICH conditions** — derive the full "done", not just the literal task text (dimensions below).
2. **AUTHOR task-exit** — write a checkable exit checklist for *this* change, one item per enriched
   condition, each with a proof type. **Before code.**
3. **EXECUTE** — make the change.
4. **VERIFY against the pre-written exit** — each item → PASS/FAIL with proof. No credit for intent.

> Steps 1–2 happen during task planning. Step 4 happens before declaring "done". Steps 1, 2, 4 are
> always mandatory; step 3 is the work itself.

## Enrichment dimensions (the step-1 checklist)

For each: either an exit-checklist item, or an explicit `N/A — reason`. **Never skip silently.**

- **States:** loading (skeleton) / empty / error (retry + fallback) / success.
- **Error matrix:** 401 / 403 / 404 / 422 / 429 / 5xx / network-timeout — each has a defined UX, not
  one generic message.
- **Edge / rare states:** from the blind-spot taxonomy — `BusyMode`, `ClosedOverlay`, `StopListBadge`,
  dead-channel, GPS-denied, `BackgroundWarning`, WakeLock failure, cart drift, token expiry mid-action.
- **Regression radius:** what this change could break nearby → which adjacent tests/flows to re-check.
- **Design system:** zero hex literals, tokens only, one shared component/pattern (no local deviation).
- **i18n:** zero hardcoded text past i18n; both languages (al/en).
- **Security/hygiene:** no cookies, no secrets/PII in client or logs, Zod-parse on responses.
- **Contract parity:** every touched api/ws is WIRED or an explicit STUB; no dead buttons; price/status
  are server-authoritative only.

## task-exit format (step-2 output)

A small block per task — e.g. an `EXIT` block in the ticket or `docs/exits/<task-id>.md`:

```
TASK: <id / short description>
ENRICHED-DONE: <full definition of done beyond the literal text>
EXIT CHECKLIST (written before code):
[ ] <condition 1> — proof: <file:line | test | command output | artifact>
[ ] <condition 2> — proof: ...
[ ] <dimension> — N/A: <reason>
...
```

After VERIFY each `[ ]` becomes `[PASS]` / `[FAIL]` / `[FLAG: inline|escalate]` with concrete proof.

## Verify discipline (step 4)

- **Proof, not words.** "Looks fine" = FAIL. Every PASS = file:line / test / output / artifact.
- **Artifact, not intent.** Re-read the **actually produced** diff/file, not the plan.
- **Triage findings:** **inline-fix** (cosmetic / states / tokens — fix now) vs **escalate**
  (contract / price-status business logic / security — don't touch, raise separately).
- **Floor before judgement:** run the mechanical gates first, *then* the substantive review. A red
  floor = not done; substantive review is moot until it's green.

## Mechanical floor (already exists — a tripwire, not a loop)

The existing hooks remain unchanged and serve as an involuntary floor: `post-edit-gates` /
`lint:gates` / `typecheck` + grep guards (hex colours, `new WebSocket` past the hook,
`document.cookie`, `as any` past Zod). This is **not a loop** — it's an automatic fail-safe that trips
on its own.

> Observed friction (2026-06-24): `post-edit-gates` greps the **whole file**, not the diff — so it
> false-positives on any edit to a file that merely *references* a masked-phone field or
> `Math.random()…token` in pre-existing code. Verify the diff introduced no PII/money/insecure-random,
> then proceed; never weaken the gate to silence it.

## Optional minimal enforcement of the rule itself

If it ever needs guaranteeing that the rule wasn't "forgotten": a tiny hook that checks only the
**presence** of an EXIT block for a task (not its content or results) — like `require-classification`.
This is **not** a verification loop, it's a presence-check. Not needed by default; the rule rides on
the ordering of steps.

## Honest limits

A rule the agent applies to itself does **not** fully kill self-bias — the model can still grade its
own work leniently. The mitigation without a separate loop is exactly "exit BEFORE code" (locks
expectations up-front) + the mechanical floor (objective line). If that proves insufficient for
critical tasks, *then* (and only then) consider a separate verify pass by a different model — but
that's a different mechanism, not this rule.

## Anti-patterns (do NOT)

- ❌ Turn the rule into a separate loop / background process / deferred post-check.
- ❌ Skip enrich+exit on "small" changes.
- ❌ Wait for a full spec to start enriching (the rule is spec-independent).
- ❌ Write exit criteria **after** the code (that's rationalising the result, not a rule).
- ❌ Credit an item by intent / "looks fine".

## Relationship to Bottleneck-Analysis

The only link to [Bottleneck-Analysis](bottleneck-analysis.md): once task-exit reports carry
timestamps, they become its best future feed (repeated FAIL categories + repeated escalate-flags =
repeats; long-span / many FAIL→fix tasks = bottlenecks). Otherwise the two are independent — this rule
is always-inside-the-task; that action is rarely-outside. Do not merge them.
