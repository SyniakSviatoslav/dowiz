// lib/ui/ui.css against the components that emit its classes, and against the
// rules the header of ui.css promises.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { VARIANTS } from './button.js';
import { TONES } from './core.js';

const here = new URL('.', import.meta.url);
const css = readFileSync(new URL('ui.css', here), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const rules = [...css.matchAll(/([^{}@]+)\{([^{}]*)\}/g)].map(m => ({ sel: m[1].trim(), body: m[2] }));
const has = cls => new RegExp(`\\.${cls}(?![\\w-])`).test(css);
const js = readdirSync(here).filter(f => f.endsWith('.js')).map(f => readFileSync(new URL(f, here), 'utf8')).join('\n');

test('ui.css: no literal colour; every colour is a token', () => {
  const hex = css.match(/#[0-9a-fA-F]{3,8}\b/g) || [];
  assert.deepEqual(hex, []);
  // positive twin: the scan does see a hex when there is one
  assert.ok(/#[0-9a-fA-F]{3,8}\b/.test('.x{color:#fff}'));
});

test('ui.css: every class a component emits has a rule', () => {
  const emitted = new Set([...js.matchAll(/['"` ](ui-[a-z0-9-]*[a-z0-9])(?=['"` ])/g)].map(m => m[1]));
  assert.ok(emitted.size > 30, `found ${emitted.size} classes -- the scan itself works`);
  const missing = [...emitted].filter(c => !has(c));
  assert.deepEqual(missing, []);
  for (const v of VARIANTS) assert.ok(has(`ui-btn--${v}`), `ui-btn--${v}`);
  for (const t of TONES.filter(t => t !== 'neutral')) {
    assert.ok(has(`ui-badge--${t}`) || t === 'neutral', `ui-badge--${t}`);
    assert.ok(has(`ui-toast--${t}`) || ['accent', 'info'].includes(t), `ui-toast--${t}`);
  }
});

test('ui.css: every interactive control is at least --tap (44px)', () => {
  const CONTROLS = ['ui-btn', 'ui-iconbtn', 'ui-chip', 'ui-seg-b', 'ui-tab', 'ui-row', 'ui-input'];
  for (const c of CONTROLS) {
    const r = rules.find(x => x.sel === `.${c}`);
    assert.ok(r, `.${c} has a base rule`);
    const mh = r.body.match(/min-height:([^;]+)/);
    assert.ok(mh, `.${c} sets min-height`);
    const v = mh[1].trim();
    const ok = /^var\(--(tap|tap-md|tap-lg|ui-btn-h)\)$/.test(v) || (parseInt(v, 10) >= 44 && v.endsWith('px'));
    assert.ok(ok, `.${c} min-height ${v} is under --tap`);
  }
  // nothing anywhere sets a px min-height under 44
  for (const m of css.matchAll(/min-height:\s*(\d+)px/g)) assert.ok(+m[1] >= 44, `min-height ${m[1]}px`);
});

test('ui.css: motion comes from tokens -- no raw duration or cubic-bezier at a call site', () => {
  for (const m of css.matchAll(/transition:([^;}]+)/g)) {
    assert.doesNotMatch(m[1], /cubic-bezier|\d+m?s\b/, `raw timing in transition: ${m[1].slice(0, 60)}`);
    assert.match(m[1], /var\(--motion-/, `transition without a --motion token: ${m[1].slice(0, 60)}`);
  }
});

test('ui.css: money never tweens', () => {
  for (const r of rules.filter(x => /\.ui-amount|\.money/.test(x.sel))) {
    assert.doesNotMatch(r.body, /transition|animation/, `${r.sel} animates`);
  }
});

test('ui.css: [hidden] beats a component display rule', () => {
  assert.match(css, /\[class\^="ui-"\]\[hidden\][^{]*\{display:none!important\}/);
});

test('ui.css: every looping animation is stopped under reduced motion', () => {
  const rm = css.match(/prefers-reduced-motion:reduce\)\{([\s\S]*)\}\s*$/);
  assert.ok(rm, 'a reduced-motion block closes the file');
  const loops = rules.filter(r => /animation:[^;]*infinite/.test(r.body)).map(r => r.sel);
  assert.ok(loops.length >= 3);
  for (const sel of loops) assert.ok(rm[1].includes(sel), `${sel} keeps looping under reduced motion`);
});

// An icon is a CSS mask keyed by class; a name /lib/icons.css does not define
// draws NOTHING, silently (the courier's radio marks were `circle` and
// `circle-check-filled` for months, and neither exists). So every name a
// component, the courier's screens or the gallery asks for is checked here.
test('icons: every icon name the system, the courier and the gallery use is drawn by /lib/icons.css', () => {
  const drawn = new Set([...readFileSync(new URL('../icons.css', here), 'utf8').matchAll(/\.ti-([a-z0-9-]+)/g)].map(m => m[1]));
  const src = js + ['../../courier/screens.js', '../../courier/app.js', '../../ui-gallery/gallery.js']
    .map(f => readFileSync(new URL(f, here), 'utf8')).join('\n');
  const named = new Set();
  // `icon: 'x'`, `icon: q ? 'x' : 'y'`, `iconEnd: 'x'` -- every string after `:` or `?`
  for (const m of src.matchAll(/\bicon(?:End)?\s*(:[^,})]+)/g))
    for (const q of m[1].matchAll(/[?:]\s*'([a-z0-9-]+)'/g)) named.add(q[1]);
  // `icon('x')`, a default `ic = 'x'`, and the courier's `toast(msg, 'x')`
  for (const m of src.matchAll(/\bicon\('([a-z0-9-]+)'|\bic\s*=\s*'([a-z0-9-]+)'|\btoast\([^;\n]*,\s*'([a-z0-9-]+)'\)/g))
    named.add(m[1] || m[2] || m[3]);
  assert.ok(named.size > 25, `found ${named.size} icon names -- the scan itself works`);
  const missing = [...named].filter(n => !drawn.has(n));
  assert.deepEqual(missing, [], 'icons that would draw nothing');
  // positive twin: a name that is not there is reported
  assert.equal(drawn.has('circle-check-filled'), false);
});
