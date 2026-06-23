import { test, expect } from '@playwright/test';

// GUARDRAIL for the "storefront blinks empty under load" blocker.
// Root cause (diagnosed + reproduced on staging): the public menu ran the full read_public_menu
// query on every hit and the route checked out TWO operational-pool connections per request
// (Promise.all of the menu fn + a redundant locations lookup). A concurrent customer burst
// exhausted the pool (max 8) → excess requests waited connectionTimeoutMillis (5s) → HTTP 500
// → the FE rendered an empty storefront (MenuPage catch → setMenu(null)).
//
// Fix (F1 in-process cache + F2 drop redundant query + F3 bigger pool + F4 set-based availability).
// RED before the fix: a 30-wide burst produced ~15-20 × HTTP 500 @ ~5.1s (curl-reproduced).
// GREEN after: every request 200 with a non-empty menu (the cache collapses the burst to ~1 DB hit).
//
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test menu-load --reporter=list
const MENU = '/public/locations/demo/menu';
const BURST = 30;

test('public menu survives a concurrent burst with zero 5xx and always has products', async ({ request }) => {
  // Warm the cache once (the first cold request legitimately hits the DB).
  const warm = await request.get(MENU, { headers: { accept: 'application/json' } });
  expect(warm.status(), 'warm-up request ok').toBe(200);

  const results = await Promise.all(
    Array.from({ length: BURST }, () => request.get(MENU, { headers: { accept: 'application/json' } })),
  );

  const statuses = results.map((r) => r.status());
  const fivexx = statuses.filter((s) => s >= 500);
  expect(fivexx, `no 5xx under ${BURST}-wide burst (got: ${statuses.join(',')})`).toHaveLength(0);

  // Every successful response must carry a non-empty menu — never a blank storefront.
  for (const r of results) {
    expect(r.status(), 'each burst request is 200').toBe(200);
    const body = await r.json();
    const products = (body.categories ?? []).flatMap((c: any) => c.products ?? []);
    expect(products.length, 'menu has products under load').toBeGreaterThan(0);
    expect(body.location_name ?? body.locationName, 'location_name present (F2)').toBeTruthy();
  }
});
