# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: onboarding-e2e.spec.ts >> E2E: Login → Onboarding → Reliability >> Step 3b: Logout protects admin pages
- Location: e2e\tests\onboarding-e2e.spec.ts:107:3

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: true
Received: false
```

# Page snapshot

```yaml
- generic [ref=e3]:
  - complementary [ref=e4]:
    - generic [ref=e5]:
      - generic [ref=e6]:
        - generic [ref=e7]: 🍱
        - heading "Dowiz" [level=2] [ref=e8]
      - button "" [ref=e9] [cursor=pointer]:
        - generic [ref=e10]: 
    - navigation [ref=e11]:
      - button " Paneli" [ref=e12] [cursor=pointer]:
        - generic [ref=e13]: 
        - generic [ref=e14]: Paneli
      - button " Porosite" [ref=e15] [cursor=pointer]:
        - generic [ref=e16]: 
        - generic [ref=e17]: Porosite
      - button " Menu" [ref=e18] [cursor=pointer]:
        - generic [ref=e19]: 
        - generic [ref=e20]: Menu
      - button " Furnizimet" [ref=e21] [cursor=pointer]:
        - generic [ref=e22]: 
        - generic [ref=e23]: Furnizimet
      - button " Postieret" [ref=e24] [cursor=pointer]:
        - generic [ref=e25]: 
        - generic [ref=e26]: Postieret
      - button " Analitika" [ref=e27] [cursor=pointer]:
        - generic [ref=e28]: 
        - generic [ref=e29]: Analitika
      - button " Klientet" [ref=e30] [cursor=pointer]:
        - generic [ref=e31]: 
        - generic [ref=e32]: Klientet
      - button " Brandingu" [ref=e33] [cursor=pointer]:
        - generic [ref=e34]: 
        - generic [ref=e35]: Brandingu
      - button " Cilesimet" [ref=e36] [cursor=pointer]:
        - generic [ref=e37]: 
        - generic [ref=e38]: Cilesimet
    - generic [ref=e39]:
      - generic [ref=e41]:
        - button "SQ" [ref=e42] [cursor=pointer]
        - button "EN" [ref=e43] [cursor=pointer]
        - button "UA" [ref=e44] [cursor=pointer]
      - button " Dil" [ref=e45] [cursor=pointer]:
        - generic [ref=e46]: 
        - text: Dil
  - text:  
  - main [ref=e47]:
    - generic [ref=e48]:
      - generic [ref=e49]:
        - generic [ref=e51]: 
        - generic [ref=e52]:
          - generic [ref=e53]: Welcome to your Dashboard
          - generic [ref=e54]: Here you can manage incoming orders, track couriers, and monitor your store readiness. Use the sidebar to navigate between sections.
        - button "Dismiss hint" [ref=e55] [cursor=pointer]:
          - generic [ref=e56]: 
      - generic [ref=e57]:
        - generic [ref=e58]:
          - generic [ref=e59]: "0"
          - generic [ref=e60]: Ne pritje
        - generic [ref=e61]:
          - generic [ref=e62]: "0"
          - generic [ref=e63]: Duke u pergatitur
        - generic [ref=e64]:
          - generic [ref=e65]: "0"
          - generic [ref=e66]: Gati
        - generic [ref=e67]:
          - generic [ref=e68]: "0"
          - generic [ref=e69]: Ne dorezim
        - generic [ref=e70]:
          - generic [ref=e71]: 0k
          - generic [ref=e72]: Totali
      - generic [ref=e73]:
        - generic [ref=e74]:
          - generic [ref=e75]:
            - heading "Porosite Live" [level=2] [ref=e76]
            - paragraph [ref=e77]: "0"
          - generic [ref=e78]:
            - button "Live" [ref=e79] [cursor=pointer]
            - button "Historiku" [ref=e80] [cursor=pointer]
        - generic [ref=e81]:
          - generic [ref=e82]:
            - generic [ref=e83]: 
            - textbox "Search orders by name or ID" [ref=e84]:
              - /placeholder: Kerko
          - button " Export CSV" [ref=e85] [cursor=pointer]:
            - generic [ref=e86]: 
            - text: Export CSV
      - generic [ref=e87]:
        - generic [ref=e88]:
          - button "Te gjitha" [pressed] [ref=e89] [cursor=pointer]
          - button "Ne pritje" [ref=e90] [cursor=pointer]
          - button "Konfirmuar" [ref=e91] [cursor=pointer]
          - button "Duke u pergatitur" [ref=e92] [cursor=pointer]
          - button "Gati" [ref=e93] [cursor=pointer]
          - button "Ne dorezim" [ref=e94] [cursor=pointer]
        - combobox [ref=e95]:
          - option "Newest first" [selected]
          - option "Oldest first"
          - option "Highest total"
      - generic [ref=e96]:
        - heading "Error" [level=3] [ref=e97]
        - paragraph [ref=e98]: Failed to load active orders
      - generic [ref=e99]:
        - generic [ref=e100]:
          - generic [ref=e101]: 
          - heading "Postieret Live" [level=3] [ref=e102]
        - generic [ref=e104]:
          - generic:
            - region "Map" [ref=e105]
            - button "Map marker" [ref=e106] [cursor=pointer]: AK
            - button "Map marker" [ref=e107] [cursor=pointer]: BH
          - generic:
            - generic [ref=e108]:
              - button "Zoom in" [ref=e109] [cursor=pointer]
              - button "Zoom out" [ref=e111] [cursor=pointer]
              - button "Drag to rotate map, click to reset north" [ref=e113]
            - group [ref=e115]:
              - generic "Toggle attribution" [ref=e116] [cursor=pointer]
              - generic [ref=e117]:
                - link "MapLibre" [ref=e118] [cursor=pointer]:
                  - /url: https://maplibre.org/
                - text: "|"
                - link "OpenFreeMap" [ref=e119] [cursor=pointer]:
                  - /url: https://openfreemap.org
                - link "© OpenMapTiles" [ref=e120] [cursor=pointer]:
                  - /url: https://www.openmaptiles.org/
                - text: Data from
                - link "OpenStreetMap" [ref=e121] [cursor=pointer]:
                  - /url: https://www.openstreetmap.org/copyright
      - generic [ref=e122]:
        - generic [ref=e123]:
          - generic [ref=e124]: 
          - heading "Gatishmeria e Dyqanit" [level=3] [ref=e125]
          - generic [ref=e126]: 5/8
        - generic [ref=e129]:
          - generic [ref=e130]:
            - generic [ref=e131]: 
            - generic [ref=e132]: Menu
            - generic [ref=e133]: 
          - generic [ref=e134]:
            - generic [ref=e135]: 
            - generic [ref=e136]: Numri i telefonit
            - generic [ref=e137]: 
          - generic [ref=e138]:
            - generic [ref=e139]: 
            - generic [ref=e140]: Adresa e dorezimit
            - generic [ref=e141]: 
          - generic [ref=e142]:
            - generic [ref=e143]: 
            - generic [ref=e144]: Postieret
            - generic [ref=e145]: 
          - generic [ref=e146]:
            - generic [ref=e147]: 
            - generic [ref=e148]: Brandingu
            - generic [ref=e149]: 
          - generic [ref=e150]:
            - generic [ref=e151]: 
            - generic [ref=e152]: Alergjenet
            - generic [ref=e153]: 
          - generic [ref=e154]:
            - generic [ref=e155]: 
            - generic [ref=e156]: Porosit
            - generic [ref=e157]: 
          - generic [ref=e158]:
            - generic [ref=e159]: 
            - generic [ref=e160]: Metoda e pageses
            - generic [ref=e161]: 
