// The words the guest booking adds, in the storefront's three languages.
// Merged into the shared table on import (the way admin/tableqr.js does), so
// the shared i18n.js is not edited by this lane.
//
// ASCII QUOTES ONLY: a typographic quote once took down a whole console.

import { T, LANGS } from '/store/i18n.js';

const WORDS = {
  sq: {
    bkName: 'Emri', bkPhone: 'Telefoni', bkWho: 'Rezervimi në emër të',
    bkNeedName: 'Shkruani emrin e rezervimit', bkNameLong: 'Emri është shumë i gjatë',
    bkNeedPhone: 'Shkruani një numër telefoni që restoranti mund të thërrasë',
    bkCtaAny: 'Rezervo, çdo tavolinë', bkAnyTable: 'Çdo tavolinë e lirë',
    bkNoPlanBook: 'Ky lokal nuk ka publikuar planin e sallës. Rezervoni dhe restoranti ju ul ku ka vend.',
    bkMine: 'Rezervimet e mia', bkMineNone: 'Asnjë rezervim në këtë shfletues.',
    bkCancel: 'Anulo rezervimin', bkCancelSure: 'Ta anulojmë këtë rezervim?', bkCancelled: 'Rezervimi u anulua',
    bkShare: 'Kopjo lidhjen e rezervimit', bkCopied: 'Lidhja u kopjua', bkParty: 'persona',
    bkLoadFail: 'Rezervimi nuk u lexua', bkRef: 'Numri',
    bkStRequested: 'Në pritje të konfirmimit', bkStConfirmed: 'I konfirmuar', bkStSeated: 'Në tavolinë',
    bkStCompleted: 'Përfundoi', bkStDeclined: 'Refuzuar nga restoranti', bkStCancelled: 'Anuluar nga ju',
    bkStCancelledVenue: 'Anuluar nga restoranti', bkStNoShow: 'Nuk erdhët',
  },
  en: {
    bkName: 'Name', bkPhone: 'Phone', bkWho: 'Booking under',
    bkNeedName: 'Enter the name for the booking', bkNameLong: 'That name is too long',
    bkNeedPhone: 'Enter a phone number the restaurant can call',
    bkCtaAny: 'Book, any table', bkAnyTable: 'Any free table',
    bkNoPlanBook: 'This venue has not published a floor plan. Book, and the restaurant seats you where there is room.',
    bkMine: 'My bookings', bkMineNone: 'No bookings in this browser.',
    bkCancel: 'Cancel the booking', bkCancelSure: 'Cancel this booking?', bkCancelled: 'The booking is cancelled',
    bkShare: 'Copy the booking link', bkCopied: 'Link copied', bkParty: 'guests',
    bkLoadFail: 'The booking did not load', bkRef: 'Ref',
    bkStRequested: 'Waiting for the restaurant', bkStConfirmed: 'Confirmed', bkStSeated: 'Seated',
    bkStCompleted: 'Completed', bkStDeclined: 'Declined by the restaurant', bkStCancelled: 'Cancelled by you',
    bkStCancelledVenue: 'Cancelled by the restaurant', bkStNoShow: 'Missed',
  },
  uk: {
    bkName: "Ім'я", bkPhone: 'Телефон', bkWho: "Бронювання на ім'я",
    bkNeedName: "Вкажіть ім'я для бронювання", bkNameLong: "Ім'я задовге",
    bkNeedPhone: 'Вкажіть номер телефону, за яким ресторан може подзвонити',
    bkCtaAny: 'Забронювати будь-який стіл', bkAnyTable: 'Будь-який вільний стіл',
    bkNoPlanBook: 'Заклад ще не опублікував план зали. Бронюйте, і ресторан посадить вас туди, де є місце.',
    bkMine: 'Мої бронювання', bkMineNone: 'У цьому браузері бронювань немає.',
    bkCancel: 'Скасувати бронювання', bkCancelSure: 'Скасувати це бронювання?', bkCancelled: 'Бронювання скасовано',
    bkShare: 'Скопіювати посилання', bkCopied: 'Посилання скопійовано', bkParty: 'гостей',
    bkLoadFail: 'Бронювання не завантажилось', bkRef: 'Номер',
    bkStRequested: 'Чекає підтвердження', bkStConfirmed: 'Підтверджено', bkStSeated: 'За столом',
    bkStCompleted: 'Завершено', bkStDeclined: 'Ресторан відхилив', bkStCancelled: 'Скасовано вами',
    bkStCancelledVenue: 'Скасовано рестораном', bkStNoShow: 'Не прийшли',
  },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);
