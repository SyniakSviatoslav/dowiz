# Resolution — Safe-Reversal / Restore Spine (Data · Schema · Deploy)

- **Round:** STEP 3 (RESOLVE) — Triadic Council. Conductor dispositions every Breaker finding +
  both Counsel ETHICAL-STOPs. **DESIGN-ONLY, docs-only.** No product code, config, or migration
  touched; no build/deploy/commit performed in this round.
- **Date:** 2026-07-03
- **Inputs dispositioned:** `proposal.md` (architect), `breaker-findings.md` (15: C1·H4·M7·L3),
  `counsel-opinion.md` (PROCEED-WITH-REVISIONS; ESC-1, ESC-2 + 3 non-blocking advice + 1 open Q).
- **Red-line class:** data-recovery (irreversibility) + GDPR-erasure + money/RLS-adjacent →
  **COUNCIL-gated by definition.** This document does **not** self-certify: the conductor re-attacks
  the revised design next; the human operator is final on every needs-human item below.

---

## 0. Verification ledger — every finding re-checked against the LIVE working tree

The Breaker re-verified its own citations; I independently re-read the load-bearing source. All 15
findings **CONFIRMED against live source**, with one precision correction (BRK-15).

| # | Claim | Live-source check | Verdict |
|---|---|---|---|
| BRK-1 | `manifest.rowCounts` = 9 hardcoded tables, counted live AFTER dump | `manifest.ts:39-49` (hardcoded `orders…customers`, `SELECT COUNT(*)` on live pool); `index.ts:135` checksum + `:123-133` upload happen **before** `:140` `generateManifest`; `smoke-checks.ts:88` `outlier` skips when `base===undefined` | **CONFIRMED** |
| BRK-2 | Drill self-DoSes prod / spills PII | `restore-sandbox.ts:28-38` `CREATE DATABASE` on `DATABASE_URL_ADMIN`; `config:126` optional | **CONFIRMED** |
| BRK-3 | Scratch restore role/extension-incompatible | `dump.ts:27-28` `--no-owner --no-acl` (policies/extensions survive); builder-method + role DDL present | **CONFIRMED** |
| BRK-4 | Deploy image-ref races; no concurrency | `ci.yml:1-8` no `concurrency:`; `:133-134` `deploy: needs: validate` | **CONFIRMED** |
| BRK-5 | `checkPIIFree` ⊥ full-PII backup; `createSessionPool(arg)` discards arg | `smoke-checks.ts:235` `passed: totalPii===0`; `db/src/index.ts:48` `createSessionPool()` takes **no** param, uses `env.DATABASE_URL_SESSION` → arg at `backup-verify.ts:317` silently discarded (checks run on **live** DB) | **CONFIRMED** |
| BRK-6 | `keyId` decorative; A-3⊥A-6 | `encrypt.ts:25` `keyId:'primary'` constant; `backup-restore.ts:89-92` reads `env.BACKUP_ENCRYPTION_KEY`, ignores `manifest.encryption.keyId` | **CONFIRMED** |
| BRK-7 | A-7 write-only-token ⊥ "never delete last N verified" | R2 native lifecycle is time-based; write-only writer has no deletion path to host the guard | **CONFIRMED (design-internal)** |
| BRK-8 | In-window erasures lost with no trace | `config:84` hourly RPO; `gdpr_erasure_requests` only in the dump → erasure completed after snapshot is unrecoverable | **CONFIRMED** |
| BRK-9 | Lint text-match bypassed by builder-method DDL + raw COMMIT | grep: `pgm.dropTable/dropColumn` in `1780338982027:62-68`, `1780694000000:10`, `1780338982026:25-28`; raw `pgm.db.query('COMMIT')` at `1790000000011:102`, `1790000000042:52` | **CONFIRMED** |
| BRK-10 | Pre-migrate snapshot ordering ungated | `ci.yml:150-153` `migrate:up` + `fly.toml:14-15` `release_command` = double-migrate | **CONFIRMED** |
| BRK-11 | `/version` absent; `GIT_SHA` unset in prod | grep `/version` → none; `config:124` `GIT_SHA` optional, only Sentry `server.ts:62`; `ci.yml:155-159` sets no `GIT_SHA`; split-identity `RENDER_GIT_COMMIT` `config:166` used by `manifest.ts:58` | **CONFIRMED** |
| BRK-12 | Build-time client pin can't detect provider-side major upgrade | `config:9` `DATABASE_URL_MIGRATIONS` = session pooler `:5432`; `index.ts:108` dumps via it; boot-probe checks binary only | **CONFIRMED** |
| BRK-13 | Rollback rests on unverified flyctl contract | proposal §3.2 self-hedges shape "to be re-verified"; machines-era flyctl removed `releases rollback` | **CONFIRMED (self-admitted)** |
| BRK-14 | SPA cache vs `/version` | `/version` = API route; SPA shell served cached via `spa-proxy` (hotspot) | **CONFIRMED (plausible; drill scope gap real)** |
| BRK-15 | Cited restore-CLI lines don't exist | `backup-restore.ts` is 222 lines; full-restore is a 4-line stub `:212-215`; no `runFullRestore`/`:238-252` | **CONFIRMED — with correction (below)** |

