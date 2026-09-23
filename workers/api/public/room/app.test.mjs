// The room app's SHAPE, checked against the files: the service worker's shell
// is the real module graph, nothing inline that the CSP would drop, money
// drawn only through lib/money.js. Reads the source; runs no browser.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { dirname, resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOM = dirname(fileURLToPath(import.meta.url));
const PUBLIC = resolve(ROOM, '..');
const url = f => '/' + relative(PUBLIC, f).split('\\').join('/');
const read = f => readFileSync(f, 'utf8');

/// Every file reachable from `entry` by a static `import ... from '...'`.
function graph(entry) {
  const seen = new Set(), stack = [entry];
  while (stack.length) {
    const f = stack.pop();
    if (seen.has(f)) continue;
    seen.add(f);
    for (const m of read(f).matchAll(/^\s*import\s[^'"]*?from\s+['"]([^'"]+)['"]/gm)) {
      const spec = m[1];
      stack.push(spec.startsWith('/') ? resolve(PUBLIC, '.' + spec) : resolve(dirname(f), spec));
    }
  }
  return [...seen];
}

const SHELL = (() => {
  const src = read(resolve(ROOM, 'sw.js'));
  const body = src.slice(src.indexOf('const SHELL = ['), src.indexOf('];', src.indexOf('const SHELL = [')));
  return [...body.matchAll(/'([^']+)'/g)].map(m => m[1]);
})();

test('the service worker caches the whole static module graph of app.js', () => {
  const mods = graph(resolve(ROOM, 'app.js'));
  assert.ok(mods.length >= 10, 'graph walk found only ' + mods.length);
  for (const f of mods) {
    assert.ok(existsSync(f), 'imported file missing: ' + f);
    assert.ok(SHELL.includes(url(f)), url(f) + ' is imported but not in sw.js SHELL');
  }
});

test('every shell entry exists on disk (a 404 leaves the shell incomplete)', () => {
  for (const p of SHELL) {
    const f = p.endsWith('/') ? resolve(PUBLIC, '.' + p, 'index.html') : resolve(PUBLIC, '.' + p);
    assert.ok(existsSync(f), p + ' is in the shell and not on disk');
  }
  const html = read(resolve(ROOM, 'index.html'));
  for (const m of html.matchAll(/(?:href|src)="(\/[^"]+)"/g)) assert.ok(SHELL.includes(m[1]), m[1] + ' is loaded by index.html but not cached');
});

const sources = readdirSync(ROOM).filter(f => /\.(js|html)$/.test(f)).map(f => [f, read(resolve(ROOM, f))]);

test('nothing inline that style-src/script-src self would drop', () => {
  for (const [f, s] of sources) {
    assert.doesNotMatch(s, /\sstyle="/, f + ' writes a style= attribute (CSP drops it)');
    assert.doesNotMatch(s, /<style[\s>]/, f + ' has an inline <style>');
    assert.doesNotMatch(s, /\son[a-z]+="/, f + ' has an inline event handler');
  }
  assert.doesNotMatch(read(resolve(ROOM, 'index.html')), /<script(?![^>]*\ssrc=)[^>]*>/, 'inline <script> in index.html');
});

test('money is drawn only through lib/money.js: no Intl currency, no /100', () => {
  for (const [f, s] of sources) {
    const code = s.replace(/^\s*\/\/.*$/gm, '');
    assert.doesNotMatch(code, /Intl\.NumberFormat/, f);
    assert.doesNotMatch(code, /\/\s*100\b/, f + ' divides by 100');
    assert.doesNotMatch(code, /toFixed\(/, f);
  }
});

test('JS strings are delimited by ASCII quotes (rule 11)', () => {
  for (const [f, s] of sources.filter(([f]) => f.endsWith('.js'))) {
    assert.doesNotMatch(s, /[(,:=\[]\s*[\u2018\u2019\u201C\u201D][^\n]*[\u2018\u2019\u201C\u201D]\s*[,)\]}]/, f + ' has a typographic-quote-delimited string');
  }
});
