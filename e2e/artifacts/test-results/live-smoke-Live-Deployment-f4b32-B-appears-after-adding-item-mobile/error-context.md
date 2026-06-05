# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: live-smoke.spec.ts >> Live Deployment Smoke — demo tenant >> cart FAB appears after adding item
- Location: e2e\tests\live-smoke.spec.ts:71:3

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: "2"
Received: "0"
```

# Page snapshot

```yaml
- generic [ref=e1]:
  - banner [ref=e2]:
    - generic [ref=e3]: Demo Location
    - button "Cart" [ref=e4]: "0"
  - generic [ref=e6]:
    - heading "Demo Location" [level=1] [ref=e7]
    - paragraph
  - navigation [ref=e8]:
    - generic [ref=e9]:
      - button "Picat" [ref=e10]
      - button "Pizzas" [ref=e11]
      - button "Pizzas" [ref=e12]
  - main [ref=e13]:
    - generic [ref=e14]:
      - generic [ref=e15]:
        - heading "Picat" [level=2] [ref=e16]
        - generic [ref=e17]:
          - article [ref=e18]:
            - generic [ref=e19]:
              - heading "Margherita" [level=3] [ref=e20]
              - paragraph [ref=e21]: Salce domate, mocarela
              - generic [ref=e22]:
                - text: 1200 ALL
                - button "Add" [active] [ref=e23]
          - article [ref=e24]:
            - generic [ref=e25]:
              - heading "Pepperoni" [level=3] [ref=e26]
              - generic [ref=e27]:
                - text: 1500 ALL
                - button "Add" [ref=e28]
      - generic [ref=e29]:
        - heading "Pizzas" [level=2] [ref=e30]
        - generic [ref=e31]:
          - article [ref=e32]:
            - generic [ref=e33]:
              - heading "Margherita" [level=3] [ref=e34]
              - generic [ref=e35]:
                - text: 1200 ALL
                - button "Add" [ref=e36]
          - article [ref=e37]:
            - generic [ref=e38]:
              - heading "Pepperoni" [level=3] [ref=e39]
              - generic [ref=e40]:
                - text: 1500 ALL
                - button "Add" [ref=e41]
      - generic [ref=e42]:
        - heading "Pizzas" [level=2] [ref=e43]
        - generic [ref=e44]:
          - article [ref=e45]:
            - generic [ref=e46]:
              - heading "Margherita" [level=3] [ref=e47]
              - generic [ref=e48]:
                - text: 1200 ALL
                - button "Add" [ref=e49]
          - article [ref=e50]:
            - generic [ref=e51]:
              - heading "Pepperoni" [level=3] [ref=e52]
              - generic [ref=e53]:
                - text: 1500 ALL
                - button "Add" [ref=e54]
  - link "·Cart·0" [ref=e56] [cursor=pointer]:
    - /url: /s/demo/checkout
    - text: ·Cart·0
  - combobox [ref=e58]:
    - option "SQ" [selected]
    - option "EN"
