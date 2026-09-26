// The redesign's words (lane W-UX, 2026-09-26), PURE: no imports, so node
// tests the table itself (`ux-words.test.mjs`). `admin/ux-i18n.js` merges it
// into the console's dictionary at import. A language the console gains later
// (ru is next) falls back to English until its words are added here; nothing
// below lists the languages.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
export const WORDS = {
  sq: {
    appearance: 'Pamja', theme_auto: 'Si telefoni', theme_light: 'E çelët', theme_dark: 'E errët',
    profileTitle: 'Gjuha, monedha, pamja', findSetting: 'Kërko një cilësim…', noSetting: 'Asnjë cilësim me këtë emër',
    settingsVenue: 'Lokali', settingsLinks: 'Lidhjet', settingsData: 'Të dhënat dhe siguria',
    staffSub: 'rolet dhe hyrjet', deliveryAreaSub: 'zonat dhe tarifat',
    sideMap: 'Cilësimet dhe më shumë',
  },
  en: {
    appearance: 'Appearance', theme_auto: 'Like the phone', theme_light: 'Light', theme_dark: 'Dark',
    profileTitle: 'Language, currency, appearance', findSetting: 'Find a setting…', noSetting: 'No setting by that name',
    settingsVenue: 'The venue', settingsLinks: 'Connections', settingsData: 'Data and safety',
    staffSub: 'roles and sign-ins', deliveryAreaSub: 'zones and fees',
    sideMap: 'Settings and more',
  },
  uk: {
    appearance: 'Вигляд', theme_auto: 'Як у телефоні', theme_light: 'Світлий', theme_dark: 'Темний',
    profileTitle: 'Мова, валюта, вигляд', findSetting: 'Знайти налаштування…', noSetting: 'Немає налаштування з такою назвою',
    settingsVenue: 'Заклад', settingsLinks: 'Підключення', settingsData: 'Дані та безпека',
    staffSub: 'ролі та входи', deliveryAreaSub: 'зони та тарифи',
    sideMap: 'Налаштування та інше',
  },
};

/// Merge these words into a console dictionary `T` ({ lang: { key: word } }),
/// language by language. A language `T` has that WORDS lacks gets English,
/// so a key never falls through `t()` as itself; a language WORDS has that
/// `T` lacks is added whole. Returns `T`.
export function merge(T, words = WORDS){
  const en = words.en || {};
  for (const lang of new Set([...Object.keys(T), ...Object.keys(words)])) {
    T[lang] = Object.assign(T[lang] || {}, en, words[lang] || {});
  }
  return T;
}
