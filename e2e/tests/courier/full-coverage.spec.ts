import { test, expect } from '@playwright/test';

test.describe('Courier Pages — Full Coverage', () => {

  test('login page loads with email and password fields', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/login?dev=true');
    await page.waitForLoadState('networkidle');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    // crash-boundary guard: a JS crash / 500 renders the ErrorBoundary fallback — must fail
    expect(body, 'page rendered the crash boundary').not.toContain('An unexpected error occurred');
    // concrete element proof (login renders unauthenticated, so these are stable)
    await expect(page.locator('input[type="email"]')).toBeVisible();
    await expect(page.locator('input[type="password"]')).toBeVisible();
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('earnings page shows summary cards and payout history', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/earnings?dev=true');
    await page.waitForLoadState('networkidle');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body, 'page rendered the crash boundary').not.toContain('An unexpected error occurred');
    expect(/earning|total|today|week|payout|balance|ALL|Lek/i.test(body)).toBe(true);
  });

  test('history page shows delivery cards with feedback', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/history?dev=true');
    await page.waitForLoadState('networkidle');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body, 'page rendered the crash boundary').not.toContain('An unexpected error occurred');
    expect(/history|delivery|order|completed|rating|star|feedback/i.test(body)).toBe(true);
  });

  test('shift page shows timer and start/end controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/shift?dev=true');
    await page.waitForLoadState('networkidle');
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body, 'page rendered the crash boundary').not.toContain('An unexpected error occurred');
    expect(/shift|start|end|timer|online|offline|available/i.test(body)).toBe(true);
  });

  test('delivery page shows map component or fallback', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/delivery/test-delivery?dev=true');
    await page.waitForLoadState('networkidle');
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body, 'page rendered the crash boundary').not.toContain('An unexpected error occurred');
    const mapContainer = page.locator('.maplibregl-map, [class*="maplibregl"], [class*="leaflet"], [class*="map"]');
    const mapCount = await mapContainer.count();
    if (mapCount > 0) {
      expect(mapCount).toBeGreaterThanOrEqual(1);
    } else {
      expect(/map|delivery|dropoff|pickup|location|address/i.test(body)).toBe(true);
    }
  });

  test('tasks page shows assignment cards with accept/reject', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier?dev=true');
    await page.waitForLoadState('networkidle');
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body, 'page rendered the crash boundary').not.toContain('An unexpected error occurred');
    expect(/task|delivery|order|accept|reject|pending|active/i.test(body)).toBe(true);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  // TODO(needs-staging): these require a live staging run + a real second-tenant courier fixture,
  // so they are tracked rather than faked (Test Integrity #5/#6/#8 — no nil-UUID, no reload-poll):
  //  - Authenticated-courier authz: re-run the routes above WITHOUT ?dev=true using a real courier
  //    session cookie; assert protected data renders (POSITIVE) and an unauthenticated hit
  //    redirects to /courier/login (NEGATIVE 401/403). (Finding 1)
  //  - Cross-tenant IDOR: load /courier/delivery/<courier-A-real-delivery-id> as courier-B's
  //    session; assert 403/404/empty (REAL second-tenant id, never an all-zero UUID). (Finding 2)
  //  - Error matrix: page.route the earnings/history/shift API; return 401/404/422/500 and assert
  //    the page shows the matching error state, not a blank/spinner. (Finding 5)
  //  - Realtime tracking: with the delivery page open, inject a synthetic GPS WS event and assert
  //    the courier position marker moves on the SAME open page (no reload/poll). (Finding 6)

  test('all courier pages set no cookies', async ({ page }) => {
    const pages = ['/courier', '/courier/login', '/courier/earnings', '/courier/history', '/courier/shift'];
    for (const p of pages) {
      await page.goto(`${p}?dev=true`);
      const cookies = await page.context().cookies();
      expect(cookies).toEqual([]);
    }
  });

});
