// PURE. "Suggest" from the dish's RECIPE (W-TASTE2 S7b), as the dish sheet shows it beside the
// draft from the words: the hub's harmonic extension over the dish--supply graph
// (services/catalogue/sense/recipe.rs). A second source, never merged by itself: the owner taps
// "Use it" to fill what is still empty, and saves as always. With no recipes it says so.
// Relative imports only (sense-recipe-view.test.mjs). ASCII QUOTES ONLY in this file.

import { wordKey } from '../lib/sense-words.js';
import { button } from '../lib/ui/button.js';

export const RECIPE_WORDS = {
  sq: { srFrom: 'Nga receta', srUse: 'Përdore', srShare: 'pjata me shije të deklaruar ndajnë përbërësit e saj',
        'sr_no-recipes': 'Ende asnjë recetë: kjo sugjerim shfaqet kur pjatat kanë receta.', 'sr_no-recipe': 'Kjo pjatë nuk ka recetë.',
        'sr_no-neighbours': 'Asnjë pjatë me shije të deklaruar nuk ndan përbërësit e saj.', sr_faint: 'Receta nuk tregon asgjë të qartë.' },
  en: { srFrom: 'From the recipe', srUse: 'Use it', srShare: 'dishes with a declared taste share its supplies',
        'sr_no-recipes': 'No recipes yet: this suggestion appears once dishes have recipes.', 'sr_no-recipe': 'This dish has no recipe.',
        'sr_no-neighbours': 'No dish with a declared taste shares its supplies.', sr_faint: 'The recipe says nothing clear.' },
  uk: { srFrom: 'З рецепта', srUse: 'Використати', srShare: 'страв із заявленим смаком мають спільні з нею продукти',
        'sr_no-recipes': 'Рецептів ще немає: ця підказка з\'явиться, коли страви матимуть рецепти.', 'sr_no-recipe': 'У цієї страви немає рецепта.',
        'sr_no-neighbours': 'Жодна страва із заявленим смаком не має спільних з нею продуктів.', sr_faint: 'Рецепт нічого чіткого не каже.' },
  ru: { srFrom: 'Из рецепта', srUse: 'Использовать', srShare: 'блюд с заявленным вкусом имеют общие с ним продукты',
        'sr_no-recipes': 'Рецептов пока нет: эта подсказка появится, когда у блюд будут рецепты.', 'sr_no-recipe': 'У этого блюда нет рецепта.',
        'sr_no-neighbours': 'Ни одно блюдо с заявленным вкусом не имеет общих с ним продуктов.', sr_faint: 'Рецепт ничего чёткого не говорит.' },
};

const STATES = ['no-recipes', 'no-recipe', 'no-neighbours', 'faint'];

/// `r` = the Suggest answer's `recipe`. '' for a missing answer or a block the hub could not read.
export function recipeMarkup(r){
  if (!r || typeof r !== 'object') return '';
  if (STATES.includes(r.state)) return `<p class="small muted" data-t="sr_${r.state}"></p>`;
  if (r.state !== 'drafted' || !r.draft) return '';
  const keys = (r.why || []).map(w => String(w.key || '')).filter(Boolean);
  return `<p class="small"><b data-t="srFrom"></b>: ${keys.map(k => `<span data-t="${wordKey(k)}"></span>`).join(', ')}
    <span class="muted">(${Number(r.neighbours) | 0} <span data-t="srShare"></span>)</span>
    ${button({ variant: 'ghost', label: { t: 'srUse' }, id: 'srUse', attrs: { 'data-tour': 'sense.recipe' } })}</p>`;
}

/// The recipe draft filling what the owner left empty (what they set stays theirs).
export function fillEmpty(draft, r){
  const out = { taste: { ...draft.taste }, texture: { ...draft.texture }, aroma: { ...draft.aroma } };
  for (const dim of ['taste', 'texture', 'aroma']) for (const [id, n] of Object.entries(r?.draft?.[dim] || {})) if (!(id in out[dim])) out[dim][id] = n;
  return out;
}
