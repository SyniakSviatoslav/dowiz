# AUDIT SYNTHESIS — dowiz/DeliveryOS — 2026-07-03

**Consolidates six READ-ONLY lane audits** (no re-audit performed; every claim traces to a source doc):
`audit-money-orders` (3C/5H/6M/4L) · `audit-security` (1C/3H/5M/8L + 11 latent-RLS) · `audit-reliability`
(4C/10H/17M/13L) · `audit-architecture` (16 findings) · `audit-test-integrity` (6C/8H/7M/5L) ·
`audit-frontend` (3C/27H/34M/26L). All in `docs/design-review/`, same date.

---

## 1. Executive summary

**211 live findings + 11 latent-RLS findings = 222 total** across six lanes; 17 lane-flagged
CRITICALs dedupe to **9 live-today CRITICALs** touching real users or data. The dominant failure mode
is NOT missing design — the right primitives exist and are individually sound (RS256 JWT, WS room
authz, the order state machine, derivePalette AA, webhook HMAC). The failure mode is **partial
adoption plus unverified glue**: `withTenant` covers 22 of 65 route files, the state-machine fold
covers 2 of 5 terminal paths, the refund ledger has 1 writer on a ~6-writer surface, 124 contract
schemas have 0 importers, and CI executes essentially none of the genuinely good tests.

**Single scariest live bug:** the inclusive-tax double-charge (money C1) — under the schema-DEFAULT
configuration every taxed order is charged the extracted VAT **again** on top of a tax-inclusive
price, and the FE/BE fee-parity test *certifies* the bug because it pins mirror==mirror instead of an
independent expectation.

**The compounding horror:** none of this would surface. CI runs zero unit tests, the post-deploy
"regression" E2E skips every assertion on the only env it runs against (all-skipped = green),
`/health` hardcodes the workers aggregate to `'ok'`, and the backup restore-drill has never once been
able to pass against the artifact format the writer actually produces.

**Verdict:** a sound skeleton wearing unverified flesh — fix the four systemic roots (§3) and whole
finding-classes collapse; fix any single symptom without the CI root and you cannot prove it stays fixed.

---

## 2. Live-today CRITICAL list (deduped)

Bugs affecting real users/data right now. Format: title — `file:line` — failure — lane(s) — red-line class.

