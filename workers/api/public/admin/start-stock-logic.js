// START STOCK, THE PURE PART (W-STOCK P1): which pack lines a dish's name
// proposes, the two CSV files the existing importers read, and when a
// skeleton is ready to be a recipe. No DOM, no fetch: `start-stock-logic.test.mjs`.
//
// THE GRAMS ARE THE OWNER'S. A skeleton names the lines and leaves every
// quantity empty; only a dish whose every line the owner filled in (or
// removed) goes into the recipes file. A recipe with a guessed 40 g of salmon
// would reserve the wrong amount on every order under a ledger that looks right.
//
// ASCII QUOTES ONLY in this file.

import { PACK, CATS, RULES, BOX } from './start-stock-pack.js';

/// Lower case, accents off, ё -> е: the form names and stems are compared in.
export const norm = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '').normalize('NFC');
const STEMS = RULES.map(r => ({ ...r, k: r.k.map(norm) }));
const ORDER = new Map(PACK.map((p, i) => [p.id, i]));

/// The pack lines a dish's name proposes, in PACK order; [] = unmatched.
export function match(name){
  const words = norm(name).split(/[^\p{L}\p{N}]+/u).filter(Boolean);
  const fired = STEMS.filter(r => r.k.some(k => words.some(w => w.startsWith(k))));
  if (!fired.length) return [];
  const ids = new Set(fired.flatMap(r => r.add));
  const food = fired.some(r => !r.drink);
  if (food) ids.add(BOX);
  return [...ids].sort((a, b) => ORDER.get(a) - ORDER.get(b));
}

/// Skeletons for every dish that has no recipe yet, and the dishes no rule
/// matched. `dishes`: the owner's products ({id, name, bom}).
export function skeletons(dishes){
  const out = { drafts: [], unmatched: [] };
  for (const d of dishes || []) {
    if ((d.bom || []).length) continue; // a recipe exists: the owner's, never overwritten here
    const ids = match(d.name);
    if (!ids.length) { out.unmatched.push({ id: d.id, name: d.name }); continue; }
    out.drafts.push({ id: d.id, name: d.name, lines: ids.map(supply => ({ supply, qty: null })) });
  }
  return out;
}

/// A whole number of base units from what was typed ("40", "1,5 kg" is 1500); null otherwise.
export function amount(raw, unit){
  const s = String(raw ?? '').trim().toLowerCase().replace(',', '.');
  if (!s) return null;
  const m = s.match(/^(\d+(?:\.\d+)?)\s*(kg|l|g|ml)?$/);
  if (!m) return null;
  const n = Number(m[1]) * (m[2] === 'kg' || m[2] === 'l' ? 1000 : 1);
  if (!Number.isFinite(n) || n <= 0 || !Number.isInteger(n)) return null;
  if ((m[2] === 'kg' || m[2] === 'g') && unit !== 'g') return null;
  if ((m[2] === 'l' || m[2] === 'ml') && unit !== 'ml') return null;
  return n;
}

/// Ready: at least one line, and every line has a quantity above zero.
export const ready = d => d.lines.length > 0 && d.lines.every(l => Number.isInteger(l.qty) && l.qty > 0);

const cell = v => {
  const s = String(v ?? '');
  return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
};
const row = cells => cells.map(cell).join(',');

/// What the pack is bought in, as words no language owns: "10 kg", "5 l", "x100".
export function packName(p){
  if (p.unit === 'unit') return `x${p.pack}`;
  const big = p.unit === 'g' ? 'kg' : 'l';
  return p.pack >= 1000 && p.pack % 1000 === 0 ? `${p.pack / 1000} ${big}` : `${p.pack} ${p.unit}`;
}

/// The supplies file `POST /api/owner/supplies/import` reads, the names and
/// groups in `lang`. `ids`: the pack lines chosen.
export function suppliesCsv(ids, lang){
  const want = new Set(ids);
  const lines = [row(['id', 'name', 'unit', 'kind', 'category', 'kcal', 'protein', 'fat', 'carbs', 'weight_per_unit', 'clean_pm', 'cook_pm', 'pack', 'pack_qty'])];
  for (const p of PACK.filter(x => want.has(x.id))) {
    const nut = p.nut || [];
    const name = p.n[lang] || p.n.en, cat = CATS[p.cat]?.[lang] || CATS[p.cat]?.en || '';
    lines.push(row([p.id, name, p.unit, p.kind, cat, nut[0], nut[1], nut[2], nut[3], p.wpu ?? '',
      p.clean && p.clean !== 1000 ? p.clean : '', p.cook && p.cook !== 1000 ? p.cook : '', p.pack ? packName(p) : '', p.pack ?? '']));
  }
  return lines.join('\n') + '\n';
}

/// The recipes file `POST /api/owner/recipes/import` reads: the READY drafts
/// only, one row per line, the quantity with its unit.
export function recipesCsv(drafts){
  const unitOf = id => PACK.find(p => p.id === id)?.unit || 'g';
  const lines = [row(['dish', 'ingredient', 'qty'])];
  for (const d of drafts.filter(ready)) for (const l of d.lines) lines.push(row([d.id, l.supply, `${l.qty} ${unitOf(l.supply)}`]));
  return lines.join('\n') + '\n';
}

/// The pack lines a venue has not got yet (by id), and the ones it has.
export function split(have){
  const got = new Set((have || []).map(s => s.id));
  return { fresh: PACK.filter(p => !got.has(p.id)).map(p => p.id), had: PACK.filter(p => got.has(p.id)).map(p => p.id) };
}
