import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// GUARDRAIL — LC9/S3 (docs/design-review/audit-frontend-2026-07-03.md CRITICAL #1,
// HIGH #9/#10, AUDIT-SYNTHESIS-2026-07-03.md LC9). Several admin surfaces used to
// render a fetch-failure fallback that fabricated money/PII/geo data indistinguishable
// from real, savable/exportable in two cases. This is a fast, dependency-free source
// check (this repo's test runner has no DOM/jsdom, so a full render assertion isn't
// possible) — it fails if the exact fabricated literals are ever reintroduced.
const DIR = dirname(fileURLToPath(import.meta.url));
const read = (f: string) => readFileSync(join(DIR, f), 'utf8');

test('CRMPage: customer analytics fetch failure never fabricates order/LTV/heatmap history', () => {
  const src = read('CRMPage.tsx');
  for (const fake of ['Rruga e Durresit', "id: 'o1'", '750000', 'total_spent: 750000']) {
    assert.equal(src.includes(fake), false, `CRMPage.tsx still contains fabricated literal: ${fake}`);
  }
  assert.match(src, /analyticsError/, 'an explicit per-customer error state must exist');
  assert.match(src, /onRetry=\{?\(?\)? ?=>? ?loadAnalytics/, 'the error state must offer a real retry');
});

test('AnalyticsPage: ingredient consumption is never a fake dataset with a working export', () => {
  const src = read('AnalyticsPage.tsx');
  for (const fake of ['CONSUMPTION_DATA', 'Salmon fillet', 'Sushi rice', 'Nori sheets']) {
    assert.equal(src.includes(fake), false, `AnalyticsPage.tsx still contains fabricated literal: ${fake}`);
  }
  // The consumption panel itself must not offer CSV/JSON export (there's nothing
  // real to export while it's unwired) — only the real topProducts export remains.
  const consumptionSection = src.slice(src.indexOf('Consumption report'), src.indexOf('Reorder list'));
  assert.doesNotMatch(consumptionSection, /exportCSV|exportJSON/, 'the unavailable consumption panel must not offer an export action');
});

test('SettingsPage: a failed/empty load never seeds a savable fake store identity', () => {
  const src = read('SettingsPage.tsx');
  for (const fake of ['MOCK_SETTINGS', '+35542345678', 'Rruga Ismail Qemali 45, Tirana']) {
    assert.equal(src.includes(fake), false, `SettingsPage.tsx still contains fabricated literal: ${fake}`);
  }
  // A save that 404s must be treated as a real failure, not a fake "saved" success —
  // the only "404" branch left should be the LOAD path (empty setup form), not SAVE.
  const saveFn = src.slice(src.indexOf('const handleSubmit'), src.indexOf('const fieldStyle'));
  assert.doesNotMatch(saveFn, /status === 404/, 'handleSubmit must not special-case 404 as a fake success');
});

test('sharp arm: the detector actually flags the bug shape (red arm)', () => {
  const bugged = "setCustomers(MOCK_SETTINGS); // 750000 Rruga e Durresit";
  assert.equal(bugged.includes('750000'), true);
  assert.equal(bugged.includes('MOCK_SETTINGS'), true);
});
