# Reflection: plane-maintainer agent's first cloud SENSE run

## CONTEXT
First-ever cloud-scheduled firing of the plane-maintainer routine (`docs/governance/
plane-maintainer-agent.md`). SENSE step (`node scripts/plane-report.mjs --github-issue-on-fail`
+ `pnpm verify:all --ci`) surfaced a real, well-evidenced hard-fail state on `main`, plus two
smaller process frictions during my own run. Filed: PR #8 (fix), issue #9 (escalation), PR #10
(digest). Ledger row #50 attempted but blocked (see WHY 3).

## DECISIONS
- Fixed `scripts/plane-report.mjs`'s `capture()` bug in-envelope (pure JS bug, no red-line
  surface, not deployed) rather than deferring it — small enough to fully verify red→green in
  one pass.
- Did NOT attempt to author the 9 missing guardrail scripts `verify-all.ts`/`plane-guard.mjs`
  reference. Escalated instead: one is RLS/privilege-adjacent (red-line per charter), the real
  fix location (`chore/design-system-prune`) is a 1327-file branch too large to merge
  unilaterally, and 3 of the 9 are unverifiable against every branch/reflog/dangling-object I
  checked — a human call on all three counts, not a same-run fix.
- Left `docs/regressions/REGRESSION-LEDGER.md` row #50 unwritten rather than self-invoking
  `.claude/state/serious-override` to push past the block.

## WHERE
`scripts/plane-report.mjs`, `scripts/lib/capture-merge.{mjs,test.mjs}` (PR #8);
`docs/governance/plane-status-2026-07-02.md`, `loops/runs/dep-baseline.json` (PR #10);
`scripts/verify-all.ts`, `scripts/plane-guard.mjs`, `docs/regressions/REGRESSION-LEDGER.md`
rows 27/29/30/32/33/47/48 (read, not written — issue #9); `.claude/hooks/serious-gate.sh` (read
only — the block that stopped the ledger-row edit); `loops/runs/predictions.jsonl` (3 predictions
recorded but stuck unresolvable, see WHY 2).

## WHY-causal

**1. The referenced-but-unmerged-branch pattern repeated within hours of being named.**
The previous session's own reflection ([[plane-maintainer-agent-2026-07-02]] — literally the
session that wrote this charter) already named this exact failure class in its NEXT-TIME list:
*"Before any remote-config update that references repo paths... check the referenced files exist
on the branch the remote consumer reads."* Commit `a84f6d7`'s own message even disclosed a
narrower version of it ("3 wiring checks live on the unmerged phase0/gate-rearm lineage"). Yet
`verify-all.ts` on `main` still references 9 scripts across two commits' worth of work
(06-29/06-30 AND 07-02), undercounting the gap both times. **Root cause:** a lesson written in
prose (a reflection, a commit-message aside) doesn't defend against a repeat unless something
mechanical checks it at commit time — e.g. a guardrail that fails a commit touching
`verify-all.ts` if any referenced `node scripts/X.mjs` / `pnpm Y` token doesn't resolve to a real
file/script on the current branch. Ironically, `scripts/guardrail-ledger-integrity.mjs` (also
missing) was supposed to be exactly this class of self-check, just for ledger numbering, not
script-reference integrity — the same root cause (advisory-only prose, no deterministic gate) may
explain *why* it too never actually landed anywhere: nothing forced the wiring/script pair to
ship atomically.

**2. My own predict→emit ordering broke this run's own calibration data.** The charter's
TELEMETRY instruction says "at each step boundary emit," and I emitted a `sense`-start event as
the very first action of SENSE, before recording today's 3 predictions. `plane-telemetry.mjs
resolve`'s M1 anti-backdating guard (rightly) refuses to resolve a prediction whose
`ts_predicted` is not strictly before the run's first event — so none of today's 3 predictions
(`78ef9fdaaaf3`/`532355773f21`/`34c1bb5f6736`) can ever be resolved, by design. **Root cause:**
the charter names "predict" and "emit at every step boundary" as two separate musts without
sequencing them relative to each other, and the natural reading of "SENSE step" (run the gates,
emit as I go) put an emit before the predict. This is a documentation gap, not a tooling bug — the
M1 guard did exactly its job (caught a would-be-backdated-looking prediction), the charter just
doesn't say "predict is the FIRST action of the run, before any other telemetry call."

**3. A path-substring gate false-positives on the one file it should welcome edits to.**
`serious-gate.sh`'s SERIOUS regex includes the bare token `ledger` (aimed at financial/payment
ledgers) but matches on the file PATH, not content — so it fires on every edit to
`docs/regressions/REGRESSION-LEDGER.md` itself, the governance regression ledger, regardless of
what the diff says. The allowlist at the top of the hook (`docs/design/*|docs/adr/*|docs/
governance/*|loops/*|.claude/*`) already exempts sibling governance-doc directories on the theory
that appending a *record* of an already-reviewed decision isn't itself a new serious decision —
`docs/regressions/*` is the one ledger-shaped doc directory that isn't on that list, so it's the
one place that theory doesn't apply, purely by omission.

## CONFIDENCE
High on findings 1 and 3 — both are directly reproduced (a failing `pnpm verify:all --ci` run;
a failing Edit tool call with the exact deny reason quoted). Medium-high on finding 2 — the
refusal is reproduced, but whether "predict must be the first telemetry call of the run" is the
*intended* reading of the charter (vs. me having simply gotten the order wrong some other way) is
my interpretation, not confirmed by the charter's author.

## NEXT-TIME
- Before adding a `verify-all.ts`/`plane-guard.mjs` step that shells out to `node scripts/X.mjs`
  or `pnpm Y`: confirm the referenced file/script is committed on the SAME branch as the wiring,
  in the SAME commit or PR — not "landing soon on another branch." A `git show HEAD:<path>` check
  in the same pre-merge review would have caught this before merge, twice.
- Sequence charter step 1 explicitly: **predict first**, before any `emit`, `resolve`, or gate
  run that itself calls `plane-telemetry.mjs emit`. (`plane-report.mjs` itself emits a
  `report_start` event — a bare `node scripts/plane-report.mjs` early in SENSE would trip the same
  M1 refusal for anyone.)
- If `docs/regressions/REGRESSION-LEDGER.md` append is meant to stay routine (it's the "add a
  guardrail + a row" ratchet rule in CLAUDE.md, invoked on *every* fix), add it to
  `serious-gate.sh`'s always-pass allowlist alongside the other governance-doc directories — the
  file's role (append-only record of already-reviewed fixes) matches the allowlist's own stated
  rationale, and the current keyword match is friction with no corresponding safety benefit.

## LINK
- [[plane-maintainer-agent-2026-07-02]] · [[memory-corpus-meta-patterns-2026-07-02]]
- PR #8 (fix), issue #9 (escalation), PR #10 (digest)
