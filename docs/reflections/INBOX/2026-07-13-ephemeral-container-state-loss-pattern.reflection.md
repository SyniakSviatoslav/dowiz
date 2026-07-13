# Reflection: ephemeral-container state loss recurred across 2 independent governance scripts (2026-07-13)

**WHAT:** Two unrelated maintainer scripts (`scripts/plane-telemetry.mjs` `cmdResolve`, ledger #57;
`scripts/new-dep-scan.mjs` baseline, ledger #58) both silently no-op'd every day because each wrote
its working state to a path under `loops/runs/` that either wasn't gitignore-carved-out
(`dep-baseline.json`) or was carved out on the branch but never fetched back into the working
checkout before being read (`predictions.jsonl`). Also found and fixed, same run: `main`'s
`pnpm build` was red for 36+ hours (`f0bd996`, missing `StorageProvider` import) after landing via a
direct push with a failing CI check nobody followed up on.

**WHY (causal, not just where):** This maintainer runs in a genuinely fresh cloud container every
firing — no persistent disk survives between runs. Every piece of scratch state under `loops/runs/`
is gitignored by default (`loops/runs/*`), with individual files carved back in one at a time
(`metrics.jsonl`, `routing.jsonl`, `registry.json` — the 2026-07-02 loop-history fix). That carve-out
was applied where someone noticed the loss (loop run history); it was never applied as a *default
posture* for new files written to that directory. Two more scripts (`new-dep-scan.mjs`,
`plane-telemetry.mjs`) were added after that fix, each writing new scratch state to the same
gitignored directory, and neither the author nor CI had a reason to notice — the failure mode is
silent (a script that "worked" every day, just never actually compared against history) rather than
a crash. The pattern is structural: **any new file under `loops/runs/` is durable-by-default-false
unless someone remembers to carve it out individually**, and nothing forces that reminder at
write-time.

**Secondary WHY (calibration, not a bug in the fix):** While root-causing #57, found that ALL 4
outstanding predictions from the 2026-07-11 and 2026-07-12 runs are now *permanently* unresolvable
— not because of the fetch-branch bug just fixed, but because those runs called `predict` *after*
their first telemetry `emit`, so the M1 anti-backdating check (correctly) refuses them forever. The
charter's own SENSE-step ordering ("resolve yesterday's ... predict today's ... run sense commands")
reads left-to-right as calibration-before-sensing, but the actual prior runs ran sense first and
squeezed calibration in mid-run. This run avoided repeating it (predict called before any emit), but
the charter text doesn't make the ordering constraint explicit, so it's one edit away from
recurring.

**Root cause found while doing something else:** the whole session started because `pnpm typecheck`
failed on an unbuilt `packages/config` (yet another instance of this container starting with
incomplete state — `node_modules` itself was also missing `@eslint/js` and other devDeps until
`pnpm install` was run). Chasing that unblocked `pnpm build`, which surfaced the real, unrelated
`main` build break. Three fixes this session were each found as a side effect of unblocking the
previous one, not from a planned audit.

**Candidate ratchets (for council/librarian):**
1. **Guardrail (Tier-1):** a `plane-guard` check (or extend `guardrail-ledger-integrity.mjs`'s
   sibling scripts) that greps `scripts/*.mjs` for `join(ROOT, 'loops/runs/`-style path
   construction and asserts every such literal path has a matching `!loops/runs/<name>` line in
   `.gitignore`. Would have caught both #57's and #58's root cause the day each script was written,
   not 2-3 runs later.
2. **Lesson (Tier-2) / charter edit:** make the SENSE step's calibration ordering explicit and
   mechanically checkable — "call `predict` for today's run before emitting any other telemetry
   event this run" — rather than relying on prose order. Candidate: `cmdEmit` could soft-warn (not
   block, calibration stays advisory) if it fires before any `predict` call exists for that
   `run_id`, mirroring the M1 check's own logic in reverse.
3. **Process (human call, not enacted here):** `main`'s branch protection does not appear to require
   a passing CI check before push/merge — `f0bd996` landed with `conclusion: failure` and sat broken
   36+ hours. This is a one-instance finding (N=1, not the N≥3 recurrence threshold for an autonomous
   guardrail), flagged in today's digest for human decision, not acted on directly (GitHub repo
   settings are outside this agent's autonomy envelope regardless).

---