```

# Test source

```ts
  28  |     // Verify no cookies set
  29  |     const cookies = await page.context().cookies();
  30  |     const authCookies = cookies.filter(c => c.name.includes('token') || c.name.includes('session') || c.name.includes('auth'));
  31  |     expect(authCookies.length).toBe(0);
  32  | 
  33  |     console.log('LOGIN PASSED: Token in localStorage, no cookies');
  34  |   });
  35  | 
  36  |   test('Step 1b: After login, admin pages are accessible', async ({ page }) => {
  37  |     // Login first
  38  |     await page.goto(`${BASE}/login`);
  39  |     await page.fill('input[type="email"]', 'test@dowiz.com');
  40  |     await page.fill('input[type="password"]', 'test123456');
  41  |     await page.click('button[type="submit"]');
  42  |     await page.waitForURL('**/admin**', { timeout: 15000 });
  43  | 
  44  |     // Dashboard should load without errors
  45  |     const errors: string[] = [];
  46  |     page.on('pageerror', (err) => errors.push(err.message));
  47  |     await page.waitForTimeout(2000);
  48  |     
  49  |     const criticalErrors = errors.filter(e => !e.includes('favicon'));
  50  |     console.log('Page errors:', criticalErrors.length > 0 ? criticalErrors : 'none');
  51  |   });
  52  | 
  53  |   test('Step 2: Onboarding page loads', async ({ page }) => {
  54  |     // Login
  55  |     await page.goto(`${BASE}/login`);
  56  |     await page.fill('input[type="email"]', 'test@dowiz.com');
  57  |     await page.fill('input[type="password"]', 'test123456');
  58  |     await page.click('button[type="submit"]');
  59  |     await page.waitForURL('**/admin**', { timeout: 15000 });
  60  | 
  61  |     // Navigate to onboarding
  62  |     await page.goto(`${BASE}/admin/onboarding`);
  63  |     await page.waitForLoadState('networkidle');
  64  |     await page.waitForTimeout(2000);
  65  | 
  66  |     // Should show onboarding steps
  67  |     const body = await page.textContent('body') || '';
  68  |     const hasOnboarding = body.includes('Restaurant') || body.includes('step') || body.includes('Step');
  69  |     console.log('Onboarding visible:', hasOnboarding);
  70  |     
  71  |     // Take screenshot for evidence
  72  |     await page.screenshot({ path: 'e2e/artifacts/onboarding-loaded.png' });
  73  |   });
  74  | 
  75  |   test('Step 3: Clean re-login preserves state', async ({ browser }) => {
  76  |     // Create fresh context (simulates new browser session)
  77  |     const context = await browser.newContext({ storageState: undefined });
  78  |     const page = await context.newPage();
  79  | 
  80  |     // Login fresh
  81  |     await page.goto(`${BASE}/login`);
  82  |     await page.fill('input[type="email"]', 'test@dowiz.com');
  83  |     await page.fill('input[type="password"]', 'test123456');
  84  |     await page.click('button[type="submit"]');
  85  |     await page.waitForURL('**/admin**', { timeout: 15000 });
  86  |     await page.waitForTimeout(2000);
  87  | 
  88  |     // Check what page we land on (dashboard vs onboarding)
  89  |     const url = page.url();
  90  |     const body = await page.textContent('body') || '';
  91  |     const isDashboard = url.includes('/admin') && !url.includes('/onboarding');
  92  |     
  93  |     console.log('After clean re-login:');
  94  |     console.log('  URL:', url);
  95  |     console.log('  Is dashboard:', isDashboard);
  96  | 
  97  |     // Verify data is still there by checking API
  98  |     await page.goto(`${BASE}/admin/menu`, { timeout: 10000 });
  99  |     await page.waitForTimeout(2000);
  100 |     
  101 |     const menuBody = await page.textContent('body') || '';
  102 |     console.log('  Menu page has content:', menuBody.length > 200);
  103 | 
  104 |     await context.close();
  105 |   });
  106 | 
  107 |   test('Step 3b: Logout protects admin pages', async ({ page }) => {
  108 |     // Login first
  109 |     await page.goto(`${BASE}/login`);
  110 |     await page.fill('input[type="email"]', 'test@dowiz.com');
  111 |     await page.fill('input[type="password"]', 'test123456');
  112 |     await page.click('button[type="submit"]');
  113 |     await page.waitForURL('**/admin**', { timeout: 15000 });
  114 | 
  115 |     // Clear token
  116 |     await page.evaluate(() => {
  117 |       localStorage.removeItem('dos_access_token');
  118 |       sessionStorage.removeItem('dos_access_token');
  119 |     });
  120 | 
  121 |     // Try to access admin
  122 |     await page.goto(`${BASE}/admin`, { timeout: 10000 });
  123 |     await page.waitForTimeout(2000);
  124 | 
  125 |     const url = page.url();
  126 |     const redirectedToLogin = url.includes('/login');
  127 |     console.log('After logout, redirected to login:', redirectedToLogin);
> 128 |     expect(redirectedToLogin).toBe(true);
      |                               ^ Error: expect(received).toBe(expected) // Object.is equality
  129 |   });
  130 | });
  131 | 
```