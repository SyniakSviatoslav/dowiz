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
          - generic [ref=e22]: "0"
          - generic [ref=e23]: Ne pritje
        - generic [ref=e24]:
          - generic [ref=e25]: "0"
          - generic [ref=e26]: Duke u pergatitur
        - generic [ref=e27]:
          - generic [ref=e28]: "0"
          - generic [ref=e29]: Gati
        - generic [ref=e30]:
          - generic [ref=e31]: "0"
          - generic [ref=e32]: Ne dorezim
        - generic [ref=e33]:
          - generic [ref=e34]: 0k
          - generic [ref=e35]: Totali
      - generic [ref=e36]:
        - generic [ref=e37]:
          - generic [ref=e38]:
            - heading "Porosite Live" [level=2] [ref=e39]
            - paragraph [ref=e40]: "0"
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
        - heading "Error" [level=3] [ref=e60]
        - paragraph [ref=e61]: Failed to load active orders
      - generic [ref=e62]:
        - generic [ref=e63]:
          - generic [ref=e64]: 
          - heading "Postieret Live" [level=3] [ref=e65]
        - generic [ref=e67]:
          - generic:
            - region "Map" [ref=e68]
            - button "Map marker" [ref=e69] [cursor=pointer]: AK
            - button "Map marker" [ref=e70] [cursor=pointer]: BH
          - generic:
            - generic [ref=e71]:
              - button "Zoom in" [ref=e72] [cursor=pointer]
              - button "Zoom out" [ref=e74] [cursor=pointer]
              - button "Drag to rotate map, click to reset north" [ref=e76]
            - group [ref=e78]:
              - generic "Toggle attribution" [ref=e79] [cursor=pointer]
              - generic [ref=e80]:
                - link "MapLibre" [ref=e81] [cursor=pointer]:
                  - /url: https://maplibre.org/
                - text: "|"
                - link "OpenFreeMap" [ref=e82] [cursor=pointer]:
                  - /url: https://openfreemap.org
                - link "© OpenMapTiles" [ref=e83] [cursor=pointer]:
                  - /url: https://www.openmaptiles.org/
                - text: Data from
                - link "OpenStreetMap" [ref=e84] [cursor=pointer]:
                  - /url: https://www.openstreetmap.org/copyright
      - generic [ref=e85]:
        - generic [ref=e86]:
          - generic [ref=e87]: 
          - heading "Gatishmeria e Dyqanit" [level=3] [ref=e88]
          - generic [ref=e89]: 5/8
        - generic [ref=e92]:
          - generic [ref=e93]:
            - generic [ref=e94]: 
            - generic [ref=e95]: Menu
            - generic [ref=e96]: 
          - generic [ref=e97]:
            - generic [ref=e98]: 
            - generic [ref=e99]: Numri i telefonit
            - generic [ref=e100]: 
          - generic [ref=e101]:
            - generic [ref=e102]: 
            - generic [ref=e103]: Adresa e dorezimit
            - generic [ref=e104]: 
          - generic [ref=e105]:
            - generic [ref=e106]: 
            - generic [ref=e107]: Postieret
            - generic [ref=e108]: 
          - generic [ref=e109]:
            - generic [ref=e110]: 
            - generic [ref=e111]: Brandingu
            - generic [ref=e112]: 
          - generic [ref=e113]:
            - generic [ref=e114]: 
            - generic [ref=e115]: Alergjenet
            - generic [ref=e116]: 
          - generic [ref=e117]:
            - generic [ref=e118]: 
            - generic [ref=e119]: Porosit
            - generic [ref=e120]: 
          - generic [ref=e121]:
            - generic [ref=e122]: 
            - generic [ref=e123]: Metoda e pageses
            - generic [ref=e124]: 
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