// "RECIPE NEEDS REVIEW (ingredient removed: Name)" -- the words (lane W-TKUI,
// 2026-10-12, operator TZ 3; the flag is /root/lanes/CONTRACT-QTY.md section 2:
// a ghost recipe line the hub answers as `recipeReview`, cleared by saving the
// recipe without it). Merged into the console's T once, on import.
// ASCII QUOTES ONLY as string delimiters (DOWIZ-COMMON-RULES rule 11).

import { T } from '/admin/i18n.js';

export const WORDS = {
  en: { rc_notInStock: 'not in stock any more', rc_gone: '{names}: no longer in stock — take the line out with x or add it to stock again', rc_lineBad: 'Not saved: check the quantity of {names}. Type an amount (e.g. 0,5 g, 1/2, 1,5 kg) or remove the line with x.', rc_qtyUnreadable: 'Cannot read this quantity. Use a number such as 0,5 or 1/2, with g, kg, ml, l or pcs.', rc_shown: 'Showing {n} of {total}. Type to search the rest.', rc_ticked: 'Selected: {n}', rv_badge: 'Recipe needs review (ingredient removed: {name})', rv_short: 'Recipe needs review', rv_removed: 'removed', rv_hint: 'Take the removed line out (x) or add its replacement, then save: the badge goes by itself.' },
  uk: { rc_notInStock: 'більше немає на складі', rc_gone: '{names}: більше немає на складі — приберіть рядок кнопкою x або знову додайте його на склад', rc_lineBad: 'Не збережено: перевірте кількість — {names}. Введіть число (напр. 0,5 г, 1/2, 1,5 кг) або видаліть рядок кнопкою x.', rc_qtyUnreadable: 'Не вдалося прочитати кількість. Введіть число, напр. 0,5 або 1/2, з г, кг, мл, л чи шт.', rc_shown: 'Показано {n} з {total}. Почніть вводити назву, щоб знайти решту.', rc_ticked: 'Вибрано: {n}', rv_badge: 'Потрібен перегляд рецептури (видалено інгредієнт: {name})', rv_short: 'Потрібен перегляд рецептури', rv_removed: 'видалено', rv_hint: 'Приберіть видалений рядок (x) або додайте заміну і збережіть: плашка зникне сама.' },
  sq: { rc_notInStock: 'nuk është më në magazinë', rc_gone: '{names}: nuk është më në magazinë — hiqeni rreshtin me x ose shtojeni përsëri në magazinë', rc_lineBad: 'Nuk u ruajt: kontrolloni sasinë e {names}. Shkruani një sasi (p.sh. 0,5 g, 1/2, 1,5 kg) ose hiqeni rreshtin me x.', rc_qtyUnreadable: 'Sasia nuk lexohet. Përdorni një numër si 0,5 ose 1/2, me g, kg, ml, l ose copë.', rc_shown: 'Shfaqen {n} nga {total}. Shkruani për të kërkuar të tjerat.', rc_ticked: 'Zgjedhur: {n}', rv_badge: 'Receta kërkon rishikim (përbërës i hequr: {name})', rv_short: 'Receta kërkon rishikim', rv_removed: 'i hequr', rv_hint: 'Hiqni rreshtin e hequr (x) ose shtoni zëvendësuesin, pastaj ruani: shenja ikën vetë.' },
  ru: { rc_notInStock: 'больше нет на складе', rc_gone: '{names}: больше нет на складе — уберите строку кнопкой x или снова добавьте её на склад', rc_lineBad: 'Не сохранено: проверьте количество — {names}. Введите число (напр. 0,5 г, 1/2, 1,5 кг) или удалите строку кнопкой x.', rc_qtyUnreadable: 'Не удалось прочитать количество. Введите число, напр. 0,5 или 1/2, с г, кг, мл, л или шт.', rc_shown: 'Показано {n} из {total}. Начните вводить название, чтобы найти остальные.', rc_ticked: 'Выбрано: {n}', rv_badge: 'Требуется пересмотр рецептуры (удалён ингредиент: {name})', rv_short: 'Требуется пересмотр рецептуры', rv_removed: 'удалён', rv_hint: 'Уберите удалённую строку (x) или добавьте замену и сохраните: плашка снимется сама.' },
};
for (const l of Object.keys(WORDS)) Object.assign(T[l] ||= {}, WORDS[l]);

/// PURE: the badge text for a review list [{ supply, name }] in language `l`.
export const badgeText = (review, l = 'en') => {
  const names = (review || []).map(r => r.name || r.supply).filter(Boolean);
  return names.length ? (WORDS[l] || WORDS.en).rv_badge.replace('{name}', names.join(', ')) : '';
};