**BRK-15 precision correction (load-bearing for citation integrity):** the stale `:243-247`/`:252`
(H9) and `:238` (H10) citations are in the **source audit** `docs/design-review/audit-reliability-2026-07-03.md:90,94`
— NOT in the ADR. The **ADR §Context's own** restore-CLI citation is `scripts/backup-restore.ts:212 → exit 1`,
which is **accurate**. So the finding's substance holds (the `pg_restore`-driver code A-4 designs
against was never present — the live path is a stub), but its attribution "the ADR itself cites
238-252" is imprecise: the stale lines live in the audit that the proposal's A-4 transitively cites.
This makes the fix narrower (correct the audit, not the ADR).

---

## 1. Disposition table — all 15 Breaker findings

Legend: **FIX** = design revised (how, stated). **ACCEPT-RISK** = justified residual + owner (register §5).
**DEFER-FLAG** = parked + owner + re-entry trigger.

### CRITICAL

**BRK-1 · Layer-1 restore proof structurally false-green → FIX** *(mandatory per task; concrete falsifiable mechanism below)*

The current proof cannot fail on a broken restore for four independent reasons: (a) `manifest.rowCounts`
covers only 9 hardcoded tables (`manifest.ts:39-43`) so any of the other ~40 can silently vanish;
(b) counts are taken on the **live** pool **after** dump+checksum+upload (`index.ts:140` vs `:135/:123`)
so they never describe the artifact; (c) `checkRowCounts` skips any table whose baseline is `undefined`
and only flags `count===0 || >base*10` (`smoke-checks.ts:88`) — a 99%-truncated `products` passes;
(d) the drill's smoke pool is built with a discarded arg (`createSessionPool(sandboxUrl)` → `db/index.ts:48`
ignores it) so it asserts against the **live** DB, never the scratch restore.

**Revised design — a drill that fails RED on a corrupt/partial artifact before it can pass green:**

1. **Full-coverage manifest, snapshot-consistent counts.** Replace the hardcoded 9-table list with a
   rowCount over **every base table in `public`** enumerated from `pg_catalog`/`information_schema` at
   dump time. Take the counts inside the **same MVCC snapshot as the dump**: `pg_dump --snapshot=<id>`
   where `<id>` comes from `pg_export_snapshot()` in the transaction that also runs the `COUNT(*)`s.
   The manifest counts then describe *exactly* the artifact's bytes (kills BRK-1b flakiness). Alternative
   if snapshot export is impractical on Supavisor: derive the authoritative table set + object count from
   the artifact itself via `pg_restore --list` TOC, and assert TOC-object-count == restored-object-count.
2. **Assert against the SCRATCH restore, not live.** Fix the root: `createSessionPool` grows an optional
   `connectionString` param (small `packages/db` change, protect-path ack) OR the drill constructs an
   explicitly-parameterized `Pool` for the scratch target. No drill assertion may run on `env.DATABASE_URL_*`.
3. **Strict per-table equality, no skip.** Replace the `outlier` heuristic (`smoke-checks.ts:84-91`)
   with strict `scratch.count(table) === manifest.rowCounts[table]` for the **full** table set, and assert
   `set(scratch tables) === set(manifest tables)` (a manifest table missing/empty in scratch = RED).
4. **Restore canary + freshness** (retain from §1.5): each cycle writes `(backup_id, taken_at)` to a
   `backup_canary` control table **before** dumping; post-restore assert `canary.backup_id === manifest.backupId`
   (catches wrong-artifact / silent no-op onto a pre-existing scratch) and `canary.taken_at` within
   cadence+slack (catches green-against-stale-artifact, the H6 dead-writer class).
5. **`pg_restore` TOC integrity gate:** assert `pg_restore --list` object count == objects actually
   restored (Breaker open-Q1) — a partially-failed restore that still lands the canary table is caught here.
