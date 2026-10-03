// The Published-menu card's words (W-PUBUI, over BN2), kept out of the
// console's shared dictionary and MERGED into it at import, the shape
// `kitchen-i18n.js` set: `T` is the console's own table, so `t()`, `data-t`
// and `retranslate()` read these like any other key. more.js imports this
// file for the row's name; publish.js for the card. Nothing below lists the
// languages: lib/langs.js is the list, and the langs gate holds every
// language here to every key.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { T } from '/admin/i18n.js';

export const WORDS = {
  sq: {
    pubMenu: 'Menyja e publikuar', pubMenuSub: 'vitrina në CDN',
    pub_hint: 'Çdo ndryshim i menysë publikohet si skedarë që vitrina lexon së pari, që klientët ta hapin menynë pa kaluar nga hub-i.',
    pub_state: 'Publikimi', pub_onHint: 'Ndezur: çdo ndryshim i menysë, i çmimeve ose i cilësimeve publikohet vetë.',
    pub_offHint: 'Publikimi është fikur derisa të lidhet depoja CDN. Deri atëherë vitrina e lexon menynë nga hub-i.',
    pub_last: 'Publikuar së fundi', pub_genCatalog: 'menyja', pub_genWords: 'përkthimet', pub_genSettings: 'cilësimet',
    pub_objects: 'skedarë', pub_photos: 'foto', pub_never: 'Ende asgjë e publikuar.', pub_now: 'Publiko tani', pub_written: 'Skedarë të shkruar',
  },
  en: {
    pubMenu: 'Published menu', pubMenuSub: 'storefront on the CDN',
    pub_hint: 'Every menu change is published as files the storefront reads first, so customers open the menu without going through the hub.',
    pub_state: 'Publishing', pub_onHint: 'On: every menu, price or settings change publishes itself.',
    pub_offHint: 'Publishing is off until the CDN bucket is attached. Until then the storefront reads the menu through the hub.',
    pub_last: 'Last published', pub_genCatalog: 'menu', pub_genWords: 'translations', pub_genSettings: 'settings',
    pub_objects: 'files', pub_photos: 'photos', pub_never: 'Nothing published yet.', pub_now: 'Publish now', pub_written: 'Files written',
  },
  uk: {
    pubMenu: 'Опубліковане меню', pubMenuSub: 'вітрина на CDN',
    pub_hint: 'Кожна зміна меню публікується як файли, які вітрина читає першими, тож клієнти відкривають меню без звернення до хабу.',
    pub_state: 'Публікація', pub_onHint: 'Увімкнено: кожна зміна меню, цін чи налаштувань публікується сама.',
    pub_offHint: 'Публікацію вимкнено, доки не підключено сховище CDN. До того вітрина читає меню через хаб.',
    pub_last: 'Остання публікація', pub_genCatalog: 'меню', pub_genWords: 'переклади', pub_genSettings: 'налаштування',
    pub_objects: 'файлів', pub_photos: 'фото', pub_never: 'Ще нічого не опубліковано.', pub_now: 'Опублікувати зараз', pub_written: 'Записано файлів',
  },
  ru: {
    pubMenu: 'Опубликованное меню', pubMenuSub: 'витрина на CDN',
    pub_hint: 'Каждое изменение меню публикуется как файлы, которые витрина читает первыми, поэтому клиенты открывают меню без обращения к хабу.',
    pub_state: 'Публикация', pub_onHint: 'Включено: каждое изменение меню, цен или настроек публикуется само.',
    pub_offHint: 'Публикация выключена, пока не подключено хранилище CDN. До тех пор витрина читает меню через хаб.',
    pub_last: 'Последняя публикация', pub_genCatalog: 'меню', pub_genWords: 'переводы', pub_genSettings: 'настройки',
    pub_objects: 'файлов', pub_photos: 'фото', pub_never: 'Ещё ничего не опубликовано.', pub_now: 'Опубликовать сейчас', pub_written: 'Записано файлов',
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
