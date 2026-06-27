import { test, expect } from '@playwright/test';

const ORDER_ID_404 = '00000000-0000-0000-0000-000000000001';

test.describe('Client Order Status — Live Deployment', () => {

  test('CR-5: order status page loads against live deployment', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    // Deterministic load proof: wait for the stepper to mount instead of a blind sleep.
    await page.waitForSelector('[data-testid=order-status-badge]', { timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('[data-testid=order-eta-headline]')).toBeVisible();
  });

  test('CR-5: status timeline renders progression steps', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForSelector('[data-testid=order-progress]', { timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Scope the count to the timeline container; `data-active` is unique to step cells
    // (the optional *-time labels lack it), so a stray match elsewhere can't inflate it.
    const statusSteps = page.locator('[data-testid=order-progress] [data-active]');
    const stepCount = await statusSteps.count();
    expect(stepCount).toBeGreaterThanOrEqual(4);
    expect(stepCount).toBeLessThanOrEqual(10);
  });

  test('order not found shows appropriate message', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto(`/s/test-slug/order/${ORDER_ID_404}?dev=true`);
    // The not-found / unavailable EmptyState always offers a back-to-menu CTA.
    await page.waitForSelector('[data-testid=order-back-to-menu]', { timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    // Assert the specific not-found/unavailable copy (en/sq) — a spinner or shell won't satisfy this.
    await expect(
      page.locator('text=/not found|nuk u gjet|no longer active|nuk është më aktive/i')
    ).toBeVisible({ timeout: 8000 });
  });

  test('no cookies are set on status page', async ({ page }) => {
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForSelector('[data-testid=order-status-badge]', { timeout: 15000 });
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('CR-5: order status shows estimated arrival time text', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForSelector('[data-testid=order-eta-headline]', { timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    await expect(page.locator('text=/Mbërritja|Estimated arrival|Очікуваний/i')).toBeVisible({ timeout: 8000 });
    const etaText = await page.locator('text=/\\d+\\s*min/').first().textContent();
    // Assert a plausible ETA value, not merely a non-empty string ('0 min' / placeholder must fail).
    const etaMinutes = parseInt(etaText ?? '', 10);
    expect(etaMinutes).toBeGreaterThan(0);
    expect(etaMinutes).toBeLessThan(180);
    // The arrival section must NOT advertise a stale/zero placeholder.
    await expect(page.locator('text=/^0\\s*min$/')).toHaveCount(0);
  });

  test('CR-6: share location button hidden before IN_DELIVERY', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForSelector('[data-testid=order-status-badge]', { timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const shareBtn = page.locator('text=Share my location with courier');
    await expect(shareBtn).toHaveCount(0);
  });

  test('CR-5: Order Details section renders items', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForSelector('[data-testid=order-status-badge]', { timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    // Ordinal text should contain number of items — matches sq/en/uk
    const body = await page.textContent('body');
    expect(body).toMatch(/1x|Dragon Roll|ALL/i);
  });

  test('CR-5: Total price renders', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForSelector('[data-testid=order-status-badge]', { timeout: 15000 });
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body).toMatch(/ALL/);
  });

  test('no JS errors on status page load', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug/order/o_mock_123?dev=true');
    await page.waitForSelector('[data-testid=order-status-badge]', { timeout: 15000 });
    expect(errors).toEqual([]);
  });
});
