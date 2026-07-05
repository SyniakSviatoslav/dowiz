# ADR — Safe-Reversal / Restore Spine (Data · Schema · Deploy)

- **Status:** DRAFT — Council STEP 1 (FRAME + PROPOSE). Awaiting Breaker findings + Counsel opinion;
  no production code authorized by this ADR.
- **Date:** 2026-07-03
- **Deciders:** Triadic Council (architect + breaker + counsel) → human operator (red-line:
  data-recovery / irreversibility)
- **Design doc:** `docs/design/safe-reversal-spine/proposal.md` (mechanism detail, verified
  file:line evidence, back-of-envelope, drills)
- **Source audits:** `docs/design-review/audit-reliability-2026-07-03.md` (C1–C3, H6, H9, H10,
  M11–M16), `docs/design-review/recon3-privacy-ops-2026-07-03.md` (O-H2/H5/H6/H7, O-M1/M4/M8),
  `docs/design-review/AUDIT-SYNTHESIS-2026-07-03.md` (LC7)

## Context

The 2026-07-03 audit round found the single biggest ops gap: **no structural undo at any layer.**
Verified against source, not just the audit docs:

- **Data.** Prod (Supabase **Free** — no provider PITR/backups) has **never taken a backup**:
  `BACKUP_ENABLED` defaults `'false'` (`packages/config/src/index.ts:82`) and is unset in
  `fly.toml`; the runtime image has no `pg_dump`/`pg_restore` (`Dockerfile:41-56`); the restore
  CLI's full-restore path is a stub (`scripts/backup-restore.ts:212` → exit 1) and resolves its
  manifest from `backup_metadata` on the very DB a disaster loses (`:64-86`); the daily "restore
  drill" reads a column that doesn't exist and validates row counts against the **live** DB
  (`workers/backup/backup-verify.ts` `selectBackup` / C2). The documented sole recovery net is
  inoperable end-to-end.
- **Schema.** Prod is forward-only by policy (`docs/phase5/rollback.md:5`) while all 157
  migrations carry never-run `down()`s, several **up()** migrations are destructive with no
  deprecation window (`DROP TABLE promotions CASCADE`, `DROP COLUMN cash_pay_with`,
  `DELETE FROM categories`), `migrate:up` runs `--no-check-order` (`package.json:38`), and the
  runner's single-transaction claim is false (migrations 011/042 COMMIT mid-stream). A bad
  destructive migration's only recovery is the (inoperable) backup restore.
- **Deploy.** Push-to-main deploys prod gated on lint/typecheck/build only
  (`ci.yml:134 deploy: needs: validate`); the post-deploy Playwright smoke is detection-only; the
  rollback runbook points at an unrecorded image tag; and migrations run **twice** per deploy
  (CI `migrate:up` at `ci.yml:150-153` + `fly.toml:14-15 release_command`), opening a
  DB-ahead-of-code window.

A live aggravator shapes the data-layer decision: the 2026-07-03 secrets incident put prod
Supabase superuser creds in git history — provider-account compromise (and with it, deletion of
any provider-side backups) is not hypothetical.

## Decision

Adopt a three-layer **safe-reversal spine**, each layer defined by a mechanism **plus a drill
that fails when reversal is broken** ("a reversal path never drilled is a hope, not a mechanism"):

