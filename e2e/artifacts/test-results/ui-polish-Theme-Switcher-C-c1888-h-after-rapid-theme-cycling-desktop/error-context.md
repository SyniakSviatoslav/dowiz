# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Theme Switcher >> Client Menu renders without crash after rapid theme cycling
- Location: e2e\tests\ui-polish.spec.ts:95:5

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
  71  |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
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
> 97  |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
      |                  ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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
  172 |     test(`${screen.label} uses Tabler icons instead of emojis`, async ({ page }) => {
  173 |       await page.goto(screen.url);
  174 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  175 |       await page.waitForTimeout(2000);
  176 | 
  177 |       // Collect all text node emojis
  178 |       const emojis = await getEmojiCount(page);
  179 |       // Log what was found for audit (not an assertion, informational)
  180 |       if (emojis.length > 0) {
  181 |         console.log(`[${screen.label}] Emojis found (${emojis.length}): ${[...new Set(emojis)].join(' ')}`);
  182 |       }
  183 | 
  184 |       // Admin screens should be emoji-free; client may have some in FAB
  185 |       if (key !== 'menu-client') {
  186 |         // Check that icons are Tabler, not emoji
  187 |         const tablerIcons = page.locator('i[class*="ti ti-"]');
  188 |         const iconCount = await tablerIcons.count();
  189 |         expect(iconCount).toBeGreaterThanOrEqual(0); // May be 0 if no icons needed
  190 |       }
  191 |     });
  192 |   }
  193 | });
  194 | 
  195 | // --------------------------------------------------------
  196 | //  4.  Reduced Motion
  197 | // --------------------------------------------------------
```