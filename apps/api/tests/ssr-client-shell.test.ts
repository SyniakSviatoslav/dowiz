import test from 'node:test';
import assert from 'node:assert/strict';
import { renderClientShell } from '../src/lib/ssr-client-renderer.js';

// Regression: the cart/checkout/status shells must load the self-hosted,
// purged Tailwind CSS (built by build-client.js -> /dist/tailwind.css) instead
// of the runtime cdn.tailwindcss.com script (which warns it shouldn't be used
// in production and adds an external-CDN runtime dependency).
test('client shell uses self-hosted Tailwind CSS, not the CDN', () => {
  const out = renderClientShell({
    title: 'Checkout', slug: 'demo', scriptUrl: '/dist/checkout/app.js', nonce: 'n1', locationId: 'l1',
  });
  assert.ok(out.includes('<link rel="stylesheet" href="/dist/tailwind.css"'), 'local Tailwind CSS link missing');
  assert.ok(!out.includes('cdn.tailwindcss.com'), 'still references the Tailwind CDN');
  // Assert the FULL well-formed tag incl. closing </script>, not just the opening
  // substring (a malformed/unclosed tag must fail this test).
  assert.ok(
    out.includes('<script type="module" src="/dist/checkout/app.js"></script>'),
    'bundle script tag missing or not well-formed',
  );
  // When locationId is provided the renderer MUST emit the dos-location-id meta.
  assert.ok(
    out.includes('<meta name="dos-location-id" content="l1"/>'),
    'dos-location-id meta missing or wrong content',
  );
});

test('client shell omits dos-location-id meta when no locationId', () => {
  const out = renderClientShell({
    title: 'Checkout', slug: 'demo', scriptUrl: '/dist/checkout/app.js', nonce: 'n1',
  });
  // Negative branch: no locationId -> the conditional meta must NOT be emitted.
  assert.ok(!out.includes('dos-location-id'), 'dos-location-id meta should be absent without locationId');
  // Positive control: the rest of the shell still renders.
  assert.ok(out.includes('<meta name="dos-slug" content="demo"/>'), 'dos-slug meta missing');
});
