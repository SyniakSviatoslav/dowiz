# Breaker Findings — Safe-Reversal / Restore Spine

- Mode: STEP 2 (BREAK) — adversarial, READ-ONLY. No product code touched; this file only.
- Target: `docs/design/safe-reversal-spine/proposal.md` + `docs/adr/ADR-safe-reversal-spine.md`.
- Every claim re-verified against the working tree (Dockerfile, fly.toml, ci.yml, `workers/backup/**`,
  `scripts/backup-restore.ts`, `packages/db/**`, migrations) — I re-read the proposal's own citations.
- Verdict: the spine's *mechanisms* are mostly sound, but its **proofs** (the drills) are not. The
  central Layer-1 drill is structurally false-green on the real chain, and two Layer-3 source-of-truth
  primitives (`deploy/prod/<n>` tags, `/version`) don't exist / race. The C2 "verification written
  against an imagined system" class the ADR exists to kill is **re-created inside the fix**, not removed.

## Severity counts
- CRITICAL: 1
- HIGH: 4
- MEDIUM: 7
- LOW: 3
- Total: 15

---

## CRITICAL

### BRK-1 · B-CONSIST / data-recovery · The Layer-1 restore proof is structurally false-green
- **Invariant broken:** a proof that passes on a broken restore is not a proof (the exact C2/M14
  false-confidence class the ADR §Consequences claims is "structurally prevented").
- **Break scenario:** the drill's row-integrity assertion (proposal §1.5.4: "per-table `COUNT(*)` vs
  `manifest.rowCounts` (recorded at dump time — the writer already produces this)") is broken two ways
  in the writer it reuses:
  1. `manifest.rowCounts` is a **hardcoded 9-table list** (`manifest.ts:39-43`: orders, order_items,
     order_item_modifiers, courier_assignments, settlement_items, courier_payouts, backup_audit_log,
     locations, customers). A restore that silently drops or truncates any of the other ~40 tables
     (`products`, `menu_versions`, `users`, `couriers`, `categories`, `gdpr_erasure_requests`, …) is
     invisible to a `rowCounts` equality check — the drill passes green on a partial restore.
  2. "recorded at dump time" is factually wrong: `generateManifest` runs `SELECT COUNT(*)` on the
     **live** `backupPool` at `index.ts:140`, i.e. **after** the dump was streamed, checksummed
     (`:135`) and uploaded (`:123`). `pg_dump` snapshots at dump *start*; the counts snapshot minutes
     later. Under any write traffic in the dump+upload window the manifest counts ≠ the dump's contents,
     so an equality assertion is **flaky-red** (or must diverge from what it claims to prove).
  The already-shipped smoke check is even weaker: `checkRowCounts` (`smoke-checks.ts:88`) flags an
  outlier only when `count === 0 || count > base*10`, and **skips the check entirely** for any table
  whose baseline is `undefined` (the ~17 EXPECTED_TABLES not in the 9-table manifest). A restore that
  truncates `products` to 1 row: base undefined → no outlier → `pii_free`/`order_totals` see one valid
  row → **drill green on a 99%-data-loss restore.**
- **Evidence:** `apps/api/src/workers/backup/manifest.ts:39-49`; `apps/api/src/workers/backup/index.ts:135,140`;
  `apps/api/src/workers/backup/smoke-checks.ts:13-23,84-91`; proposal §1.5.4, §4 (Layer-1 PROOF cell).

---

## HIGH

### BRK-2 · B-SEC / B-DATA · The daily drill spills full prod PII cross-environment (or self-DoSes prod)
- **Invariant broken:** a full-PII prod dump must not be restored into a lower-trust environment or the
  same capped cluster it is protecting (data-recovery + PII-boundary red-line).
