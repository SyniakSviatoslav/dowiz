import { test } from 'node:test';
import assert from 'node:assert/strict';
import { renderShadowPreview, type PreviewMenu } from '../src/lib/preview-render.js';

// P6-3 (council D + H3) — the labeled preview must be honest AND must NOT leak the real name into
// unfurl metadata (H3 regresses B2 otherwise).

const MENU: PreviewMenu = {
  slug: 'trattoria-x',
  name: 'Trattoria Da Marco',
  is_preview: true,
  currency: { code: 'ALL', minor_unit: 0 },
  categories: [
    {
      name: 'Pizza',
      products: [
        { name: 'Margherita', description: 'Tomato, mozzarella, basil', price: 850 },
        { name: 'Diavola', description: 'Spicy salami', price: 1100 },
      ],
    },
  ],
};

test('D: render shows the honest banner, the menu, prices, and full descriptions', () => {
  const html = renderShadowPreview(MENU);
  assert.match(html, /not a live store/i, 'honest banner present');
  assert.match(html, /Margherita/, 'menu item rendered');
  assert.match(html, /850 ALL/, 'price formatted');
  assert.match(html, /Tomato, mozzarella, basil/, 'full description rendered (operator D-render)');
  assert.match(html, /claim this preview/i, 'claim CTA present');
});

test('H3: the real restaurant name appears ONLY in the body, never in <title>/og: metadata', () => {
  const html = renderShadowPreview(MENU);
  // body has the real name
  assert.match(html, /<h1>Trattoria Da Marco<\/h1>/, 'real name in body');
  // metadata is generic — extract <title> and every og:* content and assert no real name
  const title = html.match(/<title>([^<]*)<\/title>/)?.[1] ?? '';
  assert.doesNotMatch(title, /Trattoria Da Marco/, 'real name must NOT be in <title>');
  const ogContents = [...html.matchAll(/<meta property="og:[^"]+" content="([^"]*)"/g)].map((m) => m[1]);
  for (const c of ogContents) assert.doesNotMatch(c ?? '', /Trattoria Da Marco/, 'real name must NOT be in og:* (unfurl) metadata');
  assert.ok(ogContents.length >= 2, 'og tags present (generic)');
});

test('D: noindex + never-orderable (no cart/checkout/add-to-cart on the page)', () => {
  const html = renderShadowPreview(MENU);
  assert.match(html, /<meta name="robots" content="noindex, nofollow"/, 'noindex meta present');
  assert.doesNotMatch(html, /add to cart|checkout|\bcart\b/i, 'no ordering affordance');
});

test('D: HTML is escaped (no injection from scraped names/descriptions)', () => {
  const html = renderShadowPreview({
    ...MENU,
    name: '<script>alert(1)</script>',
    categories: [{ name: 'X', products: [{ name: '<img src=x onerror=1>', price: 100 }] }],
  });
  assert.doesNotMatch(html, /<script>alert\(1\)<\/script>/, 'name escaped');
  assert.doesNotMatch(html, /<img src=x onerror=1>/, 'product name escaped');
  assert.match(html, /&lt;script&gt;/, 'escaped form present');
});
