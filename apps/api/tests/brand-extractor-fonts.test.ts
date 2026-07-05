import test from 'node:test';
import assert from 'node:assert/strict';
import { detectFonts } from '../src/lib/brand-extractor.js';
// Leaf module (no React) — the storefront font allowlist is the source of truth.
import { FONT_ALLOWLIST } from '../../../packages/ui/dist/theme/fonts.js';

// Tier-1 guardrail: website font extraction must (a) map a site's Google-Fonts / font-family usage to a
// storefront-renderable allowlist id, (b) IGNORE fonts we cannot load (so we never seed an unloadable
// font), and (c) cover EVERY allowlist family (the FONT_TABLE mirror can't drift out of sync).

test('detectFonts maps Google-Fonts links to allowlist ids (heading + body)', () => {
  const html = `<link href="https://fonts.googleapis.com/css2?family=Fraunces:wght@400;700&family=DM+Sans:wght@400;500&display=swap" rel="stylesheet">`;
  const { headingFont, bodyFont } = detectFonts(html, '');
  assert.equal(headingFont, 'fraunces');
  assert.equal(bodyFont, 'dmsans');
});

test('detectFonts reads font-family on heading vs body selectors', () => {
  const css = `h1, h2 { font-family: 'Playfair Display', serif; } body { font-family: 'Inter', sans-serif; }`;
  const { headingFont, bodyFont } = detectFonts('', css);
  assert.equal(headingFont, 'playfair');
  assert.equal(bodyFont, 'inter');
});

test('detectFonts ignores families the storefront cannot render (off-allowlist)', () => {
  const html = `<link href="https://fonts.googleapis.com/css2?family=Comic+Neue&family=Lobster" rel="stylesheet">
    <style>h1{font-family:'Comic Sans MS',cursive} body{font-family:Arial}</style>`;
  const { headingFont, bodyFont } = detectFonts(html, '');
  assert.equal(headingFont, undefined);
  assert.equal(bodyFont, undefined);
});

test('FONT_TABLE covers every storefront allowlist family and agrees on the id', () => {
  // Drive detectFonts with a Google-Fonts link for each allowlist family; a heading/both face must
  // resolve as headingFont, a body/both face as bodyFont — proving the extractor mirror is in sync.
  for (const [id, spec] of Object.entries(FONT_ALLOWLIST) as Array<[string, any]>) {
    const html = `<link href="https://fonts.googleapis.com/css2?family=${spec.family.replace(/ /g, '+')}" rel="stylesheet">`;
    const got = detectFonts(html, '');
    const resolved = got.headingFont ?? got.bodyFont;
    assert.equal(resolved, id, `allowlist font "${spec.family}" (${id}) must be recognised by the extractor`);
  }
});
