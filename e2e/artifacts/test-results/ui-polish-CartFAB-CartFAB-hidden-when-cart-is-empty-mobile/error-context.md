# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> CartFAB >> CartFAB hidden when cart is empty
- Location: e2e\tests\ui-polish.spec.ts:424:3

# Error details

```
TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
Call log:
  - waiting for locator('article.product-card') to be visible

```

# Page snapshot

```yaml
- heading "Сторінку не знайдено / Page not found" [level=1] [ref=e2]
```

# Test source

```ts
  326 |     // Product add buttons should have aria-label
  327 |     const addButtons = page.locator('button[aria-label="Add"]');
  328 |     const count = await addButtons.count();
  329 |     expect(count).toBeGreaterThanOrEqual(0);
  330 | 
  331 |     // Cart FAB has aria-label
  332 |     // Add an item first so FAB becomes visible
  333 |     const firstAddBtn = addButtons.first();
  334 |     if (await firstAddBtn.isVisible().catch(() => false)) {
  335 |       await firstAddBtn.click();
  336 |       await page.waitForTimeout(500);
  337 | 
  338 |       const fab = page.locator('#cartFabBtn');
  339 |       if (await fab.isVisible({ timeout: 3000 }).catch(() => false)) {
  340 |         const fabLabel = await fab.getAttribute('aria-label');
  341 |         expect(fabLabel).toBeTruthy();
  342 |       }
  343 |     }
  344 |   });
  345 | 
  346 |   test('admin dashboard navigation buttons have accessible names', async ({ page }) => {
  347 |     await page.goto('/admin?dev=true');
  348 |     await page.waitForSelector('h2, aside', { timeout: 20000 });
  349 |     await page.waitForTimeout(2000);
  350 | 
  351 |     // Check sidebar nav buttons exist and have accessible text
  352 |     const navButtons = page.locator('aside button, nav button');
  353 |     const count = await navButtons.count();
  354 | 
  355 |     let accessibleCount = 0;
  356 |     for (let i = 0; i < Math.min(count, 10); i++) {
  357 |       const btn = navButtons.nth(i);
  358 |       const ariaLabel = await btn.getAttribute('aria-label').catch(() => null);
  359 |       const title = await btn.getAttribute('title').catch(() => null);
  360 |       const text = await btn.textContent().catch(() => '');
  361 |       if (ariaLabel || title || (text && text.trim().length > 0)) {
  362 |         accessibleCount++;
  363 |       }
  364 |     }
  365 |     expect(count).toBeGreaterThanOrEqual(0);
  366 |   });
  367 | 
  368 |   test('search input has aria-label', async ({ page }) => {
  369 |     await page.goto('/admin?dev=true');
  370 |     await page.waitForSelector('h2', { timeout: 15000 });
  371 |     await page.waitForTimeout(2000);
  372 | 
  373 |     const searchInput = page.locator('input[aria-label*="search" i], input[aria-label*="Search" i]');
  374 |     const searchCount = await searchInput.count();
  375 |     // May or may not exist depending on page state
  376 |     expect(searchCount).toBeGreaterThanOrEqual(0);
  377 |   });
  378 | });
  379 | 
  380 | // --------------------------------------------------------
  381 | //  8.  Tabler Icons
  382 | // --------------------------------------------------------
  383 | test.describe('Tabler Icons', () => {
  384 | 
  385 |   for (const [key, screen] of Object.entries(SCREENS)) {
  386 |     test(`${screen.label} uses ti ti-* icon classes`, async ({ page }) => {
  387 |       await page.goto(screen.url);
  388 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  389 |       await page.waitForTimeout(2000);
  390 | 
  391 |       const icons = page.locator('i[class*="ti ti-"]');
  392 |       const count = await icons.count();
  393 | 
  394 |       // Analytics and dashboard sidebar should have Tabler icons
  395 |       // Client menu may have fewer
  396 |       console.log(`[${screen.label}] Tabler icon count: ${count}`);
  397 | 
  398 |       // At minimum, verify that if icons exist they use the ti ti-* pattern
  399 |       if (count > 0) {
  400 |         const firstIconClass = await icons.first().getAttribute('class');
  401 |         expect(firstIconClass).toContain('ti ti-');
  402 |       }
  403 |     });
  404 |   }
  405 | 
  406 |   test('admin dashboard sidebar uses Tabler icons for navigation', async ({ page }) => {
  407 |     await page.goto('/admin?dev=true');
  408 |     await page.waitForSelector('h2, aside', { timeout: 15000 });
  409 |     await page.waitForTimeout(2000);
  410 | 
  411 |     // Desktop sidebar: check aside > nav > button > i.ti
  412 |     const sidebarIcons = page.locator('aside i[class*="ti ti-"]');
  413 |     const count = await sidebarIcons.count();
  414 |     // Should have navigation icons
  415 |     console.log(`Admin sidebar Tabler icons: ${count}`);
  416 |   });
  417 | });
  418 | 
  419 | // --------------------------------------------------------
  420 | //  9.  CartFAB
  421 | // --------------------------------------------------------
  422 | test.describe('CartFAB', () => {
  423 | 
  424 |   test('CartFAB hidden when cart is empty', async ({ page }) => {
  425 |     await page.goto('/s/test-slug?dev=true');
> 426 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
      |                ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
  427 | 
  428 |     const fab = page.locator('#cartFabBtn');
  429 |     await expect(fab).not.toBeVisible({ timeout: 3000 });
  430 |   });
  431 | 
  432 |   test('CartFAB appears with count after adding item', async ({ page }) => {
  433 |     await page.goto('/s/test-slug?dev=true');
  434 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  435 | 
  436 |     const addBtn = page.locator('article.product-card button[aria-label="Add"]').first();
  437 |     await addBtn.click();
  438 |     await page.waitForTimeout(500);
  439 | 
  440 |     const fab = page.locator('#cartFabBtn');
  441 |     await expect(fab).toBeVisible({ timeout: 5000 });
  442 |     await expect(fab).toContainText('1');
  443 |   });
  444 | 
  445 |   test('CartFAB bounce animation class applied after add', async ({ page }) => {
  446 |     await page.goto('/s/test-slug?dev=true');
  447 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  448 | 
  449 |     // Add item
  450 |     await page.locator('article.product-card button[aria-label="Add"]').first().click();
  451 |     await page.waitForTimeout(200);
  452 | 
  453 |     const fab = page.locator('#cartFabBtn');
  454 |     await expect(fab).toBeVisible({ timeout: 5000 });
  455 | 
  456 |     // Check for bounce class within a short window after add
  457 |     const hasBounce = await fab.evaluate(el => el.classList.contains('cart-bounce'));
  458 |     // cart-bounce class may have already been removed (350ms animation)
  459 |     // Verify the CSS class exists in stylesheets
  460 |     const bounceClassExists = await page.evaluate(() => {
  461 |       const sheets = Array.from(document.styleSheets);
  462 |       for (const sheet of sheets) {
  463 |         try {
  464 |           const text = Array.from(sheet.cssRules || []).map(r => r.cssText).join('\n');
  465 |           if (text.includes('.cart-bounce')) return true;
  466 |         } catch { /* skip cross-origin */ }
  467 |       }
  468 |       return false;
  469 |     });
  470 |     // CSS rule must exist � the class is defined in index.css
  471 |     expect(bounceClassExists).toBeTruthy();
  472 |   });
  473 | 
  474 |   test('CartFAB count increments with multiple adds', async ({ page }) => {
  475 |     await page.goto('/s/test-slug?dev=true');
  476 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  477 | 
  478 |     const addButtons = page.locator('article.product-card button[aria-label="Add"]');
  479 |     const availableCount = await addButtons.count();
  480 | 
  481 |     if (availableCount >= 2) {
  482 |       await addButtons.first().click();
  483 |       await page.waitForTimeout(300);
  484 |       await addButtons.nth(1).click();
  485 |       await page.waitForTimeout(300);
  486 |       await addButtons.first().click();
  487 |       await page.waitForTimeout(300);
  488 | 
  489 |       const fab = page.locator('#cartFabBtn');
  490 |       await expect(fab).toContainText('3');
  491 |     }
  492 |   });
  493 | 
  494 |   test('CartFAB opens cart drawer on click', async ({ page }) => {
  495 |     await page.goto('/s/test-slug?dev=true');
  496 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  497 | 
  498 |     // Add an item
  499 |     await page.locator('article.product-card button[aria-label="Add"]').first().click();
  500 |     await page.waitForTimeout(500);
  501 | 
  502 |     const fab = page.locator('#cartFabBtn');
  503 |     await expect(fab).toBeVisible({ timeout: 3000 });
  504 |     await fab.click();
  505 | 
  506 |     // Cart drawer should open
  507 |     await expect(page.locator('text=Your Cart')).toBeVisible({ timeout: 5000 });
  508 |   });
  509 | });
  510 | 
  511 | // --------------------------------------------------------
  512 | //  10. Responsive
  513 | // --------------------------------------------------------
  514 | test.describe('Responsive Layout', () => {
  515 | 
  516 |   for (const [, screen] of Object.entries(SCREENS)) {
  517 |     test(`${screen.label} renders at mobile width (390px)`, async ({ page }) => {
  518 |       await page.setViewportSize(VIEWPORTS.mobile);
  519 |       await page.goto(screen.url);
  520 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  521 |       await page.waitForTimeout(1000);
  522 | 
  523 |       const body = await page.textContent('body');
  524 |       expect(body).toBeTruthy();
  525 |       expect(body!.length).toBeGreaterThan(0);
  526 | 
```