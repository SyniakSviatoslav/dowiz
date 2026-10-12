// THE DISH SHEET'S RECIPE PICKER, THE RULES, PURE (lane W-TKUI, 2026-10-12,
// operator TZ 4): which supplies the picker lists for a search and a kind,
// and which of them the owner TICKED -- kept in a Set the owner owns, not in
// the checkboxes, because the list is redrawn on every keystroke and a redrawn
// checkbox is an unticked one (the bug: clearing the search box reset every
// tick). No DOM, no clock, no network: node runs every rule here
// (`menu-recipe-logic.test.mjs`).
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

/// The picker's list: the supplies of `kind` ('all' or one kind) matching `q`
/// by name or category, not already on the recipe. Book order is kept.
export function bookList(book, { q = '', kind = 'all', draft = [] } = {}){
  const needle = String(q || '').toLowerCase().trim();
  return (book || []).filter(s => (kind === 'all' || s.kind === kind)
    && (!needle || `${s.name} ${s.category || ''}`.toLowerCase().includes(needle))
    && !draft.some(l => l.supply === s.id));
}

/// A tick set: ids the owner ticked, whatever the search box shows now.
export const picks = () => new Set();

/// The owner ticked (`on`) or unticked `id`. Returns the same set.
export function setPick(picked, id, on){
  if (on) picked.add(id); else picked.delete(id);
  return picked;
}

/// What "Add selected" adds: every ticked supply of the book, in book order,
/// hidden by the search or not, that the recipe does not carry yet.
export function pickedOf(book, picked, draft = []){
  return (book || []).filter(s => picked.has(s.id) && !draft.some(l => l.supply === s.id));
}

/// TZ 6: the saved dish folded into the menu the console holds, in place.
/// `saved` is the hub's answer to the save (the dish as stored); the row
/// keeps what the answer does not carry (translations, category name). A
/// dish the list does not know is left to a full reload (returns false).
export function mergeSaved(products, saved){
  if (!saved?.id) return false;
  const i = (products || []).findIndex(p => p.id === saved.id);
  if (i < 0) return false;
  const was = products[i];
  const next = { ...was, ...saved, translations: { ...(was.translations || {}), ...(saved.translations || {}) } };
  if (saved.categoryId == null) next.categoryId = was.categoryId;
  if (saved.categoryName == null) next.categoryName = was.categoryName;
  products[i] = next;
  return true;
}
