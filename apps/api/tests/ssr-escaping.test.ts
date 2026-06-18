import test from 'node:test';
import assert from 'node:assert/strict';
import { renderMenuPage } from '../src/lib/ssr-renderer.js';

// U2 regression: the html`` (htm + preact-render-to-string) templates already
// escape interpolations. Hand-calling escapeHtml on top of that double-encoded
// any '&' — e.g. a venue "Dubin & Sushi" rendered as "Dubin &amp;amp; Sushi"
// in <title>/OG tags/body. This asserts single-escaping end to end.

function fakePool() {
  const locationRow = {
    id: 'l1', name: 'Dubin & Sushi', slug: 'demo',
    currency_code: 'EUR', currency_minor_unit: 2,
    default_locale: 'en', supported_locales: ['en'],
    address: 'Rruga A & B', public_phone: null, hours_json: null, geo: null,
  };
  const menu = {
    menu_version: 1, default_locale: 'en', supported_locales: ['en'],
    currency: { code: 'EUR', minor_unit: 2 },
    categories: [{
      id: 'c1', available_names: { en: 'Mains & Sides' },
      products: [{
        id: 'p1', available_names: { en: 'Fish & Chips' },
        available_descriptions: { en: 'Cod & fries' }, image_key: null, price: 1000,
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

test('SSR menu page escapes ampersands exactly once', async (t) => {
  // unique slug avoids the module-level 60s render cache
  const out = await renderMenuPage('demo-u2-test', fakePool() as any, 'https://example.test');

  await t.test('never double-encodes anywhere in the document', () => {
    assert.ok(!out.includes('&amp;amp;'), 'found double-encoded &amp;amp; in output');
  });

  await t.test('venue name is single-escaped in <title> and OG tags and body', () => {
    assert.ok(out.includes('<title>Dubin &amp; Sushi'), 'title not single-escaped');
    assert.ok(out.includes('property="og:title" content="Dubin &amp; Sushi'), 'og:title not single-escaped');
    assert.ok(out.includes('<h1>Dubin &amp; Sushi</h1>'), 'h1 not single-escaped');
  });

  await t.test('product + category names are single-escaped', () => {
    assert.ok(out.includes('Fish &amp; Chips'), 'product name not single-escaped');
    assert.ok(out.includes('Mains &amp; Sides'), 'category name not single-escaped');
    assert.ok(!out.includes('Fish & Chips'), 'raw unescaped product name leaked');
  });
});
