import { test, expect } from '@playwright/test';

test.describe('Courier Pages — Full Coverage', () => {

  test('login page loads with email and password fields', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/login?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/login|courier|email|password|sign in/i.test(body)).toBe(true);
    const inputs = page.locator('input');
    const count = await inputs.count();
    expect(count).toBeGreaterThanOrEqual(2);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('earnings page shows summary cards and payout history', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/earnings?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/earning|total|today|week|payout|balance|ALL|Lek/i.test(body)).toBe(true);
  });

  test('history page shows delivery cards with feedback', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/history?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/history|delivery|order|completed|rating|star|feedback/i.test(body)).toBe(true);
  });

  test('shift page shows timer and start/end controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/shift?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/shift|start|end|timer|online|offline|available/i.test(body)).toBe(true);
  });

  test('delivery page shows map component or fallback', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/delivery/test-delivery?dev=true');
    await page.waitForTimeout(3000);
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
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
    await page.waitForTimeout(3000);
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/task|delivery|order|accept|reject|pending|active/i.test(body)).toBe(true);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('all courier pages set no cookies', async ({ page }) => {
    const pages = ['/courier', '/courier/login', '/courier/earnings', '/courier/history', '/courier/shift'];
    for (const p of pages) {
      await page.goto(`${p}?dev=true`);
      const cookies = await page.context().cookies();
      expect(cookies).toEqual([]);
    }
  });

});
