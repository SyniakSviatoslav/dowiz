# Design Proposal — Safe-Reversal / Restore Spine (Data · Schema · Deploy)

- Status: **STEP 1 — FRAME + PROPOSE** (Triadic Council). Design-only; NO production code in this round.
  Breaker attack + Counsel opinion follow; conductor dispositions in `resolution.md`.
- Date: 2026-07-03
- Author: System Architect (DeliveryOS)
- Companion ADR: `docs/adr/ADR-safe-reversal-spine.md` (DRAFT)
- Source findings: `docs/design-review/audit-reliability-2026-07-03.md` (C1–C3, H6, H9, H10, M11–M14),
  `docs/design-review/recon3-privacy-ops-2026-07-03.md` (O-H2, O-H4/H5/H6, O-H7, O-M1, O-M4, O-M8),
  `docs/design-review/AUDIT-SYNTHESIS-2026-07-03.md` (LC7, §3 R-C).
- Relates / does-not-contradict: `docs/design/ci-pre-prod-verification/proposal.md` (the staging
  gate + preflight jobs — this proposal *consumes* that gate, it does not redesign it),
  ADR-pg-privilege-hardening (grant migrations that a restore must re-apply), Phase-0
  protect-paths (backup/restore code is inside the protected surface), the open-source goal
  (ADR-020: the spine must not depend on Supabase-proprietary features as its *only* net).
- Red-line class: **data-recovery** (irreversibility). Everything here is COUNCIL-gated by definition.

---

## 0. Frame — what "safe reversal" means here, and what exists today

