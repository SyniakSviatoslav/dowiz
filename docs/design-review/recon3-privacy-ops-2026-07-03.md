# Recon #3 — Privacy / Compliance / Data-Governance + Operational / Deploy / DR Maturity

**Date:** 2026-07-03 · **Mode:** READ-ONLY (findings only, no edits) · **Scope:** governance + ops maturity,
the angles **not** covered by recon #1 (GDPR-erasure stranding, backup-inoperable, tax/VAT) or recon #2
(integrations, performance, states/observability, supply-chain).
**Method:** 5 parallel read-only lanes (PII-inventory · consent/logs/cross-border · DSAR/audit ·
deploy/rollback/migration · flags/secrets/observability), every finding confirmed against source lines;
the anonymizer/erasure gaps were **independently found by two lanes** (dual-confirmed).

> **Framing (read first).** This repo already has a *real* compliance apparatus —
> `compliance/{data-map,RoPA,subprocessors,consent-records,privacy-invariants,env-classification}.md`,
> `compliance/dpia/courier-gps.md`, `compliance/policies/{privacy-policy-v1,terms-v1}.md`,
> `compliance/contracts/dpa-with-owners-template.md`, a `/privacy` page, a `compliance:gate` CI check,
> mature Pino + Sentry PII redaction, and EU compute (`fly.toml` `primary_region = "fra"`). So the
> findings below are overwhelmingly **gaps between the documented policy and the shipped code**, not
> "there's no policy." Several are *doc-vs-code integrity failures* — a control that the data-map/runbook
> claims exists but the code does not implement.

---

## Severity tally

| Severity | Privacy/Compliance | Ops/DR | Total |
|----------|--------------------|--------|-------|
| **HIGH** | 6 | 7 | **13** |
| **MED**  | 8 | 8 | **16** |
| **LOW**  | 3 | 1 | **4**  |
| | 17 | 16 | **33** |

None are live-exploitable RCE-class; the HIGHs are either **demonstrable GDPR-right failures on the most
sensitive PII** or **availability/recoverability gaps** where a bad deploy/migration is slow-MTTR or
outright unrecoverable.

---

## ⚑ The single biggest COMPLIANCE gap

**A "completed" right-to-be-forgotten (Art.17) demonstrably leaves the most sensitive PII in place —
precise home GPS coordinates, a photograph of the customer's home entrance (in Postgres *and* R2,
forever), and their direct messenger handle — and never touches the order estate at all.**
The GDPR worker anonymizes only the `customers` row (`anonymizer-gdpr.ts:62-65` →
`lib/anonymizer/index.ts:73-84`, no order cascade), and `anonymizeOrder` itself omits `delivery_lat`,
`delivery_lng`, and `delivery_photo_key` (`index.ts:210-222`). `compliance/data-map.md:14` **claims**
"delivery_lat/lng → anonymized_at NULLs" — a documented control that is not implemented. This is worse
than a paperwork gap: the system reports `completed`, the operator/regulator believes erasure happened,
and the leftover data is maximally sensitive. **Runner-up:** no Art.15 access / Art.20 portability export
exists *anywhere*, and the storefront is phone-based with no customer login → no auditable
subject-initiated channel for any right (owner-goodwill-mediated only).

## ⚑ The single biggest OPS-MATURITY gap

