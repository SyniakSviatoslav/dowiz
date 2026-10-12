// "RECIPE NEEDS REVIEW (ingredient removed: Name)" -- the words (lane W-TKUI,
// 2026-10-12, operator TZ 3; the flag is /root/lanes/CONTRACT-QTY.md section 2:
// a ghost recipe line the hub answers as `recipeReview`, cleared by saving the
// recipe without it). Merged into the console's T once, on import.
// ASCII QUOTES ONLY as string delimiters (DOWIZ-COMMON-RULES rule 11).

import { T } from '/admin/i18n.js';

export const WORDS = {
  en: { rv_badge: 'Recipe needs review (ingredient removed: {name})', rv_short: 'Recipe needs review', rv_removed: 'removed', rv_hint: 'Take the removed line out (x) or add its replacement, then save: the badge goes by itself.' },
  uk: { rv_badge: 'Потрібен перегляд рецептури (видалено інгредієнт: {name})', rv_short: 'Потрібен перегляд рецептури', rv_removed: 'видалено', rv_hint: 'Приберіть видалений рядок (x) або додайте заміну і збережіть: плашка зникне сама.' },
  sq: { rv_badge: 'Receta kërkon rishikim (përbërës i hequr: {name})', rv_short: 'Receta kërkon rishikim', rv_removed: 'i hequr', rv_hint: 'Hiqni rreshtin e hequr (x) ose shtoni zëvendësuesin, pastaj ruani: shenja ikën vetë.' },
  ru: { rv_badge: 'Требуется пересмотр рецептуры (удалён ингредиент: {name})', rv_short: 'Требуется пересмотр рецептуры', rv_removed: 'удалён', rv_hint: 'Уберите удалённую строку (x) или добавьте замену и сохраните: плашка снимется сама.' },
};
for (const l of Object.keys(WORDS)) Object.assign(T[l] ||= {}, WORDS[l]);

/// PURE: the badge text for a review list [{ supply, name }] in language `l`.
export const badgeText = (review, l = 'en') => {
  const names = (review || []).map(r => r.name || r.supply).filter(Boolean);
  return names.length ? (WORDS[l] || WORDS.en).rv_badge.replace('{name}', names.join(', ')) : '';
};
