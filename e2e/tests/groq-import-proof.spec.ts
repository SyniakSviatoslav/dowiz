import { test, expect } from '@playwright/test';
import { readFileSync } from 'fs';
import { resolve } from 'path';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
let authToken: string;

test('0 — get owner auth token', async ({ request }) => {
  const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
  expect(res.status()).toBe(200);
  const body = await res.json();
  expect(body.access_token).toBeTruthy();
  authToken = body.access_token;
});

test('1 — upload menu-sq.pdf via AI import preview (Groq)', async ({ request }) => {
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

  const body = await res.json();
  console.log(`[IMPORT] Status: ${res.status()}`);
  console.log(`[IMPORT] issues: ${JSON.stringify(body.issues || []).slice(0, 1000)}`);
  console.log(`[IMPORT] draft summary: cats=${body.draft?.categories?.length}, prods=${body.draft?.products?.length}`);

  // Must NOT have an LLM parse error
  const llmErrors = (body.issues || []).filter((i: any) => i.code === 'PARSE_ERROR');
  expect(llmErrors.length).toBe(0);

  // Must have parsed categories and products
  const draft = body.draft || {};
  expect(draft.categories?.length).toBeGreaterThan(0);
  expect(draft.products?.length).toBeGreaterThan(0);

  // Print what we got
  console.log(`Categories: ${draft.categories.length}`);
  for (const c of draft.categories.slice(0, 3)) console.log(`  cat: ${c.name}`);
  console.log(`Products: ${draft.products.length}`);
  for (const p of draft.products.slice(0, 5)) console.log(`  ${p.name} — ${p.price} ALL`);

  // Must have import_session_id for commit
  expect(body.import_session_id).toBeTruthy();
});

test('2 — commit the import', async ({ request }) => {
  const pdfBytes = readFileSync(resolve('menu-sq.pdf'));
  const previewRes = await request.post(`${BASE}/api/owner/menu/import/preview`, {
    headers: { Authorization: `Bearer ${authToken}` },
    multipart: {
      file: { name: 'menu-sq.pdf', mimeType: 'application/pdf', buffer: pdfBytes },
      mode: 'merge',
    },
    timeout: 120000,
  });
  const preview = await previewRes.json();

  const res = await request.post(`${BASE}/api/owner/menu/import/commit`, {
    headers: { Authorization: `Bearer ${authToken}` },
    data: { import_session_id: preview.import_session_id, force: true },
    timeout: 60000,
  });

  expect([200, 201]).toContain(res.status());
  console.log('Import committed successfully');
});