- **Break scenario:** the existing drill restores into `createSandboxDatabase()`, which runs
  `CREATE DATABASE dowiz_restore_sandbox_<ts>` on `DATABASE_URL_ADMIN` (`restore-sandbox.ts:13,28-38`) —
  the **same Supabase cluster** as prod. Supabase Free caps total storage at 500 MB; restoring a ~100 MB
  prod dump into a co-located sandbox roughly doubles the footprint every run and can trip the cap →
  Supabase forces the project read-only → **the recovery drill takes prod down.** The proposal §1.5.3
  instead routes scheduled runs to "a dedicated `dowiz_drill` database on the staging Fly PG" — but the
  dump is full plaintext customer PII (names/phones/`delivery_address`/`delivery_lat,lng`/
  `delivery_photo_key`, per the recon3 PII inventory), and staging shares broad E2E credentials. That
  moves maximally-sensitive prod PII into a lower-trust store **daily**, while the same proposal EU-pins
  R2 for exactly this data (A-7). The drill's blast radius is never analyzed.
- **Evidence:** `apps/api/src/lib/restore-sandbox.ts:12-44`; `packages/config/src/index.ts:126`
  (`DATABASE_URL_ADMIN` optional); proposal §1.5.3, §1.7 (500 MB cap), §1.6 (full-PII dump); recon3 PII
  inventory table (`delivery_lat/lng/photo_key` plaintext).

### BRK-3 · B-CONSIST / B-SEC · Scratch restore is role/extension-incompatible → drill can't go green honestly
- **Invariant broken:** the drill must exercise the *real* disaster artifact end-to-end; a scratch
  target that cannot accept the prod dump's DDL makes the proof unrunnable or fictional.
- **Break scenario:** the prod dump (`--no-owner --no-acl`, `dump.ts:27-28`) still emits `CREATE POLICY …
  TO <role>` and `CREATE EXTENSION` (grants are stripped by `--no-acl`; policies and extensions are not).
  9 migrations reference Supabase roles (`authenticated`/`anon`/`service_role`/`supabase_admin`, e.g.
  `1780421100048_backup-metadata.ts:23` `CREATE POLICY … TO authenticated`), and
  `1780310044710_extensions-and-enums.ts:7-8` requires `citext`+`pgcrypto`. Proposal §1.5.3's "CI variant
  restores into a `postgres:16` service container" has **none** of those roles. Under A-4's strict stderr
  rule (whitelist *only* "does not exist, skipping"), a `CREATE POLICY … TO authenticated` on a missing
  role is a hard error — **not** a whitelisted skip → the drill fails every run. To make it pass, the
  operator must pre-seed the full Supabase role set into the scratch DB (ci.yml `fresh-provision` already
  hand-creates only `dowiz_migrator`+`dowiz_app`, `ci.yml:99-101`), at which point the drill validates a
  role-set that diverges from a genuine fresh-Supabase restore.
- **Evidence:** `apps/api/src/workers/backup/dump.ts:23-32`; grep: 9 migrations with Supabase-role refs;
  `packages/db/migrations/1780310044710_extensions-and-enums.ts:7-8`;
  `packages/db/migrations/1780421100048_backup-metadata.ts:22-23`; `.github/workflows/ci.yml:99-101`;
  proposal §1.5.3, A-4.

### BRK-4 · B-OPS · Deploy image-ref source-of-truth races on concurrent pushes
- **Invariant broken:** the rollback source of truth must deterministically name the previous *good*
  image; a race that mislabels it turns rollback into "deploy a random version."
- **Break scenario:** `ci.yml` has **no `concurrency:` block** (whole file verified). Two pushes to
  `main` start two `deploy` jobs. §3.2 records the shipped ref via `flyctl releases --json | .[0]` and
  pushes git tag `deploy/prod/<run-number>`. `.[0]` (latest release) is a global race: job A can read
  the release *after* job B's `flyctl deploy` landed, tagging image B as `deploy/prod/<A>`. The rollback
  path (§3.4) resolves "previous good" as `deploy/prod/<n-1>` — which under the race points at an image
  that was never the last-good serving version → `rollback-prod.sh` deploys the wrong build during an
  incident.
- **Evidence:** `.github/workflows/ci.yml:133-159` (no `concurrency`, `deploy: needs: validate`);
  proposal §3.2, §3.4.

