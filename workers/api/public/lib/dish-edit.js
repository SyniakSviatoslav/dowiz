// The dish sheet's published values: what a save sends back. PURE, so node
// tests it (`dish-edit.test.mjs`); `admin/menu.js` owns the DOM.
//
// THE DEFECT (audit D19). The sheet opened with kcal, weight and ingredients
// PREFILLED from the stored dish and sent all of them back on every save. The
// hub reads a value sent in the same request as the owner's own ("typed"), so
// a recipe change never reached the published numbers: add shrimp to a roll
// and the storefront still listed no shrimp, and the stale kcal was now marked
// as typed by hand. A value is sent now only if the owner EDITED it in this
// sheet; untouched, the hub derives it from the recipe (or keeps it, when the
// dish has none).

/// The sheet's fields, by the name each is tracked under.
export const FIELDS = ['ings', 'kcal', 'protein', 'fat', 'carbs', 'weight'];
const NUTRITION = [['kcal', 'kcal'], ['protein', 'protein'], ['fat', 'fat'], ['carbs', 'carbs']];

/// A number typed into a box, or null for empty or not a number.
const num = v => {
  const s = String(v ?? '').trim().replace(',', '.');
  if (!s) return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
};

/// `values`: the boxes' text by field name. `edited`: the field names the owner
/// changed in this sheet. Answers the keys of the save body for these fields.
export function publishedFields(values, edited) {
  const out = {};
  if (edited.has('ings')) {
    out.ingredients = String(values.ings || '').split(',').map(s => s.trim()).filter(Boolean);
  }
  // Nutrition travels as one object: one edited box sends all four as they
  // stand, so the hub stores the owner's whole panel and marks it theirs.
  if (NUTRITION.some(([f]) => edited.has(f))) {
    const n = {};
    for (const [f, k] of NUTRITION) { const v = num(values[f]); if (v !== null) n[k] = v; }
    if (Object.keys(n).length) out.nutrition = n;
  }
  if (edited.has('weight')) {
    const w = num(values.weight);
    if (w !== null) out.weight_g = w;
  }
  return out;
}
