// The words of two W-LOST screens (A13 lost sales, R13 option recipes), PURE:
// no imports, so node tests the table itself. `lost-sales-i18n.js` merges it
// into the console's dictionary at import. lib/langs.js is the list of
// languages; the langs gate holds every language here to every key.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11); an apostrophe
// inside a word is U+02BC.
export const WORDS = {
  sq: {
    ls_title: 'Porosi të humbura nga mungesa e stokut', ls_hint: 'Shporta që arka refuzoi sepse mungonte një përbërës: sa herë, sa porcione dhe sa vlenin.',
    ls_dish: 'Pjata', ls_refused: 'Refuzime', ls_portions: 'Porcione', ls_revenue: 'Të ardhura të humbura', ls_day: 'Dita',
    ls_none: 'Asnjë porosi e humbur në këtë periudhë.', ls_limit: 'Numërohet një herë për përbërës në 10 minuta: numri është minimal.',
    ob_title: 'Receta e opsioneve', ob_hint: 'Çfarë merr nga rafti një opsion, p.sh. salmon shtesë 20 g. Rezervohet bashkë me pjatën.',
    ob_none: 'Kjo pjatë nuk ka opsione.', ob_supply: 'Përbërësi', ob_qty: 'Sasia', ob_add: 'Shto rresht', ob_save: 'Ruaj recetën e opsionit', ob_badQty: 'Sasia është numër i plotë mbi zero.',
  },
  en: {
    ls_title: 'Orders lost to stock-outs', ls_hint: 'Baskets the checkout refused because an ingredient ran out: how often, how many portions and what they were worth.',
    ls_dish: 'Dish', ls_refused: 'Refused', ls_portions: 'Portions', ls_revenue: 'Revenue lost', ls_day: 'Day',
    ls_none: 'No order lost to a stock-out in this period.', ls_limit: 'Counted once per ingredient per 10 minutes, so this is a lower bound.',
    ob_title: 'Option recipes', ob_hint: 'What an option takes off the shelf, e.g. extra salmon 20 g. It is reserved with the dish.',
    ob_none: 'This dish has no options.', ob_supply: 'Ingredient', ob_qty: 'Quantity', ob_add: 'Add a line', ob_save: 'Save the option recipe', ob_badQty: 'A quantity is a whole number above zero.',
  },
  uk: {
    ls_title: 'Замовлення, втрачені через брак продуктів', ls_hint: 'Кошики, які каса відхилила, бо скінчився інгредієнт: скільки разів, скільки порцій і на яку суму.',
    ls_dish: 'Страва', ls_refused: 'Відмов', ls_portions: 'Порцій', ls_revenue: 'Втрачений виторг', ls_day: 'День',
    ls_none: 'За цей період жодного замовлення не втрачено через брак продуктів.', ls_limit: 'Рахується раз на інгредієнт за 10 хвилин, тож це нижня межа.',
    ob_title: 'Техкарта опцій', ob_hint: 'Що опція бере з полиці, напр. додатковий лосось 20 г. Резервується разом зі стравою.',
    ob_none: 'У цієї страви немає опцій.', ob_supply: 'Інгредієнт', ob_qty: 'Кількість', ob_add: 'Додати рядок', ob_save: 'Зберегти техкарту опції', ob_badQty: 'Кількість - ціле число більше нуля.',
  },
  ru: {
    ls_title: 'Заказы, потерянные из-за нехватки продуктов', ls_hint: 'Корзины, которые касса отклонила, потому что закончился ингредиент: сколько раз, сколько порций и на какую сумму.',
    ls_dish: 'Блюдо', ls_refused: 'Отказов', ls_portions: 'Порций', ls_revenue: 'Потерянная выручка', ls_day: 'День',
    ls_none: 'За этот период ни один заказ не потерян из-за нехватки продуктов.', ls_limit: 'Считается раз на ингредиент за 10 минут, поэтому это нижняя граница.',
    ob_title: 'Техкарта опций', ob_hint: 'Что опция берёт с полки, напр. дополнительный лосось 20 г. Резервируется вместе с блюдом.',
    ob_none: 'У этого блюда нет опций.', ob_supply: 'Ингредиент', ob_qty: 'Количество', ob_add: 'Добавить строку', ob_save: 'Сохранить техкарту опции', ob_badQty: 'Количество - целое число больше нуля.',
  },
};

/// Add the words to every language the console has; a language this table
/// lacks reads English rather than the bare key.
export function merge(T, words = WORDS){
  const en = words.en || {};
  for (const lang of new Set([...Object.keys(T), ...Object.keys(words)])) {
    const table = T[lang] = T[lang] || {};
    for (const [k, v] of Object.entries({ ...en, ...(words[lang] || {}) })) if (!(k in table)) table[k] = v;
  }
  return T;
}
