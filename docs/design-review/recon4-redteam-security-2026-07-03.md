# Recon #4 — DEEP RED-TEAM: Exploitation Chains (READ-ONLY) — 2026-07-03

**Mode:** offensive. Not a checklist — real kill-chains built from the prior recon findings, each hop
verified against the live working tree (file:line). Authorized testing of the operator's own system.
No code changed.

**Chains from:** `audit-security`, `recon2-{integrations,supplychain,concurrency}`, `recon3-privacy-ops`,
`AUDIT-SYNTHESIS` (all 2026-07-03). This doc **weaponizes** those atomic findings into start-to-finish
attacker wins and identifies the single links that collapse the most chains.

> **Standing posture.** `dowiz_api` holds **BYPASSRLS** today → RLS is inert; the *only* live tenant
> boundary is the explicit `WHERE location_id=…` predicate + the per-route guard stack. Every "live"
> chain below rides a hop where one of those two is missing. "dark" = behind a default-off flag; the
> flip-gate is named so the reviewer knows the launch blocker.

Ranked by (exploitability × impact). **7 chains. 1 CRITICAL-live crown jewel.**

---

## Precondition primitive P0 — a self-service customer token scoped to ANY tenant (the master key)

Every "live" account/tenant chain below starts here, so it is stated once.

- Place **one** order on the victim's public storefront `/s/:slug` (no account, no approval). On success
  the API mints a customer JWT: `orders.ts:626-630` → `issueCustomerToken({ orderId, locationId, customerId })`.
- The claim shape is `role:'customer'`, `locationId:<victim location>` — `packages/platform/src/auth/jwt.ts:127-129`.
- That `locationId` is exactly what the tenant guard reads: `plugins/auth.ts:127-131` — for `role==='customer'`
  `requireLocationAccess` **returns success** whenever `user.locationId === :locationId`, with **no owner check**.
  Identically for `role==='courier'` at `:135-139` (`activeLocationId === :locationId`).

**Consequence:** a mere customer (or courier) of location L satisfies the tenant gate on **every**
`/api/owner/locations/L/*` route. The *only* thing standing between them and the owner plane is a
`requireRole(['owner'])` hook — and two owner route files forgot it (Chains 1 & 2). Cost of the master
key: one public order. No enumeration, no secret.

---

## CHAIN 1 — 🔴 CRITICAL (live) — Public order → owner-plane escalation → courier account takeover

**Attacker goal #1 (account takeover). This is the crown jewel: fully self-service, works today, no flag.**

**Preconditions:** none beyond P0 (place one public order at victim L). To hijack a *specific existing*
courier, also know that courier's email string (low-entropy: couriers are often published/known).

**Kill chain:**
1. **P0** → customer JWT with `locationId=L`.
2. **Mint a courier invite as a "customer."** `POST /api/owner/locations/L/courier-invites`
   — `routes/owner/courier-invites.ts:20-21` installs only `verifyAuth` + `requireLocationAccess`;
   **`requireRole` is never imported** (`:6-7`). The customer token passes the tenant gate (P0).
   Body `role` is attacker-chosen (`:27-30`, no allow-list). The response **returns the plaintext `code`
   once** (`:73`) and the `inviteId`.
3. **Redeem into an account you control — or overwrite a victim's.** `POST /invites/:inviteId/redeem`
   (`routes/courier/auth.ts:23`) proves only the argon2 `code` (`:68`) — never that the caller-supplied
   `email` (`:47`) matches the invite's `invited_email_hash`. The INSERT is
   `ON CONFLICT (email_hash) DO UPDATE SET password_hash = EXCLUDED.password_hash` (`:89-94`):
   - Redeem with **your own** email → you are now a courier of L (privilege escalation customer→courier).
   - Redeem with a **victim courier's** email → their `password_hash` is **silently reset to yours** and
     you are handed a live courier JWT + 14-day refresh session (`:131-142`) for **their** account.
