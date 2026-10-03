// The Offline sales pane's words (W-OFFSALE), merged into the console's
// table at import, the shape `publish-i18n.js` set: `T` is the console's own
// table, so `t()`, `data-t` and `retranslate()` read these like any other key.
// wire.js imports this file for the tile's name; offline.js for the pane.
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { T } from '/admin/i18n.js';

export const WORDS = {
  sq: {
    of_title: 'Shitjet pa rrjet', of_sub: 'kesh pa NIVF, afati 48 orë',
    of_hint: 'Shitjet me kesh që tableti bëri pa rrjet, me afatin 48 orë për fiskalizim (Ligji 87/2019). Dërgimi te tatimet është i fikur: fiskalizojini në arkën tuaj eBills.',
    of_count: 'Shitje', of_oldest: 'Më e vjetra', of_overdue: 'Me afat të kaluar', of_conflicts: 'Me ndryshime',
    of_none: 'Asnjë shitje pa rrjet.', of_deadline: 'afati', of_sold: 'shitur', of_synced: 'dërguar',
    of_sendOff: 'Dërgimi te tatimet është i fikur për platformën.',
    of_fiscal_queued: 'në radhë', of_fiscal_registered: 'fiskalizuar', of_fiscal_not_queued: 'pa dokument',
    of_alerted: 'njoftuar', of_c_price_changed: 'çmimi ndryshoi', of_c_off_sale: 'hequr nga shitja', of_c_unknown: 'pjatë e fshirë',
    of_c_no_price: 'pa çmim', of_c_options: 'kërkon zgjedhje', of_c_unreadable: 'e palexueshme', of_c_clock: 'ora e tabletit',
    of_c_currency: 'valuta', of_c_tax: 'tatimi', of_c_stock: 'magazina', of_c_quantity: 'sasia',
  },
  en: {
    of_title: 'Offline sales', of_sub: 'cash without NIVF, 48 h deadline',
    of_hint: 'Cash sales the tablet made with no network, with their 48 h fiscal deadline (Law 87/2019). Sending to the tax authority is off: fiscalise them in your eBills till.',
    of_count: 'Sales', of_oldest: 'Oldest', of_overdue: 'Past the deadline', of_conflicts: 'With changes',
    of_none: 'No offline sales.', of_deadline: 'deadline', of_sold: 'sold', of_synced: 'sent',
    of_sendOff: 'Sending to the tax authority is off for the platform.',
    of_fiscal_queued: 'queued', of_fiscal_registered: 'fiscalised', of_fiscal_not_queued: 'no document',
    of_alerted: 'alerted', of_c_price_changed: 'price changed', of_c_off_sale: 'taken off sale', of_c_unknown: 'dish deleted',
    of_c_no_price: 'no price', of_c_options: 'needs options', of_c_unreadable: 'unreadable', of_c_clock: 'tablet clock',
    of_c_currency: 'currency', of_c_tax: 'tax', of_c_stock: 'stock', of_c_quantity: 'quantity',
  },
  uk: {
    of_title: 'Продажі без мережі', of_sub: 'готівка без NIVF, строк 48 год',
    of_hint: 'Продажі за готівку, зроблені планшетом без мережі, зі строком фіскалізації 48 год (Закон 87/2019). Надсилання до податкової вимкнено: фіскалізуйте їх у своїй касі eBills.',
    of_count: 'Продажі', of_oldest: 'Найстаріший', of_overdue: 'Строк минув', of_conflicts: 'Зі змінами',
    of_none: 'Продажів без мережі немає.', of_deadline: 'строк', of_sold: 'продано', of_synced: 'надіслано',
    of_sendOff: 'Надсилання до податкової вимкнено для платформи.',
    of_fiscal_queued: 'у черзі', of_fiscal_registered: 'фіскалізовано', of_fiscal_not_queued: 'без документа',
    of_alerted: 'сповіщено', of_c_price_changed: 'ціна змінилась', of_c_off_sale: 'знято з продажу', of_c_unknown: 'страву видалено',
    of_c_no_price: 'без ціни', of_c_options: 'потрібні опції', of_c_unreadable: 'нечитабельно', of_c_clock: 'годинник планшета',
    of_c_currency: 'валюта', of_c_tax: 'податок', of_c_stock: 'склад', of_c_quantity: 'кількість',
  },
  ru: {
    of_title: 'Продажи без сети', of_sub: 'наличные без NIVF, срок 48 ч',
    of_hint: 'Продажи за наличные, сделанные планшетом без сети, со сроком фискализации 48 ч (Закон 87/2019). Отправка в налоговую выключена: фискализируйте их в своей кассе eBills.',
    of_count: 'Продажи', of_oldest: 'Самая старая', of_overdue: 'Срок истёк', of_conflicts: 'С изменениями',
    of_none: 'Продаж без сети нет.', of_deadline: 'срок', of_sold: 'продано', of_synced: 'отправлено',
    of_sendOff: 'Отправка в налоговую выключена для платформы.',
    of_fiscal_queued: 'в очереди', of_fiscal_registered: 'фискализировано', of_fiscal_not_queued: 'без документа',
    of_alerted: 'оповещено', of_c_price_changed: 'цена изменилась', of_c_off_sale: 'снято с продажи', of_c_unknown: 'блюдо удалено',
    of_c_no_price: 'без цены', of_c_options: 'нужны опции', of_c_unreadable: 'нечитаемо', of_c_clock: 'часы планшета',
    of_c_currency: 'валюта', of_c_tax: 'налог', of_c_stock: 'склад', of_c_quantity: 'количество',
  },
};

for (const [l, dict] of Object.entries(WORDS)) {
  T[l] = T[l] || {};
  for (const [key, v] of Object.entries(dict)) if (!(key in T[l])) T[l][key] = v;
}
