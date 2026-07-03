import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { hasDishData } from './dishNutrition.js';

// Proves the conditional-render decision for the product-detail "What's inside" section
// (MenuPage.tsx): shown ONLY when the dish actually carries nutrition/ingredient data, hidden
// entirely otherwise — never an empty panel, never a row of zeros.

describe('hasDishData — "What\'s inside" section visibility', () => {
  it('no macros, no ingredients → hidden', () => {
    assert.equal(hasDishData(undefined, undefined), false);
    assert.equal(hasDishData(null, null), false);
    assert.equal(hasDishData({}, []), false);
  });

  it('all-zero macros, no ingredients → hidden (never render a row of zeros)', () => {
    assert.equal(hasDishData({ kcal: 0 }, []), false);
  });

  it('a positive kcal alone → shown', () => {
    assert.equal(hasDishData({ kcal: 480 }, []), true);
    assert.equal(hasDishData({ kcal: 480 }, null), true);
  });

  it('a negative or non-finite kcal is treated as absent → hidden', () => {
    assert.equal(hasDishData({ kcal: -10 }, []), false);
    assert.equal(hasDishData({ kcal: NaN }, []), false);
    assert.equal(hasDishData({ kcal: Infinity }, []), false);
  });

  it('zero kcal but a real ingredient list → shown', () => {
    assert.equal(hasDishData({ kcal: 0 }, ['Tomato', 'Mozzarella']), true);
  });

  it('an ingredient array of only empty/null/undefined entries → hidden', () => {
    assert.equal(hasDishData({ kcal: 0 }, ['', null, undefined]), false);
  });

  it('both kcal and ingredients present → shown', () => {
    assert.equal(hasDishData({ kcal: 520 }, ['Beef', 'Bun']), true);
  });
});
