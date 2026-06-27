import { test, expect } from '@playwright/test';

test.describe('Courier Tasks', () => {

  test('tasks page loads without JS errors', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    await page.goto('/courier?dev=true');
    // Deterministic wait + render proof (replaces waitForTimeout + body.length>100 + loose
    // body-text regex): the Tasks shell <h1> must be visible. A 500 splash / redirect / blank
    // error page has no courier heading, so this goes red when the page is actually broken.
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest')
    );
    expect(criticalErrors).toEqual([]);
  });

  test('tasks page shows delivery assignments or empty state', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    await page.goto('/courier?dev=true');
    // Deterministic shell proof (replaces waitForTimeout + body.length>100 + the no-op
    // `if(count>0){expect(count).toBeGreaterThanOrEqual(1)}` tautology that skipped itself
    // whenever count was 0): the Tasks heading must render for either the list or empty state.
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // TODO(needs-staging): seed a known active courier assignment for this courier, then assert an
    // EXACT task-card count (>=1) instead of the unconditional shell — requires a live seeded
    // courier + assignment on staging (the `?dev=true` mock has no list fixture).
  });

  test('no cookies on courier pages', async ({ page }) => {
    await page.goto('/courier?dev=true');
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('delivery page loads with map and dropoff info', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/delivery/test-id?dev=true');
    // Deterministic wait + render proof (replaces waitForTimeout + body.length + loose regex
    // whose words also appear in nav/labels): the loaded delivery renders its cash-collection
    // card. A 500/redirect/empty page has no [data-testid=task-cash-amount] → test goes red.
    await expect(page.locator('[data-testid=task-cash-amount]')).toBeVisible();
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
  });

  // CR-5: Delivery page shows ETA to destination
  test('delivery page shows estimated arrival time', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/delivery/test-id?dev=true');
    await page.waitForTimeout(3000);
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(/min|eta|to destination|arrival|time/i.test(body)).toBe(true);
  });

  // CR-1: Delivery page shows customer instructions
  test('delivery page shows customer dropoff instructions', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/courier/delivery/test-id?dev=true');
    await page.waitForTimeout(3000);
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    // Mock data has instructions: "Call when near"
    expect(/Call when near|instructions|note/i.test(body)).toBe(true);
  });

});
