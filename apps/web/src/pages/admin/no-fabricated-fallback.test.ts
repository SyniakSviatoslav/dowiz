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

// The detector. Given a source string and the set of fabricated-data literals that must
// never ship, it returns the ones actually present (clean → [], bugged → the offenders).
// The real source pins AND the red arm both route through THIS one function, so if the
// detector is ever defeated (e.g. hollowed to `return []`, or made to over-match), the red
// arm below goes RED. That is what gives the guardrail its discriminating power — the old
// red arm only re-asserted that a locally-defined string contained substrings it obviously
// contained, which proved nothing about the detector.
function fabricatedLiteralsIn(src: string, forbidden: readonly string[]): string[] {
  return forbidden.filter((literal) => src.includes(literal));
}

// The exact fabricated literals each surface's fetch-failure fallback used to invent.
const CRM_FAKES = ['Rruga e Durresit', "id: 'o1'", '750000', 'total_spent: 750000'] as const;
const ANALYTICS_FAKES = ['CONSUMPTION_DATA', 'Salmon fillet', 'Sushi rice', 'Nori sheets'] as const;
const SETTINGS_FAKES = ['MOCK_SETTINGS', '+35542345678', 'Rruga Ismail Qemali 45, Tirana'] as const;

test('CRMPage: customer analytics fetch failure never fabricates order/LTV/heatmap history', () => {
  const src = read('CRMPage.tsx');
  assert.deepEqual(
    fabricatedLiteralsIn(src, CRM_FAKES), [],
    'CRMPage.tsx still contains fabricated customer-analytics literal(s)',
  );
  assert.match(src, /analyticsError/, 'an explicit per-customer error state must exist');
  assert.match(src, /onRetry=\{?\(?\)? ?=>? ?loadAnalytics/, 'the error state must offer a real retry');
});

test('AnalyticsPage: ingredient consumption is never a fake dataset with a working export', () => {
  const src = read('AnalyticsPage.tsx');
  assert.deepEqual(
    fabricatedLiteralsIn(src, ANALYTICS_FAKES), [],
    'AnalyticsPage.tsx still contains fabricated consumption literal(s)',
  );
  // The consumption panel itself must not offer CSV/JSON export (there's nothing
  // real to export while it's unwired) — only the real topProducts export remains.
  const consumptionSection = src.slice(src.indexOf('Consumption report'), src.indexOf('Reorder list'));
  assert.doesNotMatch(consumptionSection, /exportCSV|exportJSON/, 'the unavailable consumption panel must not offer an export action');
});

test('SettingsPage: a failed/empty load never seeds a savable fake store identity', () => {
  const src = read('SettingsPage.tsx');
  assert.deepEqual(
    fabricatedLiteralsIn(src, SETTINGS_FAKES), [],
    'SettingsPage.tsx still contains fabricated store-identity literal(s)',
  );
  // A save that 404s must be treated as a real failure, not a fake "saved" success —
  // the only "404" branch left should be the LOAD path (empty setup form), not SAVE.
  const saveFn = src.slice(src.indexOf('const handleSubmit'), src.indexOf('const fieldStyle'));
  assert.doesNotMatch(saveFn, /status === 404/, 'handleSubmit must not special-case 404 as a fake success');
});

test('red arm: the detector discriminates — FLAGS a bugged fixture, PASSES a clean one', () => {
  // A realistic fetch-failure fallback that fabricates money / PII / geo — the exact shapes
  // the LC9 fix removed. This fixture is FED THROUGH the same detector the real pins use, so
  // the arm exercises the detector's logic (not a tautology on a local string).
  const BUGGED_FIXTURE = `
    function loadAnalytics() {
      // fabricated fetch-failure fallback (the LC9 bug):
      setSettings(MOCK_SETTINGS);
      setCustomers([{ id: 'o1', total_spent: 750000, address: 'Rruga e Durresit' }]);
      const store = { phone: '+35542345678', address: 'Rruga Ismail Qemali 45, Tirana' };
      const bom = CONSUMPTION_DATA; // ['Salmon fillet', 'Sushi rice', 'Nori sheets']
    }
  `;
  // A clean error-state fallback: it surfaces a real error + retry, inventing nothing.
  const CLEAN_FIXTURE = `
    async function loadAnalytics() {
      try {
        const r = await fetch('/api/analytics');
        if (!r.ok) { setAnalyticsError(true); return; }
        setCustomers(await r.json());
      } catch { setAnalyticsError(true); }
    }
  `;
  const ALL_FAKES = [...CRM_FAKES, ...ANALYTICS_FAKES, ...SETTINGS_FAKES];

  // FLAG: the detector must catch the fabrication — and a representative literal from EACH
  // surface, so a detector hollowed for even one surface is caught.
  assert.ok(fabricatedLiteralsIn(BUGGED_FIXTURE, ALL_FAKES).length > 0, 'detector must FLAG a fabricating fixture');
  assert.ok(fabricatedLiteralsIn(BUGGED_FIXTURE, CRM_FAKES).length > 0, 'detector must FLAG the CRM money/geo fabrication');
  assert.ok(fabricatedLiteralsIn(BUGGED_FIXTURE, ANALYTICS_FAKES).length > 0, 'detector must FLAG the Analytics dataset fabrication');
  assert.ok(fabricatedLiteralsIn(BUGGED_FIXTURE, SETTINGS_FAKES).length > 0, 'detector must FLAG the Settings identity fabrication');

  // PASS: a clean error-state fixture must NOT be flagged (guards against an over-matching
  // detector that would false-positive and be silently loosened later).
  assert.deepEqual(fabricatedLiteralsIn(CLEAN_FIXTURE, ALL_FAKES), [], 'detector must PASS a clean error-state fixture');
});
