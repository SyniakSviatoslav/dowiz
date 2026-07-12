/* eslint-disable @typescript-eslint/no-explicit-any -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';

// GUARDRAIL for the "storefront blinks empty under load" blocker.
// Root cause (diagnosed + reproduced on staging): the public menu ran the full read_public_menu
// query on every hit and the route checked out TWO operational-pool connections per request
// (Promise.all of the menu fn + a redundant locations lookup). A concurrent customer burst
// exhausted the pool (max 8) → excess requests waited connectionTimeoutMillis (5s) → HTTP 500
// → the FE rendered an empty storefront (MenuPage catch → setMenu(null)).
//
// Fix (F1 in-process cache + F2 drop redundant query + F3 bigger pool + F4 set-based availability).
// RED before the fix: a 20-wide burst produced 20 × HTTP 500 @ ~5.1s (== pool connectionTimeout),
// curl-reproduced. GREEN after: zero 5xx; served responses carry a non-empty menu in ~0.1-0.3s
// (the cache collapses the burst to ~1 DB hit).
//
// The precise regression invariant is ZERO 5xx — that is the bug (a 500 → blank storefront). A 429
// from the global per-IP limiter (100/min) is DELIBERATE backpressure, not the blink, and is only
// reachable here because a test hammers from ONE IP (real customers are distinct IPs the cache
// serves from memory); so 429 is tolerated, but every NON-throttled response must be a real menu.
// BURST stays under the per-IP minute budget to keep the run non-flaky.
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
  // THE regression assertion: not a single server error under load.
  const fivexx = statuses.filter((s) => s >= 500);
  expect(fivexx, `zero 5xx under ${BURST}-wide burst (got: ${statuses.join(',')})`).toHaveLength(0);

  // Every served (non-throttled) response must carry a non-empty menu — never a blank storefront.
  let served = 0;
  for (const r of results) {
    if (r.status() === 429) continue; // deliberate per-IP backpressure, not the bug
    expect(r.status(), 'served response is 200').toBe(200);
    const body = await r.json();
    const products = (body.categories ?? []).flatMap((c: any) => c.products ?? []);
    expect(products.length, 'menu has products under load').toBeGreaterThan(0);
    expect(body.location_name ?? body.locationName, 'location_name present (F2)').toBeTruthy();
    served++;
  }
  expect(served, 'at least the warmed/served requests returned real menus').toBeGreaterThan(0);
});

// GUARDRAIL for the cache memory-exhaustion vector (the in-process Map is keyed on the
// caller-controlled ?locale). A fan-out of distinct locales must NOT 5xx/crash the instance —
// the route normalizes the locale and the Map is FIFO-bounded (MENU_CACHE_MAX_ENTRIES).
test('menu endpoint stays healthy under a distinct-locale fan-out (no 5xx)', async ({ request }) => {
  const LOCALES = Array.from({ length: 24 }, (_, i) => `zz${i}`); // unsupported → server coerces to default
  const results = await Promise.all(
    LOCALES.map((l) => request.get(`${MENU}?locale=${l}`, { headers: { accept: 'application/json' } })),
  );
  const statuses = results.map((r) => r.status());
  expect(statuses.filter((s) => s >= 500), `no 5xx on locale fan-out (got: ${statuses.join(',')})`).toHaveLength(0);
  // The unsupported locales fall back to the default menu — any served response still has products.
  const oneOk = results.find((r) => r.status() === 200);
  if (oneOk) {
    const body = await oneOk.json();
    const products = (body.categories ?? []).flatMap((c: any) => c.products ?? []);
    expect(products.length, 'fallback-locale menu still has products').toBeGreaterThan(0);
  }
});
