import test from 'node:test';
import assert from 'node:assert/strict';
import { renderMenuPage, toMajorUnits } from '../src/lib/ssr-renderer.js';

// GUARDRAIL — Albania go-to-market gap: the SSR structured-data (JSON-LD) price emitter
// hard-coded `(prod.price / 100).toFixed(2)`, double-scaling a minor_unit=0 currency (Lekë).
// A 1200-Lekë item emitted "12.00" in schema.org Offer.price while the ON-PAGE price
// (via formatPrice → minor_unit aware) correctly showed "1200" — a 100× inconsistency that
// misleads crawlers / rich results. Both now share ONE conversion: toMajorUnits().
// Display-only (JSON-LD is SEO metadata; no charged/stored value flows through here).

test('toMajorUnits — minor→major conversion honors minor_unit (no hard-coded /100)', () => {
  // ALL / Lekë: minor_unit 0 → stored value IS the major value, NEVER divided.
  assert.equal(toMajorUnits(1200, 0), 1200);
  assert.equal(toMajorUnits(800, 0), 800);
  // minor_unit 2 (e.g. EUR cents): 80000 minor → 800 major (the gap-analysis example).
  assert.equal(toMajorUnits(80000, 2), 800);
  assert.equal(toMajorUnits(1000, 2), 10);
});

// --- End-to-end: an ALL (minor_unit 0) menu must emit the whole-Lekë price in JSON-LD,
//     identical to the visible price, and NEVER the 100×-too-small "12.00". ---

function fakeAllPool() {
  const locationRow = {
    id: 'lAll', name: 'Byreku i Tiranës', slug: 'x',
    currency_code: 'ALL', currency_minor_unit: 0,
    default_locale: 'sq', supported_locales: ['sq', 'en'],
    address: null, public_phone: null, hours_json: null, geo: null,
    owner_id: 'uAll', // claimed tenant — passes the P6-2 shadow gate
  };
  const menu = {
    menu_version: 1, default_locale: 'sq', supported_locales: ['sq', 'en'],
    currency: { code: 'ALL', minor_unit: 0 },
    categories: [{
      id: 'c1', sort_order: 0, available_names: { sq: 'Byrek', en: 'Pies' },
      products: [{
        id: 'p1', price: 1200, available: true, image_key: null, attributes: null,
        available_names: { sq: 'Byrek me spinaq', en: 'Spinach pie' },
        available_descriptions: {},
      }],
    }],
  };
  return {
    connect: async () => ({
      query: async (sql: string) => {
        if (sql.includes('FROM locations')) return { rowCount: 1, rows: [locationRow] };
        if (sql.includes('read_public_menu_all_locales')) return { rowCount: 1, rows: [{ menu }] };
        return { rowCount: 0, rows: [] };
      },
      release: () => {},
    }),
  };
}

test('SSR JSON-LD emits whole-Lekë price for a minor_unit=0 (ALL) menu', async (t) => {
  const out = await renderMenuPage('all-jsonld-1200', fakeAllPool() as any, 'https://example.test');

  await t.test('JSON-LD Offer.price is the whole-Lekë value 1200 (not the 100×-off 12.00)', () => {
    const m = out.match(/<script type="application\/ld\+json"[^>]*>([\s\S]*?)<\/script>/);
    assert.ok(m, 'no JSON-LD block found');
    const parsed = JSON.parse(m![1]);
    const nodes = Array.isArray(parsed) ? parsed : [parsed];
    const menuNode = nodes.find((n: any) => n['@type'] === 'Menu');
    assert.ok(menuNode, 'no Menu node in JSON-LD');
    const item = (menuNode.hasMenuItem || []).find((i: any) => i.name === 'Byrek me spinaq');
    assert.ok(item, 'menu item not found in JSON-LD');
    // Independent expected value: 1200 Lekë stored, minor_unit 0 → "1200".
    assert.equal(item.offers.price, '1200', `Offer.price wrong (got "${item.offers.price}")`);
    assert.equal(item.offers.priceCurrency, 'ALL');
  });

  await t.test('the WRONG 100×-off value "12.00" appears nowhere in the document', () => {
    assert.ok(!out.includes('12.00'), 'the buggy /100 price "12.00" leaked into the render');
  });

  await t.test('on-page product price shows the same 1200 (JSON-LD ↔ visible parity)', () => {
    assert.ok(out.includes('1200'), 'visible product price 1200 missing');
  });
});