1. **DATA — self-managed, provider-independent restore chain as primary.** Fix the existing
   pg_dump→AES-256-GCM→R2 chain rather than buying Supabase PITR as the sole net:
   version-matched `postgresql-client` in the runtime image (+ boot probe when
   `BACKUP_ENABLED=true`); `BACKUP_ENABLED=true` in prod; restore CLI gains `--list-r2` /
   `--r2-key` so the R2 manifest (already self-sufficient: iv/authTag/keyId/checksum/rowCounts)
   is the source of truth and the primary DB is never a restore dependency; real full-restore
   with parsed `pg_restore` stderr (whitelist "does not exist, skipping" only), in-script
   re-grant step, and a prod-host guard; `BACKUP_ENCRYPTION_KEY` escrowed in two stores
   independent of Fly (SOPS/age vault + operator password manager) with a real per-rotation
   `keyId` keyring; backup writer uses a write-only R2 token, bucket pinned to R2 EU
   jurisdiction; backup role becomes read-only (not the migrations superuser).
   **Supabase Pro (~$25/mo) is a recommended *secondary* net when budget allows; the PITR add-on
   (~$100/mo) is deferred** — as the only mechanism it fails on same-account blast radius,
   covers prod only (staging is Fly PG), cannot be drilled into a scratch target, and
   contradicts the ADR-020 portability direction. Pricing re-verified at purchase.
   **Targets: RPO 1 h (hourly dump; 5 min iff PITR later) · RTO 2 h target / 4 h max**
   (mechanical path ≈ 30–55 min on a ≤100 MB DB). Retention: hourly 24 h / daily 30 d /
   weekly 90 d / **monthly 13 months (down from 7 y — Art.17 interaction; > 13 mo requires an
   explicit human legal-basis decision)**, with a "never delete the last N verified restore
   points" guard and post-restore re-application of completed erasures.
   **Proof:** daily scratch-DB round-trip drill — newest R2 manifest → decrypt → `pg_restore`
   into a scratch DB → assert restore-canary `backup_id`, per-table row counts vs manifest, and
   canary freshness; result drives `/health` (`never_run` = degraded) and a weekly scheduled CI
   run; drill demonstrated red (corrupt artifact / wrong key / stale canary) before trusted green.

2. **SCHEMA — expand-contract policy + migration-lint + pre-migrate snapshot; `down()` demoted.**
   The 157 `down()` functions are **not** salvageable as production rollbacks (several are
   destructive or provably fail on real data); the honest model is **roll forward + restore the
   automatic pre-migrate snapshot**. `down()` remains dev-only, with a prod-URL refusal guard.
   Policy: destructive DDL ships only in the contract phase, ≥ 1 release AND ≥ 7 days after the
   expand phase, annotated. Guardrail: `scripts/migration-lint.mjs` wired into the
   already-CI-gated `verify:all --ci` — fails on unannotated `DROP TABLE/COLUMN`, `RENAME`,
   `TRUNCATE`, `DELETE FROM`, `SET NOT NULL` without default, `TYPE` without `USING` in `up()`
   of any migration newer than a grandfathered baseline; also enforces monotonic timestamps for
   new files and bans `noTransaction` mixed with existing-table DDL. Mechanism: the
   `release_command` wraps migrations with an automatic encrypted pre-migrate snapshot to
   `dowiz-backups/<env>/pre-migrate/<sha>-<ts>` whenever migrations are pending; snapshot upload
   failure aborts the release (old code keeps serving).
   **Proof:** lint red/green fixtures execute in every CI run; quarterly staging drill applies a
   sacrificial bad migration, restores the pre-migrate snapshot, and asserts the destroyed rows
   return.

3. **DEPLOY — recorded image refs + auto-rollback for migration-free deploys + one-command
   manual rollback + single migration path.** Delete the CI `Migrate Database` step;
   `release_command` is the sole migration path (pre-traffic, rollout-coupled, fail-safe). CI
   records each shipped image digest as git tag `deploy/prod/<n>` (+ job summary); a `/version`
   route exposes the serving `GIT_SHA`. On red post-deploy smoke: if the deploy carried no new
   migrations → automatic `flyctl deploy --image <prev_ref>` + re-smoke; if migrations were
   present → alert with the pre-migrate snapshot id and the manual runbook (auto-rollback of
   migration-bearing deploys is deferred until the expand-contract ratchet has aged).
   `scripts/rollback-prod.sh` = one-command manual path (resolve `deploy/prod/<n-1>` → deploy →
   poll `/version` → smoke). The pre-prod staging gate, preflight jobs, and same-image
   (runtime-flag) discipline are **consumed from**
   `docs/design/ci-pre-prod-verification/proposal.md`, not redesigned here; the O-H1
   goes-green-then-dies boot-assert reconciliation is an interlock owned by the reliability lane.
   **Proof:** weekly staging rollback drill — deploy vN, roll back to vN-1, assert
   `/version == vN-1 SHA` + smoke green + wall-clock ≤ 5 min.

