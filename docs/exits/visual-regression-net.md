TASK: Critical-Path Visual Regression Net — full matrix, CI-homed, bug-sweep now (operator-approved)

ENRICHED-DONE:
- Harness reuses existing e2e (geo helpers, dev/mock-auth) + adds: visual config (reducedMotion, frozen
  tz, perceptual threshold, [data-dynamic] masks), globalSetup, loginAs(role), seedVisualState (open/
  closed/busy/stoplist locations + owner/courier + a seeded order), via a dev-gated /dev/seed-visual-state.
- Full critical-path matrix authored: client (menu→product→cart→checkout→status), owner (dashboard→
  confirm→assign→ready), courier (tasks→pickup→delivery→deliver) × {390,768,1280} × {al,en} × key states.
- Determinism: dynamic zones (RelativeTime, MapLibre, Recharts, avatars, pickup_code/QR) carry
  data-dynamic and are masked. Baselines are NOT generated/locked here (no Docker) — that runs in CI.
- CI workflow (Postgres service + pinned Playwright image) prepared as a proposed patch (.github protected).
- Bug-sweep NOW: critical-path screens captured vs staging, eyes-reviewed by parallel agents, real bugs
  fixed before any lock.

EXIT CHECKLIST (written before code):
[ ] Harness spine (config + globalSetup + loginAs + seedVisualState + MASK) typechecks — proof: pnpm typecheck
[ ] /dev/seed-visual-state endpoint is dev-gated (DEV_AUTH_SECRET), idempotent, no prod surface — proof: file:line + 404-without-secret note
[ ] Matrix specs authored for all 3 roles, reference the harness + masks — proof: file list + grep toHaveScreenshot count
[ ] data-dynamic attrs on the listed dynamic components — proof: grep data-dynamic across the critical-path components
[ ] CI workflow proposal present (Postgres + pinned image + --update then compare) — proof: docs/.../proposed-visual-ci/APPLY.md
[ ] Bug-sweep run: candidates captured vs staging, parallel vision review, findings routed/fixed — proof: findings doc + before/after for any inline fix
[ ] N/A — baseline lock + 2-3 green Docker runs + intentional-regression-red: requires Docker/CI (not in sandbox); runbook handed to CI/operator
[ ] Ledger row for the regression net guardrail

After VERIFY: each [ ] → PASS/FAIL/FLAG.
