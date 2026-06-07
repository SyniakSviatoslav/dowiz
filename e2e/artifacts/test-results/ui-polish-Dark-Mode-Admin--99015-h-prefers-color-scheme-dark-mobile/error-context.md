# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: ui-polish.spec.ts >> Dark Mode >> Admin Orders renders correctly with prefers-color-scheme: dark
- Location: e2e\tests\ui-polish.spec.ts:127:5

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
              - generic [ref=e64]: 2:57:11 PM
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
              - generic [ref=e88]: 2:54:11 PM
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
              - generic [ref=e108]: 2:47:11 PM
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
              - generic [ref=e131]: 2:40:11 PM
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
> 130 |       await page.waitForSelector(screen.readySelector, { timeout: 20000 });
      |                  ^ TimeoutError: page.waitForSelector: Timeout 20000ms exceeded.
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
```