| # | Finding | Where | One-line failure | Lane(s) | Red-line |
|---|---------|-------|------------------|---------|----------|
| LC1 | **Inclusive-tax charged twice** | `apps/api/src/routes/orders.ts:509-511` + `packages/ui/src/lib/money.ts:81-84` | With `price_includes_tax=true` (the schema DEFAULT), extracted VAT is added back on top → every taxed order overcharged; `fee-parity.test.ts` locks the bug in on both sides | money C1 (+test-integrity: mirror-lock guardrail class) | **money** |
| LC2 | **Cross-tenant order-status PATCH IDOR** | `apps/api/src/routes/orders.ts:840-905` | Owner of tenant A can PATCH any tenant B order by UUID — read has no membership JOIN, `withTenant` inert under BYPASSRLS; the GET sibling was hardened, PATCH left behind | security F1 (root: architecture F2) | **authz** |
| LC3 | **Customer post-dispatch cancel 500s on EVERY call** | `apps/api/src/routes/customer/orders.ts:308-312` | Writes `cancelled_at`/`cancellation_reason` — columns that exist in NO migration (42703 → rollback → 500); raw UPDATE bypasses `updateOrderStatus`; its `ORDER_CANCEL_AFTER_DISPATCH` event has zero subscribers; the only e2e asserts a 403 with an owner token so the happy path has never executed | money C2 + architecture F4 (machine bypass) + test-integrity (403-only proof) | **state-machine** |
| LC4 | **GDPR erasure permanently stranded `in_progress`** | `apps/api/src/workers/anonymizer-gdpr.ts:29,39,90-98` | One transient failure flips the row to `in_progress`; retry scans only `pending` → the legally mandated deletion silently never completes and never reaches `failed` | reliability C4 | **PII / legal** |
| LC5 | **Cross-tenant irreversible PII erasure** | `apps/api/src/routes/owner/gdpr.ts:48` → `apps/api/src/lib/anonymizer/index.ts:118-141` | Body `customerId` taken verbatim, anonymizer UPDATE has no `location_id` predicate → owner A irreversibly anonymizes tenant B's customer (rated HIGH only for the UUID precondition; impact is CRITICAL-class) | security F2 | **PII + authz** |
| LC6 | **Crypto refund black hole** *(DARK — flags off; must fix before flag-flip)* | `apps/api/src/lib/deliveryCompletion.ts:129-145` (sole `refund_due` writer); `apps/api/src/routes/payments-webhook.ts:64-69` | Pay-then-cancel drops the refund obligation on every cancel path except completeDelivery; cancel-then-pay marks CANCELLED orders `paid` with no status check — customer money silently kept | money C3 | **money** |
| LC7 | **Backup/DR inoperable end-to-end** | `Dockerfile:41-56` (no pg_dump/pg_restore in image); `backup-verify.ts:92-95,302,317` (drill reads nonexistent column, hashes ciphertext vs plaintext checksum, smoke-checks the LIVE DB); `scripts/backup-restore.ts:64-86` (restore needs the DB you just lost) | The "sole recovery net" cannot dump, has never validated a real artifact, and cannot locate a snapshot in the exact disaster it exists for | reliability C1+C2+C3 (+H6 lock leak, H9 exit-1=success, H10 grant strip) | **data-recovery** |
| LC8 | **Order-placing CTAs illegible on tenant brands** | `apps/web/src/pages/client/CheckoutPage.tsx:714`; `apps/web/src/routes/ClientLayout.tsx:195,285` | Place-Order / sticky-cart / cart-sheet buttons pair the wrong on-primary token → 1.29–3.74:1 contrast on 4 of 6 tested palettes — the buttons that start and place every order | frontend #2+#3 (class S2, 6 more HIGH instances) | money-adjacent (conversion) |
| LC9 | **Fake data rendered/saved as real** | `apps/web/src/pages/admin/CRMPage.tsx:79-87` (fabricated customer history into cache); `AnalyticsPage.tsx:80-89` (fake BOM with working CSV export); `SettingsPage.tsx:69-79,152` (MOCK_SETTINGS savable as live settings); `courier/DeliveryPage.tsx:264-272` + `CheckoutPage.tsx:291-292` (hardcoded Tirana/Durrës coords as real pins/routes) | Fetch-failure fallbacks present fabricated money/PII/geo data indistinguishable from real — some savable/exportable, one routes couriers to the wrong place | frontend #1 + H9/H10/H11/H20/M43 (class S3) | data-integrity |

