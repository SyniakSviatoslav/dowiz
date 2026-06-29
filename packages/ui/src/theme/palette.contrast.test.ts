import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { derivePalette, contrastRatio, parseColor } from './palette.js';

// Guardrail (sellable-polish / branding AA) — derivePalette must NEVER emit an illegible storefront,
// for ANY brand colour a tenant picks. The bug this closes: tokens.css derived the CTA fill as a naive
// `color-mix(primary 85%, black)` with hard-coded WHITE text, so a pale brand primary shipped white text
// on a pale button (sub-AA, invisible prices/CTAs). derivePalette now emits an AA-guaranteed
// {primaryStrong, onPrimary} pair, plus the existing AA {text/bg, primaryReadable/surface}.
//
// red→green: the RED arm proves the OLD naive default fails AA on a pale brand; the GREEN arm proves
// the derived pair (and every other text/surface pair) clears AA across light AND dark brands.

const rgb = (s: string) => parseColor(s)!;
const WHITE = rgb('#ffffff');

// Brands that broke the old default: a pale primary on a light bg ships white-on-pale CTAs.
const PALE_BRANDS = [
  { primary: '#ffe08a', bg: '#ffffff' }, // light amber
  { primary: '#f8c8dc', bg: '#fffafc' }, // pale rose
  { primary: '#bfe3c0', bg: '#ffffff' }, // pale mint
  { primary: '#fde047', bg: '#fffef5' }, // yellow
];
// A spread of normal brands (incl. dark) the gate must also keep AA.
const NORMAL_BRANDS = [
  { primary: '#C1121F', bg: '#fffafc' }, // crimson (the demo)
  { primary: '#0D9488', bg: '#ffffff' }, // teal
  { primary: '#ea4f16', bg: '#121212' }, // food-dark on dark
  { primary: '#B45309', bg: '#1a1a1a' }, // gold on dark
];

describe('derivePalette — branding never ships an illegible storefront', () => {
  it('RED: the old naive CTA default (primary×85%+black, white text) is sub-AA on a pale brand', () => {
    // Replicate tokens.css `color-mix(in srgb, var(--brand-primary) 85%, #000)` + hard-coded white text.
    const naiveStrong = (p: { r: number; g: number; b: number }) => ({ r: p.r * 0.85, g: p.g * 0.85, b: p.b * 0.85 });
    const failures = PALE_BRANDS.filter((b) => contrastRatio(WHITE, naiveStrong(rgb(b.primary))) < 4.5);
    assert.ok(failures.length > 0, 'expected the naive default to fail AA on at least one pale brand (the bug)');
  });

  it('GREEN: derived {onPrimary, primaryStrong} clears AA (4.5:1) for every brand', () => {
    for (const b of [...PALE_BRANDS, ...NORMAL_BRANDS]) {
      const p = derivePalette({ primary: b.primary, bg: b.bg });
      const cta = contrastRatio(rgb(p.onPrimary!), rgb(p.primaryStrong!));
      assert.ok(cta >= 4.5, `CTA text/fill ${cta.toFixed(2)}:1 < 4.5 for ${b.primary} on ${b.bg}`);
    }
  });

  it('GREEN: body text on bg and primary-as-text on surface also clear AA for every brand', () => {
    for (const b of [...PALE_BRANDS, ...NORMAL_BRANDS]) {
      const p = derivePalette({ primary: b.primary, bg: b.bg });
      const body = contrastRatio(rgb(p.text), rgb(p.bg));
      const priceText = contrastRatio(rgb(p.primaryReadable!), rgb(p.surface));
      assert.ok(body >= 4.5, `body text ${body.toFixed(2)}:1 < 4.5 for ${b.primary}`);
      assert.ok(priceText >= 4.5, `primary-readable ${priceText.toFixed(2)}:1 < 4.5 for ${b.primary}`);
    }
  });
});
