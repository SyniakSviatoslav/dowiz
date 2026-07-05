# Reflection: Sunday cross-pattern memory synthesis is structurally unreachable from the cloud checkout

## CONTEXT
Today's (2026-07-05, UTC Sunday) plane-maintainer firing reached the charter's weekly ritual
step 7 (`docs/governance/plane-maintainer-agent.md` §Weekly rituals — "recompute the memory
corpus link-graph hubs... update the meta-pattern memory"). The target,
`memory-corpus-meta-patterns-2026-07-02` and its siblings, is referenced throughout archived
reflections via `[[wiki-link]]` IDs (e.g. `[[plane-maintainer-agent-2026-07-02]]`).

## DECISIONS
- Searched exhaustively for the corpus inside this checkout: `grep -rl "memory-corpus-meta-patterns"`,
  `find` for any memory/mempalace directory. Found only `MEMORY-MAP.md`'s own note that
  Mem0/mempalace is DEFERRED (not installed) and that this repo's memory is markdown-vault-only —
  no matching corpus file anywhere in `dowiz`.
- Found the actual location named in a prior reflection's WHERE section:
  `/root/.claude/projects/-root-dowiz/memory/plane-maintainer-agent-2026-07-02.md` — an operator-
  local path (`/root/...`), not reachable from this cloud session (which runs as a different user/
  container with GitHub access scoped to `SyniakSviatoslav/dowiz` only, no cross-box filesystem
  access).
- Did NOT fabricate a synthesis over an inaccessible corpus, and did NOT silently skip the ritual —
  recorded the gap plainly in today's digest (`docs/governance/plane-status-2026-07-05.md`) per the
  "report always, success or fail" rule (pattern #11).
- Did not propose a new plane-guard check this run: the actual fix (make the corpus reachable from
  cloud firings, or relocate the ritual's target into the repo) is an operator infrastructure
  decision, not something resolvable from inside this session.

## WHERE
- `docs/governance/plane-status-2026-07-05.md` (☼ Infusion section — the escalation)
- No code/script changed for this finding.

## WHY-causal
The plane-maintainer charter was authored assuming the loop's memory corpus and its git checkout
are colocated (both on the operator's box, per the original `local-2026-07-02-*` run_ids visible in
`telemetry/plane`'s history). The cloud-scheduled variant runs in a genuinely different environment
(ephemeral container, GitHub-scoped access only) — so any ritual step that assumes local-filesystem
reach to `/root/.claude/projects/...` silently cannot execute there, and nothing surfaces that until
someone actually reaches that step and checks. This is the SAME root shape as the already-discarded
`2026-07-02-plane-maintainer-env-probe` reflection ("verify-artifact-not-proxy" — cached/assumed
remote state stood in for a checked artifact) and as the two bugs fixed today in
`scripts/plane-telemetry.mjs`/`scripts/new-dep-scan.mjs` (an ephemeral cloud checkout silently
loses/can't-see state a same-box design assumed would persist/be-reachable). Three independent
instances of "cloud-vs-local-box asymmetry" in the same week is a pattern, not a coincidence.

## CONFIDENCE
High that the corpus is genuinely unreachable from here (exhaustive repo search + explicit path
confirmation from a prior session's own WHERE note). Medium on whether this is worth a dedicated
guardrail vs. an operator-side fix (e.g. mirroring the meta-patterns corpus into
`docs/reflections/` or a repo-tracked location) — that tradeoff needs the operator, since it
touches where the "memory" canonical store lives (a MEMORY-MAP.md decision, not mine to make).

## NEXT-TIME
If this recurs a third time (next Sunday), it should be promoted: either (a) the charter's weekly
step 7 gets an explicit cloud-environment fallback ("if the corpus path is unreachable, synthesize
over `docs/reflections/ARCHIVE/` + `docs/lessons/` instead — the repo-local advisory stores this
session CAN see"), or (b) a `plane-guard` soft check that flags "Sunday ritual step 7 skipped, N
consecutive weeks" the same way `inbox-drain-liveness` flags the librarian backlog — so the gap is
visible in the deterministic gate output, not just prose in a run's digest.
