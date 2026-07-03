---
TRIGGER: packages/db/migrations/**role**
CAUSE: >
  A live prod DB role (the runtime OPERATIONAL/SESSION pool role) was rotated/repointed
  directly on prod with no staging rehearsal first. That single act exposed TWO latent
  bugs at once, sequentially: with the role's function grants missing, `startBackgroundWorkers`
  errored fast so the boot stayed inside Fly's health window; once the grants were fixed
  (reactively, on prod), the workers began running to their FULL WORKER_BOOT_BUDGET_MS budget
  for the first time — and that budget (25s) exceeded Fly's `/livez` window (15s interval, no
  grace_period) — health-failing every subsequent deploy. Neither bug was visible until the
  role rotation made the workers' real startup path execute for the first time, on prod.
ACTION: >
  When a migration/runbook step creates, ALTERs, or GRANTs a DB role that the runtime
  OPERATIONAL/SESSION pool will use → cause: prod is the first place the new role's grants (and
  therefore the real worker-startup path) actually get exercised → do: (1) apply the identical
  role/grant change to STAGING first and deploy staging under it — confirm the app actually
  reaches `/livez` inside the health window, not just that `migrate:up` succeeds; (2) run
  `node scripts/ci-migration-preflight.mjs` against the ACTUAL prod schema/role set (not a
  "should be equivalent" stand-in) before flipping the live Fly secret on prod; (3) confirm
  `apps/api/src/server.ts`'s `WORKER_BOOT_BUDGET_MS` (locked ≤5000ms by
  `apps/api/tests/worker-boot-budget-lock.test.ts`) leaves real margin under Fly's health-check
  window before any change that could make workers run longer/differently. Never let the live
  prod secret flip be the first place a role change's effects are observed.
LINK: docs/ops/PROD-UNBLOCK-RUNBOOK-2026-07-03.md ; apps/api/src/server.ts:344 (WORKER_BOOT_BUDGET_MS)
  ; commit db30d273 (25s->3s fix) ; docs/regressions/REGRESSION-LEDGER.md #51, #52
SCOPE: DB role/grant changes that affect the LIVE runtime OPERATIONAL/SESSION pool ONLY
  (packages/db/migrations/**role**, **grant**, and prod-runbook role-rotation steps). Does not
  apply to read-only/reporting roles or CI-only migration roles that never back a live pool.
STATUS: active
---

# Never rotate the live prod DB pool role without a staging rehearsal first

Source: incident 2026-07-03 — rotating `deliveryos_api_user` → `dowiz_app` directly on prod
caused repeated deploy health-failures. The trigger was operational (a role rotation), but the
bugs it exposed were pre-existing and code-level: a 25s worker-boot budget that only manifests
once the role actually has working grants (commit `db30d273`).

The fix for the immediate bug (`WORKER_BOOT_BUDGET_MS` 25s→3s) is now a locked guardrail
(`apps/api/tests/worker-boot-budget-lock.test.ts`, ledger #51) — a future regression of THAT
specific budget is caught deterministically. This lesson covers the residual, non-mechanizable
gap: the *sequencing* discipline of rotating a live-pool role. No static gate can see "did you
rehearse this on staging first" in a diff, so it stays advisory — reinforced by the deterministic
`ci-migration-preflight.mjs` check (ledger #52) that at least proves the role/grants a migration
depends on actually exist on the real target before it runs there.
