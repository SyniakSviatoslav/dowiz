// The room's words, in the three languages. sq is the default: the venue is in
// Albania. The ORDER STATUS words are not copied here: they are the console's
// (`admin/i18n.js` `st`), which `tools/gates/vocabulary.sh` holds to the
// kernel's twelve -- a fifth hand copy is how a status goes missing.
import { safeGet, safeSet } from '../store/storage.js';
import { T as ADMIN } from '../admin/i18n.js';

export const T = {
  sq: {
    waiter: 'Kamarier', 'counter-manager': 'Arkëtar-menaxher', kitchen: 'Kuzhina', owner: 'Pronar',
    loginLine: 'Salla, në dorën tuaj.', signIn: 'Hyni', email: 'Email', password: 'Fjalëkalimi',
    claimCode: 'Kodi i ftesës', haveCode: 'Kam një kod ftese', haveAccount: 'Kam llogari', claim: 'Aktivizo',
    signOut: 'Dilni', loading: 'Po ngarkohet…', refresh: 'Rifresko', back: 'Mbrapa', cancel: 'Anulo', send: 'Dërgo',
    room: 'Salla', noOrders: 'Asnjë tavolinë e hapur', table: 'Tavolina', rounds: 'raunde', due: 'Për t’u paguar',
    kitchenNoRoom: 'Kuzhina e ndjek porosinë në ekranin e kuzhinës; ky ekran është për sallën.',
    noLines: 'Asnjë artikull', subtotal: 'Nëntotali', discount: 'Zbritja', total: 'Totali', owed: 'Mbetet',
    addItem: 'Shto artikuj', addN: 'Shto {n}', search: 'Kërko', noMatch: 'Asgjë nuk përputhet', unavailable: 'mbaroi',
    remove: 'Hiq', comp: 'Falas', comped: 'Falas', less: 'Më pak', more: 'Më shumë',
    moveTable: 'Kalo në tavolinën', move: 'Kalo', reason: 'Arsyeja', whyRemove: 'Pse hiqet?', whyComp: 'Pse falas?',
    reason_mistake: 'Gabim', reason_guest_changed: 'Klienti ndryshoi', reason_unavailable: 'Mbaroi',
    reason_dropped: 'Ra', reason_other: 'Tjetër', otherText: 'Shkruani arsyen', needReason: 'Shkruani një arsye.',
    changedReload: 'Porosia ndryshoi ndërkohë. U ringarkua — shikojeni dhe provoni sërish.',
    take: 'Merr pagesën', takeN: 'Merr {a}', amount: 'Shuma', method: 'Mënyra', currency: 'Valuta', rate: 'Kursi',
    method_cash: 'Kesh', method_card: 'Kartë', method_cheque: 'Çek', method_transfer: 'Transfertë',
    method_gift_card: 'Kartë dhuratë', method_other: 'Tjetër',
    rateNeeded: 'Shkruani kursin e tabelës.', offTheBill: 'nga fatura', fillOwed: 'Sa mbetet',
    taken: 'U mor', paidInFull: 'E paguar plotësisht.', badAmount: 'Shuma nuk lexohet.', badRate: 'Kursi nuk lexohet.',
    till: 'Arka', openTill: 'Hap arkën', closeTill: 'Mbyll arkën', floatHint: 'Paratë në arkë në fillim, për çdo valutë.',
    tillFloat: 'Fillimi', counted: 'Numëruar', expected: 'Pritej', overShort: 'Tepër / mangët',
    cashMove: 'Para brenda / jashtë', payIn: 'Futje', payOut: 'Nxjerrje', reasonText: 'Arsyeja',
    count: 'Numëro arkën', countHint: 'Shkruani çfarë ka arka. Shuma që pritej shfaqet vetëm në mbyllje. Bosh = zero.',
    saveCount: 'Ruaj numërimin', closeHint: 'Pas mbylljes shihni pritjen, numërimin dhe diferencën.',
    closeSure: 'E numërova dhe e mbyll', tillOpen: 'Arka është hapur', tillClosedWord: 'Arka u mbyll',
    tillUnknown: 'Ky telefon nuk e di ende gjendjen e arkës.', blindNote: 'Numërimi u ruajt. Krahasimi del në mbyllje.',
    openedAt: 'hapur', offline: 'Pa lidhje', live: 'Në lidhje', ageS: 'para {n} s', ageM: 'para {n} min', ageH: 'para {n} orë',
    error: 'Gabim', saved: 'U ruajt', noSlug: 'Hapeni nga adresa e lokalit tuaj.', menuFailed: 'Menyja nuk u ngarkua.',
    queuedN: '{n} në pritje', queuedSaved: 'Pa lidhje — u ruajt, dërgohet vetë.', queuedChanged: 'Porosia ndryshoi; një veprim i ruajtur nuk u pranua.',
    queuedRefused: 'Një veprim i ruajtur u refuzua.', queueFull: 'Radha e plotë — nuk u ruajt.', queueNoStore: 'Ky shfletues nuk ruan — nuk u ruajt.',
    moveLines: 'Kalo artikujt', moveLinesTo: 'Te raundi', pickLines: 'Zgjidhni artikujt që kalojnë.', pickRound: 'Zgjidhni raundin ku shkojnë.',
    notAllLines: 'Të paktën një artikull duhet të mbetet; për të gjithë, anuloni raundin.', noTargets: 'Asnjë raund tjetër nuk i pranon para kuzhinës.',
    kitchenHasIt: 'Kuzhina e ka tashmë këtë raund; artikujt nuk lëvizin më.', roundPaid: 'Ky raund është paguar; nuk ndryshohet.',
    alreadyThere: 'Tavolina është tashmë aty.', notHere: 'Kjo porosi nuk gjendet më; salla u ringarkua.', moved: 'U kalua',
    moveSitting: 'Kalo tavolinën', moveSittingHint: 'Të gjitha raundet që janë ende në sallë kalojnë në tavolinën e re.',
    openTable: 'Hap tavolinë', tableName: 'Tavolina', needTable: 'Shkruani tavolinën.',
    tip: 'Bakshish', walletId: 'Portofoli (ID-ja e klientit)', badTip: 'Bakshishi nuk lexohet.', needWallet: 'Shkruani portofolin që paguan.', method_wallet: 'Portofol', walletNoTip: 'Portofoli paguan vetëm faturën; bakshishi merret me para ose kartë.',
    language: 'Gjuha', theme: 'Tema',
  },
  en: {
    waiter: 'Waiter', 'counter-manager': 'Counter manager', kitchen: 'Kitchen', owner: 'Owner',
    loginLine: 'The room, in your hand.', signIn: 'Sign in', email: 'Email', password: 'Password',
    claimCode: 'Invite code', haveCode: 'I have an invite code', haveAccount: 'I have an account', claim: 'Activate',
    signOut: 'Sign out', loading: 'Loading…', refresh: 'Refresh', back: 'Back', cancel: 'Cancel', send: 'Send',
    room: 'Room', noOrders: 'No open tables', table: 'Table', rounds: 'rounds', due: 'Due',
    kitchenNoRoom: 'The kitchen follows orders on the kitchen screen; this one is for the floor.',
    noLines: 'No items', subtotal: 'Subtotal', discount: 'Discount', total: 'Total', owed: 'Still owed',
    addItem: 'Add items', addN: 'Add {n}', search: 'Search', noMatch: 'Nothing matches', unavailable: 'sold out',
    remove: 'Remove', comp: 'Comp', comped: 'Comped', less: 'Fewer', more: 'More',
    moveTable: 'Move to table', move: 'Move', reason: 'Reason', whyRemove: 'Why remove it?', whyComp: 'Why comp it?',
    reason_mistake: 'Mistake', reason_guest_changed: 'Guest changed', reason_unavailable: 'Unavailable',
    reason_dropped: 'Dropped', reason_other: 'Other', otherText: 'Say why', needReason: 'Give a reason.',
    changedReload: 'This order changed while you were editing. Reloaded — check it and try again.',
    take: 'Take payment', takeN: 'Take {a}', amount: 'Amount', method: 'Method', currency: 'Currency', rate: 'Rate',
    method_cash: 'Cash', method_card: 'Card', method_cheque: 'Cheque', method_transfer: 'Transfer',
    method_gift_card: 'Gift card', method_other: 'Other',
    rateNeeded: 'Type the board rate.', offTheBill: 'off the bill', fillOwed: 'What is owed',
    taken: 'Taken', paidInFull: 'Paid in full.', badAmount: 'That amount does not read.', badRate: 'That rate does not read.',
    till: 'Till', openTill: 'Open till', closeTill: 'Close till', floatHint: 'What the drawer starts with, per currency.',
    tillFloat: 'Float', counted: 'Counted', expected: 'Expected', overShort: 'Over / short',
    cashMove: 'Cash in / out', payIn: 'Pay in', payOut: 'Pay out', reasonText: 'Reason',
    count: 'Count the drawer', countHint: 'Enter what the drawer holds. The expected figure appears only at close. Empty = zero.',
    saveCount: 'Save count', closeHint: 'After closing you see expected, counted and the difference.',
    closeSure: 'I counted it and am closing', tillOpen: 'Till is open', tillClosedWord: 'Till closed',
    tillUnknown: 'This phone does not know the till yet.', blindNote: 'Count saved. The comparison comes at close.',
    openedAt: 'opened', offline: 'Offline', live: 'Live', ageS: '{n} s ago', ageM: '{n} min ago', ageH: '{n} h ago',
    error: 'Error', saved: 'Saved', noSlug: 'Open this from your venue address.', menuFailed: 'The menu did not load.',
    queuedN: '{n} waiting', queuedSaved: 'Offline — saved, it will send itself.', queuedChanged: 'The order changed; a saved action was not accepted.',
    queuedRefused: 'A saved action was refused.', queueFull: 'Queue full — not saved.', queueNoStore: 'This browser will not store — not saved.',
    moveLines: 'Move lines', moveLinesTo: 'To round', pickLines: 'Pick the lines that move.', pickRound: 'Pick the round they go to.',
    notAllLines: 'At least one line must stay; to move them all, cancel the round.', noTargets: 'No other round can take them before the kitchen.',
    kitchenHasIt: 'The kitchen already has this round; its lines no longer move.', roundPaid: 'This round is paid; it cannot be changed.',
    alreadyThere: 'The table is already there.', notHere: 'This order is no longer here; the room was reloaded.', moved: 'Moved',
    moveSitting: 'Move the table', moveSittingHint: 'Every round still in the room moves to the new table.',
    openTable: 'Open a table', tableName: 'Table', needTable: 'Name the table.',
    tip: 'Tip', walletId: 'Wallet (the guest\'s id)', badTip: 'That tip does not read.', needWallet: 'Name the wallet that pays.', method_wallet: 'Wallet', walletNoTip: 'A wallet pays the bill only; take the tip in cash or on the card.',
    language: 'Language', theme: 'Theme',
  },
  uk: {
    waiter: 'Офіціант', 'counter-manager': 'Касир-менеджер', kitchen: 'Кухня', owner: 'Власник',
    loginLine: 'Зал у вашій руці.', signIn: 'Увійти', email: 'Email', password: 'Пароль',
    claimCode: 'Код запрошення', haveCode: 'У мене код запрошення', haveAccount: 'У мене є акаунт', claim: 'Активувати',
    signOut: 'Вийти', loading: 'Завантажуємо…', refresh: 'Оновити', back: 'Назад', cancel: 'Скасувати', send: 'Надіслати',
    room: 'Зал', noOrders: 'Відкритих столів немає', table: 'Стіл', rounds: 'раунди', due: 'До сплати',
    kitchenNoRoom: 'Кухня бачить замовлення на кухонному екрані; цей екран для залу.',
    noLines: 'Позицій немає', subtotal: 'Підсумок', discount: 'Знижка', total: 'Разом', owed: 'Залишилось',
    addItem: 'Додати позиції', addN: 'Додати {n}', search: 'Пошук', noMatch: 'Нічого не знайдено', unavailable: 'закінчилось',
    remove: 'Прибрати', comp: 'За рахунок закладу', comped: 'За рахунок закладу', less: 'Менше', more: 'Більше',
    moveTable: 'Пересадити за стіл', move: 'Пересадити', reason: 'Причина', whyRemove: 'Чому прибрати?', whyComp: 'Чому безкоштовно?',
    reason_mistake: 'Помилка', reason_guest_changed: 'Гість передумав', reason_unavailable: 'Немає в наявності',
    reason_dropped: 'Впало', reason_other: 'Інше', otherText: 'Вкажіть причину', needReason: 'Вкажіть причину.',
    changedReload: 'Замовлення змінилось, поки ви редагували. Оновлено — перевірте й спробуйте ще.',
    take: 'Прийняти оплату', takeN: 'Прийняти {a}', amount: 'Сума', method: 'Спосіб', currency: 'Валюта', rate: 'Курс',
    method_cash: 'Готівка', method_card: 'Картка', method_cheque: 'Чек', method_transfer: 'Переказ',
    method_gift_card: 'Подарункова картка', method_other: 'Інше',
    rateNeeded: 'Введіть курс з табло.', offTheBill: 'з рахунку', fillOwed: 'Скільки залишилось',
    taken: 'Прийнято', paidInFull: 'Сплачено повністю.', badAmount: 'Суму не прочитати.', badRate: 'Курс не прочитати.',
    till: 'Каса', openTill: 'Відкрити касу', closeTill: 'Закрити касу', floatHint: 'З чим каса починає, для кожної валюти.',
    tillFloat: 'Розмін', counted: 'Пораховано', expected: 'Очікувалось', overShort: 'Надлишок / нестача',
    cashMove: 'Внесення / вилучення', payIn: 'Внести', payOut: 'Вилучити', reasonText: 'Причина',
    count: 'Перерахувати касу', countHint: 'Введіть, що є в касі. Очікувана сума з’явиться лише при закритті. Порожньо = нуль.',
    saveCount: 'Зберегти перерахунок', closeHint: 'Після закриття видно очікуване, пораховане й різницю.',
    closeSure: 'Я перерахував і закриваю', tillOpen: 'Каса відкрита', tillClosedWord: 'Касу закрито',
    tillUnknown: 'Цей телефон ще не знає стану каси.', blindNote: 'Перерахунок збережено. Порівняння — при закритті.',
    openedAt: 'відкрито', offline: 'Без зв’язку', live: 'На зв’язку', ageS: '{n} с тому', ageM: '{n} хв тому', ageH: '{n} год тому',
    error: 'Помилка', saved: 'Збережено', noSlug: 'Відкрийте з адреси вашого закладу.', menuFailed: 'Меню не завантажилось.',
    queuedN: '{n} в черзі', queuedSaved: 'Без зв’язку — збережено, надішлеться саме.', queuedChanged: 'Замовлення змінилось; збережену дію не прийнято.',
    queuedRefused: 'Збережену дію відхилено.', queueFull: 'Черга повна — не збережено.', queueNoStore: 'Браузер не зберігає — не збережено.',
    moveLines: 'Перенести позиції', moveLinesTo: 'До раунду', pickLines: 'Виберіть позиції, що переходять.', pickRound: 'Виберіть раунд, куди вони йдуть.',
    notAllLines: 'Хоч одна позиція має лишитись; щоб перенести всі, скасуйте раунд.', noTargets: 'Жоден інший раунд не прийме їх до кухні.',
    kitchenHasIt: 'Кухня вже має цей раунд; позиції більше не переносяться.', roundPaid: 'Цей раунд сплачено; його не змінити.',
    alreadyThere: 'Стіл уже там.', notHere: 'Цього замовлення вже немає; зал оновлено.', moved: 'Перенесено',
    moveSitting: 'Пересадити стіл', moveSittingHint: 'Усі раунди, що ще в залі, переходять за новий стіл.',
    openTable: 'Відкрити стіл', tableName: 'Стіл', needTable: 'Вкажіть стіл.',
    tip: 'Чайові', walletId: 'Гаманець (ID гостя)', badTip: 'Чайові не прочитати.', needWallet: 'Вкажіть гаманець, що платить.', method_wallet: 'Гаманець', walletNoTip: 'Гаманець сплачує лише рахунок; чайові готівкою або карткою.',
    language: 'Мова', theme: 'Тема',
  },
};

export const LANGS = ['sq', 'en', 'uk'];
let current = LANGS.includes(safeGet('dw_room_lang')) ? safeGet('dw_room_lang') : 'sq';

export const lang = () => current;
export const t = k => T[current]?.[k] ?? T.en[k] ?? k;
export const statusWord = s => ADMIN[current]?.st?.[s] ?? ADMIN.en?.st?.[s] ?? s;
export const intlLocale = () => current;

export function nextLang() {
  current = LANGS[(LANGS.indexOf(current) + 1) % LANGS.length];
  safeSet('dw_room_lang', current);
  document.documentElement.lang = current;
  return current;
}

/// Static copy in index.html carries `data-t` / `data-t-attr="attr:key ..."`.
export function retranslate(root = document) {
  root.querySelectorAll('[data-t]').forEach(el => { el.textContent = t(el.dataset.t); });
  root.querySelectorAll('[data-t-attr]').forEach(el => {
    for (const pair of el.dataset.tAttr.split(' ')) {
      const [attr, key] = pair.split(':');
      if (attr && key) el.setAttribute(attr, t(key));
    }
  });
}
