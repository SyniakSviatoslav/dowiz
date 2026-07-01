import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  FONT_ALLOWLIST,
  DEFAULT_FONT_PAIRING,
  isFontId,
  fontIdsForRole,
  fontStack,
  fontPairingForCuisine,
  googleFontsHref,
} from './fonts.js';

// Guardrail — the font system's core invariant: NOTHING selectable or seeded may reference a font
// outside the allowlist. An orphan id ⇒ a font that is never loaded ⇒ a silent fallback on the
// storefront (the exact bug class this feature exists to kill). Also proves the egress-safe loader
// only ever emits allowlisted Google-Fonts URLs.

const IDS = Object.keys(FONT_ALLOWLIST);

describe('font allowlist invariants', () => {
  it('the default pairing references real allowlist ids with sane roles', () => {
    assert.ok(isFontId(DEFAULT_FONT_PAIRING.heading), 'default heading ∈ allowlist');
    assert.ok(isFontId(DEFAULT_FONT_PAIRING.body), 'default body ∈ allowlist');
    assert.notEqual(FONT_ALLOWLIST[DEFAULT_FONT_PAIRING.heading].role, 'body', 'default heading not body-only');
    assert.notEqual(FONT_ALLOWLIST[DEFAULT_FONT_PAIRING.body].role, 'heading', 'default body not heading-only');
  });

  it('every cuisine seed maps to allowlisted, role-appropriate ids', () => {
    // A representative sweep of the cuisines the demo-builder passes + unknowns (→ default).
    const cuisines = [
      'italian', 'Pizzeria', 'sushi', 'japanese', 'burger', 'american', 'cafe', 'bakery',
      'kebab', 'street', 'fine-dining', 'fine_dining', 'seafood', 'unknown-xyz', '', null, undefined,
    ];
    for (const c of cuisines) {
      const { heading, body } = fontPairingForCuisine(c as any);
      assert.ok(isFontId(heading), `heading id for "${c}" ∈ allowlist`);
      assert.ok(isFontId(body), `body id for "${c}" ∈ allowlist`);
      assert.notEqual(FONT_ALLOWLIST[heading].role, 'body', `heading for "${c}" is not a body-only face`);
      assert.notEqual(FONT_ALLOWLIST[body].role, 'heading', `body for "${c}" is not a heading-only face`);
    }
  });

  it('fontIdsForRole returns only allowlisted ids matching the role', () => {
    for (const role of ['heading', 'body'] as const) {
      const ids = fontIdsForRole(role);
      assert.ok(ids.length > 0, `${role} has selectable fonts`);
      for (const id of ids) {
        assert.ok(IDS.includes(id), `${id} ∈ allowlist`);
        const r = FONT_ALLOWLIST[id].role;
        assert.ok(r === role || r === 'both', `${id} role fits ${role}`);
      }
    }
  });

  it('fontStack resolves valid ids and falls back safely for junk', () => {
    assert.equal(fontStack('fraunces', 'heading'), FONT_ALLOWLIST.fraunces.stack);
    // Unknown / tampered id → the default face for that role, never empty / never the raw input.
    const junk = fontStack('<script>' as any, 'heading');
    assert.equal(junk, FONT_ALLOWLIST[DEFAULT_FONT_PAIRING.heading].stack);
    assert.ok(!junk.includes('<script>'));
  });

  it('googleFontsHref is egress-safe: base ids → null, non-base → allowlisted google URL, junk dropped', () => {
    // Base families are already in index.html — nothing to inject.
    assert.equal(googleFontsHref(['playfair', 'inter', 'dmsans']), null);
    // A non-base id yields a fonts.googleapis.com URL built from ITS allowlist spec only.
    const href = googleFontsHref(['bebas', 'spacegrotesk'])!;
    assert.match(href, /^https:\/\/fonts\.googleapis\.com\/css2\?/);
    assert.ok(href.includes(FONT_ALLOWLIST.bebas.googleSpec));
    assert.ok(href.includes(FONT_ALLOWLIST.spacegrotesk.googleSpec));
    // A tampered/unknown id can never produce a fetch.
    assert.equal(googleFontsHref(['https://evil.example/x', 'not-a-font']), null);
  });
});
