// THE PERSON CHOOSES THE LIGHT (design doc HUB-UX-2026-09-26 §2.6). A pass is
// dark and an office is light; the phone's setting is not the room's. Three
// choices, remembered per device: auto (the phone decides), light, dark.
//
// PURE where it can be: `pick`, `next` and `colorFor` take values and return
// values; `apply` takes a document and writes `data-theme` and the browser's
// `theme-color`, so `theme.test.mjs` drives it with a stub document. Nothing
// here reads the clock or the network.
//
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

export const THEMES = ['auto', 'light', 'dark'];
export const KEY = 'dw_theme';
/// The page ground in each scheme, as index.html's `theme-color` metas say it.
export const GROUND = { light: '#f2f1ec', dark: '#141416' };

/// A stored value, made safe: anything that is not a theme is `auto`.
export const pick = v => (THEMES.includes(v) ? v : 'auto');

/// The choice after tapping the same control again: auto -> light -> dark -> auto.
export const next = v => THEMES[(THEMES.indexOf(pick(v)) + 1) % THEMES.length];

/// The browser-chrome colour for a choice: a forced scheme's ground, or null
/// for auto (each meta keeps its own `media` and the phone picks).
export const colorFor = v => (pick(v) === 'auto' ? null : GROUND[pick(v)]);

/// Write the choice onto `doc`: `<html data-theme>` (removed for auto) and the
/// `theme-color` metas. Returns the theme that was applied.
export function apply(doc, v){
  const theme = pick(v);
  const root = doc.documentElement;
  if (theme === 'auto') root.removeAttribute('data-theme'); else root.setAttribute('data-theme', theme);
  const color = colorFor(theme);
  for (const m of doc.querySelectorAll('meta[name="theme-color"]')) {
    if (color) { if (!m.dataset.auto) m.dataset.auto = m.getAttribute('content') || ''; m.setAttribute('content', color); }
    else if (m.dataset.auto != null) { m.setAttribute('content', m.dataset.auto); delete m.dataset.auto; }
  }
  return theme;
}

/// The stored choice, read through `get` (a storage getter that may throw).
export function stored(get){
  try { return pick(get(KEY)); } catch { return 'auto'; }
}

/// Remember `v` through `set` (a storage setter that may throw) and apply it.
export function choose(doc, v, set){
  const theme = pick(v);
  try { set(KEY, theme); } catch { /* private mode: the choice lasts the page */ }
  return apply(doc, theme);
}
