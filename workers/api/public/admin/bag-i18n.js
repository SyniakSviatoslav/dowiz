// The words of "Bring guests direct" (W-QR): the bag insert, its welcome
// offer and its card. Imported by more.js (the row's own label) and bag.js.
// Merged into the shared table on import, never replacing a key it has.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { T, LANGS } from '/admin/i18n.js';

export const WORDS = {
  sq: {
    bagQr: 'Sillni klientet direkt', bagQrSub: 'kod QR per qeset e dergesave',
    bag_hint: 'Nje karte ne cdo qese: klienti qe erdhi nga Wolt ose Glovo porosit heren tjeter direkt nga ju, pa komision.',
    bag_warn: 'Kontrolloni kontraten tuaj me Wolt/Glovo: disa ndalojne fletepalosje ne qese ose kerkojne te njejtin cmim. dowiz nuk zbaton asgje atje. Cmimet e menuse nuk ndryshojne; bonusi eshte shtese.',
    bag_offer: 'Oferta e mireseardhjes', bag_offerHint: 'E njejta per te gjithe qe skanojne. Nje here per numer telefoni.',
    bag_kind: 'Cfare merr klienti', bag_k_off: 'Pa oferte', bag_k_fixed: 'Zbritje ne lek', bag_k_gift: 'Nje pjate falas', bag_k_stamps: 'Vule e dyfishte',
    bag_value: 'Zbritja', bag_min: 'Shporta minimale', bag_gift: 'Pjata falas', bag_pct: 'Komisioni i platformes, %', bag_pctHint: 'Sa ju merr Wolt/Glovo. Bosh = nuk llogaritet kursimi.',
    bag_campaign: 'Fushata (opsionale)', bag_campaignHint: 'Nje fjale e shkurter: shkronja te vogla, shifra, -',
    bag_line: 'Heren tjeter porositni direkt', bag_fixed: '{value} zbritje', bag_fixedMin: '{value} zbritje mbi {min}', bag_giftOf: '{dish} falas', bag_stamps: 'vule e dyfishte',
    bag_card: 'Karta A6', bag_stickers: '8 ngjitese', bag_print: 'Printo', bag_svg: 'SVG', bag_png: 'PNG',
    bag_stats: 'Nga qeset', bag_scans: 'Skanime', bag_scansNone: 'nuk numerohen (pa gjurmim)', bag_orders: 'Porosi nga qeset',
    bag_guests: 'Kliente', bag_repeat: 'Porosi te perseritura', bag_saved: 'Komision i kursyer (vleresim)', bag_savedNone: 'shkruani % per vleresim',
  },
  en: {
    bagQr: 'Bring guests direct', bagQrSub: 'QR card for delivery bags',
    bag_hint: 'A card in every bag: a guest who came through Wolt or Glovo orders the next time directly from you, with no commission.',
    bag_warn: 'Check your Wolt/Glovo contract: some forbid bag inserts or require price parity. dowiz enforces nothing there. Menu prices do not change; the bonus is an extra.',
    bag_offer: 'Welcome offer', bag_offerHint: 'The same for everyone who scans. Once per phone number.',
    bag_kind: 'What the guest gets', bag_k_off: 'No offer', bag_k_fixed: 'An amount off', bag_k_gift: 'A dish on the house', bag_k_stamps: 'Double stamp',
    bag_value: 'Amount off', bag_min: 'Minimum basket', bag_gift: 'Dish on the house', bag_pct: 'Platform commission, %', bag_pctHint: 'What Wolt/Glovo takes. Empty = no saving is estimated.',
    bag_campaign: 'Campaign (optional)', bag_campaignHint: 'One short word: lowercase letters, digits, -',
    bag_line: 'Next time order direct', bag_fixed: '{value} off', bag_fixedMin: '{value} off from {min}', bag_giftOf: '{dish} on the house', bag_stamps: 'a double stamp',
    bag_card: 'A6 card', bag_stickers: '8 stickers', bag_print: 'Print', bag_svg: 'SVG', bag_png: 'PNG',
    bag_stats: 'From the bags', bag_scans: 'Scans', bag_scansNone: 'not counted (no tracking)', bag_orders: 'Orders from bags',
    bag_guests: 'Guests', bag_repeat: 'Repeat orders', bag_saved: 'Commission saved (estimate)', bag_savedNone: 'type a % to estimate',
  },
  uk: {
    bagQr: 'Гості напряму', bagQrSub: 'QR-картка в пакети доставки',
    bag_hint: 'Картка в пакеті з кожним замовленням: гість, що прийшов через Wolt чи Glovo, наступного разу замовляє напряму у вас, без комісії.',
    bag_warn: 'Перевірте договір з Wolt/Glovo: деякі забороняють вкладення в пакети або вимагають однакових цін. dowiz там нічого не контролює. Ціни меню не змінюються; бонус - це доповнення.',
    bag_offer: 'Вітальна пропозиція', bag_offerHint: 'Однакова для всіх, хто сканує. Один раз на номер телефону.',
    bag_kind: 'Що отримує гість', bag_k_off: 'Без пропозиції', bag_k_fixed: 'Знижка в леках', bag_k_gift: 'Страва в подарунок', bag_k_stamps: 'Подвійний штамп',
    bag_value: 'Знижка', bag_min: 'Мінімальний кошик', bag_gift: 'Страва в подарунок', bag_pct: 'Комісія платформи, %', bag_pctHint: 'Скільки бере Wolt/Glovo. Порожньо = економія не рахується.',
    bag_campaign: 'Кампанія (необовязково)', bag_campaignHint: 'Одне коротке слово: малі літери, цифри, -',
    bag_line: 'Наступного разу замовляйте напряму', bag_fixed: '{value} знижки', bag_fixedMin: '{value} знижки від {min}', bag_giftOf: '{dish} у подарунок', bag_stamps: 'подвійний штамп',
    bag_card: 'Картка A6', bag_stickers: '8 наліпок', bag_print: 'Друк', bag_svg: 'SVG', bag_png: 'PNG',
    bag_stats: 'З пакетів', bag_scans: 'Сканування', bag_scansNone: 'не рахуються (без стеження)', bag_orders: 'Замовлення з пакетів',
    bag_guests: 'Гості', bag_repeat: 'Повторні замовлення', bag_saved: 'Збережена комісія (оцінка)', bag_savedNone: 'введіть % для оцінки',
  },
  ru: {
    bagQr: 'Гости напрямую', bagQrSub: 'QR-карточка в пакеты доставки',
    bag_hint: 'Карточка в пакете с каждым заказом: гость, пришедший через Wolt или Glovo, в следующий раз заказывает напрямую у вас, без комиссии.',
    bag_warn: 'Проверьте договор с Wolt/Glovo: некоторые запрещают вложения в пакеты или требуют одинаковых цен. dowiz там ничего не контролирует. Цены меню не меняются; бонус - это дополнение.',
    bag_offer: 'Приветственное предложение', bag_offerHint: 'Одинаковое для всех, кто сканирует. Один раз на номер телефона.',
    bag_kind: 'Что получает гость', bag_k_off: 'Без предложения', bag_k_fixed: 'Скидка в леках', bag_k_gift: 'Блюдо в подарок', bag_k_stamps: 'Двойной штамп',
    bag_value: 'Скидка', bag_min: 'Минимальная корзина', bag_gift: 'Блюдо в подарок', bag_pct: 'Комиссия платформы, %', bag_pctHint: 'Сколько берёт Wolt/Glovo. Пусто = экономия не считается.',
    bag_campaign: 'Кампания (необязательно)', bag_campaignHint: 'Одно короткое слово: строчные буквы, цифры, -',
    bag_line: 'В следующий раз заказывайте напрямую', bag_fixed: '{value} скидки', bag_fixedMin: '{value} скидки от {min}', bag_giftOf: '{dish} в подарок', bag_stamps: 'двойной штамп',
    bag_card: 'Карточка A6', bag_stickers: '8 наклеек', bag_print: 'Печать', bag_svg: 'SVG', bag_png: 'PNG',
    bag_stats: 'Из пакетов', bag_scans: 'Сканирования', bag_scansNone: 'не считаются (без слежки)', bag_orders: 'Заказы из пакетов',
    bag_guests: 'Гости', bag_repeat: 'Повторные заказы', bag_saved: 'Сэкономленная комиссия (оценка)', bag_savedNone: 'введите % для оценки',
  },
};
for (const l of LANGS) for (const [key, v] of Object.entries({ ...WORDS.en, ...(WORDS[l] || {}) })) if (T[l] && !(key in T[l])) T[l][key] = v;