**There is no safe reversal path.** CI deploys straight to prod on push-to-main with **no staging gate**
and **no approval environment** (`ci.yml`: jobs = validate · fresh-provision · deploy; `deploy: needs:
validate` only). The post-deploy Playwright smoke runs *after* prod is already live and triggers **no
automated rollback**; the rollback runbook points at an un-pinned image tag with no source of truth and
never mentions the native `fly releases rollback`. Underneath, migrations are **forward-only** (157 files
ship `down()` but prod never runs them — dead code) and several UP migrations are **destructive**
(`DROP TABLE promotions CASCADE`, `DROP COLUMN cash_pay_with`, `DELETE FROM categories`), whose only
recovery is a point-in-time backup restore that is **separately inoperable** (recon #1). Net: a bad
deploy is a ~15-min manual git-revert→build→migrate→deploy crawl, and a bad *destructive* migration is
effectively **unrecoverable**. The root cause is architectural: safety mechanisms were added
incrementally without reconciling the ones they supersede, so the pipeline now runs **two contradictory
migration paths** and **two contradictory boot-safety stances** (below).

---

# SECTION A — PRIVACY / COMPLIANCE / DATA-GOVERNANCE

### 🔴 P-H1 — GDPR "erasure" leaves home GPS + doorway photo + messenger handle + the entire order estate  *(dual-lane confirmed)*
- **Where:** `apps/api/src/workers/anonymizer-gdpr.ts:62-65` (no order cascade); `apps/api/src/lib/anonymizer/index.ts:73-84` (customer-only when `customerId` set), `:210-222` (`anonymizeOrder` omits `delivery_lat`/`delivery_lng`/`delivery_photo_key`), `:133-161` (customer UPDATE omits `messenger_handle`/`messenger_kind`; only `customers.avatar_key` deleted from R2, never the order door photo).
- **Risk:** An Art.17 erasure marked `completed` leaves precise home coordinates (`data-map.md:14`, HIGH-RISK), a photo of the home entrance (`data-map.md:15`, HIGH-RISK — DB key + R2 object), and a direct-contact messenger handle intact **indefinitely**; the rest of the order PII survives up to `retention_days` (default 365, max **2555 ≈ 7 years**). `data-map.md:14` claims lat/lng are nulled → doc-vs-code integrity failure.
- **Fix:** In the GDPR worker enumerate the subject's orders and `anonymizeOrder` each in the same job; add `delivery_lat=NULL, delivery_lng=NULL, delivery_photo_key=NULL` + `storage.delete(delivery_photo_key)` to `anonymizeOrder`; add `messenger_handle/kind` to the customer UPDATE. (All four fixes concentrate in `lib/anonymizer/index.ts` + the GDPR worker.)

### 🔴 P-H2 — No Art.15 access / Art.20 portability export exists at all, and no customer self-service channel
- **Where:** `apps/api/src/routes/owner/gdpr.ts:1-255` is the *only* DSR code and implements erasure + retention **only**; broad grep for `art15|art20|portab|dsar|subject-access|data-export` → nothing. Every GDPR route is `requireRole(['owner'])` (`gdpr.ts:28-30`); the storefront has no customer login.
- **Risk:** A customer/regulator access-or-portability DSAR cannot be fulfilled — the capability does not exist, and the owner has no tool to produce a subject's record. RoPA marks the owner as *processor* (`RoPA.md:19`), so the controller-facing duty to the subject has no code path and rests on owner goodwill.
- **Fix:** Add authenticated `GET …/gdpr/access-export` assembling the subject's PII across all tables into machine-readable JSON; add a token-authenticated (order-track-grant/OTP) customer self-submit endpoint.

### 🔴 P-H3 — "Append-only" audit logs are not tamper-evident; the accountable tenant can delete its own trail
- **Where:** `packages/db/migrations/1780421100060_anonymization-seam.ts:57-59` (RLS policy has **no `FOR` clause → `FOR ALL`**, so UPDATE/DELETE permitted in-tenant) and `:70` (`COMMENT 'Append-only audit log'` = aspirational only). Same for `courier-audit-log.ts:21-22`, `settlement-audit-log.ts:25`. No `REVOKE UPDATE/DELETE`, no immutability trigger anywhere; `1790000000080_grant-hardening.ts:13` even notes "Audit log left writable."
- **Risk:** The tenant the audit trail is meant to hold accountable can DELETE/UPDATE its own `anonymization_audit_log` / `courier_audit_log` / `settlement_audit_log` via any in-tenant SQL path or injection → the erasure/settlement/courier trail is not admissible or tamper-evident (Art.5(2) accountability).
- **Fix:** `REVOKE UPDATE, DELETE ON <audit tables> FROM dowiz_api` + a `BEFORE UPDATE/DELETE` trigger raising exception (or split-role append-only grants).

### 🔴 P-H4 — Checkout privacy notice makes a false "never share with third parties" claim
- **Where:** `apps/web/src/pages/client/CheckoutPage.tsx:704` — *"We never sell or share your information with third parties or advertisers — it's used only to fulfil your order."* Contradicted by the company's own `data-map.md:11,14,15,26` + `subprocessors.md:10,13`: name+phone+address → Telegram, door photo → R2, lat/lng → ORS. Templates that embed the PII: `apps/api/src/notifications/locales.ts:47-65,138-140`.
- **Risk:** Materially misleading statement at the point of collection → GDPR Art.5(1)(a) transparency + unfair-commercial-practices exposure.
- **Fix:** Reword to disclose the fulfilment processors (restaurant messaging, map/routing, photo storage) and link the full policy.

### 🔴 P-H5 — No published/surfaced customer privacy policy for order/GPS/Telegram processing (Art.13)
- **Where:** `compliance/policies/privacy-policy-v1.md:1-3` is a placeholder stub (`Дата чинності: [____]`, body = `<< Встав текст >>`). The only `/privacy` surface (`apps/web/src/pages/PrivacyPage.tsx:4-7`) covers **only the waitlist email**, not orders/GPS/Telegram. Checkout has no `/privacy` link (grep → none).
- **Risk:** Art.13 requires an accessible notice covering the *actual* order-data processing (controller, recipients, transfers, retention, rights); the 2-line checkout blurb is not that and no fuller notice is reachable.
- **Fix:** Publish the real customer notice; link it from checkout; extend `/privacy` (or add `/privacy/customers`) to cover order data, Telegram/R2/ORS recipients, GPS, retention, rights.

### 🔴 P-H6 — Cross-border: customer name/phone/address transmitted to Telegram (UAE, no adequacy, no SCC)
- **Where:** `compliance/subprocessors.md:13` — Telegram, `імʼя/тел/адреса`, location `[UAE ____]`, DPA/status `[____]`. Egress: `apps/api/src/notifications/adapters/telegram.ts` + `locales.ts:47-65`.
- **Risk:** Transfer of EU-resident PII to a UAE service with no adequacy decision and no recorded SCC/safeguard → GDPR Chapter V (Art.44-46).
- **Fix:** Record SCCs + Telegram DPA, or route owner alerts through an EU-hosted channel; document the transfer in RoPA.

### 🟠 P-M1 — Subprocessor registry + RoPA are templates with unfilled DPA / location / SCC fields  *(→ HIGH at launch)*
- **Where:** `compliance/subprocessors.md` — **10** `[____]` placeholders; DPA/status blank for **every** processor (Supabase, Fly, R2, Resend, Sentry, Telegram, ORS, Groq/OpenAI, OpenRouter, Google); most locations blank. `RoPA.md:3,13,21` has blank Controller/DPO fields and `[____]` retention cells for analytics/monitoring.
- **Risk:** Art.28 (no processor DPAs on record), Art.30 (RoPA incomplete), Chapter V (transfers unrecorded). The inventory exists but proves nothing is contracted.
- **Fix:** Fill each row with signed-DPA ref + processing location + transfer mechanism; make the `compliance:gate` "no undocumented subprocessor" check fail on a blank DPA-status cell (currently it only checks the name is present).

### 🟠 P-M2 — R2 uses `region: 'auto'` (no EU jurisdiction pin) for door photos + full DB backups
- **Where:** `apps/api/src/lib/r2-storage.ts:30`; backup uploaders `apps/api/src/workers/backup/upload.ts:19`, `workers/backup/index.ts:126,155`. R2 holds door photos (`data-map.md:15`) and logical DB dumps = all customer PII (`workers/backup/index.ts:7`). `subprocessors.md:10` R2 location = blank.
- **Risk:** R2 `region:'auto'` = nearest region with no EU constraint; EU PII + full backups can land in US storage with no safeguard. R2's `jurisdiction:'eu'` option is unused.
- **Fix:** Pin the bucket to the R2 EU jurisdiction; record R2 location + DPA.

### 🟠 P-M3 — Courier GPS: continuous tracking with no in-app transparency/consent notice
- **Where:** `packages/ui/src/hooks/use-geolocation.ts:24` auto-starts `watchPosition`; `apps/web/src/pages/courier/DeliveryPage.tsx:95-105,178-206` POSTs GPS every 12s to `/courier/shifts/ping` → `courier_positions` → owner live map. No "you are being tracked" disclosure anywhere in `courier/*`. `data-map.md:18,38` flags this HIGH-RISK, "DPIA", joint-controllership **unresolved**.
- **Risk:** Worker location monitoring without informed notice → Art.13 + EDPB employee-monitoring guidance; unresolved controllership = no one accountable. *(Positive: GPS stored only during active delivery — `routes/courier/shifts.ts:83-84` — with 24h retention.)*
- **Fix:** In-app "location shared with your dispatcher while on delivery" banner; resolve controllership in the courier contract.

### 🟠 P-M4 — No versioned consent/notice record at the order collection point; dead marketing-consent field
- **Where:** `compliance/consent-records.md:1-12` lists customer-collection consent as a **TODO**; the `privacy_version + consent_at` pattern that `access_requests` already uses (`data-map.md:21`) is not applied to orders. `customers.marketing_opt_in` (`1780338982017_loyalty_seam.ts:6`, default false) is **never set** — only reset in `anonymizer/index.ts:137`. No checkout opt-in.
- **Risk:** Art.5(2)/7 accountability — no record of which notice version a customer saw; if marketing is ever enabled, opt-in would be unproven.
- **Fix:** Stamp `privacy_version` + timestamp on order creation (mirror `access_requests`); add an explicit opt-in checkbox if marketing is wanted.

### 🟠 P-M5 — No erasure receipt / proof-of-erasure to the data subject
- **Where:** `apps/api/src/workers/anonymizer-gdpr.ts:67-83` writes results only to `gdpr_erasure_requests.metadata` + `anonymization_audit_log` (internal); nothing returned to the subject; owner-mediated flow keeps the subject out of the loop.
- **Risk:** No artifact the subject/regulator can hold as proof — fails Art.5(2) demonstrability.
- **Fix:** Emit a signed receipt (request id + per-store counts + timestamp), retrievable by the initiator.

### 🟠 P-M6 — Privacy-material owner actions have no audit event; no unified `audit_events` table
- **Where:** `gdpr.ts:239-254` (`PUT …/settings/retention` changes retention 30–2555 days with **zero** audit record); `gdpr.ts:81-87` erasure-creation records only `requested_by_owner_id` on the row; `orders.ts` has no audit inserts for owner confirm/reject/cancel/refund (grep empty). No unified `audit_events`/`audit_log` table exists — only siloed courier/settlement/anonymization/contact-reveal/upload logs. The GDPR worker even logs `actor_kind='system', actor_id=null` (`anonymizer-gdpr.ts:77`), losing the initiating-owner link.
- **Risk:** An owner can shorten retention to force-expire data, or mutate orders/refunds, with no accountability trail.
- **Fix:** Add a tenant-scoped append-only `audit_events` table; write from retention changes, order mutations, refunds.

### 🟠 P-M7 — "No PII to AI" rule is documented/gated but not enforced in code
- **Where:** Export is client-side only — `apps/web/src/lib/exportCSV.ts:13-14` `cleanRow` strips only `_`-prefixed internal fields, **not** customer PII; visible name/phone/LTV columns export verbatim (`exportJSON/exportJSONL:32-42`). The Tier-0 PII controls (attestation + field redaction + per-export audit row) are still DRAFT (`docs/design/owner-data-export-ai-council-brief.md:3,43-47`). *(Note: no `docs/adr/ADR-owner-data-export.md` exists at that path despite the git-status entry — the live artifact is the council brief.)*
- **Risk:** Any owner can download raw customer name/phone as CSV/JSON and feed it to any external AI — no attestation, no redaction, no audit — while the "ETHICAL-STOP on PII" lives only in docs.
- **Fix:** Ship the redaction-toggle + attestation + audit-row gate before any PII-bearing export (or hard-strip PII columns in `cleanRow`).

### 🟠 P-M8 — Pseudonymous-PII stores never purged despite documented TTLs (false-documentation cluster)
- **Where:** `velocity_events` — comment claims "24h retention via cron" (`1780421100057_anti-fake-signals.ts:105`) + a cleanup index, but no code deletes rows (only tests); `phone_otp` + `customer_otp_sessions` — `*_cleanup_idx` on `expires_at` but no purge worker; `customer_signals` — `expires_at` (7d) + cleanup index, no purge; `customer_devices` — never anonymized, plaintext globally-`UNIQUE` `fingerprint` (a persistent cross-order tracker) linked to `customer_id`, no TTL. `phone_otp.phone` is stored **raw** (`customer/otp.ts:79-82`) against the hashed house style.
- **Risk:** Pseudonymous identifiers (phone_hash, ip_hash, device fingerprint, raw OTP phone) grow unbounded; an "erased" customer keeps a live encrypted push token + fingerprint linkage; documented retention is false.
- **Fix:** Add bounded-batch expiry DELETEs to the existing retention worker for each store; hash `phone_otp.phone` and `customer_devices.fingerprint`; delete `customer_devices` on erasure.

### 🟡 P-L1 — Public telemetry endpoint: arbitrary `props`, no RLS, blank retention, reversible unsalted IP hash
- **Where:** `apps/api/src/routes/public/telemetry.ts:21` (`props: z.record(z.any())` unvalidated, stored verbatim `:54-59`), unauthenticated `rateLimit:false` `:37-38`; migration `1790000000012_analytics-events.ts:2-10` asserts "no PII" but keeps **no RLS** (cross-tenant); retention `[____]` blank (`data-map.md:20`); IP hash unsalted sha256 truncated (`telemetry.ts:6-8`) → enumerable for IPv4.
- **Risk:** Any caller can persist PII into a cross-tenant, no-RLS, no-retention store; "anonymous" IP hash is reversible → still personal data. *(Latent: FE `analytics.ts` is an unwired stub, no transport yet.)*
- **Fix:** Server-side allowlist/scrub `props` keys; salt (or drop) the IP hash; add a retention job; document retention.

### 🟡 P-L2 — Erasure-subject phone + owner chat-id stored plaintext, retained
- **Where:** `gdpr_erasure_requests.subject_phone` (`1780421100060_anonymization-seam.ts:16`, written `gdpr.ts:82`) stores the raw phone of the erasure subject and is never cleared post-`completed` — the erasure trail retains the PII it was asked to erase; `owner_notification_targets.address` (`1780348982032:9`) stores Telegram chat_id/phone plaintext, unbounded.
- **Fix:** Null `subject_phone` on completion; consider hashing owner target addresses.

### 🟡 P-L3 — Raw client IP written to application/WS logs with no documented log-retention
- **Where:** `apps/api/src/lib/logger.ts:116` (`remoteAddress: req.ip`), `websocket.ts:336,515,520`. *(DB storage correctly hashes IP with a daily salt — this is the log surface only.)*
- **Fix:** Redact/hash IP in the log serializer or document + bound log retention.

---

# SECTION B — OPERATIONAL / DEPLOY / DR MATURITY

### 🔴 O-H1 — Boot-guards run AFTER `fastify.listen()` and FATAL-exit the whole web fleet — directly contradicts migration 042's "tolerate & continue"
- **Where:** `apps/api/src/server.ts:877` listens, then `:881` `assertAccessRequestSchedules` / `:883` `assertDeliveryTraceSchedule` `process.exit(1)` in prod (`access-request-retention.ts:160-163`). Schedule *registration* is `.catch`-swallowed (`:45-55`), and migration `1790000000042_access-request-notify-queue.ts:83-93` **deliberately swallows** an `insufficient_privilege` pgboss failure and proceeds *without* creating the queue.
- **Scenario:** on the documented pgboss-ownership drift, 042 skips queue creation → `.schedule()` silently fails → the post-listen assert finds no schedule row → every web machine passes `/livez` (200 the instant `listen()` resolves, `health.ts:61`), is marked healthy, takes traffic, then `process.exit(1)` — a **fleet-wide "goes-green-then-dies" crash-loop**. Two safety mechanisms in direct opposition.
- **Fix:** Move the schedule asserts *before* `listen()` (fail closed pre-traffic), OR make 042 fail the migration instead of swallowing — reconcile the two.

### 🔴 O-H2 — CI migrates the prod DB *ahead of* code deploy (DB-ahead-of-code window), redundant with `release_command`
- **Where:** `.github/workflows/ci.yml:150-159` runs `pnpm migrate:up` against prod `DATABASE_URL_MIGRATIONS` **before** `flyctl deploy` (`:157`); but `fly.toml:14-15` already runs the same migrations in the pre-traffic release machine on every deploy → prod migrated **twice**, the CI copy minutes early.
- **Scenario:** a contracting migration (`1781101098646` DROP COLUMN, `1790000000017` DROP TABLE CASCADE) lands while **old code still serves** → 500s for the whole build+rollout window, permanently if `flyctl deploy` then fails — the inverse of the code-ahead-of-DB gap `release_command` was added to close.
- **Fix:** Delete the CI `Migrate Database` step; let `fly.toml release_command` be the single migration path.

### 🔴 O-H3 — `deploy` gates only on `validate`; the from-scratch bootable-schema proof does NOT block prod
- **Where:** `ci.yml:134` `deploy: needs: validate`. The `fresh-provision` job (`:57-131`) — the only end-to-end proof of a bootable schema (migrate→seed→boot→serve) — is not in `needs`.
- **Scenario:** a migration that makes a fresh DB non-bootable (the exact class the job was built for) fails `fresh-provision` but `deploy` still ships because `validate` (lint/typecheck/build) passed.
- **Fix:** `deploy: needs: [validate, fresh-provision]`.

### 🔴 O-H4 — ~10 runtime feature-flags/secrets bypass the Zod schema — no validation, no fail-fast, no SoT, no runtime visibility
- **Where:** `packages/config/src/index.ts` presents as the config SoT but these are read via raw `process.env.*`, never declared in `EnvSchema`: money `PAYMENTS_PREPAID_ENABLED`/`PAYMENTS_CRYPTO_ENABLED`/`PAYMENTS_PROVIDER`/`PLISIO_SECRET_KEY` (`lib/payments/registry.ts:6-7`), dispatch `COURIER_OFFER_HANDSHAKE_ENABLED` (`owner/dashboard.ts:323`), voice `VOICE_CONTROL_ENABLED`/`VOICE_KILL` (`lib/voice-flag.ts:12`), `TG_CATEGORY_GATING`/`TG_STOREFRONT_ACTION`/`MENU_OCR_ENGINE`, ops `METRICS_TOKEN` (`lib/metrics.ts:135`)/`PROVISION_OPS_SECRET`. No flag registry and no `/config`|`/flags`|env-dump route (grep → none).
- **Scenario:** a money-flag typo (`PAYMENTS_CRYPTO_ENABLED=True`/`1`) silently evaluates false with **no boot error** — the opposite of in-schema `z.enum(['true','false'])` flags that fail-fast; and `VOICE_KILL !== 'true'` means `VOICE_KILL=TRUE` **fails open**, so a hot-kill of a misbehaving feature silently no-ops.
- **Fix:** Move every runtime flag/secret into `EnvSchema` (enum/optional), read via `loadEnv()`; make kill-switches fail-safe.

### 🔴 O-H5 — `singleTransaction: true` "all-or-nothing" is a false guarantee — two migrations `COMMIT` mid-stream
- **Where:** `scripts/migrate-runner.ts:66` sets `singleTransaction: true` ("all-or-nothing"), but `1790000000011_pgboss-bootstrap-schema.ts:98-102` and `1790000000042_access-request-notify-queue.ts:51-52` call `pgm.noTransaction()` + explicit `await pgm.db.query('COMMIT')`, then `BEGIN` again (`011:151`, `042:99`).
- **Scenario:** when a pending batch contains one of these plus a destructive migration and a *later* one fails, everything up to the `COMMIT` — including destructive DDL — is durably applied while the deploy aborts → prod left at a partial, unrecorded-relative-to-head schema with old code serving. No atomic rollback despite the runner's claim.
- **Fix:** Stop asserting atomicity, or split the noTransaction/pgboss bootstrap into its own release step so the main batch is truly single-transaction.

### 🔴 O-H6 — Forward-only prod + destructive UP migrations → only recovery is backup-restore (which is inoperable) = effectively unrecoverable
- **Where:** `docs/phase5/rollback.md:5` "No down-migration in production." All 157 files ship `down()` (verified: 0 are ever invoked in prod → dead code). Destructive UPs run at migrate time: `1790000000017_promotions_full.ts:5` `DROP TABLE promotions CASCADE`, `1781101098646_cash-pay-with-numeric.ts:3` `DROP COLUMN cash_pay_with` (money field), `1790000000019_add_categories_unique.ts:4` `DELETE FROM categories`.
- **Scenario:** a bad destructive migration has **no** in-band undo; the only recovery is a PITR backup restore (recon #1: inoperable in the deployed image) → effectively unrecoverable data loss. 🔴 red-line (irreversibility).
- **Fix:** Enforce expand/contract for any DROP/DELETE (ship the drop a release *after* code stops using it), gated by a migration-lint rule.

### 🔴 O-H7 — The documented emergency full-restore command is an unimplemented stub
- **Where:** `scripts/backup-restore.ts:212` non-dry-run path prints `"Full restore not yet implemented"` + `exit(1)`, yet `docs/backup/runbooks.md:48-55` §3 presents `pnpm backup:restore --snapshot=<id>` as the RTO≤4h recovery action. *(Distinct from "backup inoperable in image" — this is the restore driver itself being a stub.)*
- **Scenario:** during real primary-DB loss the operator runs the runbook command and gets a no-op; RTO≤4h is unachievable via the documented path.
- **Fix:** Implement full restore, or rewrite §3 to the manual `pg_restore` procedure as the *primary* step.

### 🟠 O-M1 — No staging gate/approval env in CI; and the staging-validated artifact ≠ the prod artifact
- **Where:** `ci.yml` has only validate·fresh-provision·deploy (grep `staging`/`environment:` → none) — push-to-main deploys prod with no staging job and no required-reviewer environment. Separately, `scripts/deploy-staging.sh:23-30` bakes `VITE_MENU_CHARACTERISTICS_*=true` build-args while CI deploys prod with `Dockerfile:29-36` defaults (all `false`).
- **Scenario:** staging E2E validates a *different* SPA bundle (different build-time flags) than prod ships, so "green on staging" never proves the prod bundle — and nothing forces a staging pass first.
- **Fix:** Add a `deploy-staging` job that `deploy` depends on; build prod+staging from the identical image (runtime flags, not build-time) so tested artifact == shipped artifact.

### 🟠 O-M2 — The migration drift/preflight guards built for the 2026-07-03 incident are orphans
- **Where:** `scripts/ci-migration-preflight.mjs`, `ci-schema-drift.mjs`, `ci-connection-preflight.mjs` are referenced by nothing (no workflow, no `package.json` script, absent from `verify-all.ts:17-49`, no husky hook). `stat`: authored 2026-07-03 01:43-01:46, but `ci.yml` last edited 2026-07-02 07:49 — *before* the scripts existed. They also need env (`SOURCE_URL`; `LEFT_URL`/`RIGHT_URL` prod+staging read-only DSNs) CI never provides.
- **Scenario:** the exact prod≠staging drift they detect (`telegram_connect_tokens.owner_id`, missing `dowiz_app` role) recurs and reaches prod undetected.
- **Fix:** Add a `migration-preflight` step (`SOURCE_URL`=prod read-only) that `deploy` depends on; store a prod read-only DSN secret.

### 🟠 O-M3 — The `worker` process has zero health-check/readiness coverage — a crash-loop is invisible
- **Where:** `fly.toml:27-33` defines `[[http_service.checks]]` for `processes=["web"]` only (`:22`); the `worker` process (`:19`) has no `[[checks]]` and `apps/worker/src/index.ts:32-33` `process.exit(1)` on any startup error. `release_command` doesn't exercise the worker.
- **Scenario:** a crash-looping worker silently stops ALL background jobs — notifications, dispatch, retention/GDPR sweeps, backups — while `/livez` on web stays green and the deploy reports success. Total background-plane outage, no signal.
- **Fix:** Add a Fly machine check for the worker (TCP/heartbeat probe) or a lightweight worker `/livez`.

### 🟠 O-M4 — Post-deploy E2E is detection-only: no automated rollback, no image pinning
- **Where:** `ci.yml:164-190` runs 4 Playwright suites *after* `flyctl deploy` (`:157`); a failure rolls nothing back. CI never tags/records the previous good image; `docs/phase5/rollback.md:54` tells the operator to `flyctl deploy --image <previous-tag>` (no source of truth for the tag) and never mentions native `fly releases rollback`.
- **Scenario:** a truthful red smoke means prod is *already* live-broken; MTTR = full git-revert→build→migrate→deploy (~15 min) instead of an instant release rollback.
- **Fix:** On smoke failure `flyctl releases rollback --yes`; better, make the smoke a blocking pre-promotion gate against staging.

### 🟠 O-M5 — Error-tracking & metrics are opt-in via optional, no-default secrets with no prod boot-guard → prod can boot fully blind
- **Where:** `SENTRY_DSN` + `METRICS_TOKEN` are `optional` with no default (`packages/config/src/index.ts:116`; `lib/metrics.ts:136`); unlike `JWT_PRIVATE_KEY`/`VAPID_*` (required → boot fails) and the dev-auth boot-guard (`config/src/index.ts:223-237`), nothing asserts prod has observability. Sentry no-ops on empty DSN (`lib/sentry.ts:55`). *(Sentry code + PII-scrubbing `beforeSend` are genuinely wired — `lib/sentry.ts:63-100` — the gap is that nothing guarantees the DSN is set.)*
- **Scenario:** a prod deploy missing `SENTRY_DSN` boots healthy and blind — zero error capture, no metrics — with no warning; errors surface only via user complaints.
- **Fix:** Boot assertion: `NODE_ENV=production` requires `SENTRY_DSN` (warn on missing `METRICS_TOKEN`).

### 🟠 O-M6 — No CI secret-scan gate would catch the next leak
- **Where:** `scripts/verify-secrets.ts:22-24` prints "gitleaks not installed, skipping" and does **not** increment `failures`; no workflow installs gitleaks (`ci.yml` runs `verify:secrets` with the binary absent). *(Corroborates prior audit H5 — reported here as the ops-process consequence: given the prior leaked-DB-cred incident, the gate that exists for it is a no-op in CI.)*
- **Fix:** Install gitleaks in CI and fail closed when the binary is missing.

### 🟠 O-M7 — The SOPS/age secret store is documented as the SoT but is empty
- **Where:** `.sops.yaml` + `secrets/README.md` + `scripts/secrets-env.sh` describe `secrets/*.enc.env` as the "single source of truth for local-dev secrets," but **no `*.enc.env` exists** (only `secrets/staging.env.example`).
- **Scenario:** a new dev follows the README (`sops secrets/staging.enc.env`, `source scripts/secrets-env.sh staging`) and hits a missing file; the documented store has zero content.
- **Fix:** Commit the encrypted bundle, or mark the SOPS workflow not-yet-adopted.

### 🟠 O-M8 — DR runbook currency: internal contradictions + stale commands (recovery-under-pressure fails)
- **Where (cluster):** the two DR docs disagree on RPO — `runbooks.md:7-8` says RPO **1h** (hourly), `disaster-recovery.md:16,22-23` says RPO **4h** / "every 4 hours" (code is hourly, `config:84`; `/health` drift limit 90 min, `health.ts:207`); restore flag syntax `--snapshot=<id>` / `--since=4h` (`runbooks.md:29,54`, `disaster-recovery.md:40,59`) doesn't parse — `backup-restore.ts:191,198` reads space-separated flags via `indexOf`, so `--flag=value` → −1 → usage+exit1 (even the monthly *drill* command as written fails); stale host `api.dowiz.org` (12× in `disaster-recovery.md`; also `rollback.md:29,33,58`) vs real `dowiz.fly.dev`; `runbooks.md:88` references a nonexistent `/health` field `backup.r2_reachable` (actual: `.checks.r2`) and "drift > 70 min" (code 90); `disaster-recovery.md:128` verifies a region-down outage via `/admin/health.html` served by the *down* region; no `/version` route and split commit-identity env (`GIT_SHA` vs `RENDER_GIT_COMMIT`, `server.ts:62` / `workers/backup/manifest.ts:58`).
- **Scenario:** under incident pressure the operator's copy-paste commands target a non-resolving host, fail to parse, key off the wrong RPO, and verify via an unreachable page — every runbook step degraded.
- **Fix:** Reconcile RPO to hourly reality; accept `--flag=value`; fix hosts to `dowiz.fly.dev`; correct field names/thresholds; verify region-down via the external monitor; add a `/version` route + single commit-id env.

### 🟡 O-L1 — `assertSchemaCurrent` fails OPEN on any non-"table-missing" error
- **Where:** `apps/api/src/lib/schema-guard.ts:48-56` — if reading `pgmigrations` throws anything other than "relation does not exist", it warns and **returns (boots anyway)**.
- **Scenario:** a transient error / permission change on `pgmigrations` lets a genuinely behind-schema build boot and serve 500s against a stale schema — the precise failure the guard exists to prevent.
- **Fix:** Fail closed on unknown errors (retry first).

---

## Positives observed (calibration — so the next auditor doesn't re-walk them)

- **Compute is EU** (`fly.toml:2` `primary_region = "fra"`); MessageBus payloads had PII stripped (claim-check, `data-map.md:24`).
- **Logger + Sentry PII redaction is mature** — `lib/logger.ts:42-72` (deep key+value redaction, URL secret stripping), `lib/sentry.ts:63-100` (`beforeSend`/`beforeBreadcrumb` scrub, user→id-only, tag allowlist).
- **`/health` is genuinely dependency-aware** — PG-down→503, real R2/Telegram/worker/backup/settlement/anonymizer checks; the worker-aggregate "always-ok" lie was already fixed (H7). A token-gated `/metrics` with real pool/queue gauges exists.
- **Real compliance corpus exists** — RoPA (14 activities, Art.30-shaped), data-map (25 code-grounded PII rows), a DPIA for GPS, a `compliance:gate` CI check (data-map/subprocessors/no-raw-PII/DPIA), consent+`privacy_version` correctly applied to the *waitlist* form.
- **Customer live-location sharing is genuinely opt-in** (explicit start/stop, auto-stops on DELIVERED — `OrderStatusPage.tsx:316-360`) — the good-practice counterexample to the courier gap (P-M3).
- **Some stores DO have working purge** — `courier_positions` (24h), `funnel_events` (90d), `customer_track_grants`/`access_requests`/`acquisition`/`delivery_trace` (dedicated retention workers). Encryption-at-rest is used for `couriers.*_encrypted` and `customer_devices.token_encrypted`.
- **Cross-cutting ops root cause:** safety mechanisms were added incrementally without reconciling the ones they supersede → **two contradictory migration paths** (CI `migrate:up` + Fly `release_command`) and **two contradictory boot-safety stances** (migration-042 "tolerate & continue" vs post-listen assert "die if missing"); purpose-built drift guards unwired because `ci.yml` predates them. Each new guard added a *new* failure mode instead of closing the old one.

---

## Merged PII inventory (retention lens — ✗ = no purge / doc-vs-code gap)

| Column(s) | Table | Source migration | Plaintext? | Purge / TTL |
|---|---|---|---|---|
| phone, email, display_name, google_sub | users | 1780310071220:10-13 | plaintext (totp encrypted) | none (owner acct) |
| phone / name / marketing_opt_in | customers | 1780310074262:11-12 / 1780338982017:6 | plaintext | anon (randomize/NULL/false) |
| messenger_handle, messenger_kind | customers | 1790000000038:13-14 | plaintext | **✗ (P-H1)** |
| delivery_address / delivery_instructions | orders | 1780310074262:29 / 1781103222824:5 | plaintext | NULL on anon |
| delivery_lat, delivery_lng | orders | 1780310074262:30-31 | plaintext GPS | **✗ (P-H1)** |
| delivery_photo_key (+R2 object) | orders | 1790000000039:9 | R2 key | **✗ (P-H1)** |
| customer_messenger_handle/kind, receiver_* | orders | 1790000000038:19-20 / 1790000000074:33-36 | plaintext | NULL on anon |
| client_ip_hash | orders | 1780421100054:7 | sha256 (raw never stored) | NULL on anon |
| metadata, preferences, pickup_code | orders | 1780421100057:101 / 1790000000001 / 1780310074262:40 | jsonb/text | **✗ (P-L partial)** |
| email/phone/full_name _encrypted | couriers | 1780421029538:7-11 | **encrypted** | none (deactivate) |
| lat, lng | courier_positions | 1780421100042:10-11 | plaintext GPS | ✓ 24h cron |
| phone_hash, client_ip_hash | velocity_events | 1780421100057:47-48 | hashed | **✗ despite "24h" (P-M8)** |
| phone_hash, token_hash | customer_otp_sessions | 1780421100057:77-79 | hashed | **✗ (P-M8)** |
| phone (RAW), code_hash | phone_otp | 1780421100054:22 | **phone RAW** | **✗ (P-M8)** |
| token_encrypted, fingerprint (RAW) | customer_devices | 1780348982033:9-10 | token enc, fingerprint raw | **✗ (P-M8)** |
| evidence (jsonb) | customer_signals | 1780421100057:19 | hashed ids | **✗ (expires_at unused)** |
| subject_phone (RAW) | gdpr_erasure_requests | 1780421100060:16 | plaintext | **✗ post-completion (P-L2)** |
| address (chat_id/phone) | owner_notification_targets | 1780348982032:9 | plaintext | none (P-L2) |
| ip_hash, anon_id, session_id, props | analytics_events (no RLS) | 1790000000012 | reversible ip_hash | **✗ blank retention (P-L1)** |

*Top 4 privacy fixes (P-H1) concentrate in one file — `apps/api/src/lib/anonymizer/index.ts` + the GDPR worker's order cascade.*
