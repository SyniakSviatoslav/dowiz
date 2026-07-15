# Spec-Driven DOD Reporting & Trajectory Tracking — PLAN

Date: 2026-07-15. Author: Hermes (operator-authorized). Repo: dowiz.

## Problem

The agent does autonomous work but reports only ad-hoc at the end. The operator wants a
**spec-driven, definition-of-done (DOD)** loop — like a dashboard/terms dashboarding system —
that, *before* any task/wave/roadmap:

1. Creates a BIG PLAN: header description + major key topics, risks, overall system/file
   status BEFORE the task, number of steps, approximate ETA time + ETA token consumption.
2. Polls every 60s for: completion % status, error / decision-needed alerts.
3. On finish, writes a RETRO + summary comparing ETA time vs actual, tokens est vs actual,
   issues/bugs, lessons learned, skills upgraded, and next-task status
   (autorun | full-finish | stop).

And crucially: this is not just Telegram output — it is **stored locally in vectorless
indexed tables** and **wired into automatic system tracking of improvement/degradation over
time** (trajectory trend across plans).

## Design (reuse, don't reinvent)

Built ON the existing `tools/telemetry` primitives (zero new deps):
- `log_event <kind> k=v...` → appends to `tools/telemetry/logs/<kind>.jsonl` (the vectorless
  indexed table — queryable via grep/awk/python, no vector DB).
- `tg_send <text>` → Telegram (Dowiz-Reporting) with 3-try retry.
- `bench_run` already records wall-ms + peak RSS per op (feeds resource tracking).

### Local stores (vectorless JSONL in `tools/telemetry/logs/`)
| file | one row per | keys |
|------|-------------|------|
| `plan.jsonl` | plan created | id, title, topics, risks, status_before, steps, eta_min, eta_tokens, state |
| `plan_step.jsonl` | progress tick | id, done, total, note |
| `alert.jsonl` | raised alert | id, type(error\|decision), msg |
| `trajectory.jsonl` | plan finalized | id, eta_min, actual_min, eta_tokens, actual_tokens, eta_err_pct, issues, lessons, skills, next, prior_acc_pct |

### Commands (added to `telemetry` dispatcher)
- `plan <id> <title> --steps N --eta-min M --eta-tokens T --topics "..." --risks "..." --before "..."`
  writes plan.jsonl (state=planned), sends 📋 PLAN header to TG.
- `step <id> <done> [total] [note]` → plan_step.jsonl tick.
- `alert <id> <error|decision> <msg>` → alert.jsonl + immediate TG 🚨.
- `track <id> [interval=60]` → **background-safe** 60s loop: % from latest step, elapsed vs
  ETA, 📈 heartbeat; ⚠️ ETA-overshoot if elapsed>ETA & not done; decision reminder if open
  alert; exits when plan.state=done.
- `retro <id> --tokens N --issues "..." --lessons "..." --skills "..." --next autorun|finish|stop`
  → finalize: actual_min = now - plan.ts; compares to ETA; computes `eta_err_pct`; reads prior
  trajectory rows to derive `prior_acc_pct` (rolling mean abs ETA error) → improvement/degradation
  signal; writes trajectory.jsonl (state=done); sends 🏁 RETRO.
- `status <id>` / `dashboard` → on-demand aggregate view (the "terms/dashboard").

### Improvement/degradation wiring
`trajectory.jsonl` is the long-term memory. `dashboard` prints per-plan ETA accuracy and the
rolling trend; a worsening trend (rising `eta_err_pct`) is the automatic "system degrading"
signal. `retro` also logs a `metric` event so `monitor`/Gatus-style thresholds could later fire.

## Honest limitation
This environment cannot read the live LLM token counter, so `eta_tokens` (plan) and
`actual_tokens` (retro) are **operator/agent-supplied estimates**. The ETA-vs-actual *tracking
framework* is fully real and works regardless; a future Hermes usage-hook could feed exact
figures without changing the schema.

## Files touched
- `tools/telemetry/lib.sh` — add helpers: `_plan_dir`, `_latest_step`, `_plan_get`, `_rolling_acc`.
- `tools/telemetry/telemetry` — add `plan|step|alert|track|retro|status|dashboard` cases + usage.
- (no new deps; pure bash + python3 stdlib for JSON aggregation in `dashboard`).

## Verification
Ad-hoc script: create plan → 2 step ticks → raise+resolve a decision alert → run `track` at
interval 2 in background → `retro` → assert plan.jsonl state=done, trajectory.jsonl has a row
with eta_err_pct, dashboard prints it, TG headers sent (NO_TG=1 for the script, then a real
send smoke). All local; no destructive ops.