**Ordering (prod-safe fastest):** (0) one operator-run manual pg_dump → encrypt → R2 → verified
scratch restore **today** — prod's first-ever restore point; (1) restore CLI + drill made
disaster-real and proven against that artifact; (2) Dockerfile/pg_dump + `BACKUP_ENABLED` →
automated cadence; (3) migration-lint ratchet; (4) ci.yml batch (staging gate, kill CI-migrate,
image tags, rollback); (5) pre-migrate snapshot wrapper; (6) key escrow / R2 hardening / Supabase
Pro.

**Implementable-by-agent** (with Phase-0 protect-path acknowledgement where the path is guarded):
`scripts/backup-restore.ts` fixes, drill rewrite + `@ts-nocheck` removal in `workers/backup/`,
`migration-lint` + fixtures, snapshot-wrapper script, `rollback-*.sh`, `/version` route, staging
drills, runbook corrections. **Operator-gated:** `Dockerfile`, `fly.toml`, `.github/workflows/*`
(staged as paste-ins), R2 tokens/jurisdiction/lifecycle, key escrow, Supabase plan changes,
prod-credential executions.

## Consequences

- Prod gains a rehearsed recovery path with stated, drilled bounds (RPO 1 h / RTO ≤ 4 h); a bad
  migration becomes recoverable to T-minus-seconds via the pre-migrate snapshot; a bad deploy
  becomes a ≤ 5-min image rollback instead of a ~15-min git-revert crawl.
- Three standing drills (daily data, per-CI + quarterly schema, weekly deploy) convert "restorable"
  from a doc claim into a monitored signal (`/health`, CI red, Telegram) — the C2/M14 class
  (verification written against an imagined system) is structurally prevented because every drill
  consumes the real artifact/pipeline.
- Costs: ~30 MB image growth (postgresql-client); ~$0.15/mo R2; seconds of release latency on
  migration-bearing deploys; optional $25/mo Supabase Pro later. Engineering cost is concentrated
  in scripts/drills, not new infrastructure.
- The destructive-migration lint adds friction to schema work by design (annotation + window);
  grandfathering the existing 157 keeps it a ratchet, not a rewrite.
- New failure modes accepted and mitigated: snapshot-upload failure blocks migration deploys
  (fail-safe by choice); auto-rollback is deliberately scoped to migration-free deploys to avoid
  stacking a second gamble on a first; `down()` is explicitly demoted so no operator reaches for
  an untested inverse in an incident.
- Monthly backup retention drops 7 y → 13 months (GDPR Art.17 interaction) pending Counsel; any
  longer retention requires an explicit human legal-basis decision.

## Alternatives considered

- **Supabase PITR as the primary/sole net** — rejected as sole net: same-account blast radius
  (live secrets incident), prod-only coverage, not drillable into a scratch target, ~$100/mo at
  pilot scale, portability conflict (ADR-020). Retained as a future RPO upgrade layered on the
  self-managed spine.
- **Dedicated backup Fly Machine (sidecar) instead of in-image pg_dump** — deferred: cleaner
  privilege separation but a second artifact/secret surface; the in-image fix is one Dockerfile
  line and reuses the existing (now mostly fixed) worker. Its least-privilege idea (read-only
  backup role) is adopted immediately.
- **Making the 157 `down()` migrations real production rollbacks** — rejected: many are
  destructive-by-construction or provably fail on real data (e.g. `1780421100060` down's
  `SET NOT NULL` vs anonymized rows); maintaining untested inverses recreates the exact
  false-confidence class (C2) this ADR exists to kill.
- **Auto-rollback for ALL failed deploys including migration-bearing ones** — deferred until the
  expand-contract lint has demonstrably governed several release cycles.
- **Keeping the CI `migrate:up` step and dropping `release_command`** — rejected: the CI step
  migrates ahead of code with no rollout coupling (O-H2); `release_command` is pre-traffic and
  aborts the rollout on failure.

## Proof obligations (Mandatory Proof Rule mapping — for the implementation phase)

Each mechanism lands only with its drill red→green: (1) data drill fails on corrupted artifact /
wrong key / stale canary, then passes on the real chain; (2) migration-lint fails its destructive
fixture, passes the annotated one; snapshot drill restores destroyed staging rows; (3) staging
rollback drill asserts `/version` flip. Regression-ledger rows per guardrail; none of these gates
may later be weakened (harness ratchet).
