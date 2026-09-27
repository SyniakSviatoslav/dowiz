// THE LANGUAGE SET, ONCE (lane W-RU, 2026-09-27; research 2026-09-26 Part B, row B0).
//
// Every surface -- console, storefront, room, courier, wiki, landing, the learn
// tools and the i18n eval -- imports its languages from here. The Rust twin is
// crates/dowiz-hub/src/lang.rs; tools/gates/langs.sh refuses a list of language
// codes anywhere else and a dictionary that lacks a key in any language here.
// Adding a language is one edit here, one in lang.rs, and the words.
//
// PURE: no imports, no DOM. Node tests it as it is (langs.test.mjs), and every
// module reaches it by a relative path, so node and the browser load the same.
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).

/// The UI languages, in switcher order. Albanian first: the venues are in Albania.
export const LANGS = Object.freeze(['sq', 'en', 'uk', 'ru']);

/// A language's own name, for a switcher that shows more than the code.
export const NAMES = Object.freeze({ sq: 'Shqip', en: 'English', uk: 'Українська', ru: 'Русский' });

/// BCP-47 tags for Intl (money, dates) and for the browser's recogniser and
/// voice. A recogniser told the wrong language hears nothing.
export const INTL = Object.freeze({ sq: 'sq-AL', en: 'en-GB', uk: 'uk-UA', ru: 'ru-RU' });

/// Videos are recorded and captioned in these only: the operator keeps the
/// films out of Russian (2026-09-26), so a cut never waits for a ru track.
export const MEDIA_LANGS = Object.freeze(LANGS.filter(l => l !== 'ru'));

/// Languages whose plural has three forms (1, 2-4, 5+), the East Slavic rule.
export const SLAVIC = Object.freeze(LANGS.filter(l => l === 'uk' || l === 'ru'));

export const isLang = l => LANGS.includes(l);

/// The two-letter code of a BCP-47 tag ('ru-RU' -> 'ru'), or '' when it is not one of ours.
export function norm(tag){
  const l = String(tag || '').slice(0, 2).toLowerCase();
  return isLang(l) ? l : '';
}

/// The BCP-47 tag for a code; an unknown code gets the fallback's tag.
export const tagFor = (l, fallback = 'en') => INTL[l] || INTL[fallback] || INTL.en;

/// FIRST VISIT: the first stored choice that is a language, else the first of
/// the browser's languages (navigator.languages, best first) that is one, else
/// the surface's default. A choice the person made always wins over the browser.
export function pickLang(stored = [], nav = [], fallback = 'sq'){
  for (const v of stored) if (isLang(v)) return v;
  for (const n of nav) { const l = norm(n); if (l) return l; }
  return fallback;
}

/// Plural form index for `forms` of length 2 or 3: East Slavic languages pick
/// among three (1 / 2-4 / 5+), the rest take one or other.
export function pluralIndex(lang, n, len = 3){
  if (!SLAVIC.includes(lang) || len < 3) return n === 1 ? 0 : len - 1;
  const m10 = n % 10, m100 = n % 100;
  if (m10 === 1 && m100 !== 11) return 0;
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return 1;
  return 2;
}

/// Letters only Ukrainian or only Russian writes. Cyrillic alone does not say
/// which: a message in Cyrillic with neither set is read as the UI's language
/// when that is Cyrillic, else as Russian (the larger reader base).
const ONLY_UK = /[іїєґІЇЄҐ]/;
const ONLY_RU = /[ёъыэЁЪЫЭ]/;
const CYRILLIC = /[\u0400-\u04FF]/;
const ALBANIAN = /[ëçËÇ]|\b(dhe|është|shumë|për|nuk)\b/i;

/// The language a piece of text is written in, as far as its letters tell.
export function scriptLang(text, ui = 'en'){
  const s = String(text || '');
  if (ONLY_UK.test(s)) return 'uk';
  if (ONLY_RU.test(s)) return 'ru';
  if (CYRILLIC.test(s)) return ui === 'uk' || ui === 'ru' ? ui : 'ru';
  return ALBANIAN.test(s) ? 'sq' : 'en';
}

/// A word from a per-language map with the fallback chain: the language, then
/// English, then the venue's default, then any entry at all.
export function pick(map, lang, venueDefault = 'sq'){
  if (!map) return '';
  for (const l of [lang, 'en', venueDefault]) if (map[l] != null && map[l] !== '') return map[l];
  const any = Object.values(map).find(v => v != null && v !== '');
  return any ?? '';
}
