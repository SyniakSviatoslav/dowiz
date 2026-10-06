// `node --test workers/api/public/admin/sheaf-health-view.test.mjs`
// W-TASTE2 S7a: the radius row says the figures, says "nothing to compare" at 0, never a tone,
// in four languages; the health sheet loads it.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { WORDS, radiusSub } from './sheaf-health-view.js';

test('the row shows count, max and median per mille, and that it is information only', () => {
  const h = radiusSub({ contract: 'sheaf.radius.v1', compared: 3, maxPm: 820, medianPm: 140, overHalf: 1, acts: false });
  assert.match(h, /3 · 820 · 140/);
  assert.match(h, /data-t="shR_note"/);
  assert.doesNotMatch(h, /bad|warn|alert/, 'never an alarm');
});

test('nothing compared, an error, or no answer', () => {
  assert.match(radiusSub({ compared: 0, maxPm: 0, medianPm: 0 }), /data-t="shR_none"/);
  assert.equal(radiusSub({ error: 'image <x>' }), 'image &lt;x&gt;');
  assert.equal(radiusSub(null), '');
  assert.notEqual(radiusSub({ compared: 1, maxPm: 1, medianPm: 1 }), '', 'positive twin');
});

test('four languages, and the health sheet mounts the row', () => {
  for (const l of ['sq', 'en', 'uk', 'ru']) for (const k of ['shR_title', 'shR_none', 'shR_sub', 'shR_note']) assert.ok(WORDS[l][k], `${l}.${k}`);
  const more = readFileSync(new URL('./more.js', import.meta.url), 'utf8');
  assert.match(more, /import\('\/admin\/sheaf-health\.js'\)\.then\(m => m\.healthRow\(\$\('#hBody'\), h\.sheaf\)\)/);
});
