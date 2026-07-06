---
date: 2026-07-06
slug: predictions-assumed-merge-that-never-happened
surface: governance / plane-maintainer autonomy envelope (PR review bottleneck)
qualifies: "result-vs-expectation doubt trigger (2 non-hit calibration resolutions this run)"
---

# Predictions that assume "my PR merges soon" are miscalibrated — merge is a human action outside my control

## CONTEXT
Resolving yesterday's (`run-20260705T0602`) 3 open predictions during today's SENSE step:
- `pr-13-14-ci` (conf 0.75) → **hit**: PR #13 and #14 both show CI green (validate + fresh-provision +
  Cloudflare Pages all success).
- `predictions-jsonl-durability` (conf 0.65) → **partial**: the *outcome* held (today's inbox still
  shows the 3 predictions, not reset to empty), but the stated premise — "with today's publish fix
  (PR #13) merged" — never happened. PR #13 is still open/draft.
- `inbox-drain-librarian` (conf 0.6) → **miss**: `docs/reflections/INBOX/` still has all 7 files today,
  unchanged. The librarian's drain-to-0 happened only inside draft PR #15, which never merged.

Separately, today's SCOUT step reran `node scripts/new-dep-scan.mjs` and got the *identical* "no
baseline yet" message PR #14 was written to fix two days ago — because #14 is still unmerged, so every
fresh ephemeral checkout still sees zero baseline.

By SENSE time today: **4 open draft PRs sit unmerged** — #8 (opened 2026-07-02, plane-report digest
masking fix), #13 (2026-07-05, telemetry publish fix), #14 (2026-07-05, dep-baseline persistence),
#15 (2026-07-05, digest + INBOX curation). None reviewed, none merged, none closed.

## WHERE
`docs/governance/model-calibration.md` §3 (prediction ledger) · `scripts/plane-telemetry.mjs`
(`resolve`) · GitHub PRs #8, #13, #14, #15 on `SyniakSviatoslav/dowiz`.

## WHY (causal — not just where)
The two non-hit predictions share one root: **I predicted an outcome conditioned on a step I do not
control** ("with today's publish fix merged...", "the librarian drains the backlog" — true only if
that drain lands on `main`). The charter is explicit that merge authority stays with the human (open
a PR, don't bypass); a prediction phrased around "my fix merges" is really a prediction about the
*human's review cadence*, dressed up as a prediction about my own work. That's a category error, not
bad luck — the fix (already shipped in the diff behind #13/#14/#15) is real and tested, but "real and
tested" is not the same claim as "on `main`," and the prediction blurred the two.

This is now the **second week-day in a row** this exact pattern surfaces (predictions from
`run-20260705T0602` already anticipated it partially: `cf2f24fa26a2`'s fallback method explicitly
named "even unmerged, this session's local fix still applies" — which is why that one landed
`partial` rather than a flat `miss`). The recurrence — 4 PRs, spanning 4 days, zero merges — is a
structural signal about review throughput, not a one-off. Per charter step 6 ("a recurrent failure →
propose promoting it to a new plane-guard check"), this crosses the recurrence bar.

## PROPOSED (not enacted — advisory, human/librarian judgment call)
A future soft plane-guard check (`pr-review-backlog-liveness`, GitHub-API-backed, network-optional —
degrade to N/A when GitHub MCP is unavailable) warning when ≥N maintainer-authored PRs are open for
>X days with zero review activity. Not implemented this run: (a) plane-guard.mjs is currently pure
local-file/git, no network call precedent — adding one is a real design decision, not a one-line
addition; (b) opening a 5th unreviewed PR to fix "too many unreviewed PRs" would be self-parody. Left
for the librarian/Council to weigh against the cost of a network-dependent gate check.

## FOR NEXT TIME
Phrase predictions about my own artifacts ("does the fix work," "does CI pass on it") separately from
predictions about human/process cadence ("does it merge by tomorrow") — conflating them is what turned
an accurate technical prediction into a miscalibrated one.
