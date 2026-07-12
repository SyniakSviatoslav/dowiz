import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import { resolve } from 'path';
<<<<<<< Updated upstream
=======
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';
>>>>>>> Stashed changes

// This suite MUTATES the DB (the commit step writes categories/products) and uses the
// /dev/mock-auth backdoor, which is registered ONLY on non-prod (it 404s on prod). Default
// to staging and hard-guard the target so a stray run can never write to / 404 against prod.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
let authToken: string;
let importSessionId: string;

test.beforeAll(async ({ request }) => {
  requireStaging(BASE);
  // Acquire the owner token ONCE and share it, so test order/isolation can't leave it undefined.
  const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
  expect(res.status()).toBe(200);
  const body = await res.json();
  expect(body.access_token).toBeTruthy();
  authToken = body.access_token;
});

test('0 — preview rejects an unauthenticated request (401)', async ({ request }) => {
  // Negative control: the gate must reject a no-token caller (it isn't silently letting everyone in).
  const res = await request.post(`${BASE}/api/owner/menu/import/preview`, { data: {} });
  expect(res.status()).toBe(401);
});

test('1 — upload menu-sq.pdf via AI import preview (Groq)', async ({ request }) => {
  expectJwt(authToken, 'authToken'); // beforeAll must have minted it
  const pdfPath = resolve('menu-sq.pdf');
  const pdfBytes = readFileSync(pdfPath);

  const res = await request.post(`${BASE}/api/owner/menu/import/preview`, {
    headers: { Authorization: `Bearer ${authToken}` },
    multipart: {
      file: { name: 'menu-sq.pdf', mimeType: 'application/pdf', buffer: pdfBytes },
      mode: 'merge',
    },
    timeout: 120000,
  });

  // Positive control: a valid owner gets a 200 (route returns 200 on success — menu-import.ts:155).
  expect(res.status()).toBe(200);
  const body = await res.json();
  console.log(`[IMPORT] Status: ${res.status()}`);
  console.log(`[IMPORT] issues: ${JSON.stringify(body.issues || []).slice(0, 1000)}`);

  // Must NOT have an LLM parse error
  const llmErrors = (body.issues || []).filter((i: any) => i.code === 'PARSE_ERROR');
  expect(llmErrors.length).toBe(0);

  // Must have parsed categories and products
  const dp = body.draft_preview || {};
  expect(dp.categories_to_create?.length).toBeGreaterThan(0);
  expect(dp.products_to_create?.length).toBeGreaterThan(0);

  // Print what we got
  console.log(`Categories: ${dp.categories_to_create.length}`);
  for (const c of dp.categories_to_create.slice(0, 3)) console.log(`  cat: ${c}`);
  console.log(`Products: ${dp.products_to_create.length}`);
  for (const p of dp.products_to_create.slice(0, 5)) console.log(`  ${p}`);

<<<<<<< Updated upstream
  // Must have import_session_id for commit
  expect(body.import_session_id).toBeTruthy();
=======
  // Must have import_session_id for commit — share it with test 2 (no second preview).
  expectUuid(body.import_session_id, 'import_session_id');
  importSessionId = body.import_session_id;
>>>>>>> Stashed changes
});

test('2 — commit the import', async ({ request }) => {
  // Reuse test 1's session id rather than re-running preview; a missing id fails loudly here.
  expectUuid(importSessionId, 'import_session_id from preview');

  const res = await request.post(`${BASE}/api/owner/menu/import/commit`, {
    headers: { Authorization: `Bearer ${authToken}` },
    data: { import_session_id: importSessionId, force: true },
    timeout: 60000,
  });

  const body = await res.json();
  console.log(`[COMMIT] Status: ${res.status()}, body: ${JSON.stringify(body).slice(0, 500)}`);

  // Success path returns 200 + counts (menu-import.ts:511-519). A 5xx here is a real defect to
  // escalate, not a thing to tolerate — assert the exact expected status and committed counts.
  expect(res.status()).toBe(200);
  expect(body.counts?.categories).toBeGreaterThan(0);
  expect(body.counts?.products).toBeGreaterThan(0);

  // TODO(needs-staging): cross-tenant isolation. /dev/mock-auth only ever mints the single dev
  // owner, so a real IDOR check (a SECOND owner's token committing THIS import_session_id must
  // get 403/404 — menu-import.ts:264 filters by location_id) needs a seeded second tenant on
  // staging. Cannot be faked with a nil/foreign uuid; tracked in needs_staging.
});
