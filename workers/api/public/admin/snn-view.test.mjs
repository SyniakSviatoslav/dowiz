// node --test workers/api/public/admin/snn-view.test.mjs
// W-SNN: the owner's shadow line from answers shaped exactly like `taste/snn.rs::health_json`,
// and its four languages.
import test from 'node:test';
import assert from 'node:assert/strict';
import { snnMarkup, WORDS, MODES } from './snn-view.js';
import { LANGS } from '../lib/langs.js';

test('snn: counts as percentages, the stored switch pressed, nothing per guest', () => {
  const html = snnMarkup({ mode: 'shadow', model: 20261006, compared: 40, unusable: 0, top1Pm: 431, overlapPm: 505, k: 3 });
  assert.ok(html.includes('40 <span data-t="snnCompared">'), html);
  assert.ok(html.includes('43% <span data-t="snnTop1">') && html.includes('51% <span data-t="snnOverlap">'), html);
  assert.ok(html.includes('data-snn-state="shadow"'));
  assert.equal([...html.matchAll(/data-snn-mode="(\w+)"/g)].map(m => m[1]).join(','), MODES.join(','));
  assert.ok(/<button[^>]*aria-pressed="true"[^>]*data-snn-mode="shadow"|<button[^>]*data-snn-mode="shadow"[^>]*aria-pressed="true"/.test(html), html);
  assert.ok(!html.includes('snnUnusable'), 'zero unusable is not drawn');
});

test('snn: nothing compared yet, an unknown mode reads as shadow, an error is said and escaped', () => {
  const html = snnMarkup({ mode: 'weird', compared: 0, unusable: 2, error: '<b>x</b>' });
  assert.ok(html.includes('data-t="snnNone"') && html.includes('data-snn-state="shadow"'));
  assert.ok(html.includes('2 <span data-t="snnUnusable">'));
  assert.ok(html.includes('&lt;b&gt;x&lt;/b&gt;') && !html.includes('<b>x</b>'));
  assert.equal(snnMarkup(null), '');
});

test('snn: every word in every language', () => {
  const keys = Object.keys(WORDS.en).sort();
  for (const l of LANGS) assert.deepEqual(Object.keys(WORDS[l] || {}).sort(), keys, l);
  for (const m of MODES) assert.ok(keys.includes('snn_' + m), m);
});

test('snn: the quality line shows once a next order was checked, as two percentages', () => {
  assert.ok(!snnMarkup({ mode: 'shadow', compared: 4, settled: 0 }).includes('data-snn-quality'));
  const html = snnMarkup({ mode: 'shadow', compared: 4, settled: 8, currentHitPm: 250, snnHitPm: 375 });
  assert.ok(html.includes('8 <span data-t="snnSettled">') && html.includes('25% <span data-t="snnQCurrent">') && html.includes('38% <span data-t="snnQNet">'), html);
});
