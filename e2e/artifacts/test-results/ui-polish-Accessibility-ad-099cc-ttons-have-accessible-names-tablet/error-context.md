# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Accessibility >> admin dashboard navigation buttons have accessible names
- Location: e2e\tests\ui-polish.spec.ts:346:3

# Error details

```
TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
Call log:
  - waiting for locator('h2, aside') to be visible
    40 × locator resolved to 4 elements. Proceeding with the first one: <aside class="hidden lg:flex flex-col shrink-0 bg-[var(--brand-surface)] border-r border-[var(--brand-border)] sidebar-transition overflow-hidden w-56">…</aside>

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
              - generic [ref=e64]: 3:27:37 PM
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
              - generic [ref=e88]: 3:24:37 PM
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
              - generic [ref=e108]: 3:17:37 PM
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
              - generic [ref=e131]: 3:10:37 PM
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
  248 |     const skeletonsAfterLoad = await page.locator('.skeleton-block').count();
  249 |     console.log(`Skeletons during DOMContentLoaded: ${skeletonExists}, after load: ${skeletonsAfterLoad}`);
  250 |   });
  251 | 
  252 |   test('admin-dashboard shows shimmer skeletons during load', async ({ page }) => {
  253 |     await page.goto('/admin?dev=true', { waitUntil: 'domcontentloaded' });
  254 | 
  255 |     const shimmerCount = await page.locator('.shimmer').count();
  256 |     await page.waitForTimeout(3000);
  257 |     // After loading, content should appear (orders or empty state)
  258 |     const body = await page.textContent('body');
  259 |     expect(body).toBeTruthy();
  260 |   });
  261 | 
  262 |   test('analytics shows SkeletonBase during load', async ({ page }) => {
  263 |     await page.goto('/admin/analytics?dev=true', { waitUntil: 'domcontentloaded' });
  264 |     await page.waitForTimeout(500);
  265 | 
  266 |     const pulseCount = await page.locator('.animate-pulse').count();
  267 |     await page.waitForSelector('h2', { timeout: 15000 });
  268 |     const body = await page.textContent('body');
  269 |     expect(body).toBeTruthy();
  270 |   });
  271 | });
  272 | 
  273 | // --------------------------------------------------------
  274 | //  6.  Empty States
  275 | // --------------------------------------------------------
  276 | test.describe('Empty States', () => {
  277 | 
  278 |   test('empty order list shows EmptyState component', async ({ page }) => {
  279 |     await page.goto('/admin?dev=true');
  280 |     await page.waitForTimeout(4000);
  281 | 
  282 |     // Either the empty state OR order cards appear (depends on API)
  283 |     const body = await page.textContent('body');
  284 |     expect(body).toBeTruthy();
  285 | 
  286 |     // Check if EmptyState rendered the expected classes
  287 |     const emptyStateEl = page.locator('.border-dashed');
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
> 348 |     await page.waitForSelector('h2, aside', { timeout: 20000 });
      |                ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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
```