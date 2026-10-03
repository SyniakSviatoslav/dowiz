// The offline cash sale's words (W-OFFSALE), merged into the room's table
// without renaming any word it already has -- `pass-i18n.js`'s shape. The
// fiscal sentence is the law's (Law 87/2019 art. 29: a receipt issued without
// the NIVF is fiscalised within 48 hours); `saleNoNivfSq` is printed on every
// receipt in Albanian whatever the screen's language.
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES 11).
import { T } from './i18n.js';

const SQ_LAW = 'pa NIVF — do të fiskalizohet brenda 48 orëve';

export const WORDS = {
  sq: { offlineBanner: 'Pa rrjet — vetëm shitje me para në dorë. Shitjet dërgohen kur kthehet rrjeti.',
    cashSale: 'Shitje me kesh', sellTitle: 'Shitje me kesh pa rrjet', sellHint: 'Çmimet nga menyja e ruajtur në këtë tablet.',
    menuCachedAt: 'Menyja e ruajtur: {age}', sellN: 'Shit {n}', sellConfirm: 'Konfirmo shitjen', cashTaken: 'Paratë u morën — shit',
    cashOnly: 'Vetëm kesh: karta kërkon rrjet.', receipt: 'Fatura', saleTotal: 'Totali', saleCash: 'Paguar me kesh',
    saleRef: 'Ref.', saleNoNivf: SQ_LAW, saleNoNivfSq: SQ_LAW, print: 'Printo', done: 'Mbaroi',
    saleQueued: 'Shitja u ruajt dhe do të dërgohet.', saleSynced: 'Shitja u dërgua.', saleRefused: 'Shitja u refuzua nga serveri — thirrni pronarin.',
    saleNoStore: 'Kjo pajisje nuk mund ta ruajë shitjen. Mos e bëni shitjen.', saleUnsent: '{n} shitje pa dërguar',
    saleUnsentSignOut: 'Ka shitje pa dërguar: lidheni tabletin para se të dilni.', saleUnsentClose: 'Ka shitje pa dërguar në këtë arkë: lidheni tabletin para mbylljes.', saleZero: 'Një shitje pa para nuk regjistrohet.', saleNoMenu: 'Nuk ka meny të ruajtur në këtë tablet.',
    refusal_quantity: 'Sasia 1 deri 99.', refusal_unknown: 'Kjo pjatë nuk është në meny.', refusal_off_sale: 'Kjo pjatë ka mbaruar.',
    refusal_no_price: 'Kjo pjatë nuk ka çmim.', refusal_options: 'Kjo pjatë kërkon zgjedhje: shiteni kur të kthehet rrjeti.' },
  en: { offlineBanner: 'Offline — cash sales only. Sales are sent when the network returns.',
    cashSale: 'Cash sale', sellTitle: 'Offline cash sale', sellHint: 'Prices from the menu saved on this tablet.',
    menuCachedAt: 'Saved menu: {age}', sellN: 'Sell {n}', sellConfirm: 'Confirm the sale', cashTaken: 'Cash taken — sell',
    cashOnly: 'Cash only: a card needs the network.', receipt: 'Receipt', saleTotal: 'Total', saleCash: 'Paid in cash',
    saleRef: 'Ref.', saleNoNivf: 'no NIVF — to be fiscalised within 48 hours', saleNoNivfSq: SQ_LAW, print: 'Print', done: 'Done',
    saleQueued: 'Sale saved; it will be sent.', saleSynced: 'Sale sent.', saleRefused: 'The server refused the sale — call the owner.',
    saleNoStore: 'This device cannot save the sale. Do not make the sale.', saleUnsent: '{n} unsent sale(s)',
    saleUnsentSignOut: 'There are unsent sales: connect the tablet before signing out.', saleUnsentClose: 'Unsent sales are in this drawer: connect the tablet before closing it.', saleZero: 'A sale that takes no money is not recorded.', saleNoMenu: 'No menu is saved on this tablet.',
    refusal_quantity: 'Quantity 1 to 99.', refusal_unknown: 'This dish is not on the menu.', refusal_off_sale: 'This dish is off sale.',
    refusal_no_price: 'This dish has no price.', refusal_options: 'This dish needs options: sell it when the network is back.' },
  uk: { offlineBanner: 'Офлайн — лише продаж за готівку. Продажі надійдуть, коли повернеться мережа.',
    cashSale: 'Продаж за готівку', sellTitle: 'Продаж за готівку без мережі', sellHint: 'Ціни з меню, збереженого на цьому планшеті.',
    menuCachedAt: 'Збережене меню: {age}', sellN: 'Продати {n}', sellConfirm: 'Підтвердіть продаж', cashTaken: 'Готівку отримано — продати',
    cashOnly: 'Лише готівка: картка потребує мережі.', receipt: 'Чек', saleTotal: 'Разом', saleCash: 'Оплачено готівкою',
    saleRef: 'Реф.', saleNoNivf: 'без NIVF — буде фіскалізовано протягом 48 годин', saleNoNivfSq: SQ_LAW, print: 'Друк', done: 'Готово',
    saleQueued: 'Продаж збережено; його буде надіслано.', saleSynced: 'Продаж надіслано.', saleRefused: 'Сервер відхилив продаж — покличте власника.',
    saleNoStore: 'Цей пристрій не може зберегти продаж. Не робіть продаж.', saleUnsent: 'Ненадісланих продажів: {n}',
    saleUnsentSignOut: 'Є ненадіслані продажі: підключіть планшет перед виходом.', saleUnsentClose: 'У цій касі є ненадіслані продажі: підключіть планшет перед закриттям.', saleZero: 'Продаж без грошей не записується.', saleNoMenu: 'На цьому планшеті немає збереженого меню.',
    refusal_quantity: 'Кількість від 1 до 99.', refusal_unknown: 'Цієї страви немає в меню.', refusal_off_sale: 'Ця страва знята з продажу.',
    refusal_no_price: 'У цієї страви немає ціни.', refusal_options: 'Ця страва потребує вибору опцій: продайте, коли повернеться мережа.' },
  ru: { offlineBanner: 'Офлайн — только продажа за наличные. Продажи уйдут, когда вернётся сеть.',
    cashSale: 'Продажа за наличные', sellTitle: 'Продажа за наличные без сети', sellHint: 'Цены из меню, сохранённого на этом планшете.',
    menuCachedAt: 'Сохранённое меню: {age}', sellN: 'Продать {n}', sellConfirm: 'Подтвердите продажу', cashTaken: 'Наличные получены — продать',
    cashOnly: 'Только наличные: карте нужна сеть.', receipt: 'Чек', saleTotal: 'Итого', saleCash: 'Оплачено наличными',
    saleRef: 'Реф.', saleNoNivf: 'без NIVF — будет фискализировано в течение 48 часов', saleNoNivfSq: SQ_LAW, print: 'Печать', done: 'Готово',
    saleQueued: 'Продажа сохранена; она будет отправлена.', saleSynced: 'Продажа отправлена.', saleRefused: 'Сервер отклонил продажу — позовите владельца.',
    saleNoStore: 'Это устройство не может сохранить продажу. Не делайте продажу.', saleUnsent: 'Неотправленных продаж: {n}',
    saleUnsentSignOut: 'Есть неотправленные продажи: подключите планшет перед выходом.', saleUnsentClose: 'В этой кассе есть неотправленные продажи: подключите планшет перед закрытием.', saleZero: 'Продажа без денег не записывается.', saleNoMenu: 'На этом планшете нет сохранённого меню.',
    refusal_quantity: 'Количество от 1 до 99.', refusal_unknown: 'Этого блюда нет в меню.', refusal_off_sale: 'Это блюдо снято с продажи.',
    refusal_no_price: 'У этого блюда нет цены.', refusal_options: 'Этому блюду нужен выбор опций: продайте, когда вернётся сеть.' },
};

for (const [l, dict] of Object.entries(WORDS)) {
  T[l] = T[l] || {};
  for (const [key, v] of Object.entries(dict)) if (!(key in T[l])) T[l][key] = v;
}
