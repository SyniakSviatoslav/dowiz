// AFTER A SAVE, THE CONSOLE'S COPY FOLLOWS -- THE RULE, PURE (lane W-TKUI,
// 2026-10-12, operator TZ 6). The dish sheet used to re-read the whole venue
// in four languages after every save; now one read of the menu in the console
// language is folded into the copy in place: the dish's category takes the
// answer's rows (a move or a reorder lands too), every other category loses
// the dish if it had it, and the enriched list (`S.products`, what the sheet
// opens) is rebuilt from the categories the way `app.js loadVenue` builds it,
// keeping the other languages' words from memory and the ones the save sent.
// No DOM, no clock, no network (`sync-logic.test.mjs`).
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

/// Where `id` is in a public-menu answer: [row, category] or null.
export function findDish(answer, id){
  for (const c of answer?.categories || []) { const p = (c.products || []).find(x => x.id === id); if (p) return [p, c]; }
  return null;
}

/// Fold the menu `answer` (console language `lang`) for dish `id` into `S`
/// IN PLACE (S is shared by every screen; the object itself never changes).
/// `sent` = the translations the save carried, { lang: { name, description } }.
/// False when the answer does not carry the dish: the caller reloads everything.
export function foldMenu(S, answer, id, lang, sent = {}){
  const hit = findDish(answer, id); if (!hit) return false;
  const [, cat] = hit;
  const words = new Map((S.products || []).map(p => [p.id, p.translations || {}]));
  const cats = S.categories || [];
  for (const c of cats) if (c.id !== cat.id) c.products = (c.products || []).filter(p => p.id !== id);
  const mine = cats.find(c => c.id === cat.id);
  if (mine) { mine.name = cat.name; mine.products = [...(cat.products || [])]; }
  else cats.push({ ...cat, products: [...(cat.products || [])] });
  const rows = cats.flatMap(c => (c.products || []).map(p => {
    const tr = { ...(words.get(p.id) || {}), ...(p.id === id ? sent : {}) };
    tr[lang] = { name: p.name, description: p.description || '' };
    return { ...p, categoryId: c.id, categoryName: c.name, translations: tr };
  }));
  (S.products || (S.products = [])).splice(0, S.products.length, ...rows);
  return true;
}
