# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Tabler Icons >> Analytics uses ti ti-* icon classes
- Location: e2e\tests\ui-polish.spec.ts:386:5

# Error details

```
TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
Call log:
  - waiting for locator('h2') to be visible
    2 × locator resolved to 2 elements. Proceeding with the first one: <h2 class="text-lg font-bold">Dowiz</h2>
    39 × locator resolved to 3 elements. Proceeding with the first one: <h2 class="text-lg font-bold">Dowiz</h2>

```

# Page snapshot

```yaml
- generic [ref=e3]:
  - text:           
  - generic [ref=e4]:
    - generic [ref=e5]:
      - generic [ref=e6]: 
      - heading "Dowiz" [level=2] [ref=e7]
    - button "" [ref=e8] [cursor=pointer]:
      - generic [ref=e9]: 
  - main [ref=e10]:
    - generic [ref=e11]:
      - generic [ref=e12]:
        - generic [ref=e13]:
          - heading "Analytics" [level=2] [ref=e14]
          - paragraph [ref=e15]: Performance overview for your restaurant
        - generic [ref=e16]:
          - button "7 days" [ref=e17] [cursor=pointer]
          - button "30 days" [ref=e18] [cursor=pointer]
      - generic [ref=e19]:
        - heading "Overview" [level=3] [ref=e20]
        - button " Export Stats CSV" [ref=e21] [cursor=pointer]:
          - generic [ref=e22]: 
          - text: Export Stats CSV
      - generic [ref=e23]:
        - generic [ref=e24]:
          - generic [ref=e25]:
            - generic [ref=e26]: Revenue
            - generic [ref=e27]: 
          - generic [ref=e28]: 874k ALL
          - generic [ref=e29]: +18% vs last period
        - generic [ref=e30]:
          - generic [ref=e31]:
            - generic [ref=e32]: Orders
            - generic [ref=e33]: 
          - generic [ref=e34]: "63"
          - generic [ref=e35]: +11 vs last period
        - generic [ref=e36]:
          - generic [ref=e37]:
            - generic [ref=e38]: Avg Order
            - generic [ref=e39]: 
          - generic [ref=e40]: 1387 ALL
          - generic [ref=e41]: +5% vs last period
        - generic [ref=e42]:
          - generic [ref=e43]:
            - generic [ref=e44]: Delivery
            - generic [ref=e45]: 
          - generic [ref=e46]: 32 min
          - generic [ref=e47]: "-8% vs last period"
      - generic [ref=e48]:
        - generic [ref=e49]:
          - heading "Revenue Trend" [level=3] [ref=e50]
          - generic [ref=e51]: "Total: 524,400 ALL"
        - generic [ref=e52]:
          - generic [ref=e53]:
            - generic [ref=e54]: 52k
            - generic [ref=e56]: Mon
          - generic [ref=e57]:
            - generic [ref=e58]: 61k
            - generic [ref=e60]: Tue
          - generic [ref=e61]:
            - generic [ref=e62]: 48k
            - generic [ref=e64]: Wed
          - generic [ref=e65]:
            - generic [ref=e66]: 71k
            - generic [ref=e68]: Thu
          - generic [ref=e69]:
            - generic [ref=e70]: 95k
            - generic [ref=e72]: Fri
          - generic [ref=e73]:
            - generic [ref=e74]: 110k
            - generic [ref=e76]: Sat
          - generic [ref=e77]:
            - generic [ref=e78]: 87k
            - generic [ref=e80]: Sun
      - generic [ref=e81]:
        - generic [ref=e82]:
          - generic [ref=e83]:
            - heading "Top Products" [level=3] [ref=e84]
            - button " Export CSV" [ref=e85] [cursor=pointer]:
              - generic [ref=e86]: 
              - text: Export CSV
          - generic [ref=e87]:
            - generic [ref=e88]:
              - generic [ref=e90]: 
              - generic [ref=e91]:
                - generic [ref=e93]: Dragon Roll
                - generic [ref=e97]: 28 orders
              - generic [ref=e98]: 23,800 ALL
            - generic [ref=e99]:
              - generic [ref=e100]: "#2"
              - generic [ref=e101]:
                - generic [ref=e103]: Salmon Sashimi
                - generic [ref=e107]: 22 orders
              - generic [ref=e108]: 14,960 ALL
        - generic [ref=e109]:
          - generic [ref=e110]:
            - heading "Ingredient Consumption (derived)" [level=3] [ref=e111]
            - button " Export" [ref=e112] [cursor=pointer]:
              - generic [ref=e113]: 
              - text: Export
          - generic [ref=e114]:
            - generic [ref=e115]:
              - generic [ref=e116]:
                - generic [ref=e117]: Salmon fillet
                - generic [ref=e118]: 12.5 kg
              - generic [ref=e121]: Reorder
            - generic [ref=e123]:
              - generic [ref=e124]: Sushi rice
              - generic [ref=e125]: 28 kg
            - generic [ref=e129]:
              - generic [ref=e130]: Nori sheets
              - generic [ref=e131]: 240 pcs
            - generic [ref=e135]:
              - generic [ref=e136]: Avocado
              - generic [ref=e137]: 35 pcs
            - generic [ref=e141]:
              - generic [ref=e142]: Cream cheese
              - generic [ref=e143]: 6.2 kg
            - generic [ref=e147]:
              - generic [ref=e148]: Spicy mayo
              - generic [ref=e149]: 4.5 L
            - generic [ref=e152]:
              - generic [ref=e153]:
                - generic [ref=e154]: Takeout boxes
                - generic [ref=e155]: 126 pcs
              - generic [ref=e158]: Reorder
            - generic [ref=e159]:
              - generic [ref=e160]:
                - generic [ref=e161]: Chopsticks
                - generic [ref=e162]: 252 pairs
              - generic [ref=e165]: Reorder
          - paragraph [ref=e166]: Based on today's orders x recipe quantities. Estimates only.
          - button " Copy Reorder List" [ref=e167] [cursor=pointer]:
            - generic [ref=e168]: 
            - text: Copy Reorder List
      - generic [ref=e169]:
        - generic [ref=e170]:
          - heading "Order Heatmap" [level=3] [ref=e171]
          - generic [ref=e172]:
            - generic [ref=e173]: Low
            - generic [ref=e175]: Peak
        - table [ref=e178]:
          - rowgroup [ref=e179]:
            - row "Day 0-3 4-7 8-11 12-15 16-19 20-23" [ref=e180]:
              - columnheader "Day" [ref=e181]
              - columnheader "0-3" [ref=e182]
              - columnheader "4-7" [ref=e183]
              - columnheader "8-11" [ref=e184]
              - columnheader "12-15" [ref=e185]
              - columnheader "16-19" [ref=e186]
              - columnheader "20-23" [ref=e187]
          - rowgroup [ref=e188]:
            - 'row "Mon Mon 0-3: 2 orders Mon 4-7: 1 orders Mon 8-11: 4 orders Mon 12-15: 8 orders Mon 16-19: 6 orders Mon 20-23: 3 orders" [ref=e189]':
              - cell "Mon" [ref=e190]
              - 'cell "Mon 0-3: 2 orders" [ref=e191]':
                - 'generic "Mon 0-3: 2 orders" [ref=e192]'
              - 'cell "Mon 4-7: 1 orders" [ref=e193]':
                - 'generic "Mon 4-7: 1 orders" [ref=e194]'
              - 'cell "Mon 8-11: 4 orders" [ref=e195]':
                - 'generic "Mon 8-11: 4 orders" [ref=e196]'
              - 'cell "Mon 12-15: 8 orders" [ref=e197]':
                - 'generic "Mon 12-15: 8 orders" [ref=e198]'
              - 'cell "Mon 16-19: 6 orders" [ref=e199]':
                - 'generic "Mon 16-19: 6 orders" [ref=e200]'
              - 'cell "Mon 20-23: 3 orders" [ref=e201]':
                - 'generic "Mon 20-23: 3 orders" [ref=e202]'
            - 'row "Tue Tue 0-3: 1 orders Tue 4-7: 2 orders Tue 8-11: 3 orders Tue 12-15: 7 orders Tue 16-19: 5 orders Tue 20-23: 4 orders" [ref=e203]':
              - cell "Tue" [ref=e204]
              - 'cell "Tue 0-3: 1 orders" [ref=e205]':
                - 'generic "Tue 0-3: 1 orders" [ref=e206]'
              - 'cell "Tue 4-7: 2 orders" [ref=e207]':
                - 'generic "Tue 4-7: 2 orders" [ref=e208]'
              - 'cell "Tue 8-11: 3 orders" [ref=e209]':
                - 'generic "Tue 8-11: 3 orders" [ref=e210]'
              - 'cell "Tue 12-15: 7 orders" [ref=e211]':
                - 'generic "Tue 12-15: 7 orders" [ref=e212]'
              - 'cell "Tue 16-19: 5 orders" [ref=e213]':
                - 'generic "Tue 16-19: 5 orders" [ref=e214]'
              - 'cell "Tue 20-23: 4 orders" [ref=e215]':
                - 'generic "Tue 20-23: 4 orders" [ref=e216]'
            - 'row "Wed Wed 0-3: 3 orders Wed 4-7: 2 orders Wed 8-11: 5 orders Wed 12-15: 9 orders Wed 16-19: 8 orders Wed 20-23: 2 orders" [ref=e217]':
              - cell "Wed" [ref=e218]
              - 'cell "Wed 0-3: 3 orders" [ref=e219]':
                - 'generic "Wed 0-3: 3 orders" [ref=e220]'
              - 'cell "Wed 4-7: 2 orders" [ref=e221]':
                - 'generic "Wed 4-7: 2 orders" [ref=e222]'
              - 'cell "Wed 8-11: 5 orders" [ref=e223]':
                - 'generic "Wed 8-11: 5 orders" [ref=e224]'
              - 'cell "Wed 12-15: 9 orders" [ref=e225]':
                - 'generic "Wed 12-15: 9 orders" [ref=e226]'
              - 'cell "Wed 16-19: 8 orders" [ref=e227]':
                - 'generic "Wed 16-19: 8 orders" [ref=e228]'
              - 'cell "Wed 20-23: 2 orders" [ref=e229]':
                - 'generic "Wed 20-23: 2 orders" [ref=e230]'
            - 'row "Thu Thu 0-3: 2 orders Thu 4-7: 3 orders Thu 8-11: 4 orders Thu 12-15: 6 orders Thu 16-19: 7 orders Thu 20-23: 5 orders" [ref=e231]':
              - cell "Thu" [ref=e232]
              - 'cell "Thu 0-3: 2 orders" [ref=e233]':
                - 'generic "Thu 0-3: 2 orders" [ref=e234]'
              - 'cell "Thu 4-7: 3 orders" [ref=e235]':
                - 'generic "Thu 4-7: 3 orders" [ref=e236]'
              - 'cell "Thu 8-11: 4 orders" [ref=e237]':
                - 'generic "Thu 8-11: 4 orders" [ref=e238]'
              - 'cell "Thu 12-15: 6 orders" [ref=e239]':
                - 'generic "Thu 12-15: 6 orders" [ref=e240]'
              - 'cell "Thu 16-19: 7 orders" [ref=e241]':
                - 'generic "Thu 16-19: 7 orders" [ref=e242]'
              - 'cell "Thu 20-23: 5 orders" [ref=e243]':
                - 'generic "Thu 20-23: 5 orders" [ref=e244]'
            - 'row "Fri Fri 0-3: 4 orders Fri 4-7: 3 orders Fri 8-11: 6 orders Fri 12-15: 10 orders Fri 16-19: 12 orders Fri 20-23: 8 orders" [ref=e245]':
              - cell "Fri" [ref=e246]
              - 'cell "Fri 0-3: 4 orders" [ref=e247]':
                - 'generic "Fri 0-3: 4 orders" [ref=e248]'
              - 'cell "Fri 4-7: 3 orders" [ref=e249]':
                - 'generic "Fri 4-7: 3 orders" [ref=e250]'
              - 'cell "Fri 8-11: 6 orders" [ref=e251]':
                - 'generic "Fri 8-11: 6 orders" [ref=e252]'
              - 'cell "Fri 12-15: 10 orders" [ref=e253]':
                - 'generic "Fri 12-15: 10 orders" [ref=e254]'
              - 'cell "Fri 16-19: 12 orders" [ref=e255]':
                - 'generic "Fri 16-19: 12 orders" [ref=e256]'
              - 'cell "Fri 20-23: 8 orders" [ref=e257]':
                - 'generic "Fri 20-23: 8 orders" [ref=e258]'
            - 'row "Sat Sat 0-3: 5 orders Sat 4-7: 4 orders Sat 8-11: 8 orders Sat 12-15: 14 orders Sat 16-19: 16 orders Sat 20-23: 10 orders" [ref=e259]':
              - cell "Sat" [ref=e260]
              - 'cell "Sat 0-3: 5 orders" [ref=e261]':
                - 'generic "Sat 0-3: 5 orders" [ref=e262]'
              - 'cell "Sat 4-7: 4 orders" [ref=e263]':
                - 'generic "Sat 4-7: 4 orders" [ref=e264]'
              - 'cell "Sat 8-11: 8 orders" [ref=e265]':
                - 'generic "Sat 8-11: 8 orders" [ref=e266]'
              - 'cell "Sat 12-15: 14 orders" [ref=e267]':
                - 'generic "Sat 12-15: 14 orders" [ref=e268]'
              - 'cell "Sat 16-19: 16 orders" [ref=e269]':
                - 'generic "Sat 16-19: 16 orders" [ref=e270]'
              - 'cell "Sat 20-23: 10 orders" [ref=e271]':
                - 'generic "Sat 20-23: 10 orders" [ref=e272]'
            - 'row "Sun Sun 0-3: 6 orders Sun 4-7: 5 orders Sun 8-11: 7 orders Sun 12-15: 12 orders Sun 16-19: 10 orders Sun 20-23: 6 orders" [ref=e273]':
              - cell "Sun" [ref=e274]
              - 'cell "Sun 0-3: 6 orders" [ref=e275]':
                - 'generic "Sun 0-3: 6 orders" [ref=e276]'
              - 'cell "Sun 4-7: 5 orders" [ref=e277]':
                - 'generic "Sun 4-7: 5 orders" [ref=e278]'
              - 'cell "Sun 8-11: 7 orders" [ref=e279]':
                - 'generic "Sun 8-11: 7 orders" [ref=e280]'
              - 'cell "Sun 12-15: 12 orders" [ref=e281]':
                - 'generic "Sun 12-15: 12 orders" [ref=e282]'
              - 'cell "Sun 16-19: 10 orders" [ref=e283]':
                - 'generic "Sun 16-19: 10 orders" [ref=e284]'
              - 'cell "Sun 20-23: 6 orders" [ref=e285]':
                - 'generic "Sun 20-23: 6 orders" [ref=e286]'
```