6. **Remove `@ts-nocheck` from all of `workers/backup/`** — it masked every one of the C2 bugs above.
7. **Red-first proof obligation (harness):** ship demonstrated RED against (a) truncated/corrupt artifact,
   (b) wrong-key decrypt, (c) partial restore (one table dropped), (d) stale canary, (e) wrong-artifact
   canary — then green on the real chain. A drill never seen red is not a guardrail.

**Owner:** agent (drill + smoke-check rewrite + `createSessionPool` param, protect-path ack). Ties to
BRK-2/3/5 (scratch target, roles, PII-free removal).

### HIGH

**BRK-2 · Drill spills full-PII cross-env or self-DoSes prod → FIX**

Root: the scratch target is either the prod Supabase cluster (`restore-sandbox.ts` `CREATE DATABASE` on
`DATABASE_URL_ADMIN` → doubles footprint, trips the 500 MB Free cap → project forced read-only → the
drill takes prod down) or shared-cred staging Fly PG holding full-PII daily.

**Revised design — two-tier drill with an explicit PII-boundary:**
- **T1 (daily, prod-adjacent-safe, zero PII hydration):** decrypt → checksum(plaintext) → `pg_restore --list`
  TOC-count vs manifest table set → manifest canary present. Runs in-process against the artifact; restores
  **nothing** into any standing store. Safe to run daily; this is the frequent `/health` signal.
- **T2 (weekly, the real round-trip proof):** full restore into an **ephemeral, isolated, EU-pinned**
  Postgres that is destroyed at end — a throwaway CI `postgres:16` service container OR a per-drill Fly
  Machine PG, **never** the prod Supabase cluster and **never** the shared-cred staging DB. Artifact +
  scratch DB destroyed on completion; credential is scoped to the ephemeral target only.
