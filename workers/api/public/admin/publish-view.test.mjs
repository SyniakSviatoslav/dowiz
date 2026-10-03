// node --test workers/api/public/admin/publish-view.test.mjs
// The Published-menu card (W-PUBUI): the answer of GET /api/owner/publish, drawn
// against the real /lib/ui; the words come from admin/i18n.js (langs gate).
import test from 'node:test';
import assert from 'node:assert/strict';
import * as V from './publish-view.js';
import { useTranslator } from '../lib/ui/core.js';
import { render, XSS, injected } from '../lib/ui/dom-shim.mjs';

useTranslator(k => `<${k}>`);
const CDN = 'https://cdn.dowiz.org';
const OFF = { enabled: false, manifest: 'v/dubin/manifest.json', published: { generation: [0, 0, 0], objects: {}, media: [] } };
const ON = { enabled: true, manifest: 'v/dubin/manifest.json',
  published: { generation: [12, 3, 5], objects: { fragment: 'a.json', 'words/en': 'b.json', 'block/names': 'c.dwb', media: 'd.json' }, media: ['x.jpg', 'y.jpg'] } };

test('off: the card says plainly that publishing waits for the CDN bucket, and the button is disabled', () => {
  const html = V.page(OFF, CDN);
  const { root } = render(html);
  assert.ok(root.querySelector('[data-t="pub_offHint"]'), 'the one plain sentence');
  assert.ok(!root.querySelector('[data-t="pub_onHint"]'));
  const b = root.querySelector('#pubNow');
  assert.ok(b, 'the button is drawn');
  assert.ok(b.hasAttribute('disabled'), 'and disabled: a tap could only answer 503');
  assert.ok(root.querySelector('[data-t="pub_never"]'), 'nothing published yet');
  assert.ok(html.includes('data-t="off"'), 'the pill reads off');
});

test('on: the last generation, the counts and the manifest link are drawn; the button is live', () => {
  const html = V.page(ON, CDN);
  const { root } = render(html);
  assert.ok(root.querySelector('[data-t="pub_onHint"]'));
  assert.ok(!root.querySelector('#pubNow').hasAttribute('disabled'));
  assert.ok(html.includes('&lt;pub_genCatalog&gt;</span> 12'), html);
  assert.ok(html.includes('&lt;pub_genWords&gt;</span> 3'), html);
  assert.ok(html.includes('&lt;pub_genSettings&gt;</span> 5'), html);
  assert.ok(html.includes('4 <span data-t="pub_objects">'), 'four objects on record');
  assert.ok(html.includes('2 <span data-t="pub_photos">'), 'two photos copied');
  assert.equal(root.querySelector('#pubManifest').getAttribute('href'), 'https://cdn.dowiz.org/v/dubin/manifest.json');
  assert.ok(html.includes('data-t="on"'), 'the pill reads on');
});

test('no CDN for this host: no link, and the rest of the card still draws', () => {
  const html = V.page(ON, '');
  assert.ok(!html.includes('pubManifest'));
  assert.ok(html.includes('pubNow'));
});

test('cdnOrigin follows the storefront rule: cdn.<platform> for a venue host, nothing for dev hosts', () => {
  assert.equal(V.cdnOrigin({ hostname: 'dubin-sushi.dowiz.org' }), 'https://cdn.dowiz.org');
  assert.equal(V.cdnOrigin({ hostname: 'Sushi-Durres.dowiz.org' }), 'https://cdn.dowiz.org');
  for (const h of ['x.workers.dev', 'localhost', '127.0.0.1', 'dowiz.org', 'www.dowiz.org', '']) assert.equal(V.cdnOrigin({ hostname: h }), '', h);
  assert.equal(V.cdnOrigin(undefined), '');
});

test('manifestUrl and generationLine stand alone', () => {
  assert.equal(V.manifestUrl({ manifest: null }, CDN), '');
  assert.equal(V.manifestUrl(ON, ''), '');
  assert.equal(V.manifestUrl(ON, CDN), 'https://cdn.dowiz.org/v/dubin/manifest.json');
  assert.ok(V.generationLine(undefined).includes('&lt;pub_genCatalog&gt;</span> 0'));
  assert.ok(V.generationLine(['7', 'x', 2]).includes('&lt;pub_genCatalog&gt;</span> 7') && V.generationLine(['7', 'x', 2]).includes('&lt;pub_genWords&gt;</span> 0'));
});

test('a hostile manifest key is escaped, never injected', () => {
  const html = V.page({ ...ON, manifest: XSS }, CDN);
  assert.deepEqual(injected(html), [], html);
});
