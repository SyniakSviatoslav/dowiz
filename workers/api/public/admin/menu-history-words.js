// The words of the menu's edit history (W-PITR), PURE: no imports, so node
// tests the table itself. `menu-history-i18n.js` merges it into the console's
// dictionary at import. lib/langs.js is the list of languages; the langs gate
// holds every language here to every key.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11); an apostrophe
// inside a word is U+02BC.
export const WORDS = {
  sq: {
    mh_title: 'Historiku i ndryshimeve të menusë', mh_hint: 'Çdo ndryshim i menusë: kush, kur dhe çfarë. Një pjatë mund të kthehet siç ishte para një ndryshimi.',
    mh_none: 'Ende asnjë ndryshim i regjistruar.', mh_restore: 'Ktheje siç ishte para këtij ndryshimi',
    mh_restoreQ: 'Ta kthej pjatën siç ishte para këtij ndryshimi?', mh_restoreHint: 'Çmimi, emri dhe disponueshmëria bëhen siç ishin para tij. Edhe kthimi regjistrohet në historik.',
    mh_restored: 'U kthye', mh_added: 'U shtua', mh_removed: 'U hoq', mh_changed: 'U ndryshua',
    mh_unseen: 'regjistruar më vonë', mh_baseline: 'fillimi i historikut', mh_by: 'Nga',
    mh_k_product: 'Pjatë', mh_k_category: 'Kategori', mh_k_supply: 'Përbërës', mh_k_promo: 'Kod promocional', mh_k_location: 'Lokali',
  },
  en: {
    mh_title: 'Menu change history', mh_hint: 'Every change to the menu: who, when and what. A dish can be put back the way it was before a change.',
    mh_none: 'No change recorded yet.', mh_restore: 'Put back as before this change',
    mh_restoreQ: 'Put the dish back the way it was before this change?', mh_restoreHint: 'Its price, name and availability become what they were before it. The restore is recorded in the history too.',
    mh_restored: 'Put back', mh_added: 'Added', mh_removed: 'Removed', mh_changed: 'Changed',
    mh_unseen: 'recorded later', mh_baseline: 'start of the history', mh_by: 'By',
    mh_k_product: 'Dish', mh_k_category: 'Category', mh_k_supply: 'Ingredient', mh_k_promo: 'Promo code', mh_k_location: 'Venue',
  },
  uk: {
    mh_title: 'Історія змін меню', mh_hint: 'Кожна зміна меню: хто, коли і що. Страву можна повернути такою, якою вона була до зміни.',
    mh_none: 'Ще жодної записаної зміни.', mh_restore: 'Повернути як було до цієї зміни',
    mh_restoreQ: 'Повернути страву такою, якою вона була до цієї зміни?', mh_restoreHint: 'Ціна, назва і наявність стануть такими, як були до неї. Повернення теж записується в історію.',
    mh_restored: 'Повернуто', mh_added: 'Додано', mh_removed: 'Видалено', mh_changed: 'Змінено',
    mh_unseen: 'записано пізніше', mh_baseline: 'початок історії', mh_by: 'Хто',
    mh_k_product: 'Страва', mh_k_category: 'Категорія', mh_k_supply: 'Інгредієнт', mh_k_promo: 'Промокод', mh_k_location: 'Заклад',
  },
  ru: {
    mh_title: 'История изменений меню', mh_hint: 'Каждое изменение меню: кто, когда и что. Блюдо можно вернуть таким, каким оно было до изменения.',
    mh_none: 'Ещё ни одного записанного изменения.', mh_restore: 'Вернуть как было до этого изменения',
    mh_restoreQ: 'Вернуть блюдо таким, каким оно было до этого изменения?', mh_restoreHint: 'Цена, название и наличие станут такими, как были до него. Возврат тоже записывается в историю.',
    mh_restored: 'Возвращено', mh_added: 'Добавлено', mh_removed: 'Удалено', mh_changed: 'Изменено',
    mh_unseen: 'записано позже', mh_baseline: 'начало истории', mh_by: 'Кто',
    mh_k_product: 'Блюдо', mh_k_category: 'Категория', mh_k_supply: 'Ингредиент', mh_k_promo: 'Промокод', mh_k_location: 'Заведение',
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
