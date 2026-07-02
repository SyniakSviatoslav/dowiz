# Security hardening findings — blue-team sweep 2026-07-02

Produced by the security-redblue loop's BLUE arm (autonomous, read-only, no attack traffic):
security-sentinel (secrets/vuln), Application Security Engineer (RLS/authz/JWT threat map),
crt.sh asset scan, `pnpm audit`. RED arm (offensive tools vs staging from a Kali workstation)
is operator-gated per docs/security/redteam-runbook.md — NOT run here.

**Discipline:** every RED-LINE finding (auth/RLS/money/PII) → Triadic Council BEFORE any fix
(charter). No red-line code was touched by this sweep. SAFE items are ready patches, listed apart.

## 🔴 The determinant to verify FIRST (human — I can't read Fly secrets)
The live severity of #1/#2/#6/#7 flips on whether the operational write pool connects as
**BYPASSRLS**. Migration `1790000000015` made `deliveryos_operational_user` NOBYPASSRLS + SELECT-only,
but checkout/status/invite routes INSERT/UPDATE on that pool → the live pool is **likely still a
BYPASSRLS superuser**, meaning RLS is currently bypassed and app-layer WHERE predicates are the ONLY
tenant boundary. Confirm `DATABASE_URL_OPERATIONAL`'s role against the deployed secret. Refs:
`packages/db/src/index.ts:17-27`, `apps/api/src/server.ts:208-209`.

## RED-LINE findings → council track (ranked, most-exploitable-now first)
| # | Sev | Where | Class | Live? | Covered? |
|---|-----|-------|-------|-------|----------|
| 1 | HIGH | `apps/api/src/routes/orders.ts:730-736` | Owner/courier `GET /orders/:id` has NO `location_id` predicate (customer branch :755 does) → cross-tenant/IDOR read of address/customer/totals | Yes (BYPASSRLS pool) | CI: NO (rls-adversarial skip+ci:false) |
| 2 | CRIT (latent, blocks B3) | `1780338981783_anonymous_orders.ts:5-10`, `1780338981782_customer-anonymous-update.ts:6-13` | C1 anonymous RLS `USING(app_current_user() IS NULL)` = table-wide TRUE on any no-`app.user_id` conn → fail-OPEN; reachable now via courier path | Latent now / full siphon on B3 | NO (no-context case) |
| 3 | CRIT (latent, blocks B3) | `1780310071220_core-identity.ts:76-78` | C2 keystone definer `app_member_location_ids()` unpinned `search_path` (all member policies depend on it) | Latent | NO |
| 4 | HIGH | `apps/api/src/websocket.ts:122-131` | Owner `order:` room authz checks `role='owner'` but drops `status='active'` (location: sibling :117 has it) → revoked owner keeps streaming (ADR-0004 window) | Yes | Likely NO for order: room |
| 5 | HIGH | `apps/api/src/websocket.ts:154-167` | JWT (24h-14d bearer) read from WS `?token=` URL → leaks to access logs/history/Referer; off-URL path already supported at :186 (ADR-0013 addendum) | Yes | NO |
| 6 | MED | `apps/api/src/plugins/auth.ts:62-91`, `spa-proxy.ts:66` | Owner revocation not enforced at the hook; spa-proxy trusts baked `activeLocationId` skipping the ADR-0004 re-check → removed owner writes for token TTL | Yes | NO for spa-proxy |
| 7 | MED | `apps/api/src/routes/couriers.ts:16,25` | Create-invite trusts body `locationId`, ownership check is RLS-only `SELECT 1 FROM locations` → cross-tenant invite under BYPASSRLS | Yes | NO |
| 8 | MED | `apps/api/src/routes/orders.ts:196,241` + `jwt.ts:117-132` | Customer JWT sets `sub` but no `userId` → velocity/fraud throttle + idempotency fingerprint silently degrade to phone/IP | Yes | NO |

**Recurring structural root (#1/#2/#6/#7):** *identity-split × RLS-reliance* — privileged read/write
paths omit an explicit `location_id` ownership predicate and lean on RLS, while couriers/customers
carry no `userId` to seat the GUC. This is the class the guardrail below must catch.

## ✅ Controls CONFIRMED SOLID (the red tools would bounce off these — do not touch)
- JWT crypto airtight: RS256 pinned twice (`algorithms:['RS256']` + explicit header throw) → alg-confusion
  and `alg=none` both rejected; dev-kid segregated + prod short-circuit (ADR-0003). `jwt.ts:104-114`.
- SQL fully parameterized ($1/$2) — no injection surface (SQLmap would find nothing).
- Courier revocation immediate (live `courier_sessions` re-check every request).
- Courier WS order-room authz sound (ADR-0013, binding-scoped, tri-state UNAVAILABLE).
- RLS FORCE on 40+ tenant tables; SSRF-guarded brand-extractor; Zod .strict() everywhere; secure crypto.

## SAFE-to-fix (non-red-line, ready patches — not applied to keep the auth diff council-clean)
- **WS auth-success log** (`websocket.ts:163/192`, LOW): drop `role`/`sub` from the log line, keep IP.
  Mechanical, but lives in the hot auth file with 4 open red-line findings → bundle with the council fix.
- **`tmp` dep path-traversal** (GHSA-ph9p-34f9-6g65, 2 HIGH but DEV/CI-only via `@lhci/cli`): add
  `pnpm.overrides` `tmp@>=0.2.6`. Root `package.json` is protect-path → operator applies. Not a prod risk.

## Recommended guardrail (catches the #1/#2/#6/#7 class deterministically)
Extend the existing `rls-adversarial.test.ts:221` "privileged pool queries have WHERE location_id"
assertion from `workers/` to `routes/**`, and add a definer-`search_path`-pin assertion — then wire
BOTH into the CI subset (today the adversarial test is `skip`-gated + `ci:false`, so #1–#7 have NO
continuously-enforced guardrail). This is the red→green guardrail that must accompany the council fix.

## Next action
These map onto existing council tracks: B3 (NOBYPASSRLS, C1/C2) and the AUTH-ROLES-AUDIT-2026-06-29.
Recommend: (1) operator confirms the pool role; (2) run the B3/AUTH council on findings #1-#7 as one
hardening batch with the guardrail above as its DoD; (3) apply the two SAFE patches in that same batch.
Related: docs/design-review/AUTH-ROLES-AUDIT-2026-06-29.md, ADR-0004, ADR-0013, docs/security/redteam-runbook.md.
