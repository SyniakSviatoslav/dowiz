// The stock check's words (lane W-HEALTH, 2026-09-28), PURE: no imports, so
// node tests the table itself (stock-health.test.mjs). `stock-health-i18n.js`
// merges it into the console's dictionary at import. Nothing below lists the
// languages: lib/langs.js is the list, and the langs gate holds every
// language here to every key.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11); an apostrophe
// inside a word is U+02BC.
export const WORDS = {
  sq: {
    sh_title: 'Kontrolli i magazinës', sh_hint: 'A mban magazina përbërës për porosi që kanë mbaruar? Kontrolli lexon gjithë historinë e porosive.',
    sh_run: 'Kontrollo', sh_again: 'Kontrollo përsëri', sh_error: 'Kontrolli nuk u krye',
    sh_ok: 'Magazina përputhet me porositë', sh_checked: 'porosi të kontrolluara', sh_live: 'rezerva për porosi aktive',
    sh_bad: 'porosi të mbaruara ende mbajnë përbërës',
    sh_badHint: 'Këto sasi janë në raft, por magazina i tregon si të zëna. Nga konsola nuk mund të lirohen ende, as numërimi nuk i liron: njoftoni mbështetjen.',
    sh_order: 'Porosia', sh_archive: 'arkivi', sh_final: 'gjendja e fundit', sh_notArchived: 'jo në arkiv',
    sh_noHolds: 'asgjë e mbajtur në raft', sh_stale: 'porosi shfaqen ndryshe nga historia e tyre: njoftoni mbështetjen',
  },
  en: {
    sh_title: 'Stock check', sh_hint: 'Is the stock holding ingredients for orders that are over? The check reads the whole order history.',
    sh_run: 'Check', sh_again: 'Check again', sh_error: 'The check could not run',
    sh_ok: 'Stock matches the orders', sh_checked: 'orders checked', sh_live: 'holds for live orders',
    sh_bad: 'finished orders still hold ingredients',
    sh_badHint: 'These amounts are on the shelf, but stock shows them as taken. The console cannot release them yet, and a count does not release them: tell support.',
    sh_order: 'Order', sh_archive: 'archive', sh_final: 'last status', sh_notArchived: 'not in an archive',
    sh_noHolds: 'nothing held on the shelf', sh_stale: 'orders are shown differently from their history: tell support',
  },
  uk: {
    sh_title: 'Перевірка складу', sh_hint: 'Чи тримає склад інгредієнти для замовлень, які вже завершені? Перевірка читає всю історію замовлень.',
    sh_run: 'Перевірити', sh_again: 'Перевірити ще раз', sh_error: 'Перевірку не вдалося виконати',
    sh_ok: 'Склад відповідає замовленням', sh_checked: 'замовлень перевірено', sh_live: 'резервів для активних замовлень',
    sh_bad: 'завершених замовлень досі тримають інгредієнти',
    sh_badHint: 'Ці кількості є на полиці, але склад показує їх як зайняті. Консоль поки не може їх звільнити, і інвентаризація їх не звільняє: напишіть у підтримку.',
    sh_order: 'Замовлення', sh_archive: 'архів', sh_final: 'останній статус', sh_notArchived: 'не в архіві',
    sh_noHolds: 'на полиці нічого не тримається', sh_stale: 'замовлень показано інакше, ніж у їхній історії: напишіть у підтримку',
  },
  ru: {
    sh_title: 'Проверка склада', sh_hint: 'Держит ли склад ингредиенты для заказов, которые уже завершены? Проверка читает всю историю заказов.',
    sh_run: 'Проверить', sh_again: 'Проверить ещё раз', sh_error: 'Проверку не удалось выполнить',
    sh_ok: 'Склад соответствует заказам', sh_checked: 'заказов проверено', sh_live: 'резервов для активных заказов',
    sh_bad: 'завершённых заказов всё ещё держат ингредиенты',
    sh_badHint: 'Эти количества есть на полке, но склад показывает их как занятые. Консоль пока не может их освободить, и инвентаризация их не освобождает: напишите в поддержку.',
    sh_order: 'Заказ', sh_archive: 'архив', sh_final: 'последний статус', sh_notArchived: 'не в архиве',
    sh_noHolds: 'на полке ничего не удерживается', sh_stale: 'заказов показано иначе, чем в их истории: напишите в поддержку',
  },
};

/// Add the words to every language the console has; a language this table
/// lacks reads English rather than the bare key.
export function merge(T, words = WORDS){
  const en = words.en || {};
  for (const lang of new Set([...Object.keys(T), ...Object.keys(words)])) {
    // IN PLACE: the console may already hold a reference to T[lang].
    const table = T[lang] = T[lang] || {};
    for (const [k, v] of Object.entries({ ...en, ...(words[lang] || {}) })) if (!(k in table)) table[k] = v;
  }
  return T;
}