4. (Optional persistence) the redeem returns `refreshToken = sessionId.tokenPlain` (`:142`) — a 30-day
   server session (`:125`) that survives the 24 h access-token expiry.

**What's won:** full takeover of a courier account (existing or freshly minted) at L — assignments, live
GPS, cash-collection, and every delivery's unmasked customer PII. Start-to-finish from an anonymous order.

**Blast radius:** per-tenant courier plane; if the target courier is shared across tenants (one global
`couriers` row keyed on `email_hash`), the password reset locks them out **everywhere**.

**The ONE link whose fix breaks the chain:** add
`fastify.addHook('preValidation', requireRole(['owner']))` to `courier-invites.ts` (kills step 2 → no
invite to redeem). Defense-in-depth: fix step 3 by rejecting redeem when
`sha256(email) != invite.invited_email_hash` (kills the password-overwrite ATO even if an attacker holds
a legitimately-issued invite).

---

## CHAIN 2 — 🔴 HIGH (live) — Public order → owner-plane PII harvest + cross-tenant courier DoS

**Attacker goal #2 (cross-tenant/cross-role compromise), read+mutate, no UUID guessing.**

**Preconditions:** P0 only.

**Kill chain (all on `routes/owner/couriers.ts`, which has `verifyAuth`+`requireLocationAccess` but
**no `requireRole`** — `:14-15`, `requireRole` not imported `:6`):**
1. `GET /api/owner/locations/L/couriers` (`:18`) → the whole roster with **decrypted `full_name`**
   (`decryptPII`, `:46`) + masked phone/email + stats (`:28-62`). Customer token passes (P0).
2. `GET /api/owner/locations/L/couriers/:courierId/details` (`:247`) → the **unmasked customer
   `name` + `phone` + `delivery_address`** for that courier's last 20 deliveries (`:271-273`) — a bulk
   customer-PII exfil, one courier at a time.
3. `GET /api/owner/locations/L/couriers/live` (`:143`) → **live GPS + phone** of every on-shift courier
   (`:150-189`); `GET /api/owner/locations/L/orders/:orderId/route` (`:201`) → a courier's full GPS trail.
4. `PATCH /api/owner/locations/L/couriers/:courierId {"status":"deactivated"}` (`:75`) →
   `UPDATE couriers SET status=… WHERE id=$3` hits the **global** courier row (`:100`) and
   `UPDATE courier_sessions SET revoked_at=now() WHERE courier_id=$1` revokes **all** their sessions
   (`:112-116`). The audit row is mis-stamped `actor_kind='owner'` (`:107`).

**What's won:** mass PII exfiltration (courier identities + customer name/phone/address + live GPS) and a
courier denial-of-service — by someone who merely ordered a pizza.

**Blast radius:** all of L's couriers + all their recent customers; step 4's global-row mutation
disables a shared courier for **every** tenant they work for (cross-tenant side effect).

**The ONE link whose fix breaks the chain:** `requireRole(['owner'])` hook on `couriers.ts:14-15`
(kills all four steps for non-owner tokens). Also scope the step-4 mutations to the `courier_locations`
relationship, not the global row.

---

## CHAIN 3 — 🔴 HIGH/CRIT-impact (live) — Legit owner → read + destroy + irreversibly erase an arbitrary tenant

**Attacker goal #2 (cross-tenant total compromise) from a single legit owner account. `withTenant`
is inert under BYPASSRLS, so the active control is the SQL predicate — and these three sinks omit it.**

**Preconditions:** own (or self-provision) any tenant A → a normal owner JWT. Target tenant B's order /
customer UUIDs (v4, not app-enumerable; obtainable via track links, receipts, shared couriers, or the
F5 read below).

