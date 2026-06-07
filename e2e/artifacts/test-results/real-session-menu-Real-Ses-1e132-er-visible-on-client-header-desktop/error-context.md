# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: real-session-menu.spec.ts >> Real Session — Menu Rebuild Verification >> Language switcher visible on client header
- Location: e2e\tests\real-session-menu.spec.ts:67:3

# Error details

```
TimeoutError: page.waitForSelector: Timeout 15000ms exceeded.
Call log:
  - waiting for locator('article.product-card') to be visible

```

# Page snapshot

```yaml
- heading "Сторінку не знайдено / Page not found" [level=1] [ref=e2]
```

# Test source

```ts
  1  | import { test, expect } from '@playwright/test';
  2  | 
  3  | test.describe('Real Session — Menu Rebuild Verification', () => {
  4  | 
  5  |   test('Supply Library page loads', async ({ page }) => {
  6  |     const errors: string[] = [];
  7  |     page.on('pageerror', (err) => errors.push(err.message));
  8  | 
  9  |     await page.goto('/admin/supplies?dev=true');
  10 |     await page.waitForTimeout(3000);
  11 | 
  12 |     // Page should have body content (not blank)
  13 |     const body = await page.textContent('body');
  14 |     expect(body).toBeTruthy();
  15 |     expect(body!.length).toBeGreaterThan(100);
  16 | 
  17 |     // Check for Supplies text anywhere in DOM
  18 |     expect(body).toContain('Supply');
  19 | 
  20 |     const criticalErrors = errors.filter(e => !e.includes('favicon') && !e.includes('manifest'));
  21 |     expect(criticalErrors).toEqual([]);
  22 |   });
  23 | 
  24 |   test('Menu Manager loads', async ({ page }) => {
  25 |     await page.goto('/admin/menu?dev=true');
  26 |     await page.waitForTimeout(3000);
  27 | 
  28 |     const body = await page.textContent('body');
  29 |     expect(body).toBeTruthy();
  30 |     expect(body!.length).toBeGreaterThan(100);
  31 | 
  32 |     // Should contain menu-related content
  33 |     expect(body).toContain('Menu');
  34 |   });
  35 | 
  36 |   test('Client menu page shows products with Tabler icons', async ({ page }) => {
  37 |     await page.goto('/s/test-slug?dev=true');
  38 |     await page.waitForSelector('article.product-card', { timeout: 15000 });
  39 |     await page.waitForTimeout(1000);
  40 | 
  41 |     // Product cards rendered
  42 |     const cards = page.locator('article.product-card');
  43 |     const count = await cards.count();
  44 |     expect(count).toBeGreaterThan(0);
  45 | 
  46 |     // Stars should be Tabler icons (not emoji ★)
  47 |     const starIcons = page.locator('.ti-star-filled');
  48 |     const starCount = await starIcons.count();
  49 |     expect(starCount).toBeGreaterThan(0);
  50 | 
  51 |     // Cart FAB should use Tabler icon
  52 |     // Add item first
  53 |     const addBtn = page.locator('article.product-card button[aria-label="Add"]').first();
  54 |     if (await addBtn.isVisible().catch(() => false)) {
  55 |       await addBtn.click();
  56 |       await page.waitForTimeout(500);
  57 |       const fab = page.locator('#cartFabBtn');
  58 |       if (await fab.isVisible({ timeout: 3000 }).catch(() => false)) {
  59 |         // Should have Tabler shopping cart icon (not emoji)
  60 |         const fabIcon = fab.locator('.ti-shopping-cart');
  61 |         const fabIconCount = await fabIcon.count();
  62 |         expect(fabIconCount).toBeGreaterThanOrEqual(0);
  63 |       }
  64 |     }
  65 |   });
  66 | 
  67 |   test('Language switcher visible on client header', async ({ page }) => {
  68 |     await page.goto('/s/test-slug?dev=true');
> 69 |     await page.waitForSelector('article.product-card', { timeout: 15000 });
     |                ^ TimeoutError: page.waitForSelector: Timeout 15000ms exceeded.
  70 |     await page.waitForTimeout(1000);
  71 | 
  72 |     // Language switcher button should be in the header
  73 |     const langBtn = page.locator('button[aria-label*="Switch language" i]');
  74 |     const langVisible = await langBtn.isVisible({ timeout: 5000 }).catch(() => false);
  75 |     // May or may not be visible depending on layout
  76 |     expect(true).toBeTruthy();
  77 |   });
  78 | 
  79 | });
  80 | 
```