- Add a **PII-boundary section** to the design (the Breaker's core complaint: "blast radius never
  analyzed") — data-at-rest lifetime, EU jurisdiction, credential scope, destruction guarantee.

**Residual (register §5):** T2 still transits full-PII through an ephemeral EU runner for the run's
duration — accepted ONLY under ephemeral+EU+destroyed+access-controlled; NOT accepted for T1 (T1 hydrates
no PII). **Owner:** operator (ephemeral infra provisioning) + agent (tiering). Couples to BRK-3.

**BRK-3 · Scratch restore role/extension-incompatible → FIX**

`--no-owner --no-acl` (`dump.ts:27-28`) strips grants but NOT `CREATE POLICY … TO authenticated`/`anon`/
`service_role`/`supabase_admin` (9 migrations) or `CREATE EXTENSION citext/pgcrypto`
(`1780310044710:7-8`). A bare `postgres:16` container has none → under A-4's strict stderr rule these are
hard errors → drill red every run.

**Revised design:** the scratch provisioner **pre-seeds** the Supabase role set as `NOLOGIN` placeholder
roles + the required extensions **before** `pg_restore`. Then a clean restore has **no** ignorable errors,
and A-4's whitelist narrows correctly to only `--clean`'s "does not exist, skipping" (the DROP phase on a
fresh DB). **Named limitation (recorded):** a pre-seeded scratch is not byte-identical to a fresh Supabase
project — role provisioning differs. The drill therefore proves **artifact + data + restorability into a
Postgres honoring the role contract**; it does **not** prove Supabase-specific provisioning. That gap is
covered by the **annual human game-day** against a real fresh Supabase project (Counsel advice #3, adopted).
**Owner:** agent (provisioner seed) + operator (game-day).

**BRK-4 · Deploy image-ref races on concurrent pushes → FIX**

No `concurrency:` in `ci.yml` (`:1-8`); `.[0]` (latest global release) read after-the-fact mislabels the
`deploy/prod/<n>` tag under two concurrent `main` deploys.

**Revised design:** (a) add `concurrency: { group: prod-deploy, cancel-in-progress: false }` to the deploy
workflow — serialize prod deploys, queue (never cancel) an in-flight one; (b) capture the image ref from
the **deploy command's own output** (the digest this job just shipped), not a post-hoc global `.[0]` read —
removes the race even independent of concurrency (belt+suspenders). **Owner:** operator (.github paste-in).
Interlocks BRK-11, BRK-13.

**BRK-5 · `checkPIIFree` ⊥ full-PII backup; smoke pool runs on live DB → FIX**

`checkPIIFree` asserts restored data is PII-free (`smoke-checks.ts:235`), but Option A intentionally keeps
**full-PII** dumps (`manifest.ts:59-60` `piiRedacted:false, piiEncrypted:true`) — so on any real prod
restore it MUST fail. Compounded by the `createSessionPool(sandboxUrl)` arg-discard (BRK-1d/`db/index.ts:48`).

**Revised design:** `checkPIIFree` is **removed** from the restore-drill assertion set, **explicitly and
with recorded rationale** — Option A protects the backup by encryption-at-rest + EU jurisdiction + access
control, **not** by redaction; the drill proves **restorability + integrity**, and PII-absence is a
category error against a deliberately-full-PII artifact. This is NOT a silent drop (the ADR §Proof must
stop implying PII-free is a restore assertion — sync note §6). The `createSessionPool` arg fix is shared
with BRK-1. **Owner:** agent.

### MEDIUM

**BRK-6 · `keyId` decorative; A-3/A-6 self-contradict; escrow deferred too late → FIX + re-sequence**

`keyId` is the constant `'primary'` (`encrypt.ts:25`) and restore ignores `manifest.encryption.keyId`
(`backup-restore.ts:89-92`) → the "self-sufficient manifest" still depends on a single ambient Fly secret,
and A-6 parks escrow at Ordering **step 7 (last)**. During steps 2-6 the "provider-independent net" is a
single-Fly-secret key — the exact same-account blast-radius the proposal invokes to reject PITR.

**Revised design:** (a) restore must READ `manifest.encryption.keyId`, look it up in a **keyring**
(`BACKUP_KEYRING` env map or the SOPS/age vault) and **fail loud** if the manifest keyId isn't present —
this makes rotation possible without a flag-day; (b) **re-sequence:** real keyId + keyring + two-store
off-Fly escrow become **co-requisites of automated cadence going live** (Ordering step 2-3 region), NOT
step 7. Running automated backups whose only key is one Fly secret that dies with the app is a live
single-point-undecryptable risk. **Owner:** operator (holds secrets) + agent (keyring code). Sync: ADR
Ordering + §Decision.1 keyId line.

**BRK-7 · A-7 write-only-token ⊥ "never delete last N verified" → FIX**

R2 native lifecycle is **time-based**; a write-only writer has no deletion path to host a "keep last N
verified" guard, so a stalled writer (H6/C1) lets time-based lifecycle age out the last good point.

**Revised design (promotion, not deletion-guard):** on each **green** drill, the drill **copies** the
verified artifact to a separate `dowiz-backups/<env>/verified/` prefix with **no (or long) lifecycle
expiry**. Time-based lifecycle deletes only from the `hourly/daily/weekly/monthly` prefixes; `verified/`
survives by positive promotion regardless of writer health. Writer stays write-only; the drill needs read
+ write-to-`verified/`. Also fix M11: lifecycle prefixes must match the real `dowiz-backups/${NODE_ENV}/…`
path. **Owner:** operator (R2 token scopes + lifecycle) + agent (drill promotion step).

**BRK-8 · In-window erasures lost with no trace (GDPR) → FIX (survivable erasure ledger)**

An Art.17 erasure completed within the RPO window (≤1 h, `config:84`) before disaster is neither in the
dump nor recoverable; after restore the PII returns AND the request row is gone. "Re-run erasures newer
than the snapshot" structurally cannot cover the newest ones because they aren't in the snapshot.

**Revised design:** erasure **completions** are appended, at erasure time, to an append-only **R2 erasure
ledger** (`dowiz-backups/<env>/erasure-ledger/<date>.jsonl`) holding only the pseudonymous subject id +
erased-field list + completed-at (no residual PII beyond what re-erasure needs). Restore-time re-erasure
(ESC-1) reads the **ledger** — which survives in R2 independent of the DB's RPO — not just
`gdpr_erasure_requests` in the dump. This closes the in-window gap. **Owner:** this lane (ledger plumbing)
+ the GDPR-erasure lane (P-H1) for the completion hook. Ties directly into ESC-1.

**BRK-9 · Lint text-match bypassed by builder-method DDL + raw COMMIT → FIX**

Confirmed builder-method destructive DDL already in-tree (`pgm.dropTable`/`dropColumn` in
`1780338982027:62-68`, `1780694000000:10`, `1780338982026:25-28`) and raw `pgm.db.query('COMMIT')`
(`1790000000011:102`, `1790000000042:52`) — a text grep for "DROP TABLE"/"DROP COLUMN"/"noTransaction"
misses all of these.

**Revised design:** the lint matches **both** raw-SQL text AND node-pg-migrate builder methods —
`pgm.dropTable(`, `pgm.dropColumn(`, `pgm.renameColumn(`, `pgm.renameTable(`, `pgm.dropConstraint(`,
`pgm.alterColumn(… notNull:true …)` without paired default, `pgm.sql('…DROP…')` — and bans **raw**
`pgm.db.query('COMMIT'|'BEGIN'|'ROLLBACK')` in any DDL-bearing migration (not only `pgm.noTransaction()`).
Fixtures grow to include a builder-method destructive case and a raw-COMMIT case the lint must FAIL.
(AST parsing is the robust ceiling; a curated builder-method pattern set is the pragmatic ratchet floor —
either is acceptable, both ship with red-first fixtures.) **Owner:** agent. Sync: ADR §Decision.2 lint scope.

**BRK-10 · Pre-migrate snapshot ordering ungated → can capture POST-migrate state → FIX (hard interlock)**

If the §2.4 wrapper is enabled while CI `migrate:up` (`ci.yml:150-153`) still runs, CI migrates first and
`release_command` either snapshots an already-migrated schema or (because it snapshots only when migrations
are **pending**) sees zero pending and creates **no** snapshot for the CI-applied migrations → unprotected.

**Revised design:** make the interlock a **hard gate**, not prose. (a) Deleting the CI `migrate:up` step
(§3.1) is a **prerequisite** of enabling the wrapper (Ordering already has kill-CI-migrate before wrapper —
enforce it, don't merely sequence). (b) Add a CI/boot **assertion** that FAILS if both the CI `migrate:up`
step and the wrapper are simultaneously active. (c) The wrapper's "detect pending migrations → snapshot the
exact delta immediately before applying" is the correct guard **once it is the sole migration path**.
**Owner:** operator (.github + fly.toml sequencing) + agent (assertion). Sync: ADR Ordering + §Decision.2/3.

**BRK-11 · `/version` route absent + `GIT_SHA` unset → Layer-3 proof unrunnable → FIX (prerequisite)**

The proposal/ADR assume `/version` and `GIT_SHA` exist; neither does. No `/version` route (grep empty);
`GIT_SHA` optional (`config:124`), Sentry-only (`server.ts:62`), not passed by the deploy step
(`ci.yml:155-159`); split-identity `RENDER_GIT_COMMIT` (`config:166`) is what `manifest.ts:58` uses.

**Revised design (prerequisite, must land before §3.6 drill):** (a) ADD a `/version` route returning the
serving SHA; (b) POPULATE the SHA at deploy — `flyctl deploy` passes `GIT_SHA=${{ github.sha }}` (build-arg
or runtime env) so `/version` reports the real ref, not `unknown`; (c) UNIFY `GIT_SHA` vs
`RENDER_GIT_COMMIT` into one canonical var threaded to Sentry + manifest + `/version`. **Owner:** agent
(route + unify) + operator (ci.yml SHA plumbing). Interlocks BRK-4, BRK-14. Sync: ADR §Decision.3.

**BRK-12 · Build-time client pin can't detect provider-side major upgrade → FIX (+ residual accept)**

Dump runs via Supavisor session pooler `:5432` (`config:9`, `index.ts:108`); A-1 pins the client at
build time; the boot-probe checks only the binary. A provider major-upgrade → `pg_dump` refuses
(server-major > client-major) → **silent** dump failure until image rebuild.

**Revised design:** the pre-dump check compares the **client** major against the **live server** major —
`SHOW server_version` (the writer already runs this at `manifest.ts:51`) at dump time; on
`server_major > client_major` **fail loud** (Telegram `BACKUP_FAILED` + drill degraded), never silent.
**Residual (register §5):** backups still PAUSE until the operator rebuilds the image with a newer client —
accepted because it is now LOUD + alerting, not silent. **Owner:** agent (check) + operator (rebuild on
alert; confirm current Supabase PG major, open-Q5).

### LOW

**BRK-13 · Rollback primitive rests on unverified flyctl contract → DEFER-FLAG**

No design change can make an unverified CLI contract known; `flyctl releases --json` shape and the
availability of `--image` deploy on the installed flyctl must be **empirically verified**.

**Parked with hard re-entry gate:** the FIRST step of any Layer-3 rollback-script implementation is a
verification spike against the installed flyctl (confirm the image-digest field name; confirm
`flyctl deploy --image <ref>`; adapt if `.[0].ImageRef` is renamed/absent). Prefer capturing the ref from
the deploy command's own output (BRK-4 fix) to minimize contract surface. **Do not author `rollback-prod.sh`
against an assumed shape.** **Owner:** agent (spike) + operator (flyctl version). **Re-entry trigger:**
first line of Layer-3 rollback-script work.

**BRK-14 · SPA cache vs `/version`: drill green while users get old bundle → FIX (+ residual accept)**

`/version` is an API route; the SPA shell is served cached via `spa-proxy` (hotspot). After an image
rollback the CDN/browser can serve the previous SPA bundle while the API `/version` already reports vN-1 →
drill green while clients run stale FE against the rolled-back API.

**Revised design:** the §3.6 drill (and the rollback runbook step) must also assert the **served-SPA
identity** — embed the build SHA in the SPA shell and assert a **cache-busted** fetch returns the
rolled-back bundle; the rollback procedure adds an explicit SPA cache-invalidation step. **Residual
(register §5):** a brief stale-bundle window during CDN/cache propagation — accepted: bounded, self-healing
on cache expiry, API authoritative for price/status, expand-contract guarantees old-FE↔schema compat.
**Owner:** agent (drill + shell SHA) + operator (cache/CDN purge step).

**BRK-15 · Stale restore-CLI citations → FIX (doc correction, with attribution correction)**

Confirmed: live `backup-restore.ts` is 222 lines; full-restore is a stub `:212-215`; no `runFullRestore`,
no `:238-252`. **Correction to the finding's attribution:** the stale `:243-247`/`:252` (H9) and `:238`
(H10) lines are in the **source audit** `audit-reliability-2026-07-03.md:90,94`, NOT in the ADR — the ADR
§Context's restore-CLI citation (`:212 → exit 1`) is **accurate**.

**Revised design (doc-only):** (a) correct the source-audit H9/H10 to state the described `pg_restore`
driver is NOT present (stub at `:212-215`) — H9/H10 describe design intent, not current code; (b) soften
proposal §1.2 A-4's "kills H9's exit-1=success class before it's reborn" — there is no prior driver to be
"reborn"; A-4 implements fresh with strict stderr discipline; (c) the ADR §Context needs no change here.
**Owner:** agent (audit + proposal citation cleanup).

---

## 2. ESC-1 — Restore-time re-erasure: BLOCKING, post-condition-asserted, dependency confronted → design revision

**Grounded line (Counsel):** "анонімізувати-не-видаляти" + Ethics Charter (a subject who withdrew their
data must not have it "turned back on") + human-final/recorded-decision. As written, re-erasure is a
**runbook line** (proposal open-Q3 concedes this) that a stressed solo operator skips under an RTO clock —
and it re-invokes the **currently-broken** erasure worker (P-H1 / LC4, being fixed in the rls-reliability
lane), so even when run it leaves the most sensitive fields in place (C2 disease in the privacy domain).

**Design revision (adopted):**
1. **Non-skippable step of the restore SCRIPT, not a runbook line.** Restore-completion runs re-erasure of
   every completed `gdpr_erasure_requests` **sourced from the survivable R2 erasure-ledger (BRK-8 fix)**,
   not only the dump.
2. **Logs what it re-erased** (subject id, fields, timestamp) to an auditable record.
3. **Asserts its post-condition and FAILS LOUD:** for each named subject, `delivery_lat`/`delivery_lng`
   null, `delivery_photo_key` null **and the R2 object deleted**, `messenger_handle` null, name/phone
   anonymized. A restore whose re-erasure post-condition fails is reported **INCOMPLETE**, never "green" —
   by the same "reversal fails when it's broken" principle the spine already lives by.
4. **Dependency on the broken erasure worker made explicit and sequenced:** if the P-H1/LC4 erasure fix is
   not yet live, the assertion in (3) FAILS LOUD ("erasure re-application UNVERIFIED — subject X still has
   delivery_lat non-null") rather than silently succeeding. The spine therefore **cannot report a clean
   restore while erasure is broken** — it sequences AFTER the erasure fix or fails-loud-until.

**Recorded human decision required (needs-human §4, item 1 — the single most important unresolved item):**
*"Do we (a) GATE restore-completion on verified re-erasure (restore is INCOMPLETE until every completed
erasure is re-applied + asserted), or (b) accept restore-completes-with-UNVERIFIED-re-erasure + loud alert
+ a bounded manual-follow-up SLA?"* The default MUST NOT be resurrect-and-keep-silently. Owner:
human operator acting as/for the controller. This gates the **automated cadence** going live, NOT step 0.

**Counsel §5 open question escalated (needs-human §4, item 2 — legal):** does the window between
`pg_restore` completing and re-erasure asserting-done constitute processing-without-legal-basis / a
reportable Art.33/34 personal-data-breach event? If yes, re-erasure must be logged as **breach-mitigation**
and affected subjects may be owed re-notification. Run down with controller's counsel **before** the
automated cadence goes live.

---

## 3. ESC-2 — Monthly-PII-retention legal basis is a controller decision → needs-human; DR tiers self-justify

**Grounded line (Counsel):** storage-limitation (Art.5(1)(e)) + honesty (a basis living only in a config
constant tells no one) + human-final. For customer order data dowiz is the **processor**, the owner is the
**controller** (`RoPA.md:9,11,12`); a hard-coded retention on full-PII dumps is the processor deciding the
controller's duty by default.

**Design revision (adopted):**
1. **BLESS hourly-24h / daily-30d / weekly-90d as self-justifying disaster-recovery necessity** — no
   friction; disasters restore from the newest good backup. These stand.
2. **The monthly tier is NOT DR** — it is archival, outlives DR necessity, and holds resurrectable PII
   longest (`config:90` currently 7 y; proposal §1.6 proposes 13 mo). Its retention is a **controller
   storage-limitation decision**, not a processor platform-default.
3. **NEEDS-HUMAN (needs-human §4, item 3):** record an explicit **controller-level decision** stating the
   **legal basis + purpose of the monthly tier specifically**; **surface the retention term in the
   DPA-with-owners template** (`compliance/contracts/dpa-with-owners-template.md`) as a documented
   processing instruction, not an invisible constant. Counsel blesses **13 months conditioned on the basis
   being written** (e.g. annual-cycle/dispute/audit) — not shorter, but the basis MUST be recorded. If the
   original 7 y was a **statutory financial-records** obligation, it must move to a **separate, minimized,
   purpose-scoped financial archive** (invoices/settlements/tax — no GPS, no door photos), NOT be met by
   full-PII DB dumps.
4. **Record the honest structural fact:** the DR artifact is ONE co-mingled multi-tenant dump →
   per-controller retention is impossible → the processor imposes one number on every tenant → disclosure +
   a recorded basis are MORE necessary, not less. State this in the design + DPA.

Owner: human operator (controller decision) + compliance lane (DPA). Gates the **automated ladder with the
monthly tier**, NOT step 0 or the hourly/daily/weekly cadence.

---

## 4. Needs-human register (operator / controller)

| # | Item | Owner | Blocks | Default forbidden |
|---|---|---|---|---|
| **1** ⭐ | **ESC-1a: restore-completion gate** — GATE on verified re-erasure vs ACCEPT unverified-re-erasure + loud alert + SLA | operator as/for controller | automated cadence go-live | resurrect-and-keep-silently |
| 2 | ESC-1b (Counsel §5): is the pg_restore→re-erasure window an Art.33/34 reportable breach (→ breach-mitigation logging + subject re-notification)? | operator + legal counsel | automated cadence go-live | assume "merely operational" without asking |
| 3 | ESC-2: monthly-tier legal basis + purpose recorded + DPA-surfaced; financial-archive split if 7 y was statutory | operator as/for controller + compliance lane | automated ladder w/ monthly tier | ship retention as a silent config constant |
| 4 | Counsel #1: rotate leaked prod superuser + api creds BEFORE step-0 backup; use a **read-only** backup role; do NOT write a throwaway cred script (re-uses the exact leaked-script trap) | operator (SECURITY red-line) | step-0 backup | take first backup under the leaked/superuser cred |
| 5 | Counsel #2: Supabase-Pro belt timing — buy the $25/mo provider-run second net SOONER than "first revenue" (attention-independent belt) | operator (budget) | nothing (advisory) | — |
| 6 | Counsel #4: break-glass **third** escrow copy (custodian bus-factor — both current stores are one-human-held → total loss if incapacitated) | operator | nothing (continuity-critical) | leave recoverability on one person's availability |
| 7 | Open-Q5: confirm current Supabase PG major (client pin, BRK-12/A-1) + re-verify Pro/PITR pricing before purchase | operator | A-1 client pin + §1.4-secondary purchase | pin a guessed major |

---

## 5. ACCEPT-RISK register (justified residuals — no primary finding is dispositioned ACCEPT)

Rationale for zero primary ACCEPT-RISK: this is a **data-recovery / irreversibility** red-line; accepting
a restore-spine gap as-is is the wrong default. The residuals below are each **bounded + loud + owned**.

| Residual | From | Justification | Owner | Re-entry trigger |
|---|---|---|---|---|
| Backups PAUSE until image rebuild on Supabase major upgrade | BRK-12 | Now LOUD (live `SHOW server_version` check + Telegram + drill degraded), not silent; operator rebuilds on alert | operator | provider major-upgrade alert |
| Brief stale-SPA-bundle window during CDN/cache propagation post-rollback | BRK-14 | Bounded, self-healing on cache expiry; API authoritative for price/status; expand-contract guarantees compat | operator | if a rollback ever serves stale FE beyond cache TTL |
| Full-PII transits an ephemeral EU runner during the weekly T2 round-trip | BRK-2 | Accepted ONLY under ephemeral+EU+destroyed+access-controlled; T1 (daily) hydrates no PII | operator | any move of T2 onto a persistent/shared store |
| Single-custodian key escrow = total-loss-if-incapacitated | Counsel #4 | Known continuity gap; both stores one-human-held | operator | until break-glass third copy (needs-human #6) lands |

---

## 6. ADR-sync note — sections the LEAD must reconcile (I did NOT edit the ADR)

The revised design diverges from `docs/adr/ADR-safe-reversal-spine.md` at these points; the lead syncs:

- **§Decision.1 (DATA) — proof claim:** "per-table row counts vs manifest" (line 70) must become
  **all-user-tables**, snapshot-consistent, asserted against the SCRATCH restore (BRK-1); the `checkPIIFree`
  removal + rationale must be recorded so §Proof no longer implies PII-free is a restore assertion (BRK-5);
  the drill target must specify **not-prod-cluster + not-shared-cred-staging + role/extension pre-seed +
  two-tier T1/T2** (BRK-2/3).
- **§Decision.1 — manifest self-sufficiency (lines 53-54):** `keyId` is decorative; sync to real
  keyId + keyring + fail-loud, and move escrow earlier (BRK-6).
- **§Decision.1 / §Consequences — retention (lines 64-67, 138-139):** "post-restore re-application of
  completed erasures" → **blocking, post-condition-asserted, R2-ledger-sourced** (ESC-1 + BRK-8); reframe
  the monthly-tier friction from ">13 mo needs human" to **"the monthly tier itself needs a recorded
  controller basis + DPA surfacing"** (ESC-2); add the multi-tenant co-mingled-dump fact.
- **§Decision.2 (SCHEMA) — lint scope (lines 79-82):** must cover **builder-method DDL + raw COMMIT**, not
  only up()-text (BRK-9); the snapshot/kill-CI-migrate **interlock becomes a hard gate** (BRK-10).
- **§Decision.3 (DEPLOY) — lines 90-98:** `/version` + `GIT_SHA` are **prerequisites to build** (route +
  SHA population + `GIT_SHA`/`RENDER_GIT_COMMIT` unification, BRK-11); add a **`concurrency:` guard**
  (BRK-4); add the **flyctl-contract verification spike** as a precondition (BRK-13); §Proof weekly rollback
  drill must assert **served-SPA identity + cache-bust** (BRK-14).
- **§Ordering (lines 106-111):** re-sequence key-escrow/real-keyId earlier as co-requisites of cadence
  (BRK-6); add **rotate-creds-first + read-only-role + no-throwaway-script** as hard step-0 preconditions
  (Counsel #1); enforce kill-CI-migrate **before** the snapshot wrapper (BRK-10); add the **annual human
  game-day** as a fourth drill (Counsel #3, adopted).
- **§Proof obligations (lines 161-167):** add BRK-1 red-first fixtures (corrupt / partial / wrong-key /
  stale-canary), BRK-9 builder-method + raw-COMMIT fixtures, BRK-11 `/version`-populated precondition.
- **§Alternatives / A′:** adopt the read-only backup role as a step-0 co-requisite (Counsel #1), not merely
  "immediately in principle."
- **BRK-15:** ADR §Context needs NO change (its `:212` cite is accurate); the correction lands in the
  source audit + proposal §1.2 only.

---

## 7. Non-certification

This resolution is **not** a certification. Every FIX above is a **design revision on paper**; none is
proven. Per the harness, each lands only with its drill demonstrated **red→green** + a regression-ledger
row, and every gate here is a ratchet that may not later be weakened. The conductor **re-attacks the
revised design next**; the human operator is final on all 7 needs-human items, and the two ETHICAL-STOPs
remain **recorded-decision friction** until a human signs them. The urgent step-0 backup ships this week
regardless — after Counsel #1 (rotate creds + read-only role).

---

*Conductor · Triadic Council STEP 3 (RESOLVE) · design-only, docs-only · human is final · re-attack pending.*