**Live-today CRITICAL count: 9** (LC6 dark-but-built; the other 8 fire on today's deployed behavior).

Near-critical HIGHs worth flagging with these: owner PATCH `DELIVERED` permanently strands the courier
from all future dispatch (money H1, state-machine); `mark-no-show` bypasses the SYSTEM-only cancel
guard and brands innocent customers (money H2); logout leaves the refresh token alive on shared
devices (frontend #14, auth — ESCALATE); storefront GPS egress to third-party OSRM on page load with
no gesture (frontend #25, PII).

---

## 3. Cross-lane ROOT-CAUSE map

One fix per root kills the whole class. Symptom lists cite the lane findings each root explains.

### R-A. No data-access layer — the tenant seam is optional (architecture F2)
~777 raw `.query(` in routes; `withTenant` imported by **22 of 65** route files; the ADR-0004
tenant-resolution helper cloned ≥5× with divergent failure semantics; three different isolation
models inside single files.
**Explains:** security F1 (PATCH IDOR), F2 (GDPR erasure — unscoped child-FK), F5 (signals
cross-tenant read), F10/F11 (latent IDORs one rename/RLS-relax away), F3/F4 adjacency (per-file
hand-assembled guard stacks is how `requireRole` gets forgotten); money M1 (TOCTOU on the cancel
guard); reliability L9 (raw cross-tenant UPDATE in apps/worker); the ~30 copy-pasted order-SELECTs.
**Class fix:** membership-JOIN/withTenant as the mandatory seam + per-aggregate repositories + one
`getOwnerLocationId` with a single failure convention (regression-tested — authz red-line).

### R-B. Inconsistent transaction-scoped `set_config`/GUC discipline (reliability thread 2)
The canonical shape (`BEGIN` + `set_config(k,v,true)`) exists and is documented in-code, but is
violated in both directions; everything is masked today by BYPASSRLS and **bites at the NOBYPASSRLS
flip**.
**Explains:** reliability H4-rls (every customer status push silently dies post-flip — autocommit
GUC no-op), H5-rls (session GUC leaks onto the pooled connection → cross-tenant read/write hazard),
L9 (context-free worker UPDATE → silent no-op post-flip); architecture F2's `assignments.ts:109`
set_config with no BEGIN; and it compounds every security §E latent gap (unscoped anonymous policies
on orders/customers, `couriers`/`courier_sessions` with NO RLS at all — the flip **will not achieve
isolation** until §E is fixed).
**Class fix:** one audited `withTenantTx` helper + a lint/grep gate banning bare `set_config` outside
a transaction; fix security §E policy gaps in the same council as the flip.

### R-C. CI is a paper gate — detectors that lie (test-integrity C1–C6, H1–H8; reliability H7)
The post-deploy "regression" spec skips 100% of assertions on prod (all-skipped = green, C1/C2); the
entire unit/integration suite runs in **no** workflow (C3); the RLS-adversarial red-line test is
never wired and silently skips without env (C4); stage-tests "verify" security by grepping source
text and `assert.ok(true)` after finding violations (C5/C6); lint never fails on warnings (H2);
gitleaks silently skips in CI (H5); `fresh-provision` doesn't gate deploy (H6); the drift-preflight
scripts are built but unwired (H7). Sibling class: `/health` hardcodes workers `'ok'` (reliability
H7) and the backup drill can never pass (reliability C2).
**Explains:** why LC1's parity test certifies the bug, why LC3's dead route stayed green, why every
other bug in this synthesis ships undetected. **This root gates the proof of every other fix.**
**Class fix:** one CI job running `test:unit` + phase5 security tests against the already-provisioned
service DB; point the lifecycle/contract E2E at staging; `deploy.needs: [validate, fresh-provision]`;
`--max-warnings 0`; skip-of-a-should-succeed-setup = FAIL; derive health aggregates from entries.

### R-D. `contracts/` unwired — FE/BE agree by convention (architecture F3, F4)
124 Zod schemas, **0 importers** in `apps/api/src`; the live contract is mislabeled `legacy.ts`;
where contracts are used only the FE parses them; no WS event union at all; order status modeled 4+
ways, `order_type` modeled 3× with copies that disagree.
**Explains:** frontend #12 (promotions unit mismatch — 500 ALL renders "5 ALL"), #13 (minor-units
label → 100× prices), M45 (stale `deliveryType` drift); architecture's live drift examples
(`valid_from` required-vs-optional; `task_offered` emitted, never handled — offered tasks never
appear live; the PENDING label bug; ~8 WS event types handled by no client).
**Class fix:** pick ONE SoT and wire it into `fastify-type-provider-zod` `schema.body/response`
blocks (dependency already present) + a `contracts/ws-events.ts` discriminated union typed on both ends.

### R-E (secondary). pg-boss v10 runtime vs v12 types + `@ts-nocheck` (reliability thread 1)
`singletonKey` dedup is a fleet-wide NO-OP on `standard` queues (H1), handler array-shape bugs (M5),
no retry/backoff/DLQ (H2), broken `cancel` (H8), the backup lock-release arity bug (H6) — all masked
by `@ts-nocheck`. **Class fix:** align versions, remove `@ts-nocheck` from `workers/backup/`, create
affected queues `policy:'short'`.

### R-F (secondary). Frontend systemic classes S1–S6 (frontend doc)
Retry-never-clears-error (5 pages), wrong on-primary token (9+ sites), fake-data fallback (LC9),
silent mutation failure (~10 sites), zero focus traps repo-wide, undefined CSS tokens. Each is one
shared-pattern fix, not N point fixes.

---

## 4. Ranked remediation plan

Fix-path legend: **COUNCIL** = Triadic Council before any code (money/authz/RLS/PII/state-machine/
migrations red-line) · **safe-direct** = normal ship-discipline loop · **operator/protect-path** =
gated by Phase-0 protect-paths / operator approval · **quick-win** = small, reversible, provable same-day.

### Group A — Fix-the-class roots (highest leverage; do these first or in parallel lanes)

| Rank | Root | Severity | Red-line | Effort | Fix-path |
|------|------|----------|----------|--------|----------|
| A1 | **R-C: make CI real** — unit-test job + staging E2E + fresh-provision gates deploy + skip=fail + health aggregate truthful | CRIT (meta) | none (gate wiring; never weaken a gate) | M | safe-direct — prerequisite for proving everything below |
| A2 | **R-A: data-access seam** — membership-JOIN/withTenant mandate, kill the 5 helper clones, repositories per aggregate | CRIT-class | authz | L (M per surface) | **COUNCIL** (design), then staged safe-direct per surface |
| A3 | **R-B: GUC/tx discipline** — `withTenantTx` helper, fix H4-rls/H5-rls now, close security §E policy gaps with the flip | CRIT at flip | RLS | M | **COUNCIL** (bundled with the NOBYPASSRLS flip council) |
| A4 | **money terminal-state hook** — centralize `refund_due` on entering terminal non-DELIVERED with a paid payment (LC6) + widen the R2-3 fold to ALL terminals (money H1) | CRIT (dark) | money + state-machine | M | **COUNCIL** — hard gate before payments flag-flip |
| A5 | **R-D: contracts SoT** — wire contracts into fastify schema blocks + WS event union + one orderStatus map | HIGH | none | M | safe-direct |

### Group B — Point fixes, ordered by live-harm × reversibility

| Rank | Finding (lane) | Sev | Red-line | Effort | Fix-path |
|------|----------------|-----|----------|--------|----------|
| 1 | LC5 cross-tenant PII erasure — validate `customerId` same-tenant + `AND location_id` in anonymizer (security F2) | HIGH/CRIT-impact | PII+authz | S | **COUNCIL** (irreversible harm) |
| 2 | LC2 PATCH IDOR — membership JOIN on the read, 404 before transition (security F1) | CRIT | authz | S | **COUNCIL** |
| 3 | LC4 GDPR stranding — reset `status='pending'` on retryable failure (reliability C4) | CRIT | PII/legal | S | **COUNCIL** (one-liner; council can be same-day) |
| 4 | LC1 tax double-charge — inclusive branch adds 0; independent-expectation unit test (money C1) | CRIT | money | S | **COUNCIL** |
| 5 | LC7 backup/DR — pg_dump in image, fix drill format/hash/pool, `--list-r2` mode, releaseLock arg (reliability C1-C3, H6) | CRIT | data-recovery | M | operator/**protect-path** (Phase-0 branch already gates these paths) |
| 6 | LC3 customer cancel — route through `updateOrderStatus`, drop phantom columns, subscribe or replace the orphan event, customer-token 200 e2e (money C2) | CRIT | state-machine | M | **COUNCIL** |
| 7 | F3/F4 missing `requireRole(['owner'])` on couriers + courier-invites (security) | HIGH | authz | S | **COUNCIL** (two-line hooks; council fast-track) |
| 8 | Money H1 owner-PATCH DELIVERED strands courier + H2 mark-no-show guard bypass + H4 fabricated cash attestation | HIGH | money/state-machine | M | **COUNCIL** |
| 9 | Money H3 crypto override erases refusal + H5 settlement SKIP-LOCKED drops cash rows / paid payouts mutate | HIGH | money | M | **COUNCIL** (H5 touches a migration-defined fn) |
| 10 | Frontend #14 logout leaves refresh token (ESCALATE per doc) | HIGH | auth | S | **COUNCIL** |
| 11 | Frontend #25 GPS egress to OSRM on load | HIGH | PII | S | **COUNCIL** (privacy precedent: prior P0 class) |
| 12 | Security F7 invite-redeem ATO (`ON CONFLICT` password overwrite) + F6 telegram fail-open | MED | authz | S | **COUNCIL** (F7); safe-direct (F6 boot-require) |
| 13 | LC8 CTA contrast — token swap to `--color-on-primary`/`--brand-primary-strong` across S2 sites | CRIT (FE) | none | S | **quick-win** |
| 14 | LC9 fake-data class — delete fabricated fallbacks, explicit error/empty states (S3) | CRIT/HIGH (FE) | none | S–M | **quick-win**/safe-direct |
| 15 | Reliability H1 queue `policy:'short'` + H2 retry/DLQ config + H3 per-worker boot isolation | HIGH | none | S each | safe-direct |
| 16 | Reliability H7 health aggregate + H8 broken cancel + M6 dead escalation worker | HIGH/MED | none | S | safe-direct |
| 17 | Security B1 SSRF DNS-rebind (pin resolved IP) + C1 rate-limit key collapse (`hook:'preHandler'` or `clientIp()`) | MED | none | S | safe-direct |
| 18 | Security §E latent RLS: `couriers`/`courier_sessions` NO RLS, unscoped anon policies, self-mint ops tables | HIGH (latent) | RLS+migrations | M | **COUNCIL** — blocker bundled with A3/the flip |
| 19 | Money M2–M7 (crypto replay, webhook minor-unit, refund guard, dispatch double-book, cancel fan-out, `{}`→refused default) | MED | money | S–M | **COUNCIL** batch |
| 20 | Frontend S1 retry-never-clears + S4 silent-mutation-toast + S5 one modal-shell w/ focus trap + S6 undefined tokens | HIGH/MED (FE) | none | M | safe-direct (one shared pattern each) |
| 21 | Test-integrity H1 (lint the `test-stage*` files) + H5 (gitleaks fail-on-missing) + M4 (ledger rows 39-42 dup → gate currently RED) | HIGH/MED | none | S | safe-direct — never-weaken applies |
| 22 | Reliability M12 restore env-guard, M13 key-rotation keyring, M15 `--no-check-order`, M17 pgboss GRANT PUBLIC | MED | migrations (M15/M17) | S–M | operator/**COUNCIL** (M17 is a migration) |

---

## 5. Blind spots & bad-decision inventory (non-bug structural liabilities)

- **`RedisMessageBus` IS Postgres LISTEN/NOTIFY** (`message-bus.ts:242` alias "to make the test pass") — every new contributor reasons about the wrong infra; the real Redis is a KV store, the inverse of the config comment. *(arch F10)*
- **Phantom dep:** `packages/platform` declares `ioredis`, never imports it. *(arch F10)*
- **`legacy.ts` holds the LIVE order contract** while 124 organized `contracts/` schemas are decorative — the name says "don't use" on the thing everything uses. *(arch F3)*
- **`spa-proxy.ts` proxies nothing** — 885 lines, ~19 first-class owner endpoints, two URL conventions for one API, internals leaked in error bodies. *(arch F1, F5)*
- **`stubs.ts` dead + name-shadows the real NotificationProvider interface.** *(arch F10)*
- **Dead code ~650+ FE lines grep-verified importer-free** (theme/index.ts preset system carrying the stale `#ea4f16` identity, atoms Button/Input/StatusBadge cluster, use-voice-order, TourHint machinery, SatelliteMap, ALLERGEN_COLORS) + backend dead DwellEscalationWorker on a nonexistent queue + no-op `health-job`. *(frontend #30/31/71/72; reliability M6; arch F13/F16)*
- **Feature-flag sprawl, no registry:** 4 disjoint surfaces, 51 env keys across 38 files; payments/voice/courier flags bypass the Zod boot-guard entirely. *(arch F6)*
- **God-hubs:** server.ts 890 lines/91 commits (+257 lines of dev handlers duplicated in routes/dev), orders.ts POST ~640-line function, MenuPage 1768, MenuManagerPage 1292, i18n-catalog 4,296-line flat file. *(arch F7/F8/F9)*
- **Dual worker topology:** apps/worker deployed for exactly one load-bearing handler (order timeouts) — a vestigial SPOF. *(arch F13)*
- **pg-boss v10 runtime vs v12 types, hidden by `@ts-nocheck`** — a version lie that already produced 6+ findings. *(reliability thread 1)*
- **Migration hygiene:** `--no-check-order`, tenant data baked into schema history, a migration that DELETEs "duplicate" rows by heuristic, `down()`s that can't run. *(reliability M15/M16)*
- **Pool construction not centralized; ADR-0001's 14-connection budget silently exceeded** (op pool now 20); the inline backup pool escapes the RLS-superuser guard on the privileged migrations role. *(arch F11)*
- **Allergen taxonomy in 3 vocabularies** (`milk` vs `dairy` vs `seafood`) — SAFETY-adjacent; UI never imports `EU_ALLERGENS`. *(arch F12)*
- **Runbooks describe a system that doesn't exist** (dry-run "must pass" never fails; DR doc routes verification through a broken drill; restore ordering guarantees grant-stripping). *(reliability M14, H10)*
- **Theme architecture: 4+ uncoordinated mode axes, no arbiter** (tenant vars × data-surface × prefers-color-scheme × sunlight × paper); one pair already provably wrong; two z-index scales. *(frontend #74/#69)*
- **60 E2E files hardcode the prod host; the rule that catches it is warn-only.** *(test-integrity L3, H2)*
- **Repo hygiene:** `spikes/stage3-queue` is a live workspace member; `.ignored_*` stale full-source package duplicates pollute every app's node_modules. *(arch F16)*

---

## 6. What's verified SOUND (do not re-audit; do not fear-monger)

**Auth/security core** *(security §A/D + clean-sweep list)*: RS256-only JWT w/ verified kid selection,
Zod-strict claims; courier sessions re-bound per request; dev-guard fail-closed; argon2 + timing-safe
compares; WS room authz + per-frame fan-out re-authz (no cross-tenant subscribe/relay path);
menu-import LLM path (PII-redacted pre-model, Zod-validated post, array-arg exec — no shell
injection); payments webhook HMAC `timingSafeEqual` before any ledger write; admin/platform plane;
acquisition/provisioning; claim flow; the whole `/api/owner/locations/:locationId/*` group; courier
data routes; customer/orders scoping; token minting (role always server-derived).

**Reliability** *(reliability positives)*: route-handler pool discipline (no leak class recurrence);
no prepared statements on the transaction pool; statement_timeouts + superuser-connect guard;
`/livez` design; `release_command` migrations-before-traffic + schema-guard; PgMessageBus (bounded
reconnect, 8KB claim-check, per-handler isolation); transactional `order.timeout` enqueue; atomic
settlement fn; bounded retention/anonymizer workers.

**Architecture** *(arch positives)*: package graph is a clean DAG (no cycles, no upward imports);
`money.ts`/`currency.ts` complementary, one money-formatting authority; RoutingProvider/QueueProvider/
MessageBus are legitimate abstractions; `mockData.ts` dev-gated and tree-shaken (the exception —
SettingsPage's prod fallback — is LC9); apiClient is a well-built central client.

**Tests that are real** *(test-integrity solid list)*: `money-tax.test.ts`,
`order-machine-transitions.test.ts`, `pii-leak-detector.test.ts`, `health-truthfulness.test.ts`
(fastify.inject, real 503), the self-bootstrapping authz suite, `verify-fresh-provision.sh`
end-to-end assertions, and the three CI preflight scripts (well-built — just unwired, H7).

**Frontend** *(frontend verified-clean)*: derivePalette remains the single genuine AA-enforced
theming path; OTPModal composes ResponsiveDialog correctly (the pattern to copy); CartProvider,
useReorder pricing, maps consolidation shim, GPS-heartbeat cleanup, Wave-1 unit tests (no
false-green patterns).

---

*Synthesis of six lane audits dated 2026-07-03; no source re-audited. Dedup notes: shift-resurrection
(money L1 ≡ reliability M7), cross-tenant settlements regenerate (money L4 ≡ security F13), customer
cancel machine-bypass (money C2 ≡ arch F4's raw-SQL list), fee-ladder mirror (money C1 FE mirror ≡
arch F14), detectors-that-lie (test-integrity C1-C6 ≡ reliability H7/C2 class).*
