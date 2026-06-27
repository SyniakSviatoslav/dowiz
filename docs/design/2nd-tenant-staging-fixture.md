# Design (DRAFT — for review) — 2nd-tenant staging fixture for cross-tenant/IDOR tests

**Date:** 2026-06-27 · **Status:** proposal, needs your decision + Council (touches dev-auth). Enables the
~156 cross-tenant/IDOR `needs_staging` findings (bucket C) to run REAL isolation assertions — owner-A
token → tenant-B resource = 403/404 with tenant-B's **real** id, never the nil-UUID that 404s by absence.

## The one hard constraint
Staging has **no `/auth/local/register`**, and `mock-auth` mints only `dev@deliveryos.com` (single-user
**by design** — its own comments warn an email param re-opens the dev-login backdoor, ADR-0003). So the
2nd-tenant *data* is easy (see below) but minting **owner-B's token** safely is the decision.

Good news: **`/api/dev/seed-visual-state` already creates a distinct 2nd owner** — `vis-owner@dowiz.com`
+ "Visual Net Org" + its own location + an `owner/active` membership. The fixture builds on that.

## Option for minting owner-B's token (pick one — all are dev-gated product changes → Council)

| | approach | backdoor surface | recommend |
|--|----------|------------------|-----------|
| **A** | Extend `seed-visual-state` to **return a scoped token for the user IT just seeded** (vis-owner only) | minimal — the seeder mints a token only for its OWN fixture user, not arbitrary impersonation | ✅ **recommended** |
| B | `mock-auth` email-allowlist (`dev@deliveryos.com` + `vis-owner@dowiz.com`) | wider — re-opens part of the impersonation shape the comment warns against | ✗ |
| C | Seed vis-owner with a known password → mint via `/api/auth/local/login` | adds a standing credential to seed | ~ |

**A** keeps the invariant "a dev endpoint only ever authenticates a user it itself created/owns," so it's
not a general impersonation backdoor — and it stays behind `ALLOW_DEV_LOGIN + DEV_AUTH_SECRET` + a prod-404
canary. (~15 lines in `seed-visual-state`.)

## Fixture API (`e2e/fixtures/two-tenants.ts`)
```ts
export interface Tenant { ownerToken: string; locationId: string; orderId: string; productId: string; customerToken: string }
export const test = base.extend<{ twoTenants: { A: Tenant; B: Tenant } }>({
  twoTenants: async ({ request }, use) => {
    requireStaging(BASE);                                  // never prod
    const A = await provisionTenantA(request);             // mock-auth (dev owner) bound to demo + a real order
    const B = await provisionTenantB(request);             // seed-visual-state → vis-owner token + its location + a real order
    await use({ A, B });
    await cleanup([A, B]);                                  // reject the seeded orders (no fake sales)
  },
});
```

## Test pattern these unlock (tag `@needs-staging`, run only when the fixture exists)
```ts
test('IDOR: owner-A cannot read tenant-B order', async ({ twoTenants, request }) => {
  const r = await request.get(`${BASE}/api/orders/${twoTenants.B.orderId}`, { headers: bearer(twoTenants.A.ownerToken) });
  expect(r.status()).toBe(404);                            // requireLocationAccess / order-scope → hidden, not leaked
});
```
Matrix per resource (orders, owner/locations/:id/dashboard/snapshot, products, brand, settings, promotions):
A→B and B→A, owner-token and customer-token, GET + mutating verb → exact 403/404 from the route. Replaces
every nil-UUID "IDOR" the sweep flagged.

## Security + ops
- Dev-gated (`isDevAuthAllowed`), `requireStaging(BASE)` in the fixture, a canary asserting the seed/mint
  endpoints are **404 on prod**. The seeded tenant-B persists (idempotent seed); orders are reject-cleaned.
- The minted owner-B token is the seeder's fixture user only — documented as such; no arbitrary impersonation.

## Decisions I need from you
1. **Approve Option A** (seed-visual-state returns its seeded owner's token) — or pick B/C? (Council, since
   it's dev-auth.)
2. OK to **persist** the `vis-owner` tenant-B on staging as a standing QA fixture (idempotent)?
3. Wire the `@needs-staging` IDOR suite into the **staging CI job** (Wave B1) once the fixture lands?

On your nod I'll implement Option A (the ~15-line seeder change → Council), the fixture, and convert the
bucket-C nil-UUID TODOs into real A↔B isolation assertions.
