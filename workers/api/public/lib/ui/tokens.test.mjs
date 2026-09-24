// lib/tokens.css: the colour roles are AA in both themes, and no old name was lost.
//
// Reads the CSS itself and RESOLVES it -- var() with fallbacks, color-mix() in
// srgb -- the way a page with no `--brand-*` tokens would, which is the case
// the fallbacks exist for. A surface that defines its own brand tokens is
// checked by its own contrast numbers; this is the system's floor.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const css = readFileSync(new URL('../tokens.css', import.meta.url), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const decls = body => Object.fromEntries([...body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)].map(m => [m[1], m[2].trim()]));
const block = re => { const m = css.match(re); assert.ok(m, `block ${re} exists`); return decls(m[1]); };
const light = block(/:root\s*\{([\s\S]*?)\n\}/);
const dark = { ...light, ...block(/:root\[data-theme="dark"\]\s*\{([\s\S]*?)\}/) };
const darkMedia = { ...light, ...block(/:root:not\(\[data-theme="light"\]\)\s*\{([\s\S]*?)\}/) };

const hex = h => { h = h.slice(1); if (h.length === 3) h = [...h].map(c => c + c).join(''); return [0, 2, 4].map(i => parseInt(h.slice(i, i + 2), 16)); };
function split(args){ const out = []; let d = 0, cur = ''; for (const c of args) { if (c === '(') d++; if (c === ')') d--; if (c === ',' && !d) { out.push(cur.trim()); cur = ''; } else cur += c; } out.push(cur.trim()); return out; }
function resolve(v, scope, depth = 0){
  assert.ok(depth < 20, `cycle resolving ${v}`);
  v = v.trim();
  if (/^#[0-9a-f]{3,6}$/i.test(v)) return hex(v);
  let m = v.match(/^var\((.*)\)$/s);
  if (m) { const [name, fb] = split(m[1]); return name in scope ? resolve(scope[name], scope, depth + 1) : resolve(fb, scope, depth + 1); }
  m = v.match(/^color-mix\(in srgb,(.*)\)$/s);
  if (m) {
    const [a, b] = split(m[1]);
    const pa = a.match(/^(.*?)\s+([\d.]+)%$/), pb = b.match(/^(.*?)\s+([\d.]+)%$/);
    const ca = resolve(pa ? pa[1] : a, scope, depth + 1), cb = resolve(pb ? pb[1] : b, scope, depth + 1);
    const wa = pa ? +pa[2] / 100 : pb ? 1 - pb[2] / 100 : .5;
    return ca.map((x, i) => Math.round(x * wa + cb[i] * (1 - wa)));
  }
  throw new Error(`cannot resolve ${v}`);
}
const lum = rgb => { const [r, g, b] = rgb.map(c => { c /= 255; return c <= .03928 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4; }); return .2126 * r + .7152 * g + .0722 * b; };
const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + .05) / (y + .05); };

const PAIRS = [
  ['--ui-fg', '--ui-bg', 4.5], ['--ui-fg', '--ui-surface', 4.5], ['--ui-fg', '--ui-surface-2', 4.5],
  ['--ui-fg-muted', '--ui-surface', 4.5], ['--ui-fg-muted', '--ui-bg', 4.5],
  ['--ui-on-accent', '--ui-accent', 4.5],
  ['--ui-success-ink', '--ui-surface', 4.5], ['--ui-warning-ink', '--ui-surface', 4.5],
  ['--ui-danger-ink', '--ui-surface', 4.5], ['--ui-info-ink', '--ui-surface', 4.5],
  ['--ui-on-tone', '--color-success', 4.5], ['--ui-on-tone', '--color-warning', 4.5], ['--ui-on-danger', '--color-danger', 4.5],
  // non-text: the focus ring and the primary button's edge against the surface
  ['--ui-focus', '--ui-surface', 3], ['--ui-accent-edge', '--ui-surface', 3],
  // the toast is ink-on-ink inverted
  ['--ui-surface', '--ui-fg', 4.5],
];

for (const [name, scope] of [['light', light], ['dark', dark], ['dark (media)', darkMedia]]) {
  test(`tokens: every role pair is AA in ${name}`, () => {
    const report = [];
    for (const [f, b, min] of PAIRS) {
      const r = ratio(resolve(`var(${f})`, scope), resolve(`var(${b})`, scope));
      report.push(`${f}/${b} ${r.toFixed(2)}`);
      assert.ok(r >= min, `${name}: ${f} on ${b} is ${r.toFixed(2)}:1, needs ${min}`);
    }
    console.log(`# ${name}: ${report.join(' · ')}`);
  });
}

test('tokens: the names surfaces already use are all still defined', () => {
  const OLD = ['--space-1', '--space-2', '--space-3', '--space-4', '--space-5', '--space-6', '--space-8', '--space-12', '--space-16',
    '--radius-sm', '--radius-md', '--radius-lg', '--radius-full', '--elevation-1', '--elevation-2', '--elevation-3',
    '--measure', '--measure-tight', '--ease-snap', '--ease-tide', '--ease-spring', '--color-success', '--color-warning',
    '--color-danger', '--color-info', '--font-mono', '--tap', '--text-ratio', '--text-sm', '--text-xs', '--text-lg',
    '--text-xl', '--text-2xl', '--text-3xl', '--leading-tight', '--leading-normal', '--weight-medium',
    '--weight-semibold', '--weight-bold', '--z-sticky', '--z-toast'];
  for (const n of OLD) assert.ok(n in light, `${n} is still defined`);
  // positive twin: a name that was never there is not reported as present
  assert.equal('--never-a-token' in light, false);
});

test('tokens: reduced motion zeroes every --motion-* duration', () => {
  const m = css.match(/prefers-reduced-motion:\s*reduce\)\s*\{\s*:root\s*\{([^}]*)\}/);
  assert.ok(m, 'a reduced-motion :root block exists');
  const zeroed = decls(m[1]);
  for (const n of Object.keys(light).filter(k => k.startsWith('--motion-'))) assert.equal(zeroed[n], '0ms', n);
});
