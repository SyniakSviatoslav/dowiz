# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Smoke � all key screens load without errors >> Analytics has CSS theme variables defined
- Location: e2e\tests\ui-polish.spec.ts:639:5

# Error details

```
TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
Call log:
  - waiting for locator('h2') to be visible
    38 × locator resolved to 3 elements. Proceeding with the first one: <h2 class="text-lg font-bold">Dowiz</h2>

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
  623 |       await page.waitForSelector(screen.readySelector, { timeout: 25000 });
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
> 641 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
      |                  ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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