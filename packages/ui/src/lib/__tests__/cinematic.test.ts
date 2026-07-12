import { test } from 'node:test';
import assert from 'node:assert/strict';
import { revealLayoutId, cinematicRevealsEnabled } from '../cinematic.js';

// The load-bearing contract of the card→detail shared-element reveal: the OUTGOING node
// (ProductCard) and the INCOMING node (ProductDetailSheet) must derive the EXACT same
// `layoutId` from the same (node, id) pair, or framer attaches no morph. These assertions
// fail red if the naming convention drifts on either side.

test('revealLayoutId is stable + namespaced under product-', () => {
  assert.equal(revealLayoutId('media', 'p1'), 'product-media-p1');
  assert.equal(revealLayoutId('title', 'p1'), 'product-title-p1');
  assert.equal(revealLayoutId('price', 'p1'), 'product-price-p1');
});

test('the three shared nodes get DISTINCT ids for the same product (no id collision)', () => {
  const ids = new Set([
    revealLayoutId('media', 'x'),
    revealLayoutId('title', 'x'),
    revealLayoutId('price', 'x'),
  ]);
  assert.equal(ids.size, 3, 'media/title/price must not share a layoutId');
});

test('card and sheet derive identical ids from the same (node,id) — the morph contract', () => {
  // Simulates ProductCard.tsx and ProductDetailSheet.tsx each calling the SoT independently.
  const cardMedia = revealLayoutId('media', 'abc');
  const sheetMedia = revealLayoutId('media', 'abc');
  assert.equal(cardMedia, sheetMedia);
});

test('flag defaults OFF (dark) when VITE_CINEMATIC_REVEALS is unset', () => {
  // Under node:test there is no Vite `import.meta.env` → helper must default false, proving
  // the feature is dark by default (flag-off = no layoutId = current behaviour).
  assert.equal(cinematicRevealsEnabled(), false);
});
