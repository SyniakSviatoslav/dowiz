// THE NOMENCLATURE'S RULES, PURE (lane W-NOM, 2026-09-28): the list a new
// person types to add many ingredients, the packs an ingredient is bought in,
// the selection a bulk delete acts on, and the two-tap arm of a row's red
// Delete. No DOM, no clock, no network: node runs every rule here
// (`nom-logic.test.mjs`).
//
// INTEGERS ONLY: a pack is whole base units (g, ml, pieces), as the hub keeps it.
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import { amount } from './ingredients-calc.js';

/// The base units, as the hub spells them.
export const UNITS = ['g', 'ml', 'unit'];
/// What a person may type for a unit on a quick line -> the base unit.
export const UNIT_WORDS = {
  g: 'g', gr: 'g', kg: 'g', 'г': 'g', 'гр': 'g', 'кг': 'g',
  ml: 'ml', l: 'ml', 'мл': 'ml', 'л': 'ml',
  unit: 'unit', pcs: 'unit', pc: 'unit', piece: 'unit', cope: 'unit', 'copë': 'unit', 'шт': 'unit',
};
/// How long a row's red Delete stays armed after the first tap.
export const ARM_MS = 5000;
/// The hub's own bounds (supplies/quick.rs, supplies/nomenclature.rs).
export const ITEMS_MAX = 200, NAME_MAX = 80, PACKS_MAX = 6, PACK_NAME_MAX = 24;

const SEP = /\s*[;|\t]\s*/;

/// A quick list -> the items to add: one per non-empty line, `name[; unit][; group]`.
/// A second part that is not a unit is the group. Repeats (any case) are
/// dropped. `defaults` = { unit, category, kind } for what a line leaves out.
export function parseLines(text, defaults = {}){
  const out = [], seen = new Set();
  for (const raw of String(text ?? '').split(/\r?\n/)) {
    const parts = raw.split(SEP).map(s => s.trim());
    const name = parts.shift() || '';
    if (!name || seen.has(name.toLowerCase())) continue;
    seen.add(name.toLowerCase());
    let unit = defaults.unit || 'g', category = defaults.category || '';
    for (const p of parts) {
      const u = UNIT_WORDS[p.toLowerCase()];
      if (u) unit = u; else if (p) category = p;
    }
    const item = { name, unit };
    if (category) item.category = category;
    if (defaults.kind) item.kind = defaults.kind;
    out.push(item);
  }
  return out;
}

/// Why a quick list cannot be sent, as an i18n key, or null.
export function listProblem(items){
  if (!items.length) return 'nom_needOne';
  if (items.length > ITEMS_MAX) return 'nom_tooMany';
  if (items.some(i => i.name.length > NAME_MAX)) return 'nom_nameLong';
  return null;
}

/// Pack rows as typed ({ name, qty: "5 kg" }) -> [{ name, qty }] in base units,
/// or { error: key }. Empty rows are skipped; a row with only one half is wrong.
export function packsOf(rows, unit){
  const out = [];
  for (const r of rows || []) {
    const name = String(r.name ?? '').trim(), raw = String(r.qty ?? '').trim();
    if (!name && !raw) continue;
    const qty = amount(raw, unit);
    if (!name || name.length > PACK_NAME_MAX || qty == null || qty < 1) return { error: 'nom_packBad' };
    if (out.some(p => p.name.toLowerCase() === name.toLowerCase())) return { error: 'nom_packTwice' };
    out.push({ name, qty });
  }
  if (out.length > PACKS_MAX) return { error: 'nom_packMany' };
  return { packs: out };
}

/// A quantity field after tapping a pack: what it held (base units) plus one pack.
export function plusPack(current, pack, unit){
  const now = amount(current, unit);
  return (now == null || now < 0 ? 0 : now) + pack.qty;
}

/// Toggle one id in a selection (a Set), answering the new Set.
export function toggle(sel, id){
  const next = new Set(sel);
  if (next.has(id)) next.delete(id); else next.add(id);
  return next;
}

/// Select every shown id, or -- when all of them already are -- none of them.
export function toggleAll(sel, shown){
  const all = shown.length > 0 && shown.every(id => sel.has(id));
  const next = new Set(sel);
  for (const id of shown) if (all) next.delete(id); else next.add(id);
  return next;
}

/// The two-tap Delete: the first tap on `id` arms it; a second tap on the
/// SAME id within ARM_MS fires. Answers { fire, arm } -- the new armed state.
export function tap(armed, id, now){
  if (armed && armed.id === id && now - armed.at <= ARM_MS) return { fire: true, arm: null };
  return { fire: false, arm: { id, at: now } };
}

/// The hub's answer to a delete, as one line for a toast.
export function deletedLine(r, t){
  const n = (r?.deleted || []).length || (r?.stock || []).length;
  const parts = [`${t('nom_deleted')}: ${n}`];
  if ((r?.dishes || []).length) parts.push(`${t('nom_recipesChanged')}: ${r.dishes.length}`);
  return parts.join(' · ');
}
