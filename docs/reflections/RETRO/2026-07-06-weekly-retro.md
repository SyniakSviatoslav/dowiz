# Weekly Council retro — 2026-07-06

Triggered by the weekly harness curation routine: INBOX held 7 reflections at start (≥3
threshold, CLAUDE.md self-improvement loop step 5). Roster: cause-critic, pattern-critic,
ratchet-critic (isolated contexts, read-only). Executor = librarian (this run) — enacts
doc/lesson-level outputs only; guardrail-level (code) outputs are proposals for a human.

## Inputs

1. `2026-07-02-advisory-arm-revival.reflection.md`
2. `2026-07-02-governance-gates-rot-open.reflection.md`
3. `2026-07-02-plane-maintainer-env-probe.reflection.md`
4. `2026-07-02-plane-telemetry-closed-loop.reflection.md`
5. `2026-07-03-trace-config-source-before-mutating.reflection.md`
6. `ci-pre-prod-verification-2026-07-03.md`
7. `design-system-prune-collision-2026-07-02.md`

## cause-critic — verdicts

All 7 causal WHYs: **CONFIRM, high confidence, no downgrades.** Each stated cause survived a
hostile fresh read — no counter-example found that reduces any claim to a correlate, a
coincidence of timing, or a parallel-deploy effect. Four of the seven (#1, #2, #4, #5/#6 as one
occurrence) are additionally corroborated by concrete ledger evidence (rows #47, #48, #49, #51,
#52) built and proven red→green in the same sessions the reflections describe.

## pattern-critic — cross-reflection structural root

One systemic root spans 6 of 7 reflections: **mutable/cached/assumed state must be re-verified
against its authoritative source immediately before acting on it, or it silently diverges.**
Instances: remote-trigger memory-as-proxy (#3), durable-local illusion + uncommitted-toolchain
blind spot (#4), secret-store provenance (#5), prod≠staging drift (#6), gate-state-without-expiry
(#2), unowned staged git state (#7). Verdict: this is genuinely one shape, not a coincidental
grouping — each instance is "the thing I'm about to act on is a proxy, not the artifact."

Already substantially covered by three existing triggered lessons (`2026-07-02-gate-state-file-
expiry.md`, `2026-07-03-secret-store-provenance-trace.md`, `2026-07-03-prod-staging-schema-
drift.md`). No new cross-cutting guardrail proposed this pass — the high-frequency, file-pattern-
triggerable surfaces are already guarded; the residual instances (#3, #4's second root, #7) are
not file-edit-triggerable by construction (remote API calls / Bash `git commit`), so a repo-local
deterministic artifact doesn't fit them. #1 (advisory-arm) is the meta-instance of the same shape
one level up (prose obligations are "assumed to run" without re-verification against a machine-
checked artifact) and is already the reasoning behind CLAUDE.md's existing Self-improvement-loop
section.

## ratchet-critic — per-root disposition

| # | Reflection | Disposition |
|---|---|---|
| 1 | advisory-arm-revival | → **archive only.** Ledger #48 + guardrails already landed. |
| 2 | governance-gates-rot-open | → **archive only.** Ledger #47 + `docs/lessons/2026-07-02-gate-state-file-expiry.md` already distilled from it. |
| 3 | plane-maintainer-env-probe | → **CLAUDE.md pointer, PROPOSAL** (not enacted — outside librarian's writable scope). Not triggerable as a docs/lessons entry (remote API call, not a file edit). |
| 4 | plane-telemetry-closed-loop | → **archive only** for root 1 (ledger #49). → **no-op** for root 2 (operator/review guidance, not mechanizable as a repo gate). |
| 5 | trace-config-source-before-mutating | → **archive only.** Already distilled into `docs/lessons/2026-07-03-secret-store-provenance-trace.md` (ledger #52) in a prior pass; only the archive move itself was outstanding. |
| 6 | ci-pre-prod-verification-2026-07-03 | → **archive only.** Already distilled into `docs/lessons/2026-07-03-prod-staging-schema-drift.md` + `.../rotate-prod-role-staging-rehearsal.md` (ledger #51/#52); CI wiring remains a standing operator proposal (`docs/proposals/ci-pre-prod-verification-wiring.md`). |
| 7 | design-system-prune-collision | Item 1 (session-attribution pre-commit guard) → **no-op**, self-flagged infeasible. Item 2 (Tier-2 lesson) → **no-op**, would never inject (hook doesn't cover Bash `git commit`). Item 3 (re-verify commit `06471162`) → **no-op** as a standing guardrail (one-off, not mechanizable); guard-bash registration (ledger #47) is the practical forward-looking safeguard. Worktree-isolation-by-default → **PROPOSAL** for human/Council decision. |

## Outcome

- All 7 reflections moved `INBOX/` → `ARCHIVE/` with a curation footer recording the disposition.
- **Zero new lessons** distilled this pass — every confirmed root was either already fully
  captured by an existing lesson/guardrail, or is a genuine no-op (not triggerable by the
  pre-edit-lessons hook's Edit/Write/MultiEdit file-path matching).
- **Zero guardrails** written this pass (none of the confirmed roots needed a new deterministic
  check beyond what already exists).
- Two items carried forward as **PROPOSALS** for a human (see PR body): a CLAUDE.md "Remote
  State Discipline" pointer, and a worktree-isolation-by-default decision for concurrent sessions.
- Every retro line above terminates in an artifact or an explicit no-op with reason — no line is
  change-for-its-own-sake.
