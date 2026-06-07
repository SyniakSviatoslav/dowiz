# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Responsive Layout >> Admin Dashboard renders at mobile width (390px)
- Location: e2e\tests\ui-polish.spec.ts:517:5

# Error details

```
TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
Call log:
  - waiting for locator('h2') to be visible
    40 × locator resolved to 3 elements. Proceeding with the first one: <h2 class="text-lg font-bold">Dowiz</h2>

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
        - generic [ref=e14]: 
        - generic [ref=e15]:
          - generic [ref=e16]: Welcome to your Dashboard
          - generic [ref=e17]: Here you can manage incoming orders, track couriers, and monitor your store readiness. Use the sidebar to navigate between sections.
        - button "Dismiss hint" [ref=e18] [cursor=pointer]:
          - generic [ref=e19]: 
      - generic [ref=e20]:
        - generic [ref=e21]:
          - generic [ref=e22]: "1"
          - generic [ref=e23]: Ne pritje
        - generic [ref=e24]:
          - generic [ref=e25]: "2"
          - generic [ref=e26]: Duke u pergatitur
        - generic [ref=e27]:
          - generic [ref=e28]: "0"
          - generic [ref=e29]: Gati
        - generic [ref=e30]:
          - generic [ref=e31]: "1"
          - generic [ref=e32]: Ne dorezim
        - generic [ref=e33]:
          - generic [ref=e34]: 6k
          - generic [ref=e35]: Totali
      - generic [ref=e36]:
        - generic [ref=e37]:
          - generic [ref=e38]:
            - heading "Porosite Live" [level=2] [ref=e39]
            - paragraph [ref=e40]: "4"
          - generic [ref=e41]:
            - button "Live" [ref=e42] [cursor=pointer]
            - button "Historiku" [ref=e43] [cursor=pointer]
        - generic [ref=e44]:
          - generic [ref=e45]:
            - generic [ref=e46]: 
            - textbox "Search orders by name or ID" [ref=e47]:
              - /placeholder: Kerko
          - button " Export CSV" [ref=e48] [cursor=pointer]:
            - generic [ref=e49]: 
            - text: Export CSV
      - generic [ref=e50]:
        - generic [ref=e51]:
          - button "Te gjitha" [pressed] [ref=e52] [cursor=pointer]
          - button "Ne pritje" [ref=e53] [cursor=pointer]
          - button "Konfirmuar" [ref=e54] [cursor=pointer]
          - button "Duke u pergatitur" [ref=e55] [cursor=pointer]
          - button "Gati" [ref=e56] [cursor=pointer]
          - button "Ne dorezim" [ref=e57] [cursor=pointer]
        - combobox [ref=e58]:
          - option "Newest first" [selected]
          - option "Oldest first"
          - option "Highest total"
      - generic [ref=e59]:
        - generic [ref=e60]:
          - generic [ref=e61]:
            - generic [ref=e62]:
              - generic [ref=e63]: "#O_1"
              - generic [ref=e64]: 3:08:56 PM
            - generic [ref=e65]: PENDING
          - generic [ref=e66]:
            - generic [ref=e67]: No OTP
            - generic [ref=e68]: "Rep: New"
          - generic [ref=e69]:
            - generic [ref=e70]:
              - generic [ref=e71]: "Client:"
              - text: Sara Mancini
            - generic [ref=e72]:
              - generic [ref=e73]: "Phone:"
              - text: +355 69 876 543
            - generic [ref=e74]:
              - generic [ref=e75]: "Items:"
              - text: 2 items (22 ALL)
            - generic [ref=e76]:
              - generic [ref=e78]: Dragon Roll ×2
              - generic [ref=e80]: Miso Soup ×1
          - generic [ref=e81]:
            - button "Accept & Prepare" [ref=e82] [cursor=pointer]
            - button "Reject" [ref=e83] [cursor=pointer]
        - generic [ref=e84]:
          - generic [ref=e85]:
            - generic [ref=e86]:
              - generic [ref=e87]: "#O_3"
              - generic [ref=e88]: 3:05:56 PM
            - generic [ref=e89]: CONFIRMED
          - generic [ref=e90]:
            - generic [ref=e91]: No OTP
            - generic [ref=e92]: "Rep: New"
          - generic [ref=e93]:
            - generic [ref=e94]:
              - generic [ref=e95]: "Client:"
              - text: Bled Gjoni
            - generic [ref=e96]:
              - generic [ref=e97]: "Phone:"
              - text: +355 69 321 654
            - generic [ref=e98]:
              - generic [ref=e99]: "Items:"
              - text: 1 items (15 ALL)
            - generic [ref=e102]: Sashimi Platter ×1
        - generic [ref=e104]:
          - generic [ref=e105]:
            - generic [ref=e106]:
              - generic [ref=e107]: "#O_2"
              - generic [ref=e108]: 2:58:56 PM
            - generic [ref=e109]: PREPARING
          - generic [ref=e110]:
            - generic [ref=e111]: No OTP
            - generic [ref=e112]: "Rep: New"
          - generic [ref=e113]:
            - generic [ref=e114]:
              - generic [ref=e115]: "Client:"
              - text: Alina Popa
            - generic [ref=e116]:
              - generic [ref=e117]: "Phone:"
              - text: +355 69 432 187
            - generic [ref=e118]:
              - generic [ref=e119]: "Items:"
              - text: 1 items (7 ALL)
            - generic [ref=e122]: Tonkotsu Ramen ×1
            - generic [ref=e123]:
              - generic [ref=e124]: "Courier:"
              - text: Ardit
          - button "Mark Ready" [ref=e126] [cursor=pointer]
        - generic [ref=e127]:
          - generic [ref=e128]:
            - generic [ref=e129]:
              - generic [ref=e130]: "#O_4"
              - generic [ref=e131]: 2:51:56 PM
            - generic [ref=e132]: IN_DELIVERY
          - generic [ref=e133]:
            - generic [ref=e134]: No OTP
            - generic [ref=e135]: "Rep: New"
          - generic [ref=e136]:
            - generic [ref=e137]:
              - generic [ref=e138]: "Client:"
              - text: Dorina Shehu
            - generic [ref=e139]:
              - generic [ref=e140]: "Phone:"
              - text: +355 69 111 999
            - generic [ref=e141]:
              - generic [ref=e142]: "Items:"
              - text: 1 items (15 ALL)
            - generic [ref=e145]: Philadelphia Roll ×2
            - generic [ref=e146]:
              - generic [ref=e147]: "Courier:"
              - text: Ardit
      - generic [ref=e149]:
        - generic [ref=e150]:
          - generic [ref=e151]: 
          - heading "Postieret Live" [level=3] [ref=e152]
        - generic [ref=e154]:
          - generic:
            - region "Map" [ref=e155]
            - button "Map marker" [ref=e156] [cursor=pointer]: AK
            - button "Map marker" [ref=e157] [cursor=pointer]: BH
          - generic:
            - generic [ref=e158]:
              - button "Zoom in" [ref=e159] [cursor=pointer]
              - button "Zoom out" [ref=e161] [cursor=pointer]
              - button "Drag to rotate map, click to reset north" [ref=e163]
            - group [ref=e165]:
              - generic "Toggle attribution" [ref=e166] [cursor=pointer]
              - generic [ref=e167]:
                - link "MapLibre" [ref=e168] [cursor=pointer]:
                  - /url: https://maplibre.org/
                - text: "|"
                - link "OpenFreeMap" [ref=e169] [cursor=pointer]:
                  - /url: https://openfreemap.org
                - link "© OpenMapTiles" [ref=e170] [cursor=pointer]:
                  - /url: https://www.openmaptiles.org/
                - text: Data from
                - link "OpenStreetMap" [ref=e171] [cursor=pointer]:
                  - /url: https://www.openstreetmap.org/copyright
      - generic [ref=e172]:
        - generic [ref=e173]:
          - generic [ref=e174]: 
          - heading "Gatishmeria e Dyqanit" [level=3] [ref=e175]
          - generic [ref=e176]: 5/8
        - generic [ref=e179]:
          - generic [ref=e180]:
            - generic [ref=e181]: 
            - generic [ref=e182]: Menu
            - generic [ref=e183]: 
          - generic [ref=e184]:
            - generic [ref=e185]: 
            - generic [ref=e186]: Numri i telefonit
            - generic [ref=e187]: 
          - generic [ref=e188]:
            - generic [ref=e189]: 
            - generic [ref=e190]: Adresa e dorezimit
            - generic [ref=e191]: 
          - generic [ref=e192]:
            - generic [ref=e193]: 
            - generic [ref=e194]: Postieret
            - generic [ref=e195]: 
          - generic [ref=e196]:
            - generic [ref=e197]: 
            - generic [ref=e198]: Brandingu
            - generic [ref=e199]: 
          - generic [ref=e200]:
            - generic [ref=e201]: 
            - generic [ref=e202]: Alergjenet
            - generic [ref=e203]: 
          - generic [ref=e204]:
            - generic [ref=e205]: 
            - generic [ref=e206]: Porosit
            - generic [ref=e207]: 
          - generic [ref=e208]:
            - generic [ref=e209]: 
            - generic [ref=e210]: Metoda e pageses
            - generic [ref=e211]: 
```

# Test source

```ts
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
> 520 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
      |                  ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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
```