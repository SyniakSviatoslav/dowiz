// The tip's till item (D34, lane W-TIPS2): which eBills item a tip is sold as
// (`fiscal.ebills.tip_item`), picked in the fiscal sheet beside the fee item.
// PURE: no imports, so node tests it as it is; `fiscal.js` merges the words
// into the console's dictionary and draws the select from `tipOptions`.
//
// A TIP IS OUTSIDE VAT, so the item must be VAT 0 % on the till. The console
// cannot see an item's VAT (the till codes it lists carry a name only), so the
// hint says it, and the sender refuses a taxed item by name before any send
// (`fiscal/ebills_body/adjust.rs`, `tip`). No item chosen: a tipped order is
// not sent, and the queue names why.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
export const WORDS = {
  sq: {
    fx_tipItem: 'Artikulli për bakshishin',
    fx_noTip: 'Asnjë (porositë me bakshish nuk dërgohen)',
    fx_tipHint: 'Krijoni në ebills një artikull p.sh. BAKSHISH me TVSH 0 %: bakshishi është jashtë TVSH-së. Çdo bakshish dërgohet si ky artikull.',
  },
  en: {
    fx_tipItem: 'Item for the tip',
    fx_noTip: 'None (orders with a tip are not sent)',
    fx_tipHint: 'Create an item in ebills, e.g. TIP, at VAT 0 %: a tip is outside VAT. Every tip is sent as this item.',
  },
  uk: {
    fx_tipItem: 'Позиція для чайових',
    fx_noTip: 'Немає (замовлення з чайовими не надсилаються)',
    fx_tipHint: 'Створіть у ebills позицію, напр. TIP, з ПДВ 0 %: чайові поза ПДВ. Кожні чайові надсилаються як ця позиція.',
  },
  ru: {
    fx_tipItem: 'Позиция для чаевых',
    fx_noTip: 'Нет (заказы с чаевыми не отправляются)',
    fx_tipHint: 'Создайте в ebills позицию, напр. TIP, с НДС 0 %: чаевые вне НДС. Все чаевые отправляются как эта позиция.',
  },
};

/// Add the words to every language the console has, never over a word it has.
export function merge(T, words = WORDS){
  const en = words.en || {};
  for (const lang of new Set([...Object.keys(T), ...Object.keys(words)])) {
    const table = T[lang] = T[lang] || {};
    for (const [k, v] of Object.entries({ ...en, ...(words[lang] || {}) })) if (!(k in table)) table[k] = v;
  }
  return T;
}

/// The select's options: "none" first, then every till item the hub listed,
/// and the chosen code even when the till no longer lists it (as the sale
/// unit's picker keeps a lost unit), so saving never clears it silently.
export function tipOptions(items, chosen){
  const listed = (items || []).map(i => ({ value: i.code, label: `${i.code} - ${i.name}` }));
  const lost = chosen && !listed.some(o => o.value === chosen) ? [{ value: chosen, label: chosen }] : [];
  return [{ value: '', key: 'fx_noTip' }, ...listed, ...lost];
}
