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
  assert.ok(out.includes('<script type="module" src="/dist/checkout/app.js">'), 'bundle script missing');
});
