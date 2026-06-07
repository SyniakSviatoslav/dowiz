# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> CartFAB >> CartFAB opens cart drawer on click
- Location: e2e\tests\ui-polish.spec.ts:494:3

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
  426 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
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
> 496 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
      |                ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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
```