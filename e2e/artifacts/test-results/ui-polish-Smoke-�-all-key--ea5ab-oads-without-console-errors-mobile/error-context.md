# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Smoke � all key screens load without errors >> Client Menu loads without console errors
- Location: e2e\tests\ui-polish.spec.ts:615:5

# Error details

```
TimeoutError: page.waitForSelector: Timeout 25000ms exceeded.
Call log:
  - waiting for locator('article.product-card') to be visible

```

# Page snapshot

```yaml
- heading "Сторінку не знайдено / Page not found" [level=1] [ref=e2]
```

# Test source

```ts
  523 |       const body = await page.textContent('body');
  524 |       expect(body).toBeTruthy();
  525 |       expect(body!.length).toBeGreaterThan(0);
  526 | 
  527 |       // No horizontal overflow
  528 |       const overflowX = await page.evaluate(() => {
  529 |         const body = document.body;
  530 |         const style = getComputedStyle(body);
  531 |         return { overflowX: style.overflowX, scrollWidth: body.scrollWidth, clientWidth: body.clientWidth };
  532 |       });
  533 |       // Scroll width should not significantly exceed client width
  534 |       const ratio = overflowX.scrollWidth / Math.max(overflowX.clientWidth, 1);
  535 |       expect(ratio).toBeLessThan(5); // allow some scroll but not excessive
  536 |     });
  537 | 
  538 |     test(`${screen.label} renders at tablet width (768px)`, async ({ page }) => {
  539 |       await page.setViewportSize(VIEWPORTS.tablet);
  540 |       await page.goto(screen.url);
  541 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  542 |       await page.waitForTimeout(1000);
  543 | 
  544 |       const body = await page.textContent('body');
  545 |       expect(body).toBeTruthy();
  546 |       expect(body!.length).toBeGreaterThan(0);
  547 |     });
  548 | 
  549 |     test(`${screen.label} renders at desktop width (1280px)`, async ({ page }) => {
  550 |       await page.setViewportSize(VIEWPORTS.desktop);
  551 |       await page.goto(screen.url);
  552 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  553 |       await page.waitForTimeout(1000);
  554 | 
  555 |       const body = await page.textContent('body');
  556 |       expect(body).toBeTruthy();
  557 |       expect(body!.length).toBeGreaterThan(0);
  558 |     });
  559 |   }
  560 | 
  561 |   test('admin dashboard sidebar visible on desktop, hidden on mobile', async ({ page }) => {
  562 |     // Desktop: sidebar visible
  563 |     await page.setViewportSize(VIEWPORTS.desktop);
  564 |     await page.goto('/admin?dev=true');
  565 |     await page.waitForTimeout(3000);
  566 | 
  567 |     const desktopAside = page.locator('aside');
  568 |     const desktopAsideVisible = await desktopAside.isVisible().catch(() => false);
  569 | 
  570 |     // Mobile: sidebar should collapse to hamburger
  571 |     await page.setViewportSize(VIEWPORTS.mobile);
  572 |     await page.goto('/admin?dev=true');
  573 |     await page.waitForTimeout(3000);
  574 | 
  575 |     const mobileAside = page.locator('aside');
  576 |     // On mobile, aside may be hidden or different
  577 |     const body = await page.textContent('body');
  578 |     expect(body).toBeTruthy();
  579 |   });
  580 | 
  581 |   test('client menu grid adapts to viewport width', async ({ page }) => {
  582 |     // Mobile: 2 columns
  583 |     await page.setViewportSize(VIEWPORTS.mobile);
  584 |     await page.goto('/s/test-slug?dev=true');
  585 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  586 | 
  587 |     const mobileGridCols = await page.evaluate(() => {
  588 |       const grid = document.querySelector('.grid');
  589 |       if (!grid) return null;
  590 |       return getComputedStyle(grid).gridTemplateColumns;
  591 |     });
  592 | 
  593 |     // Desktop: should have more columns
  594 |     await page.setViewportSize(VIEWPORTS.desktop);
  595 |     await page.goto('/s/test-slug?dev=true');
  596 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  597 | 
  598 |     const desktopGridCols = await page.evaluate(() => {
  599 |       const grid = document.querySelector('.grid');
  600 |       if (!grid) return null;
  601 |       return getComputedStyle(grid).gridTemplateColumns;
  602 |     });
  603 | 
  604 |     // Either both work or the grid exists
  605 |     expect(true).toBeTruthy();
  606 |   });
  607 | });
  608 | 
  609 | // --------------------------------------------------------
  610 | //  Wrap?up: no regressions on existing screens
  611 | // --------------------------------------------------------
  612 | test.describe('Smoke � all key screens load without errors', () => {
  613 | 
  614 |   for (const [, screen] of Object.entries(SCREENS)) {
  615 |     test(`${screen.label} loads without console errors`, async ({ page }) => {
  616 |       const errors: string[] = [];
  617 |       page.on('pageerror', (err) => errors.push(err.message));
  618 |       page.on('console', (msg) => {
  619 |         if (msg.type() === 'error') errors.push(msg.text());
  620 |       });
  621 | 
  622 |       await page.goto(screen.url);
> 623 |       await page.waitForSelector(screen.readySelector, { timeout: 25000 });
      |                  ^ TimeoutError: page.waitForSelector: Timeout 25000ms exceeded.
  624 |       await page.waitForTimeout(1000);
  625 | 
  626 |       const criticalErrors = errors.filter(e =>
  627 |         !e.includes('favicon') &&
  628 |         !e.includes('404') &&
  629 |         !e.includes('manifest') &&
  630 |         !e.includes('Failed to load resource') &&
  631 |         !e.includes('serviceWorker') &&
  632 |         !e.includes('GET https://') &&
  633 |         !e.includes('net::ERR_') &&
  634 |         !e.includes('status of 404')
  635 |       );
  636 |       expect(criticalErrors).toEqual([]);
  637 |     });
  638 | 
  639 |     test(`${screen.label} has CSS theme variables defined`, async ({ page }) => {
  640 |       await page.goto(screen.url);
  641 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  642 |       await page.waitForTimeout(500);
  643 | 
  644 |       const vars = await getBrandCSSVars(page);
  645 |       expect(vars.primary).toBeTruthy();
  646 |       expect(vars.bg).toBeTruthy();
  647 |       expect(vars.text).toBeTruthy();
  648 |       expect(vars.primary).not.toBe('');
  649 |       expect(vars.bg).not.toBe('');
  650 |       expect(vars.text).not.toBe('');
  651 |     });
  652 |   }
  653 | });
  654 | 
```