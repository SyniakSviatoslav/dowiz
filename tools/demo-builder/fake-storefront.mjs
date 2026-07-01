// Configurable FAKE storefront for the demo-builder loop's visual-acceptance-gate anti-cheat dry-run.
// Serves /s/:slug HTML in one of several variants, keyed by a slug→variant map, so the harness can prove
// the visual gate (assertPreviewDom) PASSES a demo-quality render and NEEDS-REVIEWs every broken one —
// EVEN when the HTTP status is 200 (the no-fake-green centerpiece: the gate reads DOM, not the status code).
//
// Variants (each returns HTTP 200 — a naive "200 = pass" loop would accept all of them):
//   good      — renders menu-item cards + venue-preview-banner + preview-claim-cta, noindex, NO order
//               affordance, 0 console errors. The only PASS.
//   empty     — 200 but "No menu items" (zero menu-item cards). Gate → FAIL.
//   error     — 200 but sets x-fake-console-errors: 2 (stand-in for real page console errors). Gate → FAIL.
//   orderable — 200 with menu-item-add present (B3 never-orderable violated). Gate → FAIL.
//   noindexless — 200, good menu, but MISSING the noindex header/meta. Gate → FAIL.

import { createServer } from 'node:http';

function goodHtml(slug, orderable = false, withNoindexMeta = true) {
  const items = ['Margherita', 'Diavola', 'Quattro Formaggi', 'Burrata', 'Tiramisu']
    .map(
      (n) => `<div data-testid="menu-item"><span class="name">${n}</span><span class="price">58000 ALL</span>
        <p class="desc">A demo-quality description for ${n}.</p>${orderable ? '<button data-testid="menu-item-add">+</button>' : ''}</div>`,
    )
    .join('\n');
  return `<!doctype html><html lang="en"><head>
    <meta charset="utf-8"/><meta name="viewport" content="width=device-width, initial-scale=1"/>
    ${withNoindexMeta ? '<meta name="robots" content="noindex, nofollow"/>' : ''}
    <title>Restaurant preview · Dowiz</title></head><body>
    <div data-testid="venue-preview-banner">This is a preview mockup — it is NOT a live store and cannot take orders.</div>
    <nav data-testid="category-nav">Antipasti · Pizza · Dolci</nav>
    <main>${items}</main>
    <div data-testid="preview-claim-cta">Is this your restaurant? Claim this preview.</div>
    </body></html>`;
}

function emptyHtml() {
  return `<!doctype html><html lang="en"><head><meta name="robots" content="noindex"/>
    <title>Restaurant preview · Dowiz</title></head><body>
    <div data-testid="venue-preview-banner">preview — not a live store</div>
    <main><p>No menu items available.</p></main>
    <div data-testid="preview-claim-cta">Claim this preview.</div></body></html>`;
}

export function startFakeStorefront({ variants = {} } = {}) {
  // variants: { [slug]: 'good' | 'empty' | 'error' | 'orderable' | 'noindexless' }  (default 'good')
  const server = createServer((req, res) => {
    const m = /^\/s\/([^/?]+)/.exec(req.url || '');
    const slug = m ? m[1] : '';
    const variant = variants[slug] || 'good';
    const headers = { 'content-type': 'text/html; charset=utf-8', 'x-robots-tag': 'noindex, nofollow' };
    let html;
    switch (variant) {
      case 'empty': html = emptyHtml(); break;
      case 'error': html = goodHtml(slug); headers['x-fake-console-errors'] = '2'; break;
      case 'orderable': html = goodHtml(slug, true); break;
      case 'noindexless': html = goodHtml(slug, false, false); delete headers['x-robots-tag']; break;
      case 'good': default: html = goodHtml(slug); break;
    }
    res.writeHead(200, headers);
    res.end(html);
  });
  return new Promise((resolveP) => {
    server.listen(0, '127.0.0.1', () => {
      const port = server.address().port;
      resolveP({ baseUrl: `http://127.0.0.1:${port}`, close: () => new Promise((r) => server.close(r)) });
    });
  });
}