**Kill chain:**
1. **Destroy B's live orders (F1 / LC2).** `PATCH /api/orders/{B_order_uuid}/status {"status":"CANCELLED"}`
   — `routes/orders.ts:840-905`. Guard is only `requireRole(['owner'])` (`:841`), which A passes; the read
   is bare `SELECT id,status,location_id,type FROM orders WHERE id=$1` with **no membership JOIN** (`:862-864`);
   `locationId` is then taken **from the target's own row** (`:871`); `withTenant` (`:860`) is inert.
   `assertOwnerTargetAllowed` (`:876`) restricts *which* transition (PENDING→CANCELLED is allowed), never
   *whose* order → cancel B's live orders at will. (The GET sibling was hardened with a membership JOIN;
   the PATCH was left behind.)
2. **Irreversibly erase B's customers (F2 / LC5).** `POST /api/owner/locations/A/gdpr-requests
   {"customerId":"<B_customer_uuid>"}` — `routes/owner/gdpr.ts:48` takes body `customerId` **verbatim**
   (only the `phone` branch is location-scoped, `:50-53`); the INSERT writes
   `gdpr_erasure_requests(location_id=A, customer_id=<B's customer>)` (`:81-86`). The worker
   (`workers/anonymizer-gdpr.ts:62-65`) calls `anonymizeCustomer`, whose UPDATE has **no `location_id`
   predicate** — `lib/anonymizer/index.ts:133-141` (`WHERE id=$1`) → B's customer `name→NULL`,
   `phone→'anon_'||uuid`, permanently. No undo (forward-only migrations, restore stub — see Chain 4).
3. **Read B's customer reputation (F5).** `GET /api/owner/A/signals/compute?customer_id=<B_customer_uuid>`
   — `routes/owner/signals.ts:105-123` passes the client `customer_id` straight to `computeSignals`; the
   sink `lib/signals/compute.ts:85-87` is `SELECT no_show_count,completed_count,last_no_show_at FROM
   customers WHERE id=$1` with **no `location_id`** → B's behavioural counters + row existence.
4. **Poison B's live menu (modifier-groups injection).** As owner A, `POST /api/owner/locations/A/
   modifier-groups/:groupId/modifiers` — `routes/owner/modifier-groups.ts:136-166` inserts
   `INSERT INTO modifiers (location_id=A, group_id=<B's group uuid>, name, price_delta, …)` (`:156-161`)
   **without checking that `groupId` belongs to A**. If B's storefront resolves modifiers by `group_id`,
   the attacker-controlled `name`/`price_delta` render on B's menu (menu poisoning / price manipulation).

**What's won:** from one owner account — cancel any tenant's orders, irreversibly destroy any tenant's
customer records, read any tenant's customer data, and inject content into any tenant's menu.

**Blast radius:** unbounded across tenants (any B whose UUIDs are learned). F2 is irreversible (red-line).

**The ONE link whose fix breaks the chain:** there is no single line — this is the R-A "no mandatory
membership seam" root. The class fix (a `getOwnerLocationId` membership-JOIN seam applied to the raw-query
sinks: orders PATCH read, anonymizer UPDATE `AND location_id=$2`, signals lookup) collapses steps 1-3 at
once. The **NOBYPASSRLS flip will NOT save you** — the anonymizer UPDATE has no predicate to be re-scoped,
and `couriers`/`courier_sessions` have no RLS at all (audit §E).

---

## CHAIN 4 — 🔴 HIGH (live) — Detection evasion: the breach nobody sees and nobody can undo (force multiplier over 1–3)

**Attacker goal #4. This is what turns any chain above into a silent, unrecoverable breach.**

**Cover the erasure trail (mis-tenanted audit).** When Chain-3 step 2 erases B's customer, the audit row
is written with the **attacker's** location, not the victim's: `workers/anonymizer-gdpr.ts:64` passes
`locationId: row.location_id` (= A, from the attacker-created request) into `anonymize()`, and `:74-77`
inserts `anonymization_audit_log(... location_id = row.location_id …)` = A. Victim B's only visibility is
`GET /:locationId/gdpr-requests/:id`, which reads `anonymization_audit_log WHERE … location_id=$2` = B
(`routes/owner/gdpr.ts:190-193`) → **returns nothing**. B has no record that its customer was erased.

**Delete the trails you do write.** The audit tables are `FOR ALL` RLS (no `FOR` clause → UPDATE/DELETE
permitted in-tenant), with no `REVOKE`/immutability trigger: `anonymization-seam.ts:57-59`,
`courier-audit-log.ts:21-22`, `settlement-audit-log.ts:25` (recon3 P-H3). The `courier.deactivated`
rows Chain-2 step 4 stamps (`couriers.ts:104-108`) sit in `courier_audit_log`, which the in-tenant
attacker can `DELETE`.

**Ensure no alert fires.** Every out-of-band alarm is dead or blind: owner-notification dedup is
**process-memory** (`notifications/workers/index.ts:70`, re-sends/loses on restart — concurrency H5);
liveness state is per-instance (concurrency L1); the **`worker` process has no health check** so a
crash-looped background plane is invisible (recon3 O-M3); CI's secret-scan **no-ops** (`verify-secrets.ts:22-24`,
recon3 O-M6); Sentry can **boot blind** on a missing DSN with no guard (recon3 O-M5).

