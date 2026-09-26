// The kitchen board's words (lane W-KITCHEN, 2026-09-26), kept out of the
// console's shared dictionary and MERGED into it at import: `T` is the
// console's own table, so `t()`, `data-t` and `retranslate()` read these like
// any other key. A language the console gains later (ru is next) falls back to
// English until its words are added here; nothing below lists the languages.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { T } from '/admin/i18n.js';

export const WORDS = {
  sq: {
    tabKitchen: 'Kuzhina', kBoardHint: 'Prekni një biletë kur e shihni. Butoni i madh e çon përpara.',
    col_new: 'Të reja', col_preparing: 'Në përgatitje', col_ready: 'Gati', kAll: 'Të gjitha',
    st_sushi: 'Sushi', st_kitchen: 'Kuzhina', st_bar: 'Bar',
    kTable: 'Tavolina', kPickup: 'Merret', kDelivery: 'Dërgesë', kMin: 'min',
    bump_confirm: 'Prano', bump_preparing: 'Fillo', bump_ready: 'Gati', bump_collected: 'U dorëzua',
    stop_reject: 'Refuzo', stop_cancel: 'Anulo', kReason: 'Arsyeja', kReasonHint: 'Klienti e sheh arsyen.',
    kReasonNeeded: 'Shkruani arsyen.', kSeen: 'parë', kUnseen: 'prekni kur ta shihni',
    kAllDay: 'Gjithë ditën', kAllDayHint: 'Sa pjata presin ende në biletat e hapura.',
    kStopList: 'Stop-lista', kStopHint: 'Hiqni një pjatë nga shitja menjëherë; klientët nuk e shohin më.',
    k86: 'Hiqe (86)', kBackOn: 'Ktheje', kOffSale: 'jo në shitje', kSearchDish: 'Kërko pjatën',
    kNoTickets: 'Asnjë biletë e hapur', kNoTicketsHint: 'Porosia e re shfaqet këtu vetë, me zile.',
    kLive: 'drejtpërdrejt', kPolling: 'rifreskim', kNote: 'Shënim',
  },
  en: {
    tabKitchen: 'Kitchen', kBoardHint: 'Tap a ticket when you have seen it. The big button moves it on.',
    col_new: 'New', col_preparing: 'Preparing', col_ready: 'Ready', kAll: 'All',
    st_sushi: 'Sushi', st_kitchen: 'Kitchen', st_bar: 'Bar',
    kTable: 'Table', kPickup: 'Pickup', kDelivery: 'Delivery', kMin: 'min',
    bump_confirm: 'Accept', bump_preparing: 'Start', bump_ready: 'Ready', bump_collected: 'Handed over',
    stop_reject: 'Reject', stop_cancel: 'Cancel', kReason: 'Reason', kReasonHint: 'The customer sees the reason.',
    kReasonNeeded: 'Write the reason.', kSeen: 'seen', kUnseen: 'tap when seen',
    kAllDay: 'All day', kAllDayHint: 'How many of each dish the open tickets still need.',
    kStopList: 'Stop list', kStopHint: 'Take a dish off sale at once; customers stop seeing it.',
    k86: '86 it', kBackOn: 'Back on', kOffSale: 'off sale', kSearchDish: 'Find a dish',
    kNoTickets: 'No open tickets', kNoTicketsHint: 'A new order appears here by itself, with a ring.',
    kLive: 'live', kPolling: 'refreshing', kNote: 'Note',
  },
  uk: {
    tabKitchen: 'Кухня', kBoardHint: 'Торкніться чека, коли побачили. Велика кнопка рухає його далі.',
    col_new: 'Нові', col_preparing: 'Готуються', col_ready: 'Готові', kAll: 'Усі',
    st_sushi: 'Суші', st_kitchen: 'Кухня', st_bar: 'Бар',
    kTable: 'Стіл', kPickup: 'Самовивіз', kDelivery: 'Доставка', kMin: 'хв',
    bump_confirm: 'Прийняти', bump_preparing: 'Почати', bump_ready: 'Готово', bump_collected: 'Видано',
    stop_reject: 'Відхилити', stop_cancel: 'Скасувати', kReason: 'Причина', kReasonHint: 'Клієнт бачить причину.',
    kReasonNeeded: 'Напишіть причину.', kSeen: 'побачено', kUnseen: 'торкніться, коли побачите',
    kAllDay: 'Усього зараз', kAllDayHint: 'Скільки кожної страви ще чекають відкриті чеки.',
    kStopList: 'Стоп-лист', kStopHint: 'Зніміть страву з продажу одразу; клієнти її більше не бачать.',
    k86: 'Стоп (86)', kBackOn: 'Повернути', kOffSale: 'не в продажу', kSearchDish: 'Знайти страву',
    kNoTickets: 'Відкритих чеків немає', kNoTicketsHint: 'Нове замовлення з\'явиться тут само, зі звуком.',
    kLive: 'наживо', kPolling: 'оновлення', kNote: 'Примітка',
  },
};

/// Merge into the console's table. Existing keys are never overwritten: the
/// shared dictionary wins, so this file cannot silently rename a console word.
export function merge(table, words = WORDS){
  for (const [l, dict] of Object.entries(words)) {
    table[l] = table[l] || {};
    for (const [k, v] of Object.entries(dict)) if (!(k in table[l])) table[l][k] = v;
  }
  return table;
}

merge(T);
