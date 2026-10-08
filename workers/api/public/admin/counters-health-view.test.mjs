// `node --test workers/api/public/admin/counters-health-view.test.mjs`
// AX0: the counters row says the wake's reads and writes, the catch-ups the window could not
// answer, and the menu saves; never a tone; four languages; the health sheet mounts it.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { WORDS, countersSub } from './counters-health-view.js';

test('the row shows reads, writes, unanswered/total catch-ups and menu saves', () => {
  const h = countersSub({ since_total: 9, since_none: 2, cat_writes: 1, wake: { atMs: 1, reads: 40, writes: 3 } });
  assert.match(h, /40 · 3 · 2\/9 · 1/);
  assert.match(h, /data-t="cnt_note"/);
  assert.doesNotMatch(h, /bad|warn|alert/, 'never an alarm');
});

test('an error is said, no answer is nothing', () => {
  assert.equal(countersSub({ error: 'the object answered <500>' }), 'the object answered &lt;500&gt;');
  assert.equal(countersSub(null), '');
  assert.notEqual(countersSub({}), '', 'positive twin: an empty window is still a row of zeros');
});

test('four languages, and the health sheet mounts the row', () => {
  for (const l of ['sq', 'en', 'uk', 'ru']) for (const k of ['cnt_title', 'cnt_sub', 'cnt_note', 'cnt_ae']) assert.ok(WORDS[l][k], `${l}.${k}`);
  const more = readFileSync(new URL('./more.js', import.meta.url), 'utf8');
  assert.match(more, /import\('\/admin\/counters-health\.js'\)\.then\(m => m\.healthRow\(\$\('#hBody'\), h\.counters\)\)/);
});

test('W-AE: the points saved to Cloudflare analytics and the refusals, only when the object reports them', () => {
  const h = countersSub({ since_total: 0, wake: {}, ae: { binding: 'COUNTERS', points: 4, errors: 1, lastError: 'x' } });
  assert.match(h, /4 · 1<\/span> <span class="muted" data-t="cnt_ae">/);
  assert.doesNotMatch(h, /bad|warn|alert/, 'a refusal is a number, never an alarm');
  assert.doesNotMatch(countersSub({ wake: {} }), /cnt_ae/, 'an older object without `ae` shows no line');
});