**Ensure no recovery.** Migrations are forward-only with destructive UPs (recon3 O-H6); the documented
full-restore is a **stub that prints "not yet implemented"+exit(1)** (`scripts/backup-restore.ts:212`,
O-H7); the backup pipeline is inoperable end-to-end (synthesis LC7). The F2 anonymization is irreversible
by construction.

**What's won:** any Chain 1-3 action leaves the victim tenant with **no audit trail, no alert, no undo**.

**The ONE link whose fix breaks the chain:** log the anonymizer audit against the **subject's real
`customers.location_id`** (not the request's) — restores the victim's visibility, the single cheapest
detection restore. Structurally: `REVOKE UPDATE, DELETE` + `BEFORE UPDATE/DELETE` trigger on the three
audit tables makes the trail tamper-evident.

---

## CHAIN 5 — 🟠 MED→HIGH (live, anonymous post-leak) — Telegram secret in URL path → logged → forged live order actions

**Attacker goal #5 (secret → forge). Turns a log reader into an anonymous order-controller for any owner.**

**Kill chain:**
1. **The secret is a path segment.** `routes/telegram-webhook.ts:36` registers
   `POST /webhook/telegram/${telegramBotSecret}`.
2. **The redactor misses the path.** `lib/logger.ts:114` runs `redactUrlSecrets(req.url)` on every
   request, but `redactUrlSecrets` (`:24-40`) only scrubs the **query string** (splits on `?`, redacts
   `SENSITIVE_QUERY_PARAMS`). A secret in the *path* is logged **verbatim** on every webhook hit → Fly log
   drain / Sentry breadcrumb / any aggregator or proxy access log. **Who has log access = who gets the
   secret** (ops, anyone with a Fly/Sentry seat, any log-sink compromise).
3. **The header second factor is skippable.** `telegram-webhook.ts:57-60`: a *missing*
   `x-telegram-bot-api-secret-token` → "process the request anyway" (backward-compat). So the leaked path
   secret is the *only* factor needed.
4. **Inner authority is a non-secret chat id.** With the secret, `POST` a synthetic `callback_query`.
   Authz binds to `callback_query.from.id` → `owner_notification_targets.address` (`:167-172`). Telegram
   chat ids are **enumerable, not secret**, and are attacker-supplied in the forged body → set them to a
   victim owner's chat id and the membership check passes (it is the *real* owner's row).
5. **The action is live, not flag-gated.** `data="order.confirm:<orderId>"` → `updateOrderStatus(…,
   'CONFIRMED')` (`:282`); `order.reject_reason_*` likewise. Only `store.*`/`pref.*` are flag-gated
   (`:184-185`); `order.*` is **not** → confirm/reject a victim tenant's real orders.

