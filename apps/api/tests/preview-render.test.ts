import { test } from 'node:test';
import assert from 'node:assert/strict';
import { renderShadowPreview, type PreviewMenu } from '../src/lib/preview-render.js';

// P6-3 preview render. Link-unfurl policy CHANGED 2026-07-06 (operator directive, overriding the
// original H3 generic-OG rule): a shadow (demo) tenant now advertises its REAL identity in unfurl
// metadata (og:title = name, og:image = the per-venue card) so a pasted /s/:slug renders a product
// card in chats. The operator-protective invariants are STILL enforced here and asserted below:
//   • `noindex` stays (unfurl ≠ search index),
//   • the body carries an honest "demo — not yet live" banner + a claim/decline CTA,
//   • never-orderable (no cart/checkout),
//   • scraped strings are HTML-escaped.
// Context: docs/design/demo-preview-upgrades/PLAN.md §3.

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

const OPTS = { ogImageUrl: 'https://dowiz.example/og/trattoria-x.png', baseUrl: 'https://dowiz.example' };

test('render shows the honest demo banner, the menu, prices, full descriptions, and the claim CTA', () => {
  const html = renderShadowPreview(MENU, OPTS);
  assert.match(html, /ende jo dyqan aktiv/i, 'honest "not yet live" banner present');
  assert.match(html, /Margherita/, 'menu item rendered');
  assert.match(html, /850 ALL/, 'price formatted');
  assert.match(html, /Tomato, mozzarella, basil/, 'full description rendered (operator D-render)');
  assert.match(html, /restoranti juaj/i, 'claim CTA present');
});

test('rich unfurl: real name IS in <title> + og:title, and the per-venue card IS the og:image', () => {
  const html = renderShadowPreview(MENU, OPTS);
  assert.match(html, /<h1>Trattoria Da Marco<\/h1>/, 'real name in body');
  const title = html.match(/<title>([^<]*)<\/title>/)?.[1] ?? '';
  assert.match(title, /Trattoria Da Marco/, 'real name IS in <title> now');
  const ogTitle = html.match(/<meta property="og:title" content="([^"]*)"/)?.[1] ?? '';
  assert.match(ogTitle, /Trattoria Da Marco/, 'real name IS in og:title now');
  assert.match(html, /<meta property="og:image" content="https:\/\/dowiz\.example\/og\/trattoria-x\.png"/, 'per-venue card is og:image');
  assert.match(html, /<meta property="og:url" content="https:\/\/dowiz\.example\/s\/trattoria-x"/, 'og:url present');
  assert.match(html, /<meta name="twitter:card" content="summary_large_image"/, 'large-image twitter card');
});

test('operator-protective invariants kept: noindex + never-orderable', () => {
  const html = renderShadowPreview(MENU, OPTS);
  assert.match(html, /<meta name="robots" content="noindex, nofollow"/, 'noindex meta present');
  assert.doesNotMatch(html, /add to cart|checkout|\bcart\b/i, 'no ordering affordance');
});

test('no og:image tag when no card URL is provided (degrade cleanly, never a broken tag)', () => {
  const html = renderShadowPreview(MENU);
  assert.doesNotMatch(html, /<meta property="og:image"/, 'og:image omitted without a URL');
  assert.match(html, /<meta property="og:title"/, 'og:title still emitted');
});

test('HTML is escaped (no injection from scraped names/descriptions) — incl. og:title', () => {
  const html = renderShadowPreview(
    {
      ...MENU,
      name: '<script>alert(1)</script>',
      categories: [{ name: 'X', products: [{ name: '<img src=x onerror=1>', price: 100 }] }],
    },
    OPTS,
  );
  assert.doesNotMatch(html, /<script>alert\(1\)<\/script>/, 'name escaped in body + meta');
  assert.doesNotMatch(html, /<img src=x onerror=1>/, 'product name escaped');
  assert.match(html, /&lt;script&gt;/, 'escaped form present');
});
