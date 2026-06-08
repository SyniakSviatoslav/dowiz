# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: api-real.spec.ts >> Real API — Idempotency & Order Flow >> POST /api/orders creates order with valid data
- Location: e2e\tests\api-real.spec.ts:133:3

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: 201
Received: 429
```

# Test source

```ts
  50  |   });
  51  | 
  52  |   test('GET /public/locations/:slug/menu returns JSON', async ({ request }) => {
  53  |     const resp = await request.get(`${BASE}/public/locations/demo/menu`);
  54  |     expect(resp.status()).toBe(200);
  55  |     const body = await resp.json();
  56  |     expect(body.default_locale).toBe('sq');
  57  |     expect(body.supported_locales).toContain('sq');
  58  |     expect(body.categories.length).toBeGreaterThan(0);
  59  |   });
  60  | 
  61  |   test('GET /public/locations/:id/theme.css returns CSS', async ({ request }) => {
  62  |     const resp = await request.get(`${BASE}/public/locations/demo/theme.css`);
  63  |     expect(resp.status()).toBe(200);
  64  |     const css = await resp.text();
  65  |     expect(css).toContain('--brand-primary');
  66  |   });
  67  | 
  68  |   test('GET /s/:slug/manifest.webmanifest returns manifest', async ({ request }) => {
  69  |     const resp = await request.get(`${BASE}/s/demo/manifest.webmanifest`);
  70  |     expect(resp.status()).toBe(200);
  71  |     const body = await resp.json();
  72  |     expect(body.name).toBe('demo');
  73  |     expect(body.icons.length).toBeGreaterThan(0);
  74  |   });
  75  | 
  76  |   test('GET /api/push/vapid-public-key returns key', async ({ request }) => {
  77  |     const resp = await request.get(`${BASE}/api/push/vapid-public-key`);
  78  |     expect(resp.status()).toBe(200);
  79  |     const body = await resp.json();
  80  |     expect(body.publicKey).toBeTruthy();
  81  |   });
  82  | 
  83  |   test('POST /api/telemetry accepts events', async ({ request }) => {
  84  |     const resp = await request.post(`${BASE}/api/telemetry`, {
  85  |       data: { action: 'cart.added', locationId: 'test' }
  86  |     });
  87  |     expect(resp.status()).toBe(202);
  88  |   });
  89  | 
  90  |   test('GET /auth/google redirects to Google OAuth', async ({ request }) => {
  91  |     const resp = await request.get(`${BASE}/auth/google`, { maxRedirects: 0 });
  92  |     expect(resp.status()).toBe(302);
  93  |     const location = resp.headers()['location'];
  94  |     expect(location).toContain('accounts.google.com');
  95  |   });
  96  | 
  97  |   test('GET /robots.txt returns robots', async ({ request }) => {
  98  |     const resp = await request.get(`${BASE}/robots.txt`);
  99  |     expect(resp.status()).toBe(200);
  100 |     const text = await resp.text();
  101 |     expect(text).toContain('User-agent');
  102 |   });
  103 | });
  104 | 
  105 | test.describe('Real API — Auth-Required Endpoints', () => {
  106 | 
  107 |   test('GET /api/owner/locations/:id/dashboard/snapshot requires auth', async ({ request }) => {
  108 |     const resp = await request.get(`${BASE}/api/owner/locations/11111111-1111-1111-1111-111111111111/dashboard/snapshot`);
  109 |     expect(resp.status()).toBe(401);
  110 |   });
  111 | 
  112 |   test('GET /api/courier/me requires auth', async ({ request }) => {
  113 |     const resp = await request.get(`${BASE}/api/courier/me`);
  114 |     expect(resp.status()).toBe(401);
  115 |   });
  116 | 
  117 |   test('GET /api/owner/locations/:id/signals requires auth', async ({ request }) => {
  118 |     const resp = await request.get(`${BASE}/api/owner/locations/11111111-1111-1111-1111-111111111111/signals`);
  119 |     expect(resp.status()).toBe(401);
  120 |   });
  121 | });
  122 | 
  123 | test.describe('Real API — Idempotency & Order Flow', () => {
  124 | 
  125 |   test('POST /api/orders validates input schema', async ({ request }) => {
  126 |     const resp = await request.post(`${BASE}/api/orders`, {
  127 |       data: { invalid: true },
  128 |       headers: { 'Content-Type': 'application/json' }
  129 |     });
  130 |     expect(resp.status()).toBeGreaterThanOrEqual(400);
  131 |   });
  132 | 
  133 |   test('POST /api/orders creates order with valid data', async ({ request }) => {
  134 |     const idemKey = uuid();
  135 |     const validOrder = {
  136 |       locationId: '1f609add-062a-4bb5-89bf-d695f963ede6',
  137 |       type: 'delivery',
  138 |       items: [{ product_id: '1b4e1275-3f37-47e5-8652-1ebd6c8de04a', quantity: 1 }],
  139 |       customer: { phone: '+355600000001', name: 'Test' },
  140 |       delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test Street' },
  141 |       payment: { method: 'cash' },
  142 |       idempotency_key: idemKey,
  143 |     };
  144 | 
  145 |     const resp = await request.post(`${BASE}/api/orders`, {
  146 |       data: validOrder,
  147 |       headers: { 'Content-Type': 'application/json' }
  148 |     });
  149 |     const body = await resp.json();
> 150 |     expect(resp.status()).toBe(201);
      |                           ^ Error: expect(received).toBe(expected) // Object.is equality
  151 |     expect(body.id).toBeDefined();
  152 |     expect(body.status).toBe('PENDING');
  153 |   });
  154 | 
  155 |   test('POST /api/orders rejects duplicate idempotency key', async ({ request }) => {
  156 |     const idemKey = uuid();
  157 |     const validOrder = {
  158 |       locationId: '1f609add-062a-4bb5-89bf-d695f963ede6',
  159 |       type: 'delivery',
  160 |       items: [{ product_id: '1b4e1275-3f37-47e5-8652-1ebd6c8de04a', quantity: 1 }],
  161 |       customer: { phone: '+355600000002', name: 'Test2' },
  162 |       delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test Street 2' },
  163 |       payment: { method: 'cash' },
  164 |       idempotency_key: idemKey,
  165 |     };
  166 | 
  167 |     const resp1 = await request.post(`${BASE}/api/orders`, {
  168 |       data: validOrder,
  169 |       headers: { 'Content-Type': 'application/json' }
  170 |     });
  171 |     expect(resp1.status()).toBe(201);
  172 | 
  173 |     const resp2 = await request.post(`${BASE}/api/orders`, {
  174 |       data: validOrder,
  175 |       headers: { 'Content-Type': 'application/json' }
  176 |     });
  177 |     expect(resp2.status()).toBe(200);
  178 |     const body2 = await resp2.json();
  179 |     const body1 = await resp1.json();
  180 |     expect(body2.id).toBe(body1.id);
  181 |   });
  182 | });
  183 | 
  184 | test.describe('Real API — Security', () => {
  185 | 
  186 |   test('No cookies set on any public endpoint', async ({ request }) => {
  187 |     const endpoints = ['/s/demo', '/health', '/public/locations/demo/menu'];
  188 |     for (const ep of endpoints) {
  189 |       const resp = await request.get(`${BASE}${ep}`);
  190 |       const cookies = resp.headers()['set-cookie'];
  191 |       expect(cookies, `${ep} set cookies`).toBeUndefined();
  192 |     }
  193 |   });
  194 | 
  195 |   test('Cross-tenant access rejected', async ({ request }) => {
  196 |     const wrongLocationResp = await request.get(
  197 |       `${BASE}/api/owner/locations/00000000-0000-0000-0000-000000000001/dashboard/snapshot`
  198 |     );
  199 |     expect(wrongLocationResp.status()).toBe(401);
  200 |   });
  201 | 
  202 |   test('CSP present on SSR pages', async ({ request }) => {
  203 |     const resp = await request.get(`${BASE}/s/demo`);
  204 |     expect(resp.headers()['content-security-policy']).toBeDefined();
  205 |   });
  206 | 
  207 |   test('Rate limit headers present', async ({ request }) => {
  208 |     const resp = await request.get(`${BASE}/s/demo`);
  209 |     expect(resp.headers()['x-ratelimit-limit']).toBeDefined();
  210 |     expect(resp.headers()['x-ratelimit-remaining']).toBeDefined();
  211 |   });
  212 | });
  213 | 
  214 | test.describe('Real API — Caching', () => {
  215 | 
  216 |   test('menu_version header present on SSR', async ({ request }) => {
  217 |     const resp = await request.get(`${BASE}/s/demo`);
  218 |     expect(resp.headers()['x-menu-version']).toBeDefined();
  219 |   });
  220 | 
  221 |   test('Cache-Control set on public menu', async ({ request }) => {
  222 |     const resp = await request.get(`${BASE}/public/locations/demo/menu`);
  223 |     expect(resp.headers()['cache-control']).toContain('max-age');
  224 |   });
  225 | });
  226 | 
```