**What's won:** anonymous (once the secret is read from a log) confirm/reject of any linked owner's orders
— order-lifecycle sabotage, fake confirmations, mass rejections.

**Blast radius:** every owner with a linked Telegram target.

**The ONE link whose fix breaks the chain:** make the `x-telegram-bot-api-secret-token` header
**mandatory** and move the secret to the header only (static path) — kills the forge even if the path is
logged. Cheaper stopgap: extend `redactUrlSecrets` to mask the `/webhook/telegram/<seg>` path segment
(closes the leak, step 2). The header fix is strictly stronger.

---

## CHAIN 6 — 🟠 money — Customer overcharge (live) + crypto underpayment/double-collect (dark, pre-flip)

**Attacker goal #3 (money theft / fraud).**

**6a — Tax double-charge (LIVE today, LC1).** With `price_includes_tax=true` (the schema DEFAULT),
`applyTax` **extracts** the VAT portion *from* the tax-inclusive subtotal — `lib/money.ts:14-18`
(`net = sub*SCALE/(SCALE+rate); return sub-net`) — and `routes/orders.ts:511` then computes
`total = subtotal + deliveryFee + taxTotal`, **adding the already-included tax back on top**. Every taxed
order overcharges the customer by the VAT amount. The FE mirror (`packages/ui/src/lib/money.ts:16-33` +
`:84`) reproduces it, and the fee-parity guardrail pins mirror==mirror so it **certifies** the bug. Not
attacker-directed, but a live systemic money defect that silently extracts customer money on every order.

**6b — Crypto underpayment via leaked key (DARK — flip-gate `PAYMENTS_CRYPTO_ENABLED`, H1).**
`PLISIO_SECRET_KEY` is read via raw `process.env` outside the Zod schema (recon-supplychain M3), so its
handling has no boot validation. A leak of it lets an attacker **sign** a webhook. The `completed` branch
`routes/payments-webhook.ts:59-70` flips `payments.status='paid'`, `captured_amount_minor=amount_minor`,
`orders.payment_status='paid'` with **no comparison** of `event.amountMinor` against the charged
`payments.amount_minor` — the entire under/over-payment defense is delegated to Plisio's own
`status='mismatch'` classification (`lib/payments/plisio.ts`), never independently checked (the code
comment claiming "the route validates against payments.amount_minor" is false). → mark an order paid for
less than owed. Compounded by `parseEvent` hardcoding `minorUnit=2` for the ALL-default market (100× ledger
corruption, recon2 M1) — defeating any future amount check built on the event.

**6c — Cash + crypto double-collection (DARK, concurrency H3).** With H2's missing fulfillment gate an
unpaid crypto order is dispatchable; the deliver path reads `payment_status='pending'` and settles the
**cash** ledger (`routes/courier/assignments.ts:319-340`) while the webhook later flips `paid`
(`payments-webhook.ts:64-70`) with no `payment_outcome` cross-guard → customer pays twice, and
`workers/reconciliation.ts` has zero payment coverage → never detected, never refunded.

**The ONE link whose fix breaks the chain:** (6a) in the inclusive branch, do **not** add `taxTotal` to
the total (it is already inside the subtotal) + assert against an independent expectation. (6b) gate the
paid-flip on `event.amountMinor >= payments.amount_minor AND currency match`, route shortfall to
owner-review. These are the hard blockers named for the payments flag-flip council.

---

## CHAIN 7 — 🟠 MED (live) — OTP double-consume + throttle collapse (checkout-integrity amplifier)

**Attacker goal #6-adjacent (rate-limit bypass / single-use bypass). Amplifies Chains 1/6.**

