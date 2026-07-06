# Reflection — swarm output rots when merge-back is discipline-triggered

- **Date:** 2026-07-03
- **Trigger:** big change (touched ≥3 files: preserved voice FE, built the L5 meta-controller +
  staleness guard, wired verify-all). Also a near-miss: at-risk work one `--force` from loss.
- **Class:** harness / self-improvement loop (row #48 family).

## WHAT happened

A prior session's Sandbox-Swarm-Gate produced the entire voice front-end (~1,800 lines: UI in
`packages/ui/src/voice`, adapter in `apps/web/src/lib/voice`, + council docs) across three agent
worktrees. None of it was ever committed or merged. This session's memory brief reported it as
"built, council-reviewed, and staged for clean integration." On disk it was **untracked files in
worktrees 14 commits behind HEAD** — and the gate's own `rm --apply` runs `git worktree remove
--force`, which would have destroyed it silently.

## WHERE

`.claude/worktrees/agent-*` (untracked voice FE) · `scripts/sandbox-swarm-gate.mjs` (`rm --force`,
no staleness/at-risk check) · the memory→next-session brief (narrated readiness).

## WHY (causal root, not just location)

The Sandbox-Swarm-Gate is **designed** to let sandboxes iterate freely — that half is a substrate,
always available. But the **merge-back / preserve** half is *discipline-triggered*: it happens only
if an agent remembers to do it. Row #48's law applies exactly: *what is hook-enforced survives; what
is discipline-triggered dies.* Nothing measured worktree staleness, nothing gated a `--force` remove
against untracked work, and nothing reconciled "narrated as ready" against "un-integrated and
drifting." So the free-iteration substrate kept producing value that then silently rotted, and the
absence of measurement meant the rot was invisible — it even got *reported up* as done. An unmeasured
step cannot show that it failed.

## PROPAGATE (→ deterministic artifacts — enacted this change)

1. **Guardrail** `scripts/guardrail-sandbox-staleness.mjs` (ledger #68, red→green, wired verify-all):
   fails on a sandbox that is stale **and** carries untracked work. The merge-back step is now
   measured and gated, not discipline-only.
2. **Meta-controller** `scripts/meta-controller.mjs` (ledger #69): makes this whole class of "a
   discipline-triggered step died and nobody measured it" a *first-class detected gap* across five
   layers — and proposes the additive fix. The staleness guard above was its first real proposal.
3. **Preserve-before-anything reflex:** at-risk untracked work in an ephemeral worktree is snapshotted
   to a tracked, inert location *before* any build. Done: `docs/design/voice-control/sandbox-snapshots/`.

## Meta-lesson

Narrated-readiness ≠ real-readiness (same false-green family as ledger #48/#67). Trust the working
tree, not the brief. Every free-iteration substrate needs a *measured, gated* merge-back or its
output silently rots.
