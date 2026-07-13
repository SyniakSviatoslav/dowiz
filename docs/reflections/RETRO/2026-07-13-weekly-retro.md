# Council retro — 2026-07-13 (weekly librarian curation, meta-loop P1)

INBOX held 7 reflections at start (≥3 threshold — Council retro triggered per README).
Critics: cause-critic · pattern-critic · ratchet-critic. Executor: librarian (this session).
Each line below → artifact OR explicit no-op.

## Per-reflection verdicts (cause-critic)
- **advisory-arm-revival → CONFIRMED (high).** Prose-only obligations with no artifact a gate
  inspects; matches ledger #48 exactly.
- **governance-gates-rot-open → CONFIRMED (high).** Mutable state-file release condition with no
  expiry; 400+ blind ALLOWs in `.claude/logs/classification.log` post-06-21 corroborate. Matches
  ledger #47.
- **plane-maintainer-env-probe → CONFIRMED (medium-high).** `get`-before-`update` on cached remote
  trigger state is sound and reproducible; downgraded off "high" only because the downstream
  "secrets reach checkout" sub-claim is unverified outside the cloud session transcript.
- **plane-telemetry-closed-loop → CONFIRMED (high on both roots).** Durable-local-disk illusion
  (property of the writer confused with property of the store) and the uncommitted-toolchain
  blind spot are both directly observable, not coincidence. Matches ledger #49.
- **trace-config-source-before-mutating → CONFIRMED (high).** Deploy-target-as-diagnostic-harness
  is directly observable in command history (3 wasted Fly-secret resets vs. a 3-second local
  repro). Matches ledger #52.
- **ci-pre-prod-verification-2026-07-03 → CONFIRMED (high) on the P1–P6 pattern; DOWNGRADED to
  medium on FULL-mode migration preflight** — `ci-migration-preflight.mjs` FULL mode is
  self-tested but has never run end-to-end in CI against a real prod `pg_dump`. Matches ledger
  #51/#52.
- **design-system-prune-collision → CONFIRMED (high).** Shared, ownerless git-index state under
  concurrent commits is concrete and reproducible (two commits, `a0c9abcb`/`06471162`, are the
  proof). No ledger row exists — correctly flagged by the prior curation pass as a
  concurrent-session hazard, not a file-pattern-triggerable lesson.

## Systemic roots (pattern-critic)
- **Cluster 1 — "a proxy/stand-in trusted as authoritative without re-sync before an irreversible
  act."** Instantiated 4×: plane-maintainer-env-probe (memory note vs. re-`get` of remote trigger
  state), plane-telemetry-closed-loop (local disk vs. the real durable store), trace-config-
  source-before-mutating (assumed secret store vs. the one the job actually reads),
  ci-pre-prod-verification P2/P3 (staging schema vs. prod's actual schema/roles).
- **Cluster 2 — "recurring obligations expressed as prose discipline with no deterministic
  artifact a gate inspects, so they silently stop."** Instantiated 2×: advisory-arm-revival,
  governance-gates-rot-open.
- **One-off (not a pattern):** design-system-prune-collision's git-index-ownership hazard appears
  once; not promoted to a systemic root.

## Ratchet disposition (ratchet-critic) — each root → artifact or explicit no-op
1. **Cluster 1 → NO NEW ARTIFACT — already covered.** The 3 file-pattern-triggerable instances
   (secret-store, schema-drift, role-rotation) are already lessons (`2026-07-03-secret-store-
   provenance-trace.md`, `2026-07-03-prod-staging-schema-drift.md`, `2026-07-03-rotate-prod-role-
   staging-rehearsal.md`) backed by ledger #51/#52 guardrail scripts. The 4th instance
   (plane-maintainer's remote MCP trigger/env state) has **no hook to attach to** — the
   `pre-edit-lessons` hook only fires on `Edit`/`Write`/`MultiEdit` `tool_input.file_path`, never
   on an MCP tool call — so no lessons/ entry is possible; this stays a documented no-op.
   **Open human item (not new this retro, re-surfaced):** the 3 CI preflight scripts under ledger
   #52 are still unwired into `.github/workflows/ci.yml` (protect-path) — see PROPOSALS below.
2. **Cluster 2 → NO NEW ARTIFACT — already covered.** `2026-07-02-gate-state-file-expiry.md`
   (TRIGGER `.claude/state/**`, `.claude/hooks/**`) plus ledger #47 (`guardrail-gate-armament.mjs`)
   and #48 (`guardrail-ledger-integrity.mjs`, `loops-registry-sync.mjs --check`,
   `agent-health-pass.mjs`, hook-v2 event logging) already enact "expiry in the state, armament
   test, one log line per decision." No new artifact needed.
3. **One-off (design-system-prune-collision) → NO ACTIONABLE ARTIFACT + reason.** A shared git
   index with no session-owner under concurrent commits is a process/tooling hazard (worktree
   isolation, session scheduling), not a code-level invariant ESLint/hooks/tests can encode. Its
   own guardrail candidate #1 ("pre-commit refuses when staged paths' most-recent editor differs
   from the committing session") is self-flagged infeasible (no session-attribution primitive
   exists). Recorded as a PROPOSAL for human/Council decision, not enacted here (librarian is
   read-only on process/tooling calls of this shape).

## Librarian actions taken this pass
- All 7 INBOX reflections **archived** to `docs/reflections/ARCHIVE/` — each was already fully
  distilled into an existing lesson + ledger row (or, for plane-maintainer-env-probe and
  design-system-prune-collision, correctly has no mechanizable artifact and needed no further
  distillation beyond this retro's documented no-op).
- **No new lessons written** — ratchet-critic confirmed every confirmed root is either already
  covered by an existing lesson/guardrail pair or has no deterministic hook to attach to.
- **No lessons pruned** — `docs/lessons/INDEX.md` has 12 rows, none older than 21 days (harness-
  events.jsonl logging only started 2026-07-02, so no lesson yet qualifies for the "zero-hit,
  >30 days" prune rule), and none are fully superseded by a guardrail in a way that would make the
  lesson pure dead weight (this repo's own precedent — e.g. row #13/css-comment-star-slash — keeps
  the lesson alongside its guardrail as a pre-edit prevention layer, not a redundant duplicate).

## Monotonic check
Nothing weakened; no gate touched; no product/contract/migration file touched. Store size:
`docs/lessons/` unchanged (12 → 12); `docs/reflections/INBOX/` 7 → 0; `docs/reflections/ARCHIVE/`
+7.
