# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Theme Switcher >> cycling themes on Analytics changes CSS variables
- Location: e2e\tests\ui-polish.spec.ts:69:5

# Error details

```
TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
Call log:
  - waiting for locator('h2') to be visible
    2 × locator resolved to 2 elements. Proceeding with the first one: <h2 class="text-lg font-bold">Dowiz</h2>
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
  1   | import { test, expect } from '@playwright/test';
  2   | 
  3   | const SCREENS = {
  4   |   'menu-client':     { url: '/s/test-slug?dev=true',      readySelector: 'article.product-card', label: 'Client Menu' },
  5   |   'admin-dashboard': { url: '/admin?dev=true',              readySelector: 'h2',                  label: 'Admin Dashboard' },
  6   |   'admin-orders':    { url: '/admin/orders?dev=true',       readySelector: 'h2',                  label: 'Admin Orders' },
  7   |   'analytics':       { url: '/admin/analytics?dev=true',    readySelector: 'h2',                  label: 'Analytics' },
  8   | } as const;
  9   | 
  10  | const THEME_CLASSES = [
  11  |   'theme-crimson-classic',
  12  |   'theme-ocean-fresh',
  13  |   'theme-midnight-urban',
  14  |   'theme-sage-garden',
  15  |   'theme-royal-gold',
  16  |   'theme-coral-breeze',
  17  | ];
  18  | 
  19  | const VIEWPORTS = {
  20  |   mobile:  { width: 390, height: 844 },
  21  |   tablet:  { width: 768, height: 1024 },
  22  |   desktop: { width: 1280, height: 800 },
  23  | };
  24  | 
  25  | // --- Helpers -------------------------------------------
  26  | 
  27  | async function getBrandCSSVars(page: import('@playwright/test').Page) {
  28  |   return page.evaluate(() => {
  29  |     const style = getComputedStyle(document.documentElement);
  30  |     return {
  31  |       primary: style.getPropertyValue('--brand-primary').trim(),
  32  |       bg:      style.getPropertyValue('--brand-bg').trim(),
  33  |       text:    style.getPropertyValue('--brand-text').trim(),
  34  |       surface: style.getPropertyValue('--brand-surface').trim(),
  35  |       border:  style.getPropertyValue('--brand-border').trim(),
  36  |     };
  37  |   });
  38  | }
  39  | 
  40  | async function setThemeClass(page: import('@playwright/test').Page, className: string) {
  41  |   await page.evaluate((cls) => {
  42  |     document.documentElement.className = cls;
  43  |   }, className);
  44  | }
  45  | 
  46  | async function getEmojiCount(page: import('@playwright/test').Page): Promise<string[]> {
  47  |   return page.evaluate(() => {
  48  |     const emojiRegex = /[\p{Emoji_Presentation}\p{Emoji}\u{200D}\u{FE0F}\u{FE0E}]/gu;
  49  |     const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  50  |     const found: string[] = [];
  51  |     let node: Text | null;
  52  |     while ((node = walker.nextNode() as Text | null)) {
  53  |       const text = node.textContent || '';
  54  |       let match: RegExpExecArray | null;
  55  |       while ((match = emojiRegex.exec(text)) !== null) {
  56  |         found.push(match[0]);
  57  |       }
  58  |     }
  59  |     return found;
  60  |   });
  61  | }
  62  | 
  63  | // --------------------------------------------------------
  64  | //  1.  Theme Switcher
  65  | // --------------------------------------------------------
  66  | test.describe('Theme Switcher', () => {
  67  | 
  68  |   for (const [, screen] of Object.entries(SCREENS)) {
  69  |     test(`cycling themes on ${screen.label} changes CSS variables`, async ({ page }) => {
  70  |       await page.goto(screen.url);
> 71  |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
      |                  ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
  72  |       await page.waitForTimeout(1000);
  73  | 
  74  |       const varsBefore = await getBrandCSSVars(page);
  75  |       expect(varsBefore.primary).toBeTruthy();
  76  |       expect(varsBefore.bg).toBeTruthy();
  77  | 
  78  |       // Apply each theme class and verify variables remain set
  79  |       for (const cls of THEME_CLASSES) {
  80  |         await setThemeClass(page, cls);
  81  |         await page.waitForTimeout(150);
  82  | 
  83  |         const vars = await getBrandCSSVars(page);
  84  |         expect(vars.primary).toBeTruthy();
  85  |         expect(vars.bg).toBeTruthy();
  86  |         expect(vars.text).toBeTruthy();
  87  |       }
  88  | 
  89  |       // Restore no class � page still renders
  90  |       await setThemeClass(page, '');
  91  |       await page.waitForTimeout(200);
  92  |       await expect(page.locator(screen.readySelector).first()).toBeVisible();
  93  |     });
  94  | 
  95  |     test(`${screen.label} renders without crash after rapid theme cycling`, async ({ page }) => {
  96  |       await page.goto(screen.url);
  97  |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  98  | 
  99  |       for (let i = 0; i < 10; i++) {
  100 |         await page.evaluate((idx) => {
  101 |           const palette = [
  102 |             { p: '#C1121F', bg: '#FFFFFF' },
  103 |             { p: '#0D9488', bg: '#FFFFFF' },
  104 |             { p: '#F97316', bg: '#0C0C0C' },
  105 |             { p: '#4D7C0F', bg: '#FAFAF5' },
  106 |             { p: '#B45309', bg: '#0A0A0A' },
  107 |             { p: '#DB2777', bg: '#FFFBFB' },
  108 |           ];
  109 |           const c = palette[idx % palette.length];
  110 |           document.documentElement.style.setProperty('--brand-primary', c.p);
  111 |           document.documentElement.style.setProperty('--brand-bg', c.bg);
  112 |         }, i);
  113 |         await page.waitForTimeout(50);
  114 |       }
  115 | 
  116 |       await expect(page.locator(screen.readySelector).first()).toBeVisible();
  117 |     });
  118 |   }
  119 | });
  120 | 
  121 | // --------------------------------------------------------
  122 | //  2.  Dark Mode
  123 | // --------------------------------------------------------
  124 | test.describe('Dark Mode', () => {
  125 | 
  126 |   for (const [, screen] of Object.entries(SCREENS)) {
  127 |     test(`${screen.label} renders correctly with prefers-color-scheme: dark`, async ({ page }) => {
  128 |       await page.emulateMedia({ colorScheme: 'dark' });
  129 |       await page.goto(screen.url);
  130 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  131 |       await page.waitForTimeout(1000);
  132 | 
  133 |       const vars = await getBrandCSSVars(page);
  134 |       expect(vars.primary).toBeTruthy();
  135 |       expect(vars.bg).toBeTruthy();
  136 | 
  137 |       const body = await page.textContent('body');
  138 |       expect(body).toBeTruthy();
  139 |       expect(body!.length).toBeGreaterThan(0);
  140 |     });
  141 | 
  142 |     test(`${screen.label} light-theme class gets dark-mode overrides`, async ({ page }) => {
  143 |       await page.goto(screen.url);
  144 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  145 | 
  146 |       // Apply a light theme class
  147 |       await setThemeClass(page, 'theme-crimson-classic');
  148 |       await page.waitForTimeout(300);
  149 | 
  150 |       // Under light scheme, crimson has white bg
  151 |       await page.emulateMedia({ colorScheme: 'light' });
  152 |       const lightVars = await getBrandCSSVars(page);
  153 | 
  154 |       // Under dark scheme, crimson should get dark overrides via @media query
  155 |       await page.emulateMedia({ colorScheme: 'dark' });
  156 |       await page.waitForTimeout(300);
  157 |       const darkVars = await getBrandCSSVars(page);
  158 | 
  159 |       expect(lightVars.primary).toBeTruthy();
  160 |       expect(darkVars.primary).toBeTruthy();
  161 |       // Both modes should still work; the override is proven by CSS cascade existing
  162 |     });
  163 |   }
  164 | });
  165 | 
  166 | // --------------------------------------------------------
  167 | //  3.  Emoji-Free
  168 | // --------------------------------------------------------
  169 | test.describe('Emoji-Free DOM', () => {
  170 | 
  171 |   for (const [key, screen] of Object.entries(SCREENS)) {
```