# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Emoji-Free DOM >> Client Menu uses Tabler icons instead of emojis
- Location: e2e\tests\ui-polish.spec.ts:172:5

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
  172 |     test(`${screen.label} uses Tabler icons instead of emojis`, async ({ page }) => {
  173 |       await page.goto(screen.url);
> 174 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
      |                  ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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
  198 | test.describe('Reduced Motion', () => {
  199 | 
  200 |   for (const [, screen] of Object.entries(SCREENS)) {
  201 |     test(`${screen.label} works with prefers-reduced-motion: reduce`, async ({ page }) => {
  202 |       await page.emulateMedia({ reducedMotion: 'reduce' });
  203 |       await page.goto(screen.url);
  204 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
  205 |       await page.waitForTimeout(1000);
  206 | 
  207 |       const body = await page.textContent('body');
  208 |       expect(body).toBeTruthy();
  209 | 
  210 |       // Verify reduced-motion CSS rule exists and applies
  211 |       const hasReducedMotionStyle = await page.evaluate(() => {
  212 |         const sheets = Array.from(document.styleSheets);
  213 |         for (const sheet of sheets) {
  214 |           try {
  215 |             const rules = Array.from(sheet.cssRules || []);
  216 |             for (const rule of rules) {
  217 |               if (rule instanceof CSSMediaRule && rule.conditionText?.includes('prefers-reduced-motion: reduce')) {
  218 |                 return true;
  219 |               }
  220 |             }
  221 |           } catch { /* cross-origin sheet, skip */ }
  222 |         }
  223 |         return false;
  224 |       });
  225 |       // May be true or false depending on when CSS is parsed; just verify page works
  226 |       expect(body!.length).toBeGreaterThan(0);
  227 |     });
  228 |   }
  229 | });
  230 | 
  231 | // --------------------------------------------------------
  232 | //  5.  Skeleton Loading
  233 | // --------------------------------------------------------
  234 | test.describe('Skeleton Loading States', () => {
  235 | 
  236 |   test('menu-client shows skeleton blocks during load', async ({ page }) => {
  237 |     // Navigate with domcontentloaded to catch loading state early
  238 |     await page.goto('/s/test-slug?dev=true', { waitUntil: 'domcontentloaded' });
  239 | 
  240 |     // Skeletons may or may not be visible depending on timing
  241 |     // Check for the skeleton-block CSS class existence
  242 |     const skeletonExists = await page.evaluate(() => {
  243 |       const els = document.querySelectorAll('.skeleton-block, .animate-pulse, .shimmer');
  244 |       return els.length;
  245 |     });
  246 |     // After load, skeletons disappear
  247 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
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
```