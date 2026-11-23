// THE MENU'S MARGIN, pure and tested (operator 2026-11-23): the badge a dish
// row wears, what an unknown cost waits on, and the dish's 30-day history.
// Every number is the hub's (`GET /owner/analytics/kitchen?days=30&v=2`,
// `menu.today` and `menu.dishes`, `kitchen/menu.rs`); nothing here adds money
// up a second way. AN UNKNOWN COST IS NOT A ZERO: it stays null to the screen.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

/// Per mille as a percent, by the dish editor's own formatter: 337 -> '33.7%'.
export { pct } from './ingredients-calc.js';
import { pct } from './ingredients-calc.js';

/// The row's badge for one dish of `menu.today`: food cost and margin per
/// portion, or the word for why the cost is unknown. `null` for a dish the
/// answer does not know (no badge rather than a wrong one).
export function badge(r){
  if (!r) return null;
  if (r.portionCost == null || r.marginPortion == null) return { tone: 'warn', key: r.hasRecipe ? 'mg_unknown' : 'mg_noRecipe' };
  return { tone: r.marginPortion < 0 ? 'bad' : '', fc: r.foodCostPm ?? null, margin: r.marginPortion };
}

/// What an unknown cost waits on, each something the owner can fix: a
/// supply with no price, or the recipe itself.
export function missingOf(r){
  if (!r) return [];
  if (!r.hasRecipe) return [{ kind: 'recipe' }];
  return (r.missing || []).map(m => ({ kind: 'supply', id: m.id, name: m.name }));
}

/// The dish's row of `menu.dishes` as the sheet shows it: the facts, and
/// the cost split by basis -- [word, portions, cost] -- where the stamped
/// part is what each portion cost when sold, the rest today's recipe.
export function history(row){
  if (!row) return null;
  const known = !row.costUnknown;
  const facts = [['mg_sold', row.sold], ['mg_revenue', row.revenue, 'money'],
    known ? ['mg_cogs', row.cogs, 'money'] : ['mg_cogs', null], known ? ['mg_margin', row.margin, 'money'] : ['mg_margin', null],
    ['mg_foodCost', known && row.foodCostPm != null ? pct(row.foodCostPm) : null]];
  const basis = [];
  if (row.stamped > 0) basis.push(['mg_stamped', row.stamped, row.stampedCogs]);
  if (row.todayPortions > 0) basis.push(['mg_recipeToday', row.todayPortions, row.todayCogs ?? null]);
  return { facts, basis, attributed: row.attributed || 0 };
}

/// The till codes no dish answers to, as the hub sorted them (by revenue).
export const unmatchedOf = (menu, limit = Infinity) => (menu?.unmatched || []).slice(0, limit);