### BRK-5 · B-CONSIST · `checkPIIFree` contradicts the full-PII backup → "green on the real chain" is unreachable
- **Invariant broken:** the harness proof obligation ("green on the real chain," ADR §Proof obligations)
  must be satisfiable by the *actual* prod artifact.
- **Break scenario:** the reused smoke suite asserts the restored data is PII-free
  (`smoke-checks.ts:209-241`, `passed: totalPii === 0`, scanning `customers`/`orders`/`couriers` with
  `PiiRedactor`). But §1.6 explicitly keeps **full-PII** dumps, and prod stores `customers.name/phone`,
  `orders.delivery_address` as plaintext (recon3 inventory). On any real prod restore the redactor
  matches → `totalPii > 0` → `pii_free` fails → `allPassed` false → drill red (`backup-verify.ts:323-326`).
  Compounded today by C2c: `createSessionPool(sandboxUrl)` discards its arg (`packages/db/src/index.ts:48`),
  so the check runs against the **live** DB and fails there too. The proposal's §1.5 rewrite never
  mentions `checkPIIFree`; keeping it makes the real-chain drill un-green, dropping it silently removes
  an assertion the ADR counts as part of the proof.
- **Evidence:** `apps/api/src/workers/backup/smoke-checks.ts:209-241`;
  `apps/api/src/workers/backup/backup-verify.ts:317-326`; `packages/db/src/index.ts:48-50`; proposal §1.6.

---

## MEDIUM

### BRK-6 · B-ANTIPATTERN · "Manifest already self-sufficient" is overstated — keyId is decorative
- **Invariant broken:** internal consistency of the design (A-3 and A-6 contradict).
- **Break scenario:** A-3 claims the manifest is "already self-sufficient
  (…`encryption{iv,authTag,keyId}`…) → the DB lookup becomes an optimization, never a dependency." But
  `keyId` is the literal constant `'primary'` (`encrypt.ts:25`) and restore reads only
  `env.BACKUP_ENCRYPTION_KEY`, ignoring `manifest.encryption.keyId` (`backup-restore.ts:89-92`). So the
  manifest cannot select a key after rotation — the restore still depends on a single ambient env key
  (M13), which A-6 admits and defers to Ordering **step 7 (last)**. During Ordering steps 2-6 the
  "primary, provider-independent net" is single-key-dependent and undecryptable if the Fly secret dies
  with the DB — the exact same-account blast-radius scenario the proposal invokes to reject Supabase PITR.
- **Evidence:** `apps/api/src/workers/backup/encrypt.ts:25`; `scripts/backup-restore.ts:89-92`;
  proposal A-3 vs A-6, §5 Ordering step 7.

### BRK-7 · B-DATA · A-7 self-contradicts: "write-only token + lifecycle deletes" ⊥ "never delete last N verified"
- **Invariant broken:** retention safety guard must be implementable under the chosen deletion model.
- **Break scenario:** A-7 says the writer gets a **write-only** R2 token ("no delete — lifecycle rules do
  the deleting") *and* "add a 'never delete the last N verified restore points' guard to any retention
  logic." R2 native lifecycle rules are **time-based** (expire after N days — `r2-verify.ts:406-411` per
  M11) and cannot express "keep the last N *verified*." Under the write-only model there is no
  application deletion path to host the guard. So if the writer stalls silently (H6/C1 class), time-based
  lifecycle keeps deleting on schedule and ages out the last good restore point — the precise M11 failure
  the guard is meant to stop **remains**, because the guard has nowhere to live.
- **Evidence:** proposal A-7; audit M11 (`r2-verify.ts:406-411`, `${NODE_ENV}` prefix mismatch).

### BRK-8 · B-CONSIST / GDPR · Post-restore re-erasure loses in-window erasures with no trace
- **Invariant broken:** a completed Art.17 erasure must not silently revert; the restore must know it
  happened.
