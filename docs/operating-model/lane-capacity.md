# Lane capacity policy (measured 2026-07-02, box: 4 cores / 7.6GB RAM / shared pnpm store)

The bottleneck is CPU for verification, not agent count. Lanes divide into two classes:

## WRITE lanes (API-bound: read/edit code + docs, no builds)
- Local cost ≈ zero (each agent is network-bound). **Budget: 8–10 concurrent.**
- Rule: a WRITE lane never runs `pnpm build`/`pnpm -r`/playwright/full test suites. Single-file
  `node --test <file>` self-checks are allowed. Everything heavier goes to the VERIFY queue.
- File-collision partitioning is mandatory: one lane = one directory/surface; the shared
  integration point (hot file, registration, wiring) stays with the lead.

## VERIFY lanes (CPU-bound: build, typecheck, test suites, Playwright, hook-gated commits)
- **Budget: 2 concurrent max** (= the harness's own `min(16, cores−2)` formula on this box);
  in practice 1 while a hook-build (lint→typecheck→build) is running — those are internally
  parallel and saturate all 4 cores + 1–2GB each.
- All hook-gated commits serialize through ONE ship lane (two concurrent husky builds thrash).

## Off-box lanes (the real multiplier — use when the VERIFY queue backs up)
- **claude.ai cloud sessions** (RemoteTrigger create→run→delete, env `dowiz-maintainer`): each run
  gets its own machine with git creds. Right for: independent build/verify, research, PR prep.
  Wrong for: anything needing box secrets (fly deploy, staging DB, Telegram).
- **GitHub Actions `@claude` lanes** (apps/claude installed): PR-comment-triggered work on CI runners.
- Workflow-tool orchestration on this box inherits the 2-slot cap — prefer Agent-tool WRITE
  fan-out + serialized VERIFY, or off-box lanes.

## Hygiene that keeps the budget honest
- Worktrees share the pnpm store (`~/.local/share/pnpm/store/v3`) — a worktree costs hardlinks
  + one install's CPU, not gigabytes. Only give a worktree to a lane that must mutate files
  another lane also touches. Remove merged worktrees (`git worktree remove` + `branch -D`).
- Long-lived browser sessions and stray scanners hold RAM/CPU — check `ps aux --sort=-%cpu`
  before a wide fan-out; loop-harness `governor.ts` is the authority for loop concurrency
  (`freeRamMb` / `maxConcurrentLoops`) — don't duplicate it, feed it.
