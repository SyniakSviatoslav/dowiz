# Reflection: the PR review/merge queue is the actual bottleneck, not the maintainer loop (2026-07-14)

**WHAT:** Today's SENSE step independently rediscovered a real, live data-loss bug in
`scripts/plane-telemetry.mjs`'s `cmdPublish` (a publish call that lacks a local copy of a
previously-published file, e.g. `predictions.jsonl` before this run's first `predict` call,
silently drops that file from the `telemetry/plane` branch's tip tree). Confirmed live: the
branch's actual tip (`209fc7e`, written automatically by this morning's `plane-report.mjs` run)
had already lost `telemetry/predictions.jsonl`, present in its parent. Root-caused, wrote a
red→green regression test reproducing the exact scenario, and implemented a fix — then discovered
**PR #13, opened 2026-07-05, already contains the identical fix**, still open as an unreviewed
draft 9 days later. Rather than duplicate it, rebased/merged current `main` into PR #13's branch
(merge commit, no force-push — `guard-bash.sh` correctly blocked my first attempt at
`--force-with-lease`, which was the right call), reran the full suite (24/24 green,
`verify:all --ci` ALL PASSED), and pushed the update. Also discovered, while running `pnpm build`
to unblock a stale `pnpm typecheck` (missing `packages/config/dist` in this fresh container),
that `main`'s build has been broken for 3 straight days (`f0bd996`, missing `StorageProvider`
import in `apps/api/src/bootstrap/workers.ts:38`) — and that this, too, already has not one but
**two** duplicate one-line fix PRs open (#24, #25), both proven, both `mergeable_state: clean`,
neither merged.

Pulling the full open-PR list surfaced the real shape of the problem: **21 open PRs, 20 of them
drafts opened by this exact plane-maintainer routine between 2026-06-18 and 2026-07-13, zero
merged in that entire window.** At least four bug classes have two independent PRs each fixing
the identical issue (publish-drops-files: #11/#13; resolve-doesn't-see-branch: #21/#26;
StorageProvider import: #24/#25; dep-baseline persistence: #14/#27) — each pair separated by
several days, meaning the SAME finding was rediscovered from scratch, root-caused again, tested
again, and PR'd again, purely because the first PR sat unreviewed.

**WHY (causal, not just where):** The maintainer loop's own design is sound — SENSE correctly
re-detects real regressions every run, DIAGNOSE correctly root-causes them (not just symptoms),
HEAL correctly produces tested, narrow, proof-carrying fixes on feature branches. The loop has no
visibility into or leverage over its own output queue, though: it opens a PR and moves on: the
charter's autonomy envelope explicitly stops short of merging (`docs/governance/
plane-maintainer-agent.md`: "commit to a feature branch... open a GitHub PR" — never "merge"),
which is the correct boundary for a code change, but there is no analogous mechanism that notices
"the last N PRs I opened for the same underlying finding are still open" and escalates *that*
pattern specifically, as opposed to escalating the underlying bug (which today's SENSE step does,
loudly, every time it recurs). The result is a queue that only grows: every day the container
starts fresh, reruns the same checks, finds the same still-unfixed-on-main problem, and (mostly)
does the responsible thing and opens a fresh, well-proven PR for it — but nothing in the loop ever
asks "is anyone looking at these?" The failure mode is invisible from inside a single run because
each run's diff/PR looks individually justified and well-executed; it's only visible in aggregate
across the whole PR list, which no single day's SENSE step reads.

**Candidate ratchets (for council/librarian):**
1. **Process (human call, not enacted here):** batch-review and merge the backlog, starting with
   the two lowest-risk, highest-value items: #25 (StorageProvider, 1 line, `mergeable_state:
   clean`, unblocks `main` CI for every subsequent PR) and #13 (telemetry publish fix, governance-
   script-only, zero product surface, just re-verified live). Recommend closing the now-redundant
   duplicates (#24 superseded by #25; #21 vs #26 need a diff-check; #14 vs #27 likewise) once the
   surviving PR of each pair lands, rather than merging both.
2. **Guardrail candidate (Tier-1, `plane-guard`):** an "PR queue liveness" check — before opening
   a new PR, query `list_pull_requests` (or the local equivalent) for existing open PRs whose
   title/branch match a known bug signature (e.g. same file + same error string) and, if found,
   update/comment on the existing PR instead of opening a new one; separately, a soft-warn if this
   routine's own open-PR count exceeds some threshold (e.g. >10) with age >7 days, surfaced
   prominently in the digest ("N stale PRs awaiting review", not buried in a per-day narrative).
   This is squarely the same shape as the existing `prediction-resolution-liveness` /
   `inbox-drain-liveness` soft checks — extending the "silence made visible" principle (H3) from
   telemetry/reflections to the PR queue itself.
3. **Charter clarification:** `docs/governance/plane-maintainer-agent.md`'s REPORT step already
   says "a PR for any auto-fix... or an issue on a hard fail" — add an explicit step to check for
   an existing open PR fixing the same signature before opening a new one, so the *default*
   behavior stops compounding the backlog even before ratchet #2 exists as tooling.