# Test source

```ts
  288 |     const orderCards = page.locator('.stagger-children > *');
  289 |     const hasContent = (await emptyStateEl.count()) > 0 || (await orderCards.count()) > 0;
  290 |     expect(hasContent).toBeTruthy();
  291 |   });
  292 | 
  293 |   test('empty cart drawer shows empty message', async ({ page }) => {
  294 |     await page.goto('/s/test-slug?dev=true');
  295 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  296 | 
  297 |     // Trigger cart with localStorage hack (add then immediately clear)
  298 |     await page.evaluate(() => {
  299 |       localStorage.setItem('dos_cart_test-slug', JSON.stringify({
  300 |         version: 1,
  301 |         items: [{ id: 'tmp', productId: 'p99', name: 'Test', quantity: 1, price: 100 }]
  302 |       }));
  303 |     });
  304 |     await page.reload();
  305 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  306 | 
  307 |     // Open FAB (should appear with 1 item from localStorage)
  308 |     const fab = page.locator('#cartFabBtn');
  309 |     if (await fab.isVisible({ timeout: 3000 }).catch(() => false)) {
  310 |       await fab.click();
  311 |       const drawer = page.locator('text=Your Cart');
  312 |       await expect(drawer).toBeVisible({ timeout: 5000 });
  313 |     }
  314 |   });
  315 | });
  316 | 
  317 | // --------------------------------------------------------
  318 | //  7.  Accessibility
  319 | // --------------------------------------------------------
  320 | test.describe('Accessibility', () => {
  321 | 
  322 |   test('interactive elements have accessible names on client menu', async ({ page }) => {
  323 |     await page.goto('/s/test-slug?dev=true');
  324 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
  325 | 
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
> 388 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
      |                  ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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
```