- **Break scenario:** §1.6 asserts "`gdpr_erasure_requests` is in the dump, so the list survives the
  disaster." It survives only up to the **snapshot**. An erasure requested and completed in the RPO
  window (≤1 h, `config:84`) before the disaster is neither in the dump nor recoverable: after restore
  the subject's PII returns **and** the request row is gone — no record it was ever asked. "Re-run
  completed erasures newer than the snapshot" can only re-run erasures the snapshot still contains, i.e.
  it structurally cannot cover the most recent ones. GDPR-erasure red-line, bounded to the RPO window.
- **Evidence:** proposal §1.6; `packages/config/src/index.ts:84` (hourly); recon3 P-H1/P-L2.

### BRK-9 · B-OPS · migration-lint text-match is bypassed by builder-method DDL already in the tree
- **Invariant broken:** a guardrail with a trivial in-repo bypass is not a ratchet.
- **Break scenario:** §2.3 fails "in `up()` **text**" on `DROP TABLE`/`DROP COLUMN`/`RENAME`/…. But 5
  migrations already express destructive DDL via node-pg-migrate **builder methods** —
  `pgm.dropTable('modifier_group_translations')` (`1780338982027:62`), `pgm.dropColumn('users','password_hash')`
  (`1780694000000:10`), `pgm.renameColumn`/`pgm.dropConstraint` elsewhere — which a text grep for
  "DROP TABLE"/"DROP COLUMN" **misses**. A future contract migration written `pgm.dropColumn(...)` ships
  unannotated. Separately, the atomicity-breaker is the explicit `await pgm.db.query('COMMIT')`
  (`1790000000011:102`, `1790000000042:52`); the lint bans `pgm.noTransaction()`+DDL, not the raw COMMIT,
  so a mid-stream COMMIT without `noTransaction()` slips the O-H5 class through.
- **Evidence:** grep — 5 migrations use `pgm.dropTable/dropColumn/renameColumn`;
  `packages/db/migrations/1780338982027_ai_translations.ts:62-68`,
  `1780694000000_add-password-hash.ts:10`, `1790000000011_pgboss-bootstrap-schema.ts:98-102`,
  `1790000000042_access-request-notify-queue.ts:51-52`; proposal §2.3.

