# Meta-Controller — the L5 gated self-modification loop

> **The apex of the self-improvement stack.** A system that reads its own *verified* outcomes,
> detects where it is failing to learn or grow, and **proposes** concrete subsystem changes to
> fix or extend itself — then re-maps. It is "self-modifying" in the only responsible sense:
> propose-and-gate, never auto-apply, with an **immutable safety core** that can never be a target.
>
> Implementation: [`scripts/meta-controller.mjs`](../../../scripts/meta-controller.mjs) ·
> Proof: [`scripts/meta-controller.test.mjs`](../../../scripts/meta-controller.test.mjs) (9/9, red→green).

## The loop

```
ingest verified signals ─▶ detect gaps ─▶ PROPOSE additive artifacts ─▶ (GATE + HUMAN) ─▶ land ─▶ re-map
        (5 layers)            (pure)         (immutable-core refused)      (never auto)     (new node)
```

The controller closes the metric-reflection loop: a verified outcome that reveals a gap makes the
system build a **new** subsystem (an additive guardrail/skill/loop) or correct an existing one —
then the systems-map reflects the new node. Freedom in the inner loop (sandboxes iterate freely);
an unbreakable boundary at the edge (the gate + the immutable core).

## The immutable core — the L5 safety invariant (the whole point)

The controller can **never** propose modifying, and **refuses as a target**, the nodes that *are*
the safety of the system. A gap whose fix would touch one of these is dropped into `refusals` and
logged; the system extends itself with **new** nodes, it never rewrites its own authority.

| Immutable node | Why |
|---|---|
| **Ethics Charter** (`CLAUDE.md` §Ethics) | Non-modifiable, non-removable, overrides all (Charter §1–4). |
| **The GATE** (`scripts/sandbox-swarm-gate.mjs`, `SANDBOX-SWARM-GATE.md`) | Authority cannot rewrite its own gate — the gate is extended by new nodes, never self-edited. |
| **Authority hooks** (`protect-paths`, `red-line-doubt-gate`, `serious-gate`, `guard-bash`, `require-classification`) | The deterministic enforcement layer. Advisory signals never rewrite enforcement. |
| **`AGENTS.md`** | Standing agent rules (ponytail / test-integrity / red-lines). |
| **The controller's own safety core** (`scripts/meta-controller.mjs`) | It cannot propose modifying itself. |

There is deliberately **no env switch** that disables the immutable core (unlike other scripts'
`TEST_DISABLE_*` toggles) — an override on the safety boundary would itself violate the Charter.
The test proves the predicate *discriminates* (refuses immutable, allows non-immutable); removing
any node from the core makes the proof go **RED**.

**Probe (2026-07-03):** `guard --target` REFUSED all five immutable classes (exit 3) under direct
instruction to modify them; a red-line migration → `REVIEW-HUMAN` (exit 2); ordinary UI →
`REVIEW-GATE` (exit 0). Removing the `ethics-charter` node → 2 tests fail; restore → 9/9 green.

## The five integrated layers (what each ingests)

| Layer | Signal (real, on-disk) | Gap it detects |
|---|---|---|
| **1 · Sandbox-Swarm-Gate** | `git worktree list` drift + untracked/modified counts | `STALE_SANDBOX` — swarm output rotting / at-risk work a `--force` prune would lose |
| **2 · Skill-evolution** | `docs/design/harness/proposed-skills/*`, DRAFT loop cards | `SKILL_DRAFT` — a capability drafted (`create`) but never certified/promoted |
| **3 · Telemetry** | `.claude/logs/harness-events.jsonl` (kind tally + freshness) | `TELEMETRY_FRICTION` (a deny/block kind dominates), `STALE_TELEMETRY` (measurement stopped) |
| **4 · Metric-reflection** | `docs/reflections/INBOX/*`, `REGRESSION-LEDGER.md` | `UNRATCHETED_REFLECTION`, `UNFILLED_WHY`, `PENDING_LEDGER_PROOF` |
| **5 · Systems-map + meta-controller** | all of the above | `map` renders the living graph; `metrics` records gap-history for historical comparison |

**Probe (2026-07-03, live):** 7 gaps across all five layers — 3× stale-sandbox (high), 2
unratcheted reflections + 11 pending-ledger-proof rows (med), 14 skill-drafts + telemetry-friction
`block` 36× (low). `STALE_TELEMETRY` correctly did **not** fire (982 events, fresh — no crying wolf).
`metrics` recorded a 7-gap baseline and the delta vs the prior run.

## Commands

| Command | Effect |
|---|---|
| `report` (default) | Ingest all layers, print ranked gaps + proposed artifacts + refusals. **Writes nothing.** |
| `map` | Print the living systems-map + live telemetry/skill signals. |
| `metrics` | Record this run's gap tally to `loops/runs/meta-controller-metrics.jsonl`, print the delta vs last run. |
| `guard --target <t>` | Adversarial probe: `REFUSE` (immutable) / `REVIEW-HUMAN` (red-line) / `REVIEW-GATE` (ordinary). Exit 3/2/0. |
| `propose [--apply]` | Stage **inert** proposal drafts under `docs/reflections/meta-proposals/`. Never touches a guardrail/hook/loop. |

## What it deliberately does NOT do

- **No `apply` command.** Landing a proposal is a human-approved, GATE-passed act (Sandbox-Swarm-Gate §4).
- **Never edits an immutable node.** Refused upstream and logged.
- **`report` writes nothing.** Only `metrics` (append-only history) and `propose --apply` (inert drafts) write.

## Provenance — this loop's first real output

The controller's first proposal was `scripts/guardrail-sandbox-staleness.mjs` for the `stale-sandbox`
gap — an **additive** guard (not an edit to the immutable gate). A human approved it through the
gate; it landed red→green and is wired into `verify-all.ts`. That is the L5 loop demonstrated
end-to-end: verified gap → proposal → human approval → additive guardrail → re-map. See
`REGRESSION-LEDGER.md` #68–#69 and `docs/reflections/INBOX/2026-07-03-swarm-mergeback-rot.reflection.md`.
