// The prep list's words (W-PREP, P6/P7), PURE: no imports, so node tests the
// table itself (prep-list-view.test.mjs). `prep-list-i18n.js` merges it into
// the console's dictionary at import. Nothing here lists the languages:
// lib/langs.js is the list, and the langs gate holds every language here to
// every key.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11); an apostrophe
// inside a word is U+02BC.
export const WORDS = {
  sq: {
    pl_title: 'Lista e përgatitjes', pl_tile: 'Përgatitja e ditës',
    pl_hint: 'Sa porosi dhe porcione pritet të ketë dita, nga e njëjta ditë e javës në javët e fundit; çfarë duhet përgatitur nga gjysmëproduktet, pasi zbritet ajo që është gati në raft dhe shtohen rezervimet.',
    pl_day: 'Dita', pl_show: 'Shfaq', pl_orders: 'porosi', pl_portions: 'porcione', pl_offBy: 'zakonisht gabon me', pl_naive: 'java e kaluar do të gabonte me',
    pl_learning: 'Po mëson', pl_learningHint: 'Duhen të paktën 3 javë histori për këtë ditë të javës; deri atëherë nuk jepet asnjë numër.', pl_weeks: 'javë histori',
    pl_bands: 'Sipas orëve', pl_hoursUnknown: 'Orari i lokalit nuk është vendosur: orët janë të përgjithshme.', pl_closed: 'Lokali është i mbyllur këtë ditë.',
    pl_dishes: 'Pjatat', pl_dish: 'Pjata', pl_fromBookings: 'nga rezervimet',
    pl_preps: 'Për tʼu përgatitur', pl_prep: 'Gjysmëprodukti', pl_need: 'Nevojitet', pl_onHand: 'Gati', pl_make: 'Përgatit', pl_from: 'Nga',
    pl_raw: 'Lëndë e parë për ditën', pl_nothingToMake: 'Asgjë për tʼu përgatitur: raftet mbulojnë parashikimin.',
    pl_bookings: 'Rezervime', pl_guests: 'mysafirë', pl_unmodelled: 'Pjata pa recetë (nuk llogariten)', pl_historyError: 'Historia nuk u lexua',
    pl_limits: 'Parashikimi nuk di për festa, mot apo ngjarje; rezervimet shtohen mbi zakonin.',
    pl_card: 'Parashikimi i sotëm', pl_open: 'Hap listën',
  },
  en: {
    pl_title: 'Prep list', pl_tile: 'Todayʼs prep',
    pl_hint: 'How many orders and portions the day should see, from the same weekday over the last weeks; what to prepare of the semi-finished products, after what is ready on the shelf, with bookings added.',
    pl_day: 'Day', pl_show: 'Show', pl_orders: 'orders', pl_portions: 'portions', pl_offBy: 'usually off by', pl_naive: 'last weekʼs number would be off by',
    pl_learning: 'Learning', pl_learningHint: 'At least 3 weeks of history for this weekday are needed; until then no number is given.', pl_weeks: 'weeks of history',
    pl_bands: 'By hours', pl_hoursUnknown: 'The venueʼs hours are not set: the bands are the clockʼs.', pl_closed: 'The venue is closed this day.',
    pl_dishes: 'Dishes', pl_dish: 'Dish', pl_fromBookings: 'from bookings',
    pl_preps: 'To prepare', pl_prep: 'Semi-finished', pl_need: 'Needed', pl_onHand: 'Ready', pl_make: 'Make', pl_from: 'From',
    pl_raw: 'Raw for the day', pl_nothingToMake: 'Nothing to prepare: the shelf covers the forecast.',
    pl_bookings: 'Bookings', pl_guests: 'guests', pl_unmodelled: 'Dishes without a recipe (not counted)', pl_historyError: 'The history could not be read',
    pl_limits: 'The forecast knows nothing of holidays, weather or events; bookings are added on top of the usual.',
    pl_card: 'Todayʼs forecast', pl_open: 'Open the list',
  },
  uk: {
    pl_title: 'Список заготовок', pl_tile: 'Заготовки на день',
    pl_hint: 'Скільки замовлень і порцій очікує день, за тим самим днем тижня останніх тижнів; що заготувати з напівфабрикатів після того, що вже готове на полиці, з бронюваннями зверху.',
    pl_day: 'День', pl_show: 'Показати', pl_orders: 'замовлень', pl_portions: 'порцій', pl_offBy: 'зазвичай помиляється на', pl_naive: 'число минулого тижня помилилося б на',
    pl_learning: 'Навчається', pl_learningHint: 'Потрібно щонайменше 3 тижні історії для цього дня тижня; доти число не показується.', pl_weeks: 'тижнів історії',
    pl_bands: 'За годинами', pl_hoursUnknown: 'Години роботи закладу не задані: проміжки за годинником.', pl_closed: 'Цього дня заклад зачинено.',
    pl_dishes: 'Страви', pl_dish: 'Страва', pl_fromBookings: 'з бронювань',
    pl_preps: 'Заготувати', pl_prep: 'Напівфабрикат', pl_need: 'Потрібно', pl_onHand: 'Готово', pl_make: 'Зробити', pl_from: 'З чого',
    pl_raw: 'Сировина на день', pl_nothingToMake: 'Нічого заготовляти: полиця покриває прогноз.',
    pl_bookings: 'Бронювання', pl_guests: 'гостей', pl_unmodelled: 'Страви без техкарти (не враховано)', pl_historyError: 'Історію не вдалося прочитати',
    pl_limits: 'Прогноз нічого не знає про свята, погоду чи події; бронювання додаються зверху звичайного.',
    pl_card: 'Прогноз на сьогодні', pl_open: 'Відкрити список',
  },
  ru: {
    pl_title: 'Список заготовок', pl_tile: 'Заготовки на день',
    pl_hint: 'Сколько заказов и порций ожидает день, по тому же дню недели последних недель; что заготовить из полуфабрикатов после того, что уже готово на полке, с бронированиями сверху.',
    pl_day: 'День', pl_show: 'Показать', pl_orders: 'заказов', pl_portions: 'порций', pl_offBy: 'обычно ошибается на', pl_naive: 'число прошлой недели ошиблось бы на',
    pl_learning: 'Обучается', pl_learningHint: 'Нужно минимум 3 недели истории для этого дня недели; до тех пор число не показывается.', pl_weeks: 'недель истории',
    pl_bands: 'По часам', pl_hoursUnknown: 'Часы работы заведения не заданы: промежутки по часам.', pl_closed: 'В этот день заведение закрыто.',
    pl_dishes: 'Блюда', pl_dish: 'Блюдо', pl_fromBookings: 'из бронирований',
    pl_preps: 'Заготовить', pl_prep: 'Полуфабрикат', pl_need: 'Нужно', pl_onHand: 'Готово', pl_make: 'Сделать', pl_from: 'Из чего',
    pl_raw: 'Сырьё на день', pl_nothingToMake: 'Нечего заготавливать: полка покрывает прогноз.',
    pl_bookings: 'Бронирования', pl_guests: 'гостей', pl_unmodelled: 'Блюда без техкарты (не учтены)', pl_historyError: 'Историю не удалось прочитать',
    pl_limits: 'Прогноз ничего не знает о праздниках, погоде или событиях; бронирования добавляются сверху обычного.',
    pl_card: 'Прогноз на сегодня', pl_open: 'Открыть список',
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
