---
date: 2026-07-10
slug: fixes-proposed-never-merged
surface: governance-plane / self-improve loop / PR review cadence
qualifies: "recurrent calibration hit (2+ consecutive runs, same target) — result-vs-expectation doubt trigger"
---

# The self-improve loop proposes correct fixes but has no mechanism to land them

## CONTEXT
Daily plane-maintainer run, 2026-07-10. SENSE/DIAGNOSE/HEAL found zero hard fails
(`verify:all --ci` exit 0, `plane-guard` 12/12 hard checks pass in both static and `--staging`
mode). Calibration step resolved 3 predictions from `run-20260709T0603`, all `hit`:
- `a6d82ae33108` (pr-backlog): none of 8 named open draft PRs merged; backlog is now 12 open
  drafts, zero merges since PR #7 landed on 2026-07-02.
- `b0f6a125d345` (dep-baseline-persistence): `loops/runs/dep-baseline.json` confirmed absent on
  this fresh checkout (gitignored per `.gitignore:80-83`); PR #14, which fixes exactly this, is
  still open.
- `07d4e6013391` (cloud-egress): `fly.io` and `api.telegram.org` both still 403 via the sandbox
  proxy; `flyctl` absent; install blocked too (403 on `fly.io/install.sh`).

## WHERE
`mcp__github__list_pull_requests` (state=all) on `syniaksviatoslav/dowiz`; `.gitignore:80-83`;
`docs/governance/plane-status-2026-07-09.md` (yesterday's identical prediction).

## WHY (causal — not just where)
This is not three unrelated misses turning up as hits — it's one structural gap surfacing three
times. The plane-maintainer's own daily runs correctly diagnose real bugs (dep-baseline not
surviving ephemeral checkouts, predictions.jsonl not hydrating from the telemetry branch — the
same class of bug I had to hand-work-around today by `git show origin/telemetry/plane:...` into
`loops/runs/predictions.jsonl` before `resolve` would find yesterday's predictions) and open
correctly-scoped draft PRs against them (#14, #21, #13, #11 are all fixes for exactly this class
of "local ephemeral-checkout state doesn't survive to tomorrow" bug). But the loop's autonomy
envelope stops at "open a GitHub PR" — merging is out of scope by design (no MAY-list entry for
it), and nothing else in the system merges these PRs either. So the same root causes get
re-diagnosed and re-proposed-as-fixed day after day while the actual fix sits unmerged. The
self-improve loop is closed on the *diagnose→propose* half and open on the *land* half — which
means "hit" calibration on this target is really tracking a growing backlog, not stable ground
truth.

## CONFIDENCE
High. This is the second consecutive day (`07-09` → `07-10`) the identical target
(`pr-backlog-*`) resolved `hit` with a worsening number (8 named PRs → 12 open drafts), and two
of today's *other* hits (`dep-baseline-persistence`, and indirectly the `resolve` hydration gap I
worked around) are literally already-open unmerged PRs (#14, #21) for the same structural cause.

## NEXT-TIME
1. Don't let a correctly-diagnosed, correctly-proposed fix quietly recur as if it were new
   information — when a prediction target repeats "hit" 2+ runs in a row with the same root
   cause, that's the `result-vs-expectation` doubt trigger, not routine calibration noise.
2. This is out of the maintainer's autonomy envelope to fix directly (merging PRs is a human
   decision), so the correct action is escalation via visibility, not a workaround: this run's
   digest leads with "12 open draft PRs, zero merged in 8 days, several are tested fixes for
   recurring bugs" as the top finding, addressed to the human reviewer.
3. Candidate ratchet (for `ratchet-critic` / librarian to evaluate, not self-enacted): a
   `plane-guard` soft-warn already exists for `prediction-resolution-liveness`
   (18/33 unresolved) — consider a sibling check that flags when a *specific* prediction target
   resolves `hit` on the same root cause 3+ consecutive runs, surfacing it distinctly from routine
   backlog noise.

## LINK
[[docs/governance/plane-status-2026-07-09.md]] · [[docs/governance/plane-status-2026-07-10.md]] ·
PR #14 (chore(scout): persist the dep-baseline across ephemeral plane-maintainer runs) ·
PR #21 (fix(plane-telemetry): resolve hydrates predictions from telemetry/plane on a fresh
checkout) · PR #13 (fix(plane-telemetry): publish must not drop tip-only files) · PR #11
(fix(plane-telemetry): preserve branch-only files on a fresh-checkout publish).
