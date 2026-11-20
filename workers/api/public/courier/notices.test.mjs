// Every tap a courier makes ends in one notice: saved (or the screen's own,
// more specific word), queued when the network never carried it, and not
// saved with the reason in the courier's language when the hub refused it.
// `node --test workers/api/public/courier/notices.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { register } from 'node:module';

// courier/i18n.js imports these by absolute path; the words file is the real one.
const WORDS_URL = new URL('../lib/outcome-words.js', import.meta.url).href;
register('data:text/javascript,' + encodeURIComponent(`
  const F = { '/store/storage.js': 'export const safeGet = () => null; export const safeSet = () => {};' };
  export async function resolve(spec, ctx, next){
    if (spec === '/lib/outcome-words.js') return { url: ${JSON.stringify(WORDS_URL)}, shortCircuit: true };
    return F[spec] ? { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true } : next(spec, ctx);
  }`));
globalThis.navigator ??= { languages: ['en'], language: 'en' };

const APP = readFileSync(new URL('./app.js', import.meta.url), 'utf8');

test('the courier speaks every word a notice says, in every language', async () => {
  const { T, LANGS } = await import('./i18n.js');
  const { WORDS } = await import('../lib/outcome-words.js');
  for (const l of LANGS) for (const k of Object.keys(WORDS.en)) assert.equal(T[l][k], WORDS[l][k], `${l}.${k}`);
});

test('a refused tap is told through the one notice, never as the hub\'s bare words', () => {
  // The poll's own failure is a read, not a tap: the one place the raw message may stay.
  const raw = APP.match(/toast\(String\(e\.message \|\| e\)/g) || [];
  assert.equal(raw.length, 1, `raw hub words toasted at ${raw.length} sites; only the poll may`);
  assert.ok(/const refusedText = e =>/.test(APP) &&(APP.match(/refusedText\(e\)/g) || []).length >= 7, 'every tap\'s catch says not saved and why');
});

test('a landed tap is announced, unless its screen says something more specific', () => {
  assert.ok(/unlessToasted\(/.test(APP), 'tapped() announces a landed write after the turn');
  for (const site of [/\/courier\/shift'/, /\/chat`, \{ method: 'POST'/]) {
    const at = APP.search(site);
    assert.ok(at > 0 && /landedNotice\(\)/.test(APP.slice(at, at + 200)), `${site} announces its landing`);
  }
});
