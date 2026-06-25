# dowiz / DeliveryOS — Bottleneck-Analysis Action (governance Part 2)

> **On-demand, retrospective action** — NOT a loop, NEVER per-task. Sibling of
> [Task-Exit Rule](task-exit-rule.md) but operationally independent: the rule lives *inside* every
> task; this action looks at the corpus *from above*, rarely. Do not merge them.

## Command

```bash
python3 scripts/analyze-bottlenecks.py                 # all-time
python3 scripts/analyze-bottlenecks.py --since "3 months ago" --top 25
python3 scripts/analyze-bottlenecks.py --min-days 3    # stricter "chronic" threshold
```

The Python implementation is the canonical command (a fragile bash one-liner set was rejected): line
type is keyed on an explicit `\x01` sentinel (a date-like filename can't be mistaken for a date),
pauses are computed stateful per-file, renames are detected (`-M`), and it **fails loudly** (not-a-repo
/ shallow / 0 commits) instead of silently lying.

## Purpose

Find **what repeats** (frequency) and **where execution-time sticks** (chronicity / rework /
wall-clock) across project history, so the most frequent/chronic work can be lifted **out of
repetition** (systematised).

## Two metrics, do not conflate

- **Repeat** = appears many times (frequency). Raw count.
- **Time-bottleneck · chronicity** = recurs *spread across calendar time* (same area touched on many
  different days) — a chronic drag, not a one-day spike.
- **Time-bottleneck · rework concentration** = area concentrates fix/revert/refactor/wip — thrashing.
- **Time-bottleneck · wall-clock** = task that eats real time / many FAIL→fix cycles.

> 50 edits in one day = one heavy build (spike). 10 edits across 10 weeks = a chronic drag. These are
> **different** — count alone can't tell them apart, hence the time dimension.

## Data sources (and caveats)

- **Git** — "which code is constantly touched". Reliable, always available. Frequency, chronicity
  (by distinct days), rework concentration, pauses. The script uses this.
- **MemPalace** (`mempalace.yaml` — temporal KG, **not** Mem0) — "which themes/decisions recur".
  **Only if populated.** As of 2026-06-24 `~/.mempalace` is absent → not yet a source; theme-repeat
  comes from git only. The script reports this in STEP-ZERO.
- **Claude Code logs** (`~/.claude/projects` — per-message timestamps) — the most direct **wall-clock**
  per task. The script detects their presence but deliberately does **not** parse the format (confirm
  format first, then add).
- **Task-exit reports** (from [Task-Exit Rule](task-exit-rule.md), once they carry timestamps) — the
  native future feed: repeated FAIL categories + repeated escalate-flags = repeats; long-span /
  many-FAIL→fix tasks = bottlenecks.

## Interpretation → action

1. Read the **two ranked lists**: repeats (A/A'/themes) and time-bottlenecks (B chronicity / C rework
   / D pauses).
2. Find the **intersection (E)**: both frequent AND chronic/thrashing → systematise **first**.
3. Frequency without chronicity → usually a healthy active area, not a problem. Chronicity/thrashing
   without an obvious cause → candidate for refactor / doc / rule.
4. Lift the top intersection out of repetition into ONE of: **MemPalace L0/L1 pinned fact** (re-derived
   knowledge) · **doc** (recurring context) · **skill** (recurring *way* of doing) · **hook/rule**
   (recurring catchable mistake — possibly strengthen the Task-Exit Rule).

## STEP-ZERO (mandatory before trusting output)

The script runs these and refuses / warns on failure:

- `git rev-parse --is-shallow-repository` must be `false` (a truncated history skews churn).
- `git rev-list --count HEAD` > 0.
- MemPalace populated? If not, theme-repeat is git-only.
- Claude logs present? If so, wall-clock is mineable (manually, for now).

## Out of scope

- ❌ Running this in a task flow or per-change.
- ❌ Turning it into a standing loop.
- ❌ Trusting output before STEP-ZERO (shallow git / empty MemPalace).
