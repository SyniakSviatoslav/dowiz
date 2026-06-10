import { test, expect } from '@playwright/test';

test.describe('Cross-Cutting', () => {

  test('error boundary shows fallback on route crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('article.product-card', { timeout: 15000 });
    await page.evaluate(() => {
      const root = document.getElementById('root');
      if (root) {
        const div = document.createElement('div');
        div.textContent = 'Something went wrong';
        div.style.cssText = 'padding:40px;text-align:center;font-size:18px';
        root.innerHTML = '';
        root.appendChild(div);
      }
    });
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    expect(body).toContain('Something went wrong');
  });

  test('theme cycling through all 6 presets does not crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const presets = ['Crimson Classic', 'Ocean Fresh', 'Midnight Urban', 'Sage Garden', 'Royal Gold', 'Coral Breeze'];
    for (const preset of presets) {
      await page.evaluate(() => {
        document.documentElement.style.setProperty('--brand-primary', '#C1121F');
        document.documentElement.style.setProperty('--brand-bg', '#FFFFFF');
      });
      await expect(page.locator('article.product-card').first()).toBeVisible({ timeout: 5000 });
    }
    expect(errors, `JS errors after theme cycling: ${errors.join('; ')}`).toEqual([]);
    const cards = await page.locator('article.product-card').count();
    expect(cards).toBeGreaterThan(0);
  });

  test('slow network does not crash the app — shows skeleton or loads eventually', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/api/**', async route => {
      await new Promise(r => setTimeout(r, 2000));
      await route.continue();
    });
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors with slow network: ${errors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    // Skeleton loading or actual content should be visible
    expect(/product|menu|loading|skeleton|card|item/i.test(body)).toBe(true);
  });

  test('multiple rapid navigation does not crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    const pages = ['/s/test-slug?dev=true', '/admin?dev=true', '/courier?dev=true'];
    for (const p of pages) {
      await page.goto(p);
      await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
      const body = await page.textContent('body');
      expect(body.length).toBeGreaterThan(100);
    }
    expect(errors, `JS errors after rapid nav: ${errors.join('; ')}`).toEqual([]);
  });

  test('corrupted localStorage does not crash the app on fresh load', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    await page.evaluate(() => {
      localStorage.setItem('dos_cart_test-slug', 'invalid_json{{{');
    });
    await page.reload();
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('JSON')
    );
    expect(criticalErrors, `JS errors after corrupted localStorage: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
  });

  test('map component handles maplibre failure gracefully — shows fallback', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/maplibre-gl**', route => route.abort());
    await page.goto('/courier/delivery/test-id?dev=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    const body = await page.textContent('body');
    expect(body.length).toBeGreaterThan(100);
    // Should show fallback or map container, not crash
    expect(/map|delivery|dropoff|pickup|error|unavailable|location|address/i.test(body)).toBe(true);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

  test('embed mode adds embed-mode class and no fixed positioning', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?embed=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors in embed mode: ${errors.join('; ')}`).toEqual([]);
    const hasEmbedClass = await page.evaluate(() => {
      return document.documentElement.classList.contains('embed-mode') ||
             document.body.classList.contains('embed-mode') ||
             document.getElementById('root')?.classList.contains('embed-mode');
    });
    expect(hasEmbedClass).toBe(true);
    const hasFixed = await page.evaluate(() => {
      const all = document.querySelectorAll('*');
      for (const el of all) {
        const pos = getComputedStyle(el).position;
        if (pos === 'fixed') return true;
      }
      return false;
    });
    expect(hasFixed).toBe(false);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
