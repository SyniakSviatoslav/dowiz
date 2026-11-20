// Every write a guest makes ends in one notice: saved or the screen's own word
// in green, not saved and why in red, in the guest's language, never the hub's
// bare English. `node --test workers/api/public/store/notices.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { register } from 'node:module';

// store/i18n.js imports these by absolute path; the words file is the real one.
const WORDS_URL = new URL('../lib/outcome-words.js', import.meta.url).href;
register('data:text/javascript,' + encodeURIComponent(`
  const F = { '/store/storage.js': 'export const safeGet = () => null; export const safeSet = () => {};' };
  export async function resolve(spec, ctx, next){
    if (spec === '/lib/outcome-words.js') return { url: ${JSON.stringify(WORDS_URL)}, shortCircuit: true };
    return F[spec] ? { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true } : next(spec, ctx);
  }`));
globalThis.navigator ??= { languages: ['en'], language: 'en' };
const src = f => readFileSync(new URL(`./${f}`, import.meta.url), 'utf8');

test('the storefront speaks every word a notice says, in every language', async () => {
  const { T, LANGS } = await import('./i18n.js');
  const { WORDS } = await import('../lib/outcome-words.js');
  for (const l of LANGS) for (const k of Object.keys(WORDS.en)) assert.equal(T[l]?.[k] ?? T.en[k], WORDS[l][k], `${l}.${k}`);
});

test('a refused write is told through the one notice, never as the hub\'s bare words', () => {
  for (const f of ['chat.js', 'track.js', 'booking-mine.js']) {
    const s = src(f);
    assert.doesNotMatch(s, /toast\((esc\()?String\(e\.message/, `${f} toasts the hub's bare words`);
    assert.match(s, /toast\(refused\(e\)/, `${f} says not saved and why`);
  }
});

test('a notice carries its tone: success green, refusal red', () => {
  assert.match(src('ui.js'), /export function toast\(msg, \{ tone, icon \} = \{\}\)/, 'the toaster takes a tone');
  assert.match(src('booking.js'), /toast\(t\('bkSent'\), OK\)/, 'a sent booking is green');
  assert.match(src('booking-mine.js'), /toast\(t\('bkCancelled'\), OK\)/, 'a cancelled booking is green');
  assert.match(src('track.js'), /landed\(\)/, 'a sent note is announced');
});
