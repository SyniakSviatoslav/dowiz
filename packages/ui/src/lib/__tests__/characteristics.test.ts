import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  DESCRIPTIVE_ALLOWLIST,
  REGULATED_REGISTER,
  REGULATED_ANCHORS,
  isRegulatedTerm,
  selectDescriptiveLabels,
  regulatedSubsetActive,
  computeAllergenSurface,
  compareDishes,
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

// Guardrails #5 (floor) + #4-positive (never drop a warning) — the DETAIL-FLOOR-ONLY allergen surface.
describe('characteristics — allergen surface (#5 floor + #4-positive)', () => {
  it('#5: no allergens known → hasInfo:false (caller renders the floor, never a blank/clean state)', () => {
    assert.deepEqual(computeAllergenSurface({ allergen_status: 'unset' }, []), { known: [], hasInfo: false });
    assert.deepEqual(computeAllergenSurface(null, []), { known: [], hasInfo: false });
  });

  it('#4-positive: a recipe-derived allergen is ALWAYS surfaced regardless of attestation status', () => {
    for (const status of ['unset', 'none', 'listed', undefined]) {
      const s = computeAllergenSurface({ allergen_status: status }, ['nuts']);
      assert.ok(s.known.includes('nuts'), `recipe allergen dropped under status=${String(status)}`);
      assert.equal(s.hasInfo, true);
    }
  });

  it('conservative union — owner declaration ∪ recipe-derived (a warning is never hidden)', () => {
    const s = computeAllergenSurface({ allergen_status: 'listed', declared_allergens: ['nuts'] }, ['milk']);
    assert.deepEqual([...s.known].sort(), ['milk', 'nuts']);
  });

  it('absence is never asserted — an owner "none" attestation yields no claim, not "free-from"', () => {
    const s = computeAllergenSurface({ allergen_status: 'none', declared_allergens: [] }, []);
    assert.equal(s.hasInfo, false);
    assert.equal(s.known.length, 0);
  });
});

// Guardrail #12 (STEP-0 single-allergen-source, permanent ratchet) — the storefront filter predicate and
// every render path derive the allergen set from computeAllergenSurface (declared∪recipe), NEVER a
// recipe-only accessor. The load-bearing case the FULL-BUILD round grounded in live code (FB-C1): a
// declared-only allergen dish (owner listed it, no recipe/bom) must be RETAINED by a "contains X" view and
// must SURFACE the warning — a recipe-only predicate would silently drop it (a live false-negative).
describe('characteristics — single allergen source (#12 STEP-0)', () => {
  // Mirrors MenuPage.allergenSurfaceOf: computeAllergenSurface(attributes, recipeAllergens(p)).
  const surfaceOf = (attributes: any, recipe: string[] = []) => computeAllergenSurface(attributes, recipe);
  // The exact storefront filter predicate (MenuPage.tsx): allergenSurfaceOf(p).known.includes(target).
  const containsFilter = (attributes: any, recipe: string[], target: string) =>
    surfaceOf(attributes, recipe).known.includes(target);

  it('#12: a DECLARED-ONLY allergen dish (no recipe/bom) is RETAINED by a "contains milk" view', () => {
    const declaredOnly = { allergen_status: 'listed', declared_allergens: ['milk'] };
    assert.equal(containsFilter(declaredOnly, [], 'milk'), true, 'declared-only dish dropped by contains-milk — FB-C1 regression');
    // and it surfaces the warning (hasInfo true, milk in known) on every non-gated surface
    const s = surfaceOf(declaredOnly, []);
    assert.equal(s.hasInfo, true);
    assert.ok(s.known.includes('milk'));
  });

  it('#12: a recipe-only predicate would have DROPPED it — the regression this guardrail blocks', () => {
    // Proof the old recipe-only basis is unsafe: the recipe array alone has no "milk".
    const recipeOnly: string[] = [];
    assert.equal(recipeOnly.includes('milk'), false); // recipe-only → false (dropped)
    // The converged predicate keeps it (declared∪recipe) → true.
    assert.equal(containsFilter({ allergen_status: 'listed', declared_allergens: ['milk'] }, recipeOnly, 'milk'), true);
  });
});

// Guardrail #8 — comparison of two dishes: allergen cells explicit-BOTH (never blank), directional markers
// only on the non-regulated axes, never a macro/global winner.
describe('characteristics — comparison (#8 explicit-both + no regulated/global winner)', () => {
  const nuts = { id: 'a', name: 'A', price: 500, prepTimeMinutes: 10, attributes: { allergen_status: 'listed', declared_allergens: ['nuts'] } };
  const nodata = { id: 'b', name: 'B', price: 700, prepTimeMinutes: 20, attributes: { allergen_status: 'unset' } };

  it('#8: BOTH dishes get an explicit allergen surface — a no-data dish never renders blank/"—"', () => {
    const c = compareDishes(nuts, nodata);
    assert.equal(c.allergens.a.hasInfo, true);
    assert.ok(c.allergens.a.known.includes('nuts'));
    assert.equal(c.allergens.b.hasInfo, false); // caller renders the floor — never blank (no "free-from" by contrast)
    assert.ok(Array.isArray(c.allergens.b.known)); // surface always present, not undefined
  });

  it('directional "lower wins" only on price + prep-time (neutral facts)', () => {
    const c = compareDishes(nuts, nodata);
    assert.equal(c.price.lower, 'a'); // 500 < 700 — cheaper
    assert.equal(c.prepTime.lower, 'a'); // 10 < 20 — faster
  });

  it('no fabricated winner when an axis value is missing', () => {
    const c = compareDishes({ ...nuts, prepTimeMinutes: null }, nodata);
    assert.equal(c.prepTime.lower, null);
  });

  it('taste is side-by-side, never a winner; no macro axis carries a winner (kcal arrow = regulated verdict)', () => {
    const c = compareDishes(nuts, nodata) as Record<string, unknown>;
    assert.ok(!('lower' in (c.taste as object)));
    assert.ok(!('macros' in c) && !('kcal' in c), 'no macro winner axis may exist');
  });
});
