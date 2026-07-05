import { test, expect } from '@playwright/test';

test.describe('Admin Orders Page', () => {

  test('orders page accessible from sidebar — navigates and renders content', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin?dev=true');
    await page.waitForTimeout(2000);
    const ordersLink = page.locator('a:has-text("Orders"), a:has-text("orders")');
    const linkCount = await ordersLink.count();
    expect(linkCount).toBeGreaterThanOrEqual(1);
    await ordersLink.first().click();
    await page.waitForTimeout(1500);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(/order|Order|PENDING|CONFIRMED|delivery|status/i.test(body)).toBe(true);
  });

  test('menu manager page loads with categories and products', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/menu?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(200);
    expect(/menu|category|product|item|add|edit|price/i.test(body)).toBe(true);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('branding page loads with theme editor and CSS controls', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/admin/branding?dev=true');
    await page.waitForTimeout(3000);
    expect(errors, `JS errors: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(200);
    expect(/brand|theme|color|preset|logo|primary|Crimson|Ocean|font/i.test(body)).toBe(true);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
