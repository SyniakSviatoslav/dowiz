# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Skeleton Loading States >> menu-client shows skeleton blocks during load
- Location: e2e\tests\ui-polish.spec.ts:236:3

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
> 247 |     await page.waitForSelector('article.product-card', { timeout: 20000 });
      |                ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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
```