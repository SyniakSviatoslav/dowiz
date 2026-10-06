// `node --test workers/api/public/admin/sense-recipe-view.test.mjs`
// W-TASTE2 S7b: the dish sheet says what the recipe graph can say -- the draft and its source, or
// why there is none -- and "Use it" fills only what the owner left empty.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { RECIPE_WORDS, recipeMarkup, fillEmpty } from './sense-recipe-view.js';

const drafted = { state: 'drafted', draft: { v: 1, taste: { salty: 2 }, texture: {}, aroma: { smoky: 2 } },
  why: [{ key: 'a:smoky', from: 'recipe' }, { key: 't:salty', from: 'recipe' }], neighbours: 2, passes: 9 };

test('a drafted answer names its words, its neighbours and offers Use it', () => {
  const h = recipeMarkup(drafted);
  assert.match(h, /data-t="srFrom"/);
  assert.match(h, /data-t="sx_a_smoky"/);
  assert.match(h, /\(2 <span data-t="srShare">/);
  assert.match(h, /id="srUse"/);
  assert.match(h, /class="ui-btn ui-btn--ghost"[^>]*data-tour="sense.recipe"/, 'drawn with /lib/ui button (ui-adoption)');
});

test('no recipes, no recipe, no neighbours, faint: said in words, nothing to use', () => {
  for (const state of ['no-recipes', 'no-recipe', 'no-neighbours', 'faint']) {
    const h = recipeMarkup({ state });
    assert.match(h, new RegExp(`data-t="sr_${state}"`), state);
    assert.doesNotMatch(h, /srUse/, state);
  }
  assert.equal(recipeMarkup({ state: 'unreadable', error: 'x' }), '');
  assert.equal(recipeMarkup(null), '');
  for (const l of ['sq', 'en', 'uk', 'ru']) for (const k of ['srFrom', 'srUse', 'srShare', 'sr_no-recipes', 'sr_no-recipe', 'sr_no-neighbours', 'sr_faint']) assert.ok(RECIPE_WORDS[l][k], `${l}.${k}`);
});

test('Use it fills only what is empty; the dish sheet wires it', () => {
  const mine = { taste: { salty: 5 }, texture: {}, aroma: {} };
  assert.deepEqual(fillEmpty(mine, drafted), { taste: { salty: 5 }, texture: {}, aroma: { smoky: 2 } });
  assert.deepEqual(mine.aroma, {}, 'the owner\'s draft object is not mutated');
  const s = readFileSync(new URL('./sense.js', import.meta.url), 'utf8');
  assert.match(s, /recipeMarkup\(d\?\.recipe\)/);
  assert.match(s, /fillEmpty\(draft, d\.recipe\)/);
});
