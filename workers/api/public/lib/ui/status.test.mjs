// The status pill (ui.css `.ui-status`): the word is the status hue mixed into
// the ink, on a tint of the same hue mixed into the surface -- both in OKLCH.
// This resolves those two mixes for every status the FSM can name, on the
// system's own light and dark grounds, and holds the WORD to AA (4.5:1).
//
// The percentages are READ FROM ui.css, not repeated here: a later edit that
// makes the tint stronger or the ink weaker is measured, not trusted.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { STATUSES } from './badge.js';

const here = new URL('.', import.meta.url);
const ui = readFileSync(new URL('ui.css', here), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
const tokens = readFileSync(new URL('../tokens.css', here), 'utf8');

const rule = ui.match(/\.ui-status\{([^}]*)\}/)[1];
const TINT = +rule.match(/background:color-mix\(in oklch,var\(--st,var\(--ui-fg-muted\)\) (\d+)%,var\(--ui-surface\)\)/)[1] / 100;
const INK = +rule.match(/[;\s]color:color-mix\(in oklch,var\(--st,var\(--ui-fg-muted\)\) (\d+)%,var\(--ui-fg\)\)/)[1] / 100;
const HUE = Object.fromEntries([...tokens.matchAll(/--st-([A-Z_]+):\s*(#[0-9A-Fa-f]{6})/g)].map(m => [m[1], m[2]]));

// ── sRGB <-> OKLab (Björn Ottosson's published matrices) ──
const hex = h => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16) / 255);
const lin = c => c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4;
const gam = c => c <= .0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - .055;
function toLab(rgb){
  const [r, g, b] = rgb.map(lin);
  const l = Math.cbrt(.4122214708 * r + .5363325363 * g + .0514459929 * b);
  const m = Math.cbrt(.2119034982 * r + .6806995451 * g + .1073969566 * b);
  const s = Math.cbrt(.0883024619 * r + .2817188376 * g + .6299787005 * b);
  return [.2104542553 * l + .793617785 * m - .0040720468 * s, 1.9779984951 * l - 2.428592205 * m + .4505937099 * s, .0259040371 * l + .7827717662 * m - .808675766 * s];
}
function fromLab([L, a, b]){
  const l = (L + .3963377774 * a + .2158037573 * b) ** 3, m = (L - .1055613458 * a - .0638541728 * b) ** 3, s = (L - .0894841775 * a - 1.291485548 * b) ** 3;
  return [4.0767416621 * l - 3.3077115913 * m + .2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - .3413193965 * s, -.0041960863 * l - .7034186147 * m + 1.707614701 * s]
    .map(c => Math.min(1, Math.max(0, gam(c))));
}
const toLch = rgb => { const [L, a, b] = toLab(rgb); return [L, Math.hypot(a, b), Math.atan2(b, a)]; };
/// CSS `color-mix(in oklch, A p, B)`: L, C and hue (the shorter arc) interpolated.
function mix(A, p, B){
  const [La, Ca, Ha] = toLch(A), [Lb, Cb, Hb] = toLch(B);
  let dh = Hb - Ha; if (dh > Math.PI) dh -= 2 * Math.PI; if (dh < -Math.PI) dh += 2 * Math.PI;
  // a colour with (almost) no chroma has no hue to travel from: CSS calls it powerless
  const H = Ca < 1e-4 ? Hb : Cb < 1e-4 ? Ha : Ha + dh * (1 - p);
  const L = La * p + Lb * (1 - p), C = Ca * p + Cb * (1 - p);
  return fromLab([L, C * Math.cos(H), C * Math.sin(H)]);
}
const lum = rgb => { const [r, g, b] = rgb.map(lin); return .2126 * r + .7152 * g + .0722 * b; };
const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + .05) / (y + .05); };

export function pillRatio(hue, surface, fg){
  const bg = mix(hex(hue), TINT, hex(surface)), ink = mix(hex(hue), INK, hex(fg));
  return ratio(ink, bg);
}

// The system's own grounds (tokens.css fallbacks) and the console's, which
// every status on the owner's screen is drawn on.
const GROUNDS = {
  'system light': ['#ffffff', '#061b1a'], 'system dark': ['#1b1e20', '#f3f1ec'],
  'console light': ['#ffffff', '#15161a'], 'console dark': ['#1c1d21', '#f2f1ec'],
};

test('status pill: both mixes are read from ui.css, and every FSM status has a hue', () => {
  assert.ok(TINT > 0 && TINT < .3, `tint ${TINT}`);
  assert.ok(INK > 0 && INK < .7, `ink ${INK}`);
  for (const s of STATUSES) assert.match(HUE[s] || '', /^#/, `--st-${s}`);
});

for (const [name, [surface, fg]] of Object.entries(GROUNDS)) {
  test(`status pill: every word is AA on its own tint (${name})`, () => {
    const rows = STATUSES.map(s => [s, pillRatio(HUE[s], surface, fg)]).sort((a, b) => a[1] - b[1]);
    console.log(`# ${name}: ${rows.map(([s, r]) => `${s} ${r.toFixed(2)}`).join(' · ')}`);
    for (const [s, r] of rows) assert.ok(r >= 4.5, `${name}: ${s} is ${r.toFixed(2)}:1`);
  });
}

test('status pill: the measurement can fail (a hue as its own word on its own tint)', () => {
  // positive twin: the raw amber on a light tint of itself is well under AA,
  // which is exactly why the word is mixed into the ink.
  const amber = hex(HUE.PREPARING);
  assert.ok(ratio(amber, mix(amber, TINT, hex('#ffffff'))) < 3);
});
