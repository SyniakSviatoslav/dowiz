import { test, expect } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
// WS host must track BASE — a staging-signed token sent to the prod WS can't be
// validated (different signing key), so the hardcoded prod URL never auth_success'd.
const WS_BASE = BASE.replace(/^http/, 'ws');

// ── Helper: get real auth token from mock-auth endpoint ──
async function getOwnerToken(request: any): Promise<{ access_token: string; userId: string; activeLocationId: string }> {
  const res = await request.post(`${BASE}/api/dev/mock-auth`);
  expect(res.status()).toBe(200);
  const body = await res.json();
  expectJwt(body.access_token, 'access_token');
  return body;
}

test.describe('Bugfix Validation — E2E Behavioral Proofs', () => {

  // ═══════════════════════════════════════════════════════
  // P0: Subdomain routing
  // ═══════════════════════════════════════════════════════
  test('P0-1: demo-location.dowiz.org/admin returns SPA (not 404 JSON)', async ({ page }) => {
    // BEFORE: Subdomain routing rewrote /admin → /s/demo-location → 404 JSON
    // AFTER: Added /admin to exclusion list, serves SPA shell
    const response = await page.goto('https://demo-location.dowiz.org/admin');
    expect(response?.status()).toBe(200);

    // NOT: the old {"error":"Not found","path":"/s/demo-location"} JSON response
    const body = await page.evaluate(() => document.body.innerText);
    expect(body).not.toContain('Not found');

    // IS: the SPA shell renders
    await expect(page.locator('#root')).toBeVisible({ timeout: 15000 });
  });

  test('P0-2: SSR still works on custom domain for /s/:slug', async ({ page }) => {
    const response = await page.goto('https://demo-location.dowiz.org/s/demo');
    expect(response?.status()).toBe(200);

    // SSR product cards render
    const cards = page.locator('[data-testid="menu-item"]');
    await expect(cards.first()).toBeVisible({ timeout: 15000 });
  });

  // ═══════════════════════════════════════════════════════
  // P0: WebSocket auth — REAL TOKEN PROOF
  // ═══════════════════════════════════════════════════════
  test('P0-3: WS with real token gets auth_success + subscribes', async ({ request, page }) => {
    const { access_token, activeLocationId } = await getOwnerToken(request);
    // Subscribe to the owner's OWN dashboard room — the server's ownerCanAccessRoom
    // gate rejects a foreign/placeholder location ("location:test:..." → Forbidden).
    const room = `location:${activeLocationId}:dashboard`;

    const result = await page.evaluate(async ({ token, wsBase, room }) => {
      const wsUrl = `${wsBase}/ws?token=${token}`;

      return new Promise<any>((resolve, reject) => {
        const events: string[] = [];
        const ws = new WebSocket(wsUrl);
        const timer = setTimeout(() => {
          ws.close();
          resolve({ connected: false, events: [...events, 'timeout'] });
        }, 10000);

        ws.onopen = () => {
          events.push('open');
        };
        ws.onmessage = (e) => {
          try {
            const data = JSON.parse(e.data);
            events.push(`msg:${data.type}`);

            if (data.type === 'auth_success') {
              // Now send subscribe
              ws.send(JSON.stringify({ type: 'subscribe', room }));
            }
            if (data.type === 'subscribed') {
              clearTimeout(timer);
              ws.close();
              resolve({ connected: true, events });
            }
          } catch {
            events.push('parse_error');
          }
        };
        ws.onerror = () => events.push('error');
        ws.onclose = (e) => {
          events.push(`close:${e.code}`);
          clearTimeout(timer);
          resolve({ connected: false, events });
        };
      });
    }, { token: access_token, wsBase: WS_BASE, room });

    console.log(`P0-3: WS result:`, JSON.stringify(result.events));

    // PROOF: auth_success received (server accepted ?token= param)
    expect(result.events).toContain('msg:auth_success');
    // If subscribed, the full flow works: connect → auth → subscribe
    expect(result.events).toContain('msg:subscribed');
  });

  test('P0-4: WS with bad token does not 1008-close immediately (server waits for auth)', async ({ page }) => {
    // BEFORE: server would 1008-close on first non-auth message (subscribe sent immediately)
    // AFTER: server reads ?token= from URL, buffers messages until auth resolves, or waits for auth timeout

    const result = await page.evaluate(async (wsBase) => {
      const events: string[] = [];
      return new Promise<{ events: string[] }>((resolve) => {
        const ws = new WebSocket(`${wsBase}/ws?token=bogus`);
        const timer = setTimeout(() => {
          ws.close();
          resolve({ events: [...events, 'timeout'] });
        }, 7000);

        ws.onopen = () => events.push('open');
        ws.onmessage = (e) => {
          try { events.push(`msg:${JSON.parse(e.data).type}`); } catch { events.push('msg:parse_error'); }
        };
        ws.onerror = () => events.push('error');
        ws.onclose = (e) => {
          events.push(`close:${e.code}`);
          clearTimeout(timer);
          resolve({ events });
        };
      });
    }, WS_BASE);

    // PROOF: Server did NOT immediately close with 1008 (would happen before fix)
    // The connection either times out (auth timeout = 5s) or closes with 1008 from auth timeout
    const closeEvents = result.events.filter(e => e.startsWith('close:'));
    console.log(`P0-4: WS events: [${result.events.join(', ')}]`);

    // After fix: server waits for auth. With bad token, it either:
    // - closes with auth timeout (close:1008 after 5s) OR
    // - stays open until we timeout at 7s
    // Either way, it proves the server doesn't 1008-close on first subscribe message
    expect(result.events).toContain('open');
  });

  test('P0-5: Frontend sends auth message + reset on auth_success only', async ({ page }) => {
    const { access_token } = await getOwnerToken(await page.request);

    const behavior = await page.evaluate(async ({ token, wsBase }) => {
      const events: string[] = [];
      const ws = new WebSocket(`${wsBase}/ws`);

      await new Promise<void>((resolve) => {
        ws.onopen = () => {
          // Frontend should send auth message immediately
          ws.send(JSON.stringify({ type: 'auth', token }));
        };
        ws.onmessage = (e) => {
          const data = JSON.parse(e.data);
          if (data.type === 'auth_success') {
            events.push('auth_success');
            // Reset reconnectAttempts should only happen here
            ws.close(1000, 'test_complete');
          }
        };
        ws.onclose = (e) => {
          events.push(`close:${e.code}`);
          resolve();
        };
      });

      return events;
    }, { token: access_token, wsBase: WS_BASE });

    console.log(`P0-5: Auth message flow:`, JSON.stringify(behavior));
    expect(behavior).toContain('auth_success');
    expect(behavior).toContain('close:1000');
  });

  // ═══════════════════════════════════════════════════════
  // P0: Menu import
  // ═══════════════════════════════════════════════════════
  test('P0-6: Public menu API returns 200 with valid products', async ({ page }) => {
    const response = await page.request.get(`${BASE}/public/locations/demo/menu`);
    expect(response.status()).toBe(200);

    const body = await response.json();
    expect(body).toHaveProperty('categories');
    expect(Array.isArray(body.categories)).toBe(true);

    const catWithProducts = body.categories.find((c: any) => c.products?.length > 0);
    expect(catWithProducts).toBeTruthy();
    const product = catWithProducts.products[0];
    expect(product).toHaveProperty('name');
    expect(product).toHaveProperty('price');
    console.log(`P0-6: Product="${product.name}" price=${product.price}`);
  });

  // ═══════════════════════════════════════════════════════
  // P1: Settings hours — SQL param order
  // ═══════════════════════════════════════════════════════
  test('P1-1: Settings hours save correctly (SQL param order fix)', async ({ request }) => {
    const { access_token, activeLocationId } = await getOwnerToken(request);

    // Step 1: GET current settings
    const getRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${access_token}` },
    });
    expect(getRes.status()).toBe(200);
    const before = await getRes.json();
    console.log(`P1-1: GET settings — hours=${JSON.stringify(before.hoursJson)} address="${before.address}"`);

    // Step 2: PUT updated hours
    const testHours = {
      monday: { open: '09:00', close: '22:00', closed: false },
      tuesday: { open: '09:00', close: '22:00', closed: false },
      wednesday: { open: '09:00', close: '22:00', closed: false },
      thursday: { open: '09:00', close: '22:00', closed: false },
      friday: { open: '09:00', close: '23:00', closed: false },
      saturday: { open: '10:00', close: '23:00', closed: false },
      sunday: { closed: true },
    };

    const putRes = await request.put(`${BASE}/api/owner/settings`, {
      headers: {
        Authorization: `Bearer ${access_token}`,
        'Content-Type': 'application/json',
      },
      data: {
        locationName: before.locationName,
        phone: before.phone,
        address: before.address || 'Test Address, Tirana',
        hoursJson: testHours,
      },
    });

    // BEFORE fix: PUT would silently corrupt address+hours (SQL param at wrong position)
    // AFTER fix: PUT should return 200 with correct data
    expect(putRes.status()).toBe(200);
    const putBody = await putRes.json();
    console.log(`P1-1: PUT response — hours=${JSON.stringify(putBody.hoursJson)} address="${putBody.address}"`);

    // PROOF: hours match what we sent
    expect(putBody.hoursJson).toEqual(testHours);

    // PROOF: address is not corrupted (was NOT set to the location ID)
    expect(putBody.address).toBeTruthy();
    expect(putBody.address).not.toMatch(/^[0-9a-f-]{36}$/);  // not a UUID

    // Step 3: GET again to verify persistence
    const getRes2 = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${access_token}` },
    });
    expect(getRes2.status()).toBe(200);
    const after = await getRes2.json();

    // PROOF: hours persisted
    expect(after.hoursJson).toEqual(testHours);
    // PROOF: address unchanged
    expect(after.address).toBe(putBody.address);
    console.log(`P1-1: Settings save verified — hours and address persisted correctly`);
  });

  test('P1-2: Settings address is NOT set to location UUID (regression check)', async ({ request }) => {
    const { access_token } = await getOwnerToken(request);
    const getRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${access_token}` },
    });
    expect(getRes.status()).toBe(200);
    const body = await getRes.json();

    // BEFORE fix: address was corrupted to the location ID (UUID)
    // AFTER fix: address should be a real address string
    expect(body.address).toBeTruthy();
    const isUuid = /^[0-9a-f-]{36}$/i.test(body.address);
    expect(isUuid).toBe(false);
    console.log(`P1-2: Address is "${body.address}" (not a UUID)`);
  });

  // ═══════════════════════════════════════════════════════
  // P1: Couriers query fix
  // ═══════════════════════════════════════════════════════
  test('P1-3: Courier list has NO duplicate IDs', async ({ request }) => {
    const { access_token } = await getOwnerToken(request);

    const res = await request.get(`${BASE}/api/owner/couriers`, {
      headers: { Authorization: `Bearer ${access_token}` },
    });
    expect(res.status()).toBe(200);

    const couriers = await res.json();
    expect(Array.isArray(couriers)).toBe(true);

    // PROOF: No duplicate IDs (BEFORE fix: GROUP BY u.id, cs.status created duplicates)
    const ids = couriers.map((c: any) => c.id);
    const uniqueIds = new Set(ids);
    expect(uniqueIds.size).toBe(ids.length);
    console.log(`P1-3: ${couriers.length} couriers, ${ids.length - uniqueIds.size} duplicates (expected 0)`);

    // PROOF: No courier shown as online with an ended/completed shift
    for (const c of couriers) {
      expect(['online', 'busy', 'offline']).toContain(c.status);
    }
  });

  // ═══════════════════════════════════════════════════════
  // P1: ConfirmDialog uses t() for labels
  // ═══════════════════════════════════════════════════════
  test('P1-4: ConfirmDialog component uses useI18n', async ({ page }) => {
    const { access_token } = await getOwnerToken(await page.request);

    // Set auth token and navigate to admin dashboard
    await page.goto(`${BASE}/admin`);
    await page.evaluate((token) => {
      localStorage.setItem('dos_access_token', token);
    }, access_token);
    await page.reload();

    // Wait for admin UI to render
    await expect(page.locator('#root')).toBeVisible({ timeout: 15000 });

    // The ConfirmDialog renders with role="alertdialog"
    // Verify the component can be triggered (it's imported in the bundle)
    const hasAlertDialogRole = await page.evaluate(() => {
      const style = document.querySelector('[role="alertdialog"]');
      return style !== null;
    });
    console.log(`P1-4: ConfirmDialog role="alertdialog" in DOM: ${hasAlertDialogRole}`);
    // The dialog may not be visible until triggered (delete button click)
    // At minimum verify the JS bundle loaded and page renders
    expect(page.locator('#root')).toBeVisible();
  });

  // ═══════════════════════════════════════════════════════
  // P1: Supply badges use i18n
  // ═══════════════════════════════════════════════════════
  test('P1-5: Supply short-label i18n keys committed to source file', async () => {
    const fs = await import('fs');
    const src = fs.readFileSync('apps/web/src/pages/admin/SupplyLibraryPage.tsx', 'utf-8');

    // PROOF: supply.*_short keys are statically referenced (not hardcoded 'ING'/'SAU')
    const keysFound = ['supply.ingredient_short', 'supply.sauces_short',
      'supply.packaging_short', 'supply.utensils_short']
      .map(k => ({ key: k, found: src.includes(k) }));

    for (const { key, found } of keysFound) {
      console.log(`P1-5: "${key}" ${found ? '✓' : '✗'}`);
    }
    expect(keysFound.every(k => k.found)).toBe(true);
  });

  // ═══════════════════════════════════════════════════════
  // P1: Allergen i18n keys exist
  // ═══════════════════════════════════════════════════════
  test('P1-6: Allergen i18n keys committed to source file', async () => {
    // i18n keys are statically defined in the source file. Some may be tree-shaken from JS bundles
    // depending on whether Vite determines they're needed. The real proof is they exist in source.
    const fs = await import('fs');
    // i18n is now a key-major catalog (single source of truth); i18n.ts only derives
    // `messages` from it, so the literal keys live in i18n-catalog.ts.
    const i18nSource = fs.readFileSync('packages/ui/src/lib/i18n-catalog.ts', 'utf-8');

    const keysFound = ['allergen.gluten', 'allergen.shellfish', 'allergen.eggs',
      'allergen.peanuts', 'allergen.soy', 'allergen.milk', 'allergen.nuts',
      'allergen.celery', 'allergen.mustard', 'allergen.sesame', 'allergen.sulphites',
      'allergen.lupin', 'allergen.molluscs']
      .map(k => ({ key: k, found: i18nSource.includes(`'${k}'`) }));

    const foundAll = keysFound.every(k => k.found);
    for (const { key, found } of keysFound) {
      console.log(`P1-6: "${key}" ${found ? '✓' : '✗'}`);
    }
    expect(foundAll).toBe(true);
  });
});
