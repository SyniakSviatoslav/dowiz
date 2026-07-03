# Cross-pattern guardrail proposals (2026-07-02 synthesis lane)

**Status:** PROPOSALS ONLY — advisory. The human / librarian / Council decides what (if anything)
gets enacted. Nothing here is a change; each item is a red→green shape for a *candidate*
deterministic check derived from a pattern that recurs ≥3× in the corpus.

Source patterns: `memory-corpus-meta-patterns-2026-07-02.md` #12 (gates rot / authority-bearing
state), #13 (remote/ephemeral topology), #14 (reify the rule). Evidence memories/reflections cited
per item.

---

## P-A — `gate-release-hygiene` plane-guard check (pattern #12, recurs 3×)

**Recurrence:** `governance-gates-rot-open` reflection (clearance state files with no expiry → 400+
blind ALLOWs) · `advisory-arm-revival` reflection (obligations with no artifact die in ~a week) ·
[[meta-loop-audit-2026-07-02]] (gates silently disarmed, loop couldn't observe its own death).

**What it encodes:** every gate whose *release* depends on a mutable state file must carry the three
things the rot came from missing — an expiry, a DENY-simulating armament test, and a per-decision
log line.

**Proposal (plane-guard.mjs new check):**
- Enumerate the known state-file-released gates (serious-gate, red-line gate, Bash lane, any future
  clearance-file gate). For each, assert:
  1. every non-legacy line in its clearance/state file matches `slug|<expiry-epoch>` and no live
     clearance is past-expiry (stale clearances = FAIL);
  2. an armament test exists that simulates a DENY for that gate (grep for the gate's id in
     `scripts/guardrail-gate-armament.mjs` / the hermetic hook-sim suite) — registration alone FAILs;
  3. the gate writes a decision line to `.claude/logs/classification.log` (or `harness-events.jsonl`)
     — assert the log has grown / has a row schema, not just that the hook is registered.
- **red→green:** RED = drop an expiry from a clearance line, or delete the armament case for a gate,
  or point a gate at a log nothing writes → check exits 1. GREEN = all three present → exit 0.
- **Note:** parts already exist (`guardrail-gate-armament.mjs`, TTL clearances, `agent-health-pass.mjs`
  event log). This proposal is the *umbrella assertion* that ties expiry+armament+log together so a
  future gate cannot ship missing one leg — the generalization of the point-fixes already enacted in
  ledger #47/#48.

**Companion Tier-2 lesson (advisory, for recurring-obligation design):** TRIGGER = editing a
governance/charter/routine doc that introduces a recurring obligation. ACTION = "specify in the same
breath (a) the artifact it must produce, (b) the deterministic checker that inspects it, (c) the event
line it logs — or expect it to die within a week." LINK = `advisory-arm-revival` reflection.

---

## P-B — `remote-ref-integrity` pre-flight check (pattern #13, recurs 3×)

**Recurrence:** `plane-telemetry-closed-loop` (trigger prompt referenced scripts uncommitted locally —
"GOTCHA that bit twice") · `plane-maintainer-env-probe` (act-on-cached-remote-state near-miss;
`get` before `update`) · `design-system-prune-collision` (staged-but-uncommitted index has no owner).

**What it encodes:** a remote consumer (trigger prompt, CI workflow, webhook, routine prompt) that
references a repo path only sees what is committed on the branch it reads. Verify existence-on-remote
before wiring.

**Proposal (script `scripts/remote-ref-integrity.mjs`, optionally wired into plane-guard or a
pre-edit lesson):**
- Parse the known remote-facing config surfaces for repo-path references: the cloud trigger prompt
  (checked-in copy, if any), `.github/workflows/*.yml`, any webhook config. Extract referenced script
  paths (`scripts/*.mjs`, etc.).
- For each referenced path, assert it exists on the branch the remote consumer reads — via
  `git ls-tree origin/<branch> -- <path>` (or `git cat-file -e origin/<branch>:<path>`), NOT just on
  the working tree.
- **red→green:** RED = reference a `scripts/foo.mjs` from the trigger prompt while `foo.mjs` is only on
  disk / on a feature branch → exit 1 listing the missing remote refs. GREEN = all referenced paths
  resolve on the target remote branch → exit 0.
- **Scope caveat:** the cloud trigger *prompt* lives in claude.ai remote state, not the repo, so this
  check can only validate the checked-in surfaces (CI/webhooks/committed prompt copies). For the
  trigger prompt itself the enforcement is the lesson below, not a repo check.

**Companion Tier-2 lesson (advisory):** TRIGGER = editing a CI workflow / webhook config / any file
that names `scripts/*` or repo paths a *remote* consumer will execute. ACTION = "before shipping,
`git ls-remote` / `git cat-file -e origin/<branch>:<path>` every referenced path — remote consumers
bind to REMOTE state, not your disk. And for remote *mutable* state (triggers/envs/secrets/DNS): `get`
before `update`/`run`; treat any memory note about it as a hypothesis to re-read, never a fact to act
on." LINK = `plane-maintainer-env-probe` + `plane-telemetry-closed-loop` reflections.

**NOT proposed as a hard check (honest):** the `design-system-prune-collision` item 1 ("pre-commit
refuses to commit staged deletions the committing session didn't stage this run") is self-flagged
*may be infeasible* — no session-attribution primitive exists. The cheaper, human-decidable route is
**worktrees-by-default for concurrent sessions** (a process/Council call, not a librarian promotion).
Leaving that for the Council retro, not filing it as a deterministic check.

---

## P-C — pattern #14 (reify the rule): advisory lesson only, no hard check

**Recurrence:** [[tooling-decision-patterns-2026-07-02]] (12-rule grammar extracted) ·
[[redteam-pilot-tools]] (grammar re-applied to 14 tools) · [[plane-maintainer-agent-2026-07-02]]
(11 meta-patterns imposed as `plane-guard.mjs`).

**Why no hard check:** #14 is a move that *creates* gates; there is no clean deterministic assertion
for "did you extract the grammar." A check here would be Goodhart-bait. Propose instead a Tier-3
CLAUDE.md-pointer / lesson:

**Companion lesson (advisory):** when the same *decision* recurs ≥3× (adopt/defer/reject a tool,
classify a plane, triage a class of finding), stop re-deciding ad hoc — extract the decision grammar
and, where the rule maps to a concrete assertion, add it to `plane-guard.mjs` rather than leaving it
as prose. But keep the grammar in the **advisory** box (#4): it frames a decision, it does not make
one. This is the meta-ratchet; it belongs to the librarian/Council, not to an automated gate.

---

**Filed by:** synthesis lane (memory-corpus meta-pattern maintenance). No code changed, nothing
committed. Enact via librarian promotion (red→green + ledger row) or Council retro only.