```

# Test source

```ts
  1   | import { test, expect } from '@playwright/test';
  2   | 
  3   | test.describe('Live Deployment Smoke — demo tenant', () => {
  4   | 
  5   |   test('SSR menu renders with Albanian locale', async ({ page }) => {
  6   |     const errors: string[] = [];
  7   |     page.on('pageerror', (err) => errors.push(err.message));
  8   | 
  9   |     await page.goto('/s/demo');
  10  | 
  11  |     // Verify HTML lang is sq
  12  |     const lang = await page.locator('html').getAttribute('lang');
  13  |     expect(lang).toBe('sq');
  14  | 
  15  |     // Verify product cards render via SSR
  16  |     const cards = page.locator('article.product-card');
  17  |     await expect(cards.first()).toBeVisible({ timeout: 15000 });
  18  |     const count = await cards.count();
  19  |     expect(count).toBeGreaterThan(0);
  20  | 
  21  |     // Verify category nav renders
  22  |     const nav = page.locator('nav.sticky');
  23  |     await expect(nav).toBeVisible();
  24  | 
  25  |     // Verify Albanian text is present
  26  |     const firstCat = nav.locator('button').first();
  27  |     const catText = await firstCat.textContent();
  28  |     expect(catText).toBeTruthy();
  29  | 
  30  |     // Verify no JS errors
  31  |     const criticalErrors = errors.filter(e =>
  32  |       !e.includes('favicon') && !e.includes('manifest') && !e.includes('serviceWorker')
  33  |     );
  34  |     expect(criticalErrors).toEqual([]);
  35  |   });
  36  | 
  37  |   test('CSS variables applied correctly', async ({ page }) => {
  38  |     await page.goto('/s/demo');
  39  |     await page.waitForSelector('article.product-card', { timeout: 15000 });
  40  | 
  41  |     const primary = await page.evaluate(() =>
  42  |       getComputedStyle(document.documentElement).getPropertyValue('--brand-primary').trim()
  43  |     );
  44  |     expect(primary).toBeTruthy();
  45  |     expect(primary).not.toBe('');
  46  |   });
  47  | 
  48  |   test('i18n locale switcher works', async ({ page }) => {
  49  |     await page.goto('/s/demo');
  50  |     await page.waitForSelector('article.product-card', { timeout: 15000 });
  51  | 
  52  |     const select = page.locator('select');
  53  |     await expect(select).toBeVisible();
  54  | 
  55  |     const currentLang = await page.locator('html').getAttribute('lang');
  56  | 
  57  |     await select.selectOption('en');
  58  |     await page.waitForTimeout(1000);
  59  | 
  60  |     const htmlLang = await page.locator('html').getAttribute('lang');
  61  |     if (currentLang === 'sq' && htmlLang === 'sq') {
  62  |       console.log('Locale switch: html lang stayed sq after selecting en — checking DOM directly');
  63  |       const enEl = page.locator('[data-text-en]').first();
  64  |       await expect(enEl).toBeVisible();
  65  |       const enText = await enEl.textContent();
  66  |       console.log('First data-text-en element text:', enText);
  67  |     }
  68  |     expect(htmlLang).toBeTruthy();
  69  |   });
  70  | 
  71  |   test('cart FAB appears after adding item', async ({ page }) => {
  72  |     await page.goto('/s/demo');
  73  |     await page.evaluate(() => localStorage.clear());
  74  |     await page.reload();
  75  |     await page.waitForSelector('article.product-card', { timeout: 15000 });
  76  | 
  77  |     await page.waitForTimeout(2000);
  78  | 
  79  |     const addBtn = page.locator('article.product-card button[aria-label="Add"]').first();
  80  |     await addBtn.click();
  81  |     await addBtn.click();
  82  |     await page.waitForTimeout(1500);
  83  | 
  84  |     const headerCount = page.locator('#headerCartCount');
  85  |     const countText = await headerCount.textContent();
> 86  |     expect(countText).toBe('2');
      |                       ^ Error: expect(received).toBe(expected) // Object.is equality
  87  | 
  88  |     const fab = page.locator('#cartFabBtn');
  89  |     await expect(fab).toBeVisible({ timeout: 5000 });
  90  |   });
  91  | 
  92  |   test('embed mode hides fixed elements', async ({ page }) => {
  93  |     await page.goto('/s/demo?embed=1');
  94  |     await page.waitForSelector('article.product-card', { timeout: 15000 });
  95  | 
  96  |     // Body should have embed-mode class
  97  |     const bodyClass = await page.locator('body').getAttribute('class');
  98  |     expect(bodyClass).toContain('embed-mode');
  99  |   });
  100 | 
  101 |   test('no cookies set', async ({ page }) => {
  102 |     await page.goto('/s/demo');
  103 |     const cookies = await page.context().cookies();
  104 |     expect(cookies).toEqual([]);
  105 |   });
  106 | });
  107 | 
```