### BRK-10 · B-OPS · Pre-migrate snapshot ordering is ungated → snapshot can capture POST-migrate state
- **Invariant broken:** a "pre-migrate" snapshot must be taken before any migration in the batch runs.
- **Break scenario:** today prod is migrated **twice** — CI `migrate:up` (`ci.yml:150-153`) *then*
  `release_command` (`fly.toml:14-15`). §2.4 puts the snapshot **inside** `release_command`, before
  migrations. But if it lands while the CI `migrate:up` step still exists (Ordering §5 kills CI-migrate;
  §5's snapshot wrapper is §5-step-6, after), the snapshot runs **after** CI already applied the
  migrations → the "pre-migrate" snapshot captures the already-migrated schema → false safety, silently.
  No gate enforces the "snapshot wrapper lands with/after kill-CI-migrate" interlock the proposal itself
  flags (§3.1).
- **Evidence:** `.github/workflows/ci.yml:150-159`; `fly.toml:14-15`; proposal §2.4, §3.1, §5 (steps 5-6).

### BRK-11 · B-OPS · `/version` route does not exist and `GIT_SHA` is unset in prod → the deploy drill can't assert
- **Invariant broken:** the Layer-3 proof asserts `/version == vN-1 SHA`; the asserted signal must exist
  and be populated.
- **Break scenario:** there is **no `/version` route** (grep of `apps/api/src` returns nothing).
  `GIT_SHA` is `z.string().optional()` (`config:124`), consumed only by Sentry init (`server.ts:62`), and
  **not passed by the `ci.yml` deploy step** (`flyctl deploy` at `:157` sets no `GIT_SHA`). So even once a
  route is added it would report `unknown`; the split-identity `RENDER_GIT_COMMIT` (`config:166`,
  `manifest.ts:58`) remains. The §3.6 weekly rollback drill's core assertion has no populated source of
  truth.
- **Evidence:** grep `/version` → none; `packages/config/src/index.ts:124,166`;
  `apps/api/src/server.ts:62`; `.github/workflows/ci.yml:155-159`; proposal §3.2, §3.6.

### BRK-12 · B-SCALE / B-OPS · pg_dump runs through the Supavisor pooler; "version-matched" client can't detect a provider-side major upgrade
- **Invariant broken:** a version guard must compare against the live server it dumps, not a build-time
  constant.
- **Break scenario:** the writer dumps via `DATABASE_URL_MIGRATIONS` = **session pooler :5432** (Supavisor,
  `config:9` comment; `index.ts:108`) — no direct DB connection is declared in the secret set. A-1 pins a
  PGDG client to "the Supabase PG major" **at image-build time**; the boot-probe checks only the client
  binary (`pg_dump --version`). Supabase can major-upgrade the managed server independently of dowiz's
  deploy cadence; `pg_dump` refuses when server-major > client-major. The probe cannot see that (it never
  reads the live `server_version`), so a provider-side upgrade silently fails every subsequent dump until
  the image is rebuilt — surfaced only by a failing dump (which, per H7, degrades health only if the
  already-broken drill catches it).
- **Evidence:** `packages/config/src/index.ts:9`; `apps/api/src/workers/backup/index.ts:108`;
  `apps/api/src/workers/backup/dump.ts:34`; proposal A-1.

---

## LOW

### BRK-13 · B-OPS · Core rollback primitive rests on an unverified flyctl contract
- **Invariant broken:** a recovery command must not be designed against an unverified CLI shape.
- **Break scenario:** §3.2 itself hedges that `flyctl releases --json`'s field shape is "to be
  re-verified" and that machines-era flyctl **removed** `releases rollback`. The entire image-ref
  recording + manual rollback rests on that unverified JSON contract; if `.[0].ImageRef` is absent/renamed
  on the installed flyctl, both the tag-recording (§3.2) and `rollback-prod.sh` (§3.4) silently break.
- **Evidence:** proposal §3.2 ("exact flag/shape to be re-verified", "machines-era flyctl removed
  `releases rollback`").

### BRK-14 · B-OPS · SPA-shell cache vs `/version`: drill can be green while users get the old bundle
- **Invariant broken:** "rollback complete" must reflect what users are actually served.
- **Break scenario:** the §3.6 drill asserts the **API** `/version` == vN-1 SHA. The public SPA
  `index.html`/JS is served through `spa-proxy` with caching; after an image rollback the CDN/browser can
  keep serving the previous SPA bundle while the API `/version` already reports vN-1 → the drill goes
  green while real clients run stale frontend code against the rolled-back API.
- **Evidence:** proposal §3.2 (/version = API route), §3.6; `apps/api/src/routes/spa-proxy.ts` (cached SPA
  shell — hotspot per CLAUDE.md).

### BRK-15 · B-ANTIPATTERN · The ADR/proposal cite restore-CLI lines that don't exist in the live file
- **Invariant broken:** citation integrity — a design that re-verifies its sources must not cite absent
  code (this is the C2 "written against an imagined artifact" smell, in the ADR itself).
- **Break scenario:** ADR/audit H9/H10 cite `scripts/backup-restore.ts:238-252` (`if (code===0||code===1)
  resolve()`, `--clean … --no-acl` restore, `"✓ Restore completed"` at `:252`). The **live** file's
  non-dry-run path is a 4-line stub ending at `:215` (`"Full restore not yet implemented" … exit(1)`); no
  `runFullRestore`, no `:243-247`, no `:252` exist. A-4 is therefore designed to "kill H9's exit-1=success
  class before it's reborn" against a code path that isn't present — the described `pg_restore` driver was
  never in this file. Low impact (A-4 implements fresh), but the ADR's evidence base is partly stale.
- **Evidence:** `scripts/backup-restore.ts:187-222` (full file is 222 lines; stub at :212-215);
  ADR §Context (Deploy/Schema), audit H9/H10.
