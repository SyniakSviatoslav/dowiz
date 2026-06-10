# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: deploy-validation.spec.ts >> Deploy Validation — Live Session Proofs >> 6.2 — image upload with auth returns 200 (STRICT: 500 is a failure)
- Location: e2e\tests\deploy-validation.spec.ts:189:3

# Error details

```
Error: expect(received).toBe(expected) // Object.is equality

Expected: 200
Received: 500
```

# Test source

```ts
  97  |   });
  98  | 
  99  |   test('4.2 — create product with taste + recipeLines via owner API', async ({ request }) => {
  100 |     const res = await request.post(`${BASE}/api/owner/menu/products`, {
  101 |       data: {
  102 |         name: 'Pita Test Sushi',
  103 |         price: 950,
  104 |         description: 'RecipeLines validation test',
  105 |         available: true,
  106 |         categoryId: createdCategoryId,
  107 |         taste: { spicy: 2, salty: 1, sour: 0, sweet: 0, richness: 3 },
  108 |         recipeLines: [
  109 |           { supplyId: 's-rice', supplyName: 'Sushi Rice', qty: 100, unit: 'g', kind: 'food_ingredient', kcal: 130, proteinG: 3, fatG: 0, carbsG: 28, allergens: [] },
  110 |           { supplyId: 's-wasabi', supplyName: 'Wasabi', qty: 5, unit: 'g', kind: 'condiment', kcal: 15, proteinG: 0, fatG: 0, carbsG: 3, allergens: [] },
  111 |         ],
  112 |         stockCount: 42,
  113 |       },
  114 |       headers: { Authorization: `Bearer ${authToken}` },
  115 |     });
  116 |     expect(res.status()).toBe(201);
  117 |     const body = await res.json();
  118 |     expect(body.id).toBeTruthy();
  119 |     expect(body.name).toBe('Pita Test Sushi');
  120 |     expect(body.taste).toBeTruthy();
  121 |     expect(body.taste.spicy).toBe(2);
  122 |     expect(body.recipeLines).toBeTruthy();
  123 |     expect(body.recipeLines.length).toBe(2);
  124 |     expect(body.recipeLines[0].supplyName).toBe('Sushi Rice');
  125 |     expect(body.stockCount).toBe(42);
  126 |     createdProductId = body.id;
  127 |   });
  128 | 
  129 |   test('4.3 — PATCH product preserves recipeLines on edit', async ({ request }) => {
  130 |     const res = await request.patch(`${BASE}/api/owner/menu/products/${createdProductId}`, {
  131 |       data: {
  132 |         name: 'Pita Test Sushi Updated',
  133 |         price: 1050,
  134 |         taste: { spicy: 3, salty: 2 },
  135 |         recipeLines: [
  136 |           { supplyId: 's-rice', supplyName: 'Sushi Rice', qty: 150, unit: 'g', kind: 'food_ingredient', kcal: 195, proteinG: 4, fatG: 0, carbsG: 42, allergens: [] },
  137 |           { supplyId: 's-nori', supplyName: 'Nori Sheets', qty: 2, unit: 'unit', kind: 'food_ingredient', kcal: 10, proteinG: 1, fatG: 0, carbsG: 1, allergens: [] },
  138 |           { supplyId: 's-soy', supplyName: 'Soy Sauce', qty: 15, unit: 'ml', kind: 'condiment', kcal: 8, proteinG: 1, fatG: 0, carbsG: 1, allergens: ['soy'] },
  139 |         ],
  140 |       },
  141 |       headers: { Authorization: `Bearer ${authToken}` },
  142 |     });
  143 |     expect(res.status()).toBe(200);
  144 |     const body = await res.json();
  145 |     expect(body.name).toBe('Pita Test Sushi Updated');
  146 |     expect(body.price).toBe(1050);
  147 |     expect(body.taste.spicy).toBe(3);
  148 |     expect(body.recipeLines.length).toBe(3);
  149 |     expect(body.recipeLines[2].supplyName).toBe('Soy Sauce');
  150 |     expect(body.recipeLines[2].allergens).toContain('soy');
  151 |   });
  152 | 
  153 |   // ── 5. Public menu API: attributes shape contract ───────────────────
  154 |   test('5.1 — public menu returns attributes with taste+bom, NOT top-level kcal', async ({ request }) => {
  155 |     const res = await request.get(`${BASE}/public/locations/${locationSlug}/menu`);
  156 |     expect(res.status()).toBe(200);
  157 |     const body = await res.json();
  158 |     expect(body.categories).toBeTruthy();
  159 |     expect(body.categories.length).toBeGreaterThan(0);
  160 |     const allProducts = body.categories.flatMap((c: any) => c.products || []);
  161 |     const pitaProduct = allProducts.find((p: any) => p.name && p.name.includes('Pita Test Sushi Updated'));
  162 |     expect(pitaProduct).toBeTruthy();
  163 | 
  164 |     expect(pitaProduct.attributes).toBeTruthy();
  165 |     expect(pitaProduct.attributes.taste).toBeTruthy();
  166 |     expect(pitaProduct.attributes.taste.spicy).toBe(3);
  167 | 
  168 |     expect(pitaProduct.attributes.bom).toBeTruthy();
  169 |     expect(pitaProduct.attributes.bom.length).toBe(3);
  170 |     expect(pitaProduct.attributes.bom[2].allergens).toContain('soy');
  171 | 
  172 |     expect(pitaProduct.attributes.kcal).toBeUndefined();
  173 |     expect(pitaProduct.attributes.protein).toBeUndefined();
  174 |     expect(pitaProduct.attributes.fat).toBeUndefined();
  175 |   });
  176 | 
  177 |   // ── 6. Image upload — strict assertion (no 500 accepted) ──────────────
  178 |   test('6.1 — image upload without auth returns 401', async ({ request }) => {
  179 |     const fakePng = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==', 'base64');
  180 |     const res = await request.post(`${BASE}/api/owner/menu/products/${createdProductId}/image`, {
  181 |       headers: { Authorization: '' },
  182 |       multipart: {
  183 |         file: { name: 'test.png', mimeType: 'image/png', buffer: fakePng },
  184 |       },
  185 |     });
  186 |     expect(res.status()).toBe(401);
  187 |   });
  188 | 
  189 |   test('6.2 — image upload with auth returns 200 (STRICT: 500 is a failure)', async ({ request }) => {
  190 |     const fakePng = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==', 'base64');
  191 |     const res = await request.post(`${BASE}/api/owner/menu/products/${createdProductId}/image`, {
  192 |       headers: { Authorization: `Bearer ${authToken}` },
  193 |       multipart: {
  194 |         file: { name: 'test.png', mimeType: 'image/png', buffer: fakePng },
  195 |       },
  196 |     });
> 197 |     expect(res.status()).toBe(200);
      |                          ^ Error: expect(received).toBe(expected) // Object.is equality
  198 |     const body = await res.json();
  199 |     expect(body.imageUrl).toBeTruthy();
  200 |   });
  201 | 
  202 |   // ── 7. Menu import AI — LLM adapter detection (strict: no 500) ──────
  203 |   test('7.1 — menu import endpoint does not return 500 for auth issues', async ({ request }) => {
  204 |     const fakePng = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==', 'base64');
  205 |     const res = await request.post(`${BASE}/api/owner/menu/import/preview`, {
  206 |       headers: { Authorization: `Bearer ${authToken}` },
  207 |       multipart: {
  208 |         file: { name: 'menu.png', mimeType: 'image/png', buffer: fakePng },
  209 |         mode: 'add_only',
  210 |       },
  211 |     });
  212 |     expect(res.status()).not.toBe(401);
  213 |     expect(res.status()).not.toBe(404);
  214 |     expect(res.status()).not.toBe(500);
  215 |     const contentType = res.headers()['content-type'] || '';
  216 |     if (contentType.includes('application/json')) {
  217 |       const body = await res.json();
  218 |       if (body.issues && body.issues.length > 0) {
  219 |         const llmIssue = body.issues.find((i: any) => i.code === 'PARSE_ERROR');
  220 |         if (llmIssue) {
  221 |           console.log('LLM issue (expected if no LLM service):', llmIssue.message);
  222 |         }
  223 |       }
  224 |     }
  225 |   });
  226 | 
  227 |   // ── 8. Settlements health check fix ────────────────────────────────
  228 |   test('8.1 — health check shows settlement as OK (not BROKEN)', async ({ request }) => {
  229 |     const res = await request.get(`${BASE}/health`);
  230 |     expect(res.status()).toBe(200);
  231 |     const body = await res.json();
  232 |     expect(body.checks.settlement.status).toBe('ok');
  233 |   });
  234 | 
  235 |   // ── 9. SSR X-Menu-Version header ──────────────────────────────────
  236 |   test('9.1 — SSR page includes X-Menu-Version header', async ({ request }) => {
  237 |     const res = await request.get(`${BASE}/s/${locationSlug}`);
  238 |     expect(res.status()).toBe(200);
  239 |     expect(res.headers()['x-menu-version']).toBeTruthy();
  240 |   });
  241 | 
  242 |   // ── 10. SPA /dashboard route fix ────────────────────────────────
  243 |   test('10.1 — /dashboard returns 200 (SPA fallback)', async ({ request }) => {
  244 |     const res = await request.get(`${BASE}/dashboard`);
  245 |     expect(res.status()).toBe(200);
  246 |     const text = await res.text();
  247 |     expect(text).toContain('root');
  248 |   });
  249 | 
  250 |   // ── 11. Product data round-trip: admin → client ────────────────────
  251 |   test('11.1 — admin product list includes taste + recipeLines', async ({ request }) => {
  252 |     const res = await request.get(`${BASE}/api/owner/menu/products?category_id=${createdCategoryId}`, {
  253 |       headers: { Authorization: `Bearer ${authToken}` },
  254 |     });
  255 |     expect(res.status()).toBe(200);
  256 |     const products = await res.json();
  257 |     const found = Array.isArray(products) ? products.find((p: any) => p.id === createdProductId) : null;
  258 |     expect(found).toBeTruthy();
  259 |     expect(found.taste).toBeTruthy();
  260 |     expect(found.taste.spicy).toBe(3);
  261 |     expect(found.recipeLines.length).toBe(3);
  262 |     expect(found.stockCount).toBe(42);
  263 |   });
  264 | 
  265 |   // ── 12. Theme endpoint resolves with slug from settings ─────────────
  266 |   test('12.1 — theme endpoint returns valid data for settings slug', async ({ request }) => {
  267 |     expect(locationSlug).toBeTruthy();
  268 |     const themeRes = await request.get(`${BASE}/api/public/theme/${locationSlug}`);
  269 |     expect(themeRes.status()).toBe(200);
  270 |     const theme = await themeRes.json();
  271 |     expect(theme.primaryColor).toBeTruthy();
  272 |   });
  273 | 
  274 |   // ── 13. Cleanup: delete test product ────────────────────────────────
  275 |   test('13.1 — delete test product', async ({ request }) => {
  276 |     const res = await request.delete(`${BASE}/api/owner/menu/products/${createdProductId}`, {
  277 |       headers: { Authorization: `Bearer ${authToken}` },
  278 |     });
  279 |     expect([200, 204]).toContain(res.status());
  280 |   });
  281 | 
  282 |   test('13.2 — delete test category', async ({ request }) => {
  283 |     const res = await request.delete(`${BASE}/api/owner/menu/categories/${createdCategoryId}`, {
  284 |       headers: { Authorization: `Bearer ${authToken}` },
  285 |     });
  286 |     expect([200, 204]).toContain(res.status());
  287 |   });
  288 | });
```