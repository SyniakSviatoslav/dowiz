import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  DESCRIPTIVE_ALLOWLIST,
  REGULATED_REGISTER,
  REGULATED_ANCHORS,
  isRegulatedTerm,
  selectDescriptiveLabels,
  regulatedSubsetActive,
} from '../characteristics.js';

// Guardrail #6 (council menu-characteristics-model) — the deterministic ratchet that gates the descriptive
// band: no descriptive label may carry a regulated nutrition/health meaning, and only reviewed-allowlist
// labels may render. RED if a regulated term enters the allowlist, or if the register is weakened so it
// stops catching a known regulated term. Must stay green before the descriptive flag may ever flip.
describe('characteristics — guardrail #6 (descriptive allowlist safety)', () => {
  it('every allowlisted descriptive label clears the regulated register (the core invariant)', () => {
    for (const label of DESCRIPTIVE_ALLOWLIST) {
      assert.equal(
        isRegulatedTerm(label),
        false,
        `"${label}" matches the regulated register — a descriptive chip may never assert a nutrition/health claim`,
      );
    }
  });

  // Anti-vacuity: with an empty allowlist the first test is trivially green, so prove the register is LIVE —
  // it must actually catch known regulated terms in BOTH en and sq. Weakening the register turns this red.
  it('the regulated register catches known regulated terms in en + sq (not vacuous)', () => {
    const mustCatch = [
      // en
      'light', 'lite', 'low-calorie', 'low fat', 'reduced sugar', 'diet', 'slimming',
      'filling', 'hearty', 'keeps you full', 'satisfying',
      'high in protein', 'source of fibre', 'rich in iron', 'protein-rich', 'high-protein',
      'healthy', 'wholesome', 'good for you', 'nutritious', 'guilt-free', 'superfood',
      // sq
      'i lehtë', 'pak kalori', 'i pasur me proteina', 'ngopës', 'i shëndetshëm', 'dietik',
    ];
    for (const term of mustCatch) {
      assert.equal(isRegulatedTerm(term), true, `the regulated register MUST catch "${term}"`);
    }
  });

  it('selectDescriptiveLabels emits only allowlisted, non-regulated labels (regulated terms never surface)', () => {
    // even regulated candidates passed in are dropped — defence in depth at the call site
    assert.deepEqual(selectDescriptiveLabels(['light', 'filling', 'healthy', 'made-up-label']), []);
    // anything emitted is a subset of the reviewed allowlist
    const out = selectDescriptiveLabels(['light', 'whatever']);
    for (const l of out) assert.ok(DESCRIPTIVE_ALLOWLIST.includes(l));
  });

  it('the register is non-empty (a removed register would silently disable the gate)', () => {
    assert.ok(REGULATED_REGISTER.length >= 10, 'regulated register suspiciously small — gate may be disabled');
  });
});

// Guardrail #2 — the regulated L2 subset (light/low-calorie/source-of-protein) stays DARK until a human
// supplies a verified per-market legal anchor. The subset can never be platform-asserted by an empty table,
// even with the flag on. RED if a verified anchor is added without updating this lock (i.e. activation is a
// deliberate, audited act, never silent).
describe('characteristics — guardrail #2 (regulated subset dark until verified anchors)', () => {
  it('regulatedSubsetActive is false in v1 (no verified anchors) even when the flag is ON', () => {
    assert.equal(regulatedSubsetActive(true), false, 'regulated labels must not render without a verified anchor');
    assert.equal(regulatedSubsetActive(false), false);
  });

  it('any ACTIVE anchor carries a citation + basis + sign-off (no claim without provenance)', () => {
    for (const a of REGULATED_ANCHORS) {
      const active = !!a.verifiedBy && a.verifiedBy.trim().length > 0;
      if (active) {
        assert.ok(a.citation?.trim(), `active anchor "${a.label}" missing a regulation citation`);
        assert.ok(a.basis?.trim(), `active anchor "${a.label}" missing a numeric basis`);
      }
    }
  });

  it('every regulated anchor label is itself a regulated term (by definition)', () => {
    for (const a of REGULATED_ANCHORS) {
      assert.equal(isRegulatedTerm(a.label), true, `anchor "${a.label}" should match the regulated register`);
    }
  });
});