An operation is *safely reversible* when, after it goes wrong, a **rehearsed, provably working**
path returns the system to a known-good state within a stated time bound. The 2026-07-03 audits
established that dowiz has **no such path at any layer** — and the docs claim otherwise at every
layer, which is worse than absence (operators will reach for a net that isn't there).

Verified current state (all against the working tree, 2026-07-03):

| Layer | Claimed | Actual |
|---|---|---|
| **Data** | "R2 logical backups are the sole recovery net… restore-test every 24h" (`docs/phase5/disaster-recovery.md:10-11`) | `BACKUP_ENABLED` defaults `'false'` (`packages/config/src/index.ts:82`) and is set nowhere in `fly.toml` → **prod has never taken a backup**. Even if enabled: no `pg_dump` in the runtime image (`Dockerfile:41-56` — only `argon2 sharp @aws-sdk/*` installed at :53) → `spawn pg_dump ENOENT` (C1). The drill reads a nonexistent `backup_metadata.metadata` column (`backup-verify.ts` `selectBackup`, verified) and its row-count "validation" queries the **live** DB (C2). The restore CLI's full-restore path is a stub — `scripts/backup-restore.ts:212` prints "Full restore not yet implemented" + `exit(1)` (O-H7) — and both `--list` and manifest resolution read `backup_metadata` on **the DB you just lost** (`backup-restore.ts:64-86,168-185`, C3). The runbook's `--snapshot=<id>` syntax doesn't even parse (`indexOf`-based flags, `backup-restore.ts:189-206`; O-M8). |
| **Schema** | "No down-migration in production… data is preserved" (`docs/phase5/rollback.md:5-9`) | All 157 migrations ship a `down()` that prod never runs (dead code), while **up()** migrations are destructive with no window: `DROP TABLE promotions CASCADE` (`1790000000017`), `DROP COLUMN cash_pay_with` (`1781101098646`, a money field), `DELETE FROM categories` by heuristic (`1790000000019:6`). `migrate:up` passes `--no-check-order` (`package.json:38`). The runner's `singleTransaction: true` is a false guarantee — migrations 011/042 `COMMIT` mid-stream (O-H5). Only recovery from a bad destructive migration is the Layer-1 restore, which doesn't work → **effectively unrecoverable** (O-H6). |
| **Deploy** | "Rollback: `flyctl deploy --image <previous-tag>`" (`rollback.md:54`) | No source of truth for the previous tag; CI never records it. Push-to-main deploys prod with `deploy: needs: validate` only (`ci.yml:134`) — no staging gate, `fresh-provision` doesn't gate (O-H3). The 4 post-deploy Playwright suites (`ci.yml:161-190`) run **after** prod is live and trigger nothing on failure (O-M4). Migrations run **twice**: CI `migrate:up` at `ci.yml:150-153` minutes before `flyctl deploy`, then `fly.toml:14-15 release_command` again (O-H2 — DB-ahead-of-code window). |

**Governing principle (from the harness):** a reversal path that has never been drilled is a hope,
not a mechanism. Every layer below therefore ships with a **drill that fails when reversal is
broken** — the drill is part of the design, not an afterthought.

**Non-goals:** multi-region HA; streaming replication; making the 157 `down()` functions real
(§ Layer 2 argues this is actively harmful); redesigning the CI staging gate (owned by
`ci-pre-prod-verification`); any GDPR-erasure fixes themselves (P-H1 — separate council), though
§1.6 handles the backup↔erasure interaction.

---

## 1. Layer 1 — DATA: a backup/restore net that provably works

### 1.1 Topology fact that shapes everything

Prod DB = **Supabase Free tier** (`disaster-recovery.md:5-14`; Supavisor `:6543/:5432` URLs in
`packages/config/src/index.ts:7-9`). Staging DB = **Fly Postgres** (`dowiz-staging-db`). So any
"use the provider's native backups" answer covers prod only, and — critically — the
**2026-07-03 secrets incident is live**: prod Supabase superuser creds sat in git history. An
attacker with provider-account access can destroy the DB *and* its provider-side backups in one
motion. Therefore: **a provider-independent, separately-credentialed copy is mandatory regardless
of which option is primary.** (3-2-1 discipline: ≥2 stores, 1 outside the blast radius of the
primary credential.)

### 1.2 Option A — self-managed pg_dump → AES-GCM → R2 (fix the existing chain)

The whole chain already exists in code (`apps/api/src/workers/backup/*`,
`scripts/backup-restore.ts`) and this branch already fixed the H6 lock-arg bug and the pg-boss
queue policy (`workers/backup/index.ts:40-59,220-228` — verified). What remains broken, and the fixes:

| # | Defect (audit ref) | Fix | Who |
|---|---|---|---|
| A-1 | No `pg_dump`/`pg_restore` in runtime image (C1) | Install a **version-matched** `postgresql-client` (PGDG repo pin to the Supabase PG major; `node:22-slim`'s default bookworm client is 15 and will refuse or mis-dump a newer server) in the runtime stage; boot-probe `pg_dump --version` and FATAL when `BACKUP_ENABLED=true` and the binary is absent or major-mismatched. ~30 MB image cost. | **operator** (Dockerfile = protect-path) |
| A-2 | `BACKUP_ENABLED` never set in prod | `fly.toml [env] BACKUP_ENABLED = "true"` (worker process only needs it) | **operator** (fly.toml) |
| A-3 | Restore CLI needs the dead DB (C3) | Add `--list-r2` (enumerate `dowiz-backups/<env>/**/*.manifest.json` straight from the bucket, newest first) and `--r2-key=<manifest key>` modes. The manifest is already self-sufficient (`backupId, checksumSha256, r2Key, encryption{iv,authTag,keyId}, rowCounts` — `backup-restore.ts:28-36`); **the DB lookup becomes an optimization, never a dependency.** Also fix `--flag=value` parsing (O-M8). | agent (scripts/, protect-path ack) |
| A-4 | Full restore is a stub (O-H7) | Implement `runFullRestore`: download → decrypt → checksum → `pg_restore --clean --if-exists --no-owner --no-acl` with **stderr parsed** — whitelist only "does not exist, skipping"; any other ignored error = FAIL (kills H9's exit-1=success class before it's reborn) → **re-apply grants** (re-run the grant migrations `1790000000069/080` et al. or a generated `grants.sql`) as an in-script step, not a runbook footnote (kills H10) → print verification summary. Env guard: refuse URLs whose host matches a known-prod allowlist unless `--i-am-restoring-prod` + typed target confirmation (M12). | agent (protect-path ack) |
| A-5 | Drill validates an imagined format (C2) | Rewrite the drill per §1.5 — manifest from **R2**, plaintext-vs-plaintext checksum, scratch-DB round-trip. Remove `@ts-nocheck` from `workers/backup/` (it masked every one of C2's three bugs). | agent (protect-path ack) |
| A-6 | Key escrow single-point (M13) | `BACKUP_ENCRYPTION_KEY` currently exists only as a Fly secret — if the Fly app dies with the DB, every backup is undecryptable ciphertext. Escrow the key in **two** independent places: the SOPS/age vault (`secrets/` — currently empty, O-M7; this gives it its first real tenant) and the operator's password manager. Make `keyId` real (per-rotation id in the manifest, keyring lookup on restore) instead of the hardcoded `'primary'`. | **operator** (holds the secrets) + agent (keyring code) |
| A-7 | R2 blast-radius + retention holes (M11, P-M2) | Backup writer gets a **write-only** R2 token (no delete — lifecycle rules do the deleting); restore/drill use a read-only token. Fix lifecycle rules to match the actual `${NODE_ENV}` prefix. Add a "never delete the last N verified restore points" guard to any retention logic. Recreate the bucket with R2 `jurisdiction: 'eu'` (dumps contain all customer PII). | **operator** (Cloudflare console) |
| A-8 | Advisory lock on the transaction pooler | `handleBackup` still takes its advisory lock on `operationalPool` (`:6543` transaction mode — session locks unreliable there). Port the dedicated-client session-pool pattern already proven in `backup-verify.ts` `acquireLock/releaseLock`. | agent (protect-path ack) |

- **RPO:** 1h (hourly cron `0 * * * *` already configured, config:84). Honest statement: worst case
  = 1h + dump duration.
- **RTO:** see §1.7 back-of-envelope — mechanical path ≤ 1h; ≤ 4h including detection + human.
- **Cost:** ≈ $0.15/mo R2 storage (§1.7); zero new compute if in-image (A-1), <$1/mo if the
  sidecar variant below is chosen.

**Sub-option A′ — dedicated backup Fly Machine instead of in-image pg_dump.** A tiny scheduled
Machine (`postgres:<major>-alpine` + the dump/encrypt/upload script) triggered by Fly Machines
schedule, with its own read-only DB credential. Pros: app image stays lean; backup cadence
decoupled from app deploys; its credential can be *read-only* (pg_dump needs SELECT only) so the
backup plane can't write to prod. Cons: a second deployable artifact + secret set to govern; the
existing pg-boss scheduling/audit/manifest code is discarded or duplicated. **Verdict: keep as a
later hardening step; A (in-image) reuses the existing, now-mostly-fixed worker and is one
Dockerfile line away.** The A′ *credential* idea survives immediately though: give the backup
worker a dedicated read-only role URL instead of `DATABASE_URL_MIGRATIONS`
(`workers/backup/index.ts:108` currently dumps as the migrations superuser — least-privilege fix).

### 1.3 Option B — Supabase-native backups / PITR

Verified from `disaster-recovery.md:5-16` + provider knowledge (pricing **must be re-verified by
operator at purchase time**):

| Tier | What you get | RPO | Fit |
|---|---|---|---|
| Free (today) | **Nothing.** No PITR, no restorable provider backups, auto-pause risk | ∞ | — |
| Pro (~$25/mo) | Daily logical backups, ~7d retention, dashboard restore; removes auto-pause | **24h** | Worse RPO than fixed Option A; but buys reliability (no auto-pause) + a second independent net |
| Pro + PITR add-on (~$100/mo tier) | WAL-based point-in-time, RPO ~2 min | ~2 min | Best RPO; restore is dashboard-driven and in-place |

Tradeoffs vs Option A:
- **For B:** zero code to maintain; PITR's RPO (~2 min) is unreachable with logical dumps; restore
  is provider-tested.
- **Against B (as the only net):** (i) same-account blast radius — the leaked-creds incident makes
  "attacker deletes project + its backups" a live scenario, and provider backups die with the
  account; (ii) covers prod only — staging is Fly PG; (iii) not drillable into a scratch target
  without paying for a second project — you cannot run the §1.5 round-trip proof against it, so it
  fails the "reversal is proven, not hoped" principle; (iv) contradicts the open-source/portability
  direction (ADR-020) if it's the *sole* mechanism; (v) $100/mo for PITR is disproportionate for a
  pilot-scale app today.

### 1.4 Decision — A is the spine, B-Pro is a cheap belt when budget allows

1. **Primary: Option A fixed** — provider-independent, drillable, EU-pinned, separately
   credentialed, ~free. This is the mechanism the drill proves and the runbooks describe.
2. **Secondary (operator budget call, recommended at first revenue):** Supabase **Pro** for the
   auto-pause removal + the free daily provider-side backup as a second net. **PITR add-on
   deferred** until order volume makes >1h data loss commercially unacceptable; when adopted, RPO
   target tightens from 1h to 5 min and Option A drops to daily cadence (it remains the
   off-provider copy).

### 1.5 The Layer-1 PROOF — a restore drill that fails when restore is broken

Replace the current drill (which has never been able to pass, C2) with a **round-trip drill**
whose every assertion is against a *real artifact* and a *scratch target*:

1. **Locate from R2, not the DB:** newest `*.manifest.json` under the env prefix (same code path
   as `--list-r2` — the drill exercises the disaster path by construction).
2. **Download + decrypt** with the env key; sha256 of the **decrypted** file vs
   `manifest.checksumSha256` (plaintext-vs-plaintext — the writer hashes the plaintext temp file,
   `workers/backup/index.ts:135`).
3. **Restore into a scratch DB** — never the live one: scheduled runs restore into a dedicated
   `dowiz_drill` database on the staging Fly PG (or a throwaway schema-per-run); the CI variant
   restores into a `postgres:16` service container. `createSessionPool()` takes no args
   (C2c root) — the drill gets an explicitly-parameterized connection, or the pool factory grows a
   `connectionString` parameter (small `packages/db` change, protect-path ack).
4. **Assert row-level integrity:**
   - per-table `COUNT(*)` vs `manifest.rowCounts` (recorded at dump time — the writer already
     produces this);
   - **restore canary:** each backup cycle writes one row
     `(backup_id, taken_at)` to a `backup_canary` table *before* dumping; after restore, assert
     `canary.backup_id == manifest.backupId`. This is the assertion that fails on the two silent
     failure modes counts can miss — restoring the *wrong* artifact, or a restore that silently
     no-ops onto a pre-existing scratch schema;
   - freshness: `canary.taken_at` within (cadence + slack) of now — catches "drill green against a
     week-old artifact while the writer is dead" (the H6-class failure).
5. **pg_restore exit-code discipline** identical to A-4.
6. **Outcome is load-bearing:** result row → `backup_metadata`-adjacent ops table → `/health`
   `backup_restore` check derives from the *last drill result + age*; `never_run` becomes
   **degraded**, not ok (reliability H7 sibling). Red drill → Telegram owner alert (BUS channel
   already exists: `BACKUP_FAILED`).

**Cadence + gating:** daily scheduled (existing `RESTORE_VERIFY_CRON` 04:00 UTC slot) once fixed;
plus a **weekly CI workflow** (scheduled GitHub Action, postgres service container) so a broken
restore is a visible red ✗ in the repo, not only a Telegram ping — CI wiring is protect-path
(operator paste-in, same pattern as `ci-pre-prod-verification`).

**Red→green proof obligation at implementation time (harness rule):** the drill must first be
demonstrated failing against (a) a corrupted artifact, (b) a wrong-key decrypt, (c) an empty
scratch DB with the canary assertion disabled writer-side — then green on the real chain.
A drill never seen red is not a guardrail.

### 1.6 Retention & the GDPR interaction (flag to Counsel)

Keep the configured ladder (hourly 24h / daily 30d / weekly 90d — config:88-90) **except monthly
7y** (config:90, runbooks.md:18). A full-PII logical dump retained 7 years means every Art.17
erasure (already broken, P-H1) is silently un-erased in cold storage for 7 years. Propose:
**monthly retention 13 months**, and a documented restore step: "after any restore, re-run
completed erasure requests newer than the snapshot" (`gdpr_erasure_requests` is in the dump, so
the list survives the disaster). Anything longer than 13 months needs a legal-basis decision by a
human, not a default.

### 1.7 Back-of-envelope (RTO / RPO / cost)

DB today: Supabase Free caps at 500 MB; pilot-scale actual likely 20–100 MB. `pg_dump -Fc -Z9`
artifact ≈ 5–30 MB.

| Step (disaster path, Option A) | Estimate |
|---|---|
| Detect (health degraded / owner report) | 5–30 min (dominant human term) |
| Provision fresh Supabase project / Fly PG | 5–10 min |
| `migrate:up` fresh schema | 1–2 min |
| `--list-r2` + download + decrypt 30 MB | < 2 min |
| `pg_restore` ≤ 100 MB | 1–5 min |
| Re-grant + verify (`verify:db`, test order) | 10–15 min |
| Secrets/DNS cutover (`flyctl secrets set DATABASE_URL_*` → restart) | 10–20 min |
| **Total mechanical** | **~30–55 min** |

**Targets: RPO = 1 h (hourly dump; stretch 5 min iff PITR later) · RTO = 2 h target / 4 h max**
(unchanged from runbooks.md:7-8, but now backed by a rehearsed path; the runbooks' internal 1h-vs-4h
RPO contradiction, O-M8, resolves to 1h). Costs: R2 ≈ 10 GB total across the ladder → ~$0.15/mo;
in-image pg_dump $0; optional backup Machine <$1/mo; Supabase Pro $25/mo (optional); PITR ~$100/mo
(deferred).

---

## 2. Layer 2 — SCHEMA: migrations you can walk back from

### 2.1 The honest `down()` verdict: roll-forward + snapshot, not fake reversibility

157 `down()` functions exist and zero have ever run in prod (`rollback.md:5` policy). Are they
salvageable as real rollbacks? **No — and trying would be harmful:**
- Many are destructive-by-construction (`DROP TABLE … CASCADE` in `1790000000028:47`,
  `1780348982032:35` etc.) — "rolling back" destroys data the up() created legitimately.
- Some are provably broken already: `1780421100060:86` `down()` does `SET NOT NULL` on a column
  that anonymization legitimately nulls (M16) — it fails on any real dataset.
- Data-mutating ups are not invertible at all: `1790000000019`'s `DELETE FROM categories` has no
  inverse; no `down()` can restore heuristic-deleted rows.
- Maintaining 157 untested inverses is an unbounded false-confidence liability — exactly the
  C2-class failure (a reversal path never exercised against reality).

**Decision:** `down()` stays as a *local-dev convenience only*; the production reversal for a bad
migration is **restore the pre-migrate snapshot** (mechanism below). Document this in
`rollback.md` so nobody reaches for `migrate:down` in an incident. (`migrate:down` remains wired
in `package.json:39` for dev; a prod-URL guard on that script is a cheap extra: refuse when the
target host matches the prod allowlist.)

### 2.2 Non-destructive migration POLICY: expand-contract with a deprecation window

Standing rule (goes into the ADR + `rollback.md` + CLAUDE.md pointer):

1. **Expand:** additive change ships first (new column/table, nullable or defaulted; dual-write if
   renaming). Old code keeps working against the new schema — this is precisely what makes
   Layer-3 image rollback *safe*.
2. **Backfill/cutover:** data migrated; code switches reads; verified on staging.
3. **Contract:** the destructive DDL (`DROP`/`RENAME`/`SET NOT NULL`) ships **≥ 1 release AND
   ≥ 7 days after** the last code reference died, in its own migration, annotated (below), with a
   fresh pre-migrate snapshot proven present.

### 2.3 The guardrail: `migration-lint` that FAILS CI on destructive UP DDL

A new `scripts/migration-lint.mjs` (agent-implementable; sibling of the existing
`scripts/verify-migrations.ts`, wired into `verify:all --ci` which already gates CI —
`ci.yml:40-41` — so **no `.github` edit is needed for the lint to bite**):

- **Scope:** only migrations *newer than a committed baseline list* (the existing 157 are
  grandfathered — the lint is a ratchet, not an archaeology project).
- **Fails on, in `up()` text:** `DROP TABLE`, `DROP COLUMN`, `RENAME TO/COLUMN`, `TRUNCATE`,
  `DELETE FROM` (non-`pgboss`/non-ops-table), `ALTER COLUMN … SET NOT NULL` without a paired
  `DEFAULT`/backfill in the same file, `ALTER COLUMN … TYPE` without `USING`.
- **Escape hatch:** an explicit annotation block —
  `// @destructive reason=<…> expanded-in=<migration id> window-start=<date> snapshot=required` —
  plus the migration id listed in a reviewed `packages/db/migrations/DESTRUCTIVE.manifest`. The
  annotation is *audit trail + human friction*, not a rubber stamp: CI also asserts
  `window-start ≥ 7 days ago` relative to the referenced expand migration's commit date.
- Additionally lints: new file timestamp must be > max existing (closes the `--no-check-order`
  gap M15 going forward without breaking the already-interleaved history), and bans
  `pgm.noTransaction()` in the same file as any DDL touching an existing table (contains the
  O-H5 mid-stream-COMMIT class).
- **Red→green proof:** ship with `__fixtures__` — one destructive migration fixture the lint must
  fail, one annotated fixture it must pass, one out-of-order-timestamp fixture it must fail.

### 2.4 Pre-migrate automatic snapshot — the mechanism that makes "roll forward" survivable

`fly.toml release_command` (`dist/migrate/index.cjs`) becomes a thin wrapper (design, not code):

```
release:  detect pending migrations
          ├─ none      → exit 0 (no snapshot cost on code-only deploys)
          └─ pending   → pg_dump -Fc → encrypt → R2 `dowiz-backups/<env>/pre-migrate/<sha>-<ts>`
                         (reuses the Layer-1 chain; requires A-1's pg_dump in image — the release
                          machine runs the same image)
                       → verify upload (HEAD + size > 0)  → run migrations
                       → on snapshot FAILURE: abort the release (fail-safe: Fly keeps old code
                         serving, same semantics as a migration failure today)
```

Properties: a bad migration is now recoverable to T-minus-seconds (RPO for the migration-disaster
case ≈ 0, far better than the hourly cadence); the snapshot is keyed to the deploy SHA so the
Layer-3 runbook can name it; `pre-migrate/` gets its own retention (keep last 10). Cost: seconds
on a ≤100 MB DB, only on migration-bearing deploys. This also single-handedly de-fangs the
grandfathered-destructive-migration risk while the expand-contract culture beds in.

**Operator-gated:** the wrapper touches the release path (`fly.toml`/build script) — staged as a
paste-in like the CI wiring. **Agent-implementable:** the wrapper script itself in `scripts/`,
proven on staging first (staging deploys are inside ship-discipline).

### 2.5 The Layer-2 PROOF

- **Lint drill (CI, every run):** the fixtures in §2.3 — the gate is red if the lint stops
  catching destructive DDL.
- **Snapshot-restore drill (staging, quarterly + after any wrapper change):** on staging, apply a
  sacrificial bad migration (fixture: adds a table, deletes seed rows) → confirm the pre-migrate
  snapshot exists in R2 → restore it into the staging DB → assert the deleted seed rows are back
  (row-level assertion, same canary discipline as §1.5) → `fresh-provision`-style boot check.
  Scripted end-to-end so "quarterly" is one command; result logged to the drill ops table.
- **Existing net retained:** `fresh-provision` (ci.yml:57-131) stays the from-scratch bootability
  proof; `ci-migration-preflight.mjs` (FULL mode: prod-clone → migrate) from the CI-hardening
  proposal proves migrate-ability against *prod-shaped* data pre-deploy.

---

## 3. Layer 3 — DEPLOY: rollback as a command, not an archaeology session

### 3.1 Kill the double-migration path (pick ONE — the Fly release_command)

Delete the CI `Migrate Database` step (`ci.yml:150-153`). Rationale: `release_command` runs
migrations **pre-traffic, atomically coupled to the rollout, and aborts the rollout on failure**
(fly.toml:8-15 comment documents exactly this design); the CI step runs them minutes early against
old-serving code (O-H2's DB-ahead-of-code window) and keeps a second prod-credential in GitHub.
One migration path, one credential surface, one failure mode. **Operator-gated (.github).**
Interlock: this must land *with or after* the §2.4 wrapper design decision so the release path is
the only and the safest path.

### 3.2 Rollback source of truth: record the image ref at deploy time

Fly keeps release/image history (`flyctl releases` — today's saga rolled a machine by hand). But
the runbook's `--image <previous-tag>` has no recorded tag (O-M4). Design:

- CI deploy step captures the **image digest** it just shipped
  (`flyctl releases --json | jq -r '.[0].ImageRef'` — exact flag/shape to be re-verified against
  the installed flyctl at implementation; machines-era flyctl removed `releases rollback`, so we
  do NOT design against that subcommand) and (a) writes it to the job summary, (b) pushes a
  lightweight git tag `deploy/prod/<run-number>` → the previous good ref is always
  `deploy/prod/<n-1>`, greppable offline even when Fly's API is the thing that's down.
- A `/version` route returning `GIT_SHA` (env already threaded — `server.ts:62`) so "what is
  actually serving" is a curl, closing the split-identity gap (O-M8).

### 3.3 Automated rollback on failed post-deploy smoke

Today the 4 post-deploy suites are detection-only (ci.yml:161-190). Design:

```
deploy → record prev_ref (§3.2) → flyctl deploy → slim read-only prod smoke
   ├─ green → done
   └─ red   → IF this deploy carried NO new migrations:
              │    flyctl deploy --image <prev_ref>   (immediate strategy)
              │    re-run smoke against rolled-back prod → alert either way
              └─ ELSE (migrations present): NO auto-rollback — alert + print the
                   one-command manual runbook (§3.4) + the pre-migrate snapshot id (§2.4).
                   Rationale: old image + new schema is safe ONLY under the §2.2 expand-
                   contract policy; until the lint has been enforcing that policy for a
                   while, auto-reverting a migration-bearing deploy is a second gamble
                   stacked on the first. Revisit (flip to auto) once the ratchet has aged.
```

Note the rollback re-runs `release_command` with the OLD migrator — which no-ops (node-pg-migrate
skips applied migrations), so rollback does not fight the schema; expand-contract guarantees the
old code tolerates the newer schema. The smoke itself moves to the shape the CI-hardening proposal
defines (mutating tests → staging pre-prod; prod post-deploy = read-only health/menu checks that
cannot need `DEV_AUTH_SECRET` on prod).

### 3.4 One-command manual rollback

`scripts/rollback-prod.sh` (agent-implementable; execution is operator-run by definition):
resolve `deploy/prod/<n-1>` tag → confirm typed app name → `flyctl deploy --image <ref>` →
poll `/version` until it reports the expected SHA → run the read-only smoke → print result.
Replaces the current 5-step git-revert crawl in `rollback.md` Phase 2; the git-revert path is
demoted to "fix-forward", which is what it actually is.

### 3.5 Staging gate + health-grace (consumed, not redesigned)

- The **staging gate** (deploy-staging job + staging E2E on the same commit gating `deploy`) and
  `deploy: needs: [validate, fresh-provision, preflight, staging-verify]` are specified in
  `docs/design/ci-pre-prod-verification/proposal.md` — this spine depends on them as the
  *pre-prod* net and adds the *post-prod* reversal (§3.3). Same-image discipline (runtime flags,
  not build-args) from that proposal is what makes "green on staging" mean anything (O-M1).
- **Health-grace:** the goes-green-then-dies pattern (O-H1: post-`listen()` boot asserts
  `process.exit(1)` after `/livez` already passed) must be reconciled — asserts move pre-listen
  (fail closed pre-traffic, so a bad release aborts at `release_command`/boot instead of
  crash-looping live) — flagged here because an auto-rollback (§3.3) keyed on machine health would
  otherwise flap. Owned by the reliability lane; interlock only.

### 3.6 The Layer-3 PROOF — a rollback drill on staging

Scripted, run **weekly + after any change to the deploy pipeline**: deploy staging at HEAD
(vN) → `rollback-staging.sh` to vN-1 → assert `/version` == vN-1's SHA (the assertion that fails
if rollback silently no-ops or Fly served a cached machine) → staging smoke green → roll forward
to vN again. Wall-clock target recorded each run: **rollback ≤ 5 min** or the drill is red.
Fully agent-drivable (staging is inside ship-discipline), zero prod risk.

---

## 4. The spine, assembled — mechanism table

| Layer | Reversal mechanism | Implementable-now vs gated | PROOF (drill that fails if reversal is broken) |
|---|---|---|---|
| **1 · DATA** | Hourly pg_dump→AES-GCM→R2 (EU, write-only token), manifest-self-sufficient restore CLI with `--list-r2` + real full-restore + re-grant; key escrowed in 2 stores off-Fly; Supabase Pro as optional second net | Scripts + drill + worker fixes: **agent** (Phase-0 protect-path ack). Dockerfile pg_dump, fly.toml `BACKUP_ENABLED`, R2 tokens/jurisdiction, key escrow, Supabase plan: **operator** | **Daily scratch-DB round-trip drill**: R2-manifest → decrypt → pg_restore → assert canary `backup_id` + row counts + freshness; result drives `/health` (never_run=degraded) + weekly CI red. Proven red first (corrupt artifact / wrong key / stale canary) |
| **2 · SCHEMA** | Expand-contract policy + `migration-lint` failing CI on unannotated destructive UP DDL + **pre-migrate auto-snapshot** in release_command; `down()` demoted to dev-only, prod reversal = snapshot restore | Lint + fixtures + snapshot wrapper script + staging rehearsal: **agent** (lint bites via already-wired `verify:all --ci`). Release-path wiring (fly.toml), migration POLICY adoption: **operator** | **Lint fixtures red/green in every CI run**; quarterly staging drill: sacrificial bad migration → restore pre-migrate snapshot → assert deleted seed rows return |
| **3 · DEPLOY** | Image-ref recorded per deploy (`deploy/prod/<n>` tags + `/version`); auto-rollback on red smoke for migration-free deploys; `scripts/rollback-prod.sh` one-command manual path; single migration path (release_command only); staging gate consumed from ci-pre-prod-verification | Rollback scripts + `/version` + staging drill: **agent**. ci.yml changes (delete CI migrate, wire auto-rollback, staging gate): **operator** (.github protect-path, paste-in staged) | **Weekly staging rollback drill**: vN → vN-1, assert `/version`==vN-1 SHA + smoke green + ≤5 min wall clock |

## 5. Ordering — what makes prod safe fastest

1. **Day 0 (operator, no code): one real, verified restore point.** Manual
   `pg_dump -Fc` of prod from a workstation → encrypt (existing key) → upload to R2 → restore it
   into a scratch DB and see real rows. Prod currently has **zero** backups of any kind; this
   single act moves the worst-case from "total permanent loss" to "lose ≤ a day". Agent writes the
   ~20-line script + checklist; operator (holding prod creds) executes.
2. Restore CLI made disaster-real (A-3, A-4) + drill rewrite (§1.5), proven red→green against the
   Day-0 artifact on a scratch DB. *(agent, protect-path ack)*
3. Dockerfile pg_dump + `BACKUP_ENABLED=true` → automated hourly cadence + daily drill live.
   *(operator)*
4. Migration-lint ratchet in `verify:all --ci` (§2.3) — stops the destructive-DDL bleeding while
   everything else proceeds. *(agent)*
5. Staging gate + kill CI-migrate + image-ref recording + rollback script (§3) — one ci.yml
   paste-in batch with the ci-pre-prod-verification wiring. *(operator)*
6. Pre-migrate snapshot wrapper (§2.4) + staging snapshot-restore drill. *(agent design/staging →
   operator release-path wiring)*
7. Key escrow + R2 write-only tokens + EU jurisdiction + retention/GDPR alignment (§1.6). Supabase
   Pro when budget allows. *(operator)*

## 6. Open questions for Breaker / Counsel

1. **Breaker:** the restore canary (§1.5) — can a partially-failed pg_restore still land the
   canary table and go green? (Ordering of tables in custom-format restore; should the canary
   assert additionally on the *last*-restored table by dependency order, or on
   `pg_restore --list` TOC-count == restored-count?)
2. **Breaker:** §3.3's "no new migrations ⇒ safe auto-rollback" — enumerate deploys that break old
   images *without* migrations (env-schema additions in code, contract drift, R2 key rotation).
3. **Counsel:** §1.6 monthly-retention-vs-Art.17 — is 13 months defensible; must restore-time
   re-erasure be a *blocking* step in the restore script rather than a runbook line?
4. **Counsel/operator:** dump-as-migrations-superuser (A′ note) — least-privilege read-only backup
   role: worth the extra role now, or after the flip?
5. **Operator:** confirm current Supabase PG major (determines the PGDG client pin in A-1) and
   re-verify Pro/PITR pricing before any §1.4-secondary purchase.