- **OTP single-use is double-consumable (concurrency M1).** `routes/orders.ts:161-174` SELECTs the OTP
  session `WHERE … consumed_at IS NULL` **without a row lock**, then `UPDATE customer_otp_sessions SET
  consumed_at=now() WHERE id=$1` with **no `AND consumed_at IS NULL`** and no rowCount check (`:171-174`).
  Two concurrent POSTs with the same verified token both pass → two orders ride one verification. Same
  unguarded consume for `phone_otp` at `:312-328` and on the verify endpoint (`routes/customer/otp.ts:185-186`).
- **The per-target throttles don't engage (C1).** `orders.ts:76` `keyGenerator` reads `req.body`, but
  `@fastify/rate-limit` runs on `onRequest` (pre-body-parse) → key collapses to `req.ip`; with no
  `trustProxy` (`server.ts:82`) that is the Fly **edge** socket → one shared bucket. The DB-backed
  velocity gate is **count-then-insert** (`orders.ts:254-298`, concurrency L5) → exceeded by exactly the
  attacker's parallelism; the limiter is per-instance memory (C2) so it multiplies by machine count.

**What's won:** bypass OTP single-use (when OTP is on) and the anti-spam/anti-fraud order throttles under
concurrency — the enabling substrate for order-spam and for replaying a verified checkout.

**The ONE link whose fix breaks the chain:** make the consume atomic — `UPDATE … SET consumed_at=now()
WHERE id=$1 AND consumed_at IS NULL RETURNING id`, treat `rowCount=0` as not-verified; gate attempts via
`… WHERE attempts<5 RETURNING`.

---

## Cross-chain leverage — the fixes that collapse the most chains

| Fix | Chains broken | Why |
|-----|---------------|-----|
| **An `/api/owner/*` plane guard that requires `role==='owner'`** (or: `requireRole(['owner'])` on `couriers.ts` + `courier-invites.ts`) | **1, 2** (+ the ATO) | Kills the P0 customer/courier-token bypass — the trivially-reachable, highest-blast entry. Closes the auth.ts:127-140 non-owner tenant-pass class. |
| **Mandatory membership-JOIN seam** (`getOwnerLocationId`) on the raw-query sinks | **3** (F1/F2/F5), latent post-flip | Restores the tenant predicate that BYPASSRLS makes load-bearing. F2 also needs `AND location_id` in the anonymizer UPDATE. |
| **Anonymizer audit → subject's real `location_id`** + `REVOKE UPDATE/DELETE` on audit tables | **4** | Restores victim visibility + tamper-evidence — de-fangs the force multiplier. |
| **Telegram secret → mandatory header, static path** | **5** | Forge dies even when the path is logged. |
| **Amount/currency reconcile in the crypto webhook** + **inclusive-tax = add 0** | **6** | The two named money flip-blockers. |

**Single highest-leverage fix:** the **`/api/owner/*` owner-role plane guard**. It breaks Chains 1 and 2
and the account-takeover outright — the only chains that are (a) live today, (b) require **zero** secret
or UUID knowledge, and (c) reachable by anyone who can place a public order. Everything else needs a
leaked secret (5), a guessed UUID (3), a flag flip (6b/6c), or a race (7). The role guard removes the
master key.

---

## Verified-SOUND during this pass (don't re-walk)

- **Dev/mock-auth surface is fail-closed** — `plugins/dev-guard.ts:30-62`: `/dev|/api/dev` require both
  `ALLOW_DEV_LOGIN==='true'` **and** a constant-time-compared `DEV_AUTH_SECRET`; a courier token without a
  `jti` is rejected unless dev-login is on (`plugins/auth.ts:62-70`). No prod bypass.
- **JWT core** — RS256, kid-selected-then-verified, `alg=none` rejected, Zod-strict claims (audit §A). No
  kid-confusion / alg-swap path.
- **Crypto webhook idempotency + monotonic status** are sound; the gap is purely the missing *amount*
  reconcile (Chain 6b), not the state machine (recon2 V2).
- **R2 presign/confirm tenant-scoping** is correct (recon2 V3); the door-photo read-path authz (recon2 M5)
  is a separate, real gap not chained here.
