// The words the bag card's welcome adds to the storefront (W-QR), in every
// language it speaks. Merged into the shared table on import (the way
// booking-words.js does), so the shared i18n.js is not edited by this lane.
//
// ASCII QUOTES ONLY: a typographic quote once took down a whole console.

import { T, LANGS } from '/store/i18n.js';

export const WORDS = {
  sq: {
    bg_banner: 'Mire se erdhet nga qesja: {bonus}', bg_once: 'Nje here per numer telefoni. Cmimet e menuse jane te njejta.',
    bg_fixed: '{value} zbritje', bg_fixedMin: '{value} zbritje per porosi mbi {min}', bg_gift: '{dish} falas', bg_stamps: 'vule e dyfishte ne karten tuaj',
    bg_ok: 'Oferta e mireseardhjes u aplikua',
    bg_r_used: 'Ky numer e ka perdorur tashme oferten e mireseardhjes. Porosia u be me cmimin e zakonshem.',
    bg_r_below_minimum: 'Oferta vlen per nje shporte me te madhe. Porosia u be me cmimin e zakonshem.',
    bg_r_gift_missing: 'Shtoni pjaten falas ne shporte heren tjeter. Porosia u be me cmimin e zakonshem.',
    bg_r_nothing_left: 'Kjo porosi nuk ka me cfare te zbritet. Porosia u be.',
  },
  en: {
    bg_banner: 'Welcome from the bag: {bonus}', bg_once: 'Once per phone number. Menu prices are the same.',
    bg_fixed: '{value} off', bg_fixedMin: '{value} off an order from {min}', bg_gift: '{dish} on the house', bg_stamps: 'a double stamp on your card',
    bg_ok: 'Welcome offer applied',
    bg_r_used: 'This phone number has already had the welcome offer. Your order is placed at the usual price.',
    bg_r_below_minimum: 'The offer is for a bigger basket. Your order is placed at the usual price.',
    bg_r_gift_missing: 'Add the free dish to your basket next time. Your order is placed at the usual price.',
    bg_r_nothing_left: 'Nothing is left on this order to take off. Your order is placed.',
  },
  uk: {
    bg_banner: 'Вітаємо з пакета: {bonus}', bg_once: 'Один раз на номер телефону. Ціни меню ті самі.',
    bg_fixed: '{value} знижки', bg_fixedMin: '{value} знижки на замовлення від {min}', bg_gift: '{dish} у подарунок', bg_stamps: 'подвійний штамп на вашій картці',
    bg_ok: 'Вітальну пропозицію застосовано',
    bg_r_used: 'Цей номер уже отримав вітальну пропозицію. Замовлення оформлено за звичайною ціною.',
    bg_r_below_minimum: 'Пропозиція діє для більшого кошика. Замовлення оформлено за звичайною ціною.',
    bg_r_gift_missing: 'Наступного разу додайте подарункову страву в кошик. Замовлення оформлено за звичайною ціною.',
    bg_r_nothing_left: 'У цьому замовленні вже нічого знижувати. Замовлення оформлено.',
  },
  ru: {
    bg_banner: 'Добро пожаловать из пакета: {bonus}', bg_once: 'Один раз на номер телефона. Цены меню те же.',
    bg_fixed: '{value} скидки', bg_fixedMin: '{value} скидки на заказ от {min}', bg_gift: '{dish} в подарок', bg_stamps: 'двойной штамп на вашей карточке',
    bg_ok: 'Приветственное предложение применено',
    bg_r_used: 'Этот номер уже получил приветственное предложение. Заказ оформлен по обычной цене.',
    bg_r_below_minimum: 'Предложение действует для большей корзины. Заказ оформлен по обычной цене.',
    bg_r_gift_missing: 'В следующий раз добавьте подарочное блюдо в корзину. Заказ оформлен по обычной цене.',
    bg_r_nothing_left: 'В этом заказе уже нечего снижать. Заказ оформлен.',
  },
};
for (const l of LANGS) if (T[l]) for (const [k, v] of Object.entries({ ...WORDS.en, ...(WORDS[l] || {}) })) if (!(k in T[l])) T[l][k] = v;
