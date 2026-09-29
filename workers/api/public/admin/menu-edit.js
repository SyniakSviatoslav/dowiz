// The dish sheet's pure decisions (W-CRUD, 2026-09-29): what its translation
// boxes show, and what a save sends of the venue's own words. PURE, so node
// tests it (`menu-edit.test.mjs`); `admin/menu.js` owns the DOM.
//
// THE DEFECT. `app.js loadVenue()` reads the menu once per language with
// `?fresh=1`, and a dish with no Ukrainian name answers its BASE name in
// Ukrainian -- `fresh` only drops the customer's second fallback (ru -> en),
// never the venue's own words. The sheet then prefilled the Ukrainian box
// with the base name and every save (a price change, a stop-list flip) wrote
// it back as a Ukrainian translation. From then on the dish had a Ukrainian
// name nobody typed, and a later rename of the base name left it behind: the
// storefront kept the OLD name in that language. `cat-i18n.js` already
// compared with the base for categories; the dish sheet did not.

/// The boxes for the other languages: a language whose menu answered the base
/// words has NO translation, and its box is empty. `translations` is
/// `{ [locale]: { name, description } }` as `loadVenue` builds it.
export function translationBoxes(translations, base, langs) {
  const own = translations[base] || {};
  const out = {};
  for (const l of langs) {
    if (l === base) continue;
    const t = translations[l] || {};
    out[l] = {
      name: t.name && t.name !== own.name ? t.name : '',
      description: t.description && t.description !== (own.description || '') ? t.description : '',
    };
  }
  return out;
}

/// What a save sends of the base language and the category: only what
/// changed. A blanked name is NOT sent (the hub refuses it; the sheet keeps
/// the old one); a blanked description is sent as '' (the dish has none).
export function baseEdits(before, typed) {
  const out = {};
  const name = String(typed.name ?? '').trim();
  if (name && name !== String(before.name ?? '').trim()) out.name = name;
  const desc = String(typed.description ?? '').trim();
  if (desc !== String(before.description ?? '').trim()) out.description = desc;
  const cat = String(typed.categoryId ?? '');
  if (cat && cat !== String(before.categoryId ?? '')) out.category_id = cat;
  return out;
}
