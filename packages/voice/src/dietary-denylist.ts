// Dietary / allergen token denylist (sq / en / uk). A voice SELECT_CATEGORY whose resolved
// category name matches one of these is REJECTed — a mis-heard "show gluten-free" must never
// auto-narrow the menu into a state the user reads as an allergen-safe set. Closes the CLASS,
// not just the setFilterAllergen instance (breaker finding R2-B / C1). Tokens match as
// substrings, case- and diacritic-insensitive, so "pa gluten", "Gluten-Free", "веган" all hit.
// Over-matching is the SAFE direction: a falsely-rejected category just falls back to touch.

import { normalize } from './normalize.js';

const DIETARY_TOKENS: readonly string[] = [
  // en
  'gluten',
  'allerg',
  'vegan',
  'vegetarian',
  'veggie',
  'dairy',
  'lactose',
  'nut-free',
  'nut free',
  'peanut',
  'celiac',
  'coeliac',
  'free from',
  'free-from',
  'halal',
  'kosher',
  'organic',
  'keto',
  'paleo',
  'sugar-free',
  'sugar free',
  'diet',
  // sq (Albanian)
  'pa gluten',
  'pa laktoz',
  'pa arra',
  'vegjetarian',
  'alergj',
  'pa sheqer',
  'pa qumesht',
  'bio',
  'organik',
  // uk (Ukrainian)
  'глютен',
  'веган',
  'вегетаріан',
  'алерг',
  'без лактоз',
  'без цукру',
  'без глютен',
  'органіч',
];

const NORMALIZED_TOKENS: readonly string[] = DIETARY_TOKENS.map(normalize);

/**
 * True if the category name carries a dietary/allergen meaning a user could read as a safety
 * assertion. Such categories are not voice-selectable (touch-only) — the same class as money.
 */
export function isDietaryCategory(categoryName: string): boolean {
  const n = normalize(categoryName);
  return NORMALIZED_TOKENS.some((t) => n.includes(t));
}
