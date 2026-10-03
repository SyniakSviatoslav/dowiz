// LIVE PROBE stock.start (W-STOCK P1) on qa-durres -- main runs it after the
// deploy; a lane never runs it against production.
//
//   node tools/live-proof/probes/stock-start.mjs        (FLOWS_HOST may name another qa- hub)
//
// 1. the starter pack as the console builds it (start-stock-logic.js), DRY RUN:
//    the answer validates against the contract, 41 rows, and NOTHING is written;
// 2. Apply: every pack id is on the shelf with its defaults (pack-rice cookPm
//    2200 and a 10 kg pack: the extra columns arrived);
// 3. Apply again: the same ids, no duplicate (idempotent);
// 4. a probe dish named like a roll gets its skeleton (salmon, rice, nori, box,
//    grams empty); filled in, the recipes import's dry run writes nothing,
//    Apply writes the bom, and the STOREFRONT menu (?fresh=1) shows the weight
//    the recipe derives;
// 5. clean-up: the dish, its category and every pack supply this run added.
import { LOC, RUN, own, menu, dishes, reporter, contract, csvPost, shelf } from './stock-lib.mjs';
import * as L from '../../../workers/api/public/admin/start-stock-logic.js';
import { PACK } from '../../../workers/api/public/admin/start-stock-pack.js';

const C = contract('stock-start');
const { step, schema, verdict } = reporter('stock-start');
const ids = PACK.map(p => p.id);
let made = [], dish = null, cat = null;

try {
  const before = await shelf(own, LOC);
  step('the shelf reads', before.status === 200, `${before.status} ${before.supplies.length} supplies`);
  const had = new Set(before.supplies.map(s => s.id));
  // A supply deleted earlier is re-created fresh (W-NOM `Removed`); one still
  // here from a crashed run is counted as "had" and left as found.
  const fresh = ids.filter(id => !had.has(id));

  // ── 1. dry run ──
  const text = L.suppliesCsv(ids, 'en');
  const dry = await csvPost('/api/owner/supplies/import', text);
  step('supplies dry run answers 200', dry.status === 200, `${dry.status} ${dry.text.slice(0, 160)}`);
  schema('the dry run validates against the contract', dry.body, C.response_schema);
  step('41 pack rows, applied:false', dry.body?.applied === false && dry.body?.rows?.length === PACK.length, `${dry.body?.rows?.length} rows`);
  for (const r of (dry.body?.rows || []).slice(0, 3)) schema(`row ${r.id} validates`, r, C.supplies_row_schema);
  const rice = (dry.body?.rows || []).find(r => r.id === 'pack-rice');
  step('the preview carries the extra columns (rice cooks to 220%, 10 kg pack)', rice?.cookPm === 2200 && rice?.pack?.qty === 10000, JSON.stringify(rice));
  const mid = await shelf(own, LOC);
  step('NOTHING was written by the dry run', mid.supplies.filter(s => ids.includes(s.id)).length === before.supplies.filter(s => ids.includes(s.id)).length);

  // ── 2. apply ──
  const ap = await csvPost('/api/owner/supplies/import?apply=1', text);
  made = fresh;
  step('Apply answers 200 with written', ap.status === 200 && ap.body?.applied === true && ap.body?.written >= PACK.length, `${ap.status} written=${ap.body?.written}`);
  const after = await shelf(own, LOC);
  const on = after.supplies.filter(s => ids.includes(s.id));
  step('at least 30 pack supplies are on the shelf (P1 acceptance)', on.length >= 30, `${on.length} of ${PACK.length}`);
  const r2 = after.supplies.find(s => s.id === 'pack-rice');
  step('pack-rice carries its defaults: cookPm 2200, a 10 kg pack', r2?.cookPm === 2200 && r2?.packs?.[0]?.qty === 10000, JSON.stringify({ cookPm: r2?.cookPm, packs: r2?.packs }));
  const sal = after.supplies.find(s => s.id === 'pack-salmon');
  step('pack-salmon: cleanPm 900, kind food_ingredient, unit g', sal?.cleanPm === 900 && sal?.kind === 'food_ingredient' && sal?.unit === 'g', JSON.stringify({ cleanPm: sal?.cleanPm, kind: sal?.kind }));

  // ── 3. idempotent ──
  const again = await csvPost('/api/owner/supplies/import?apply=1', text);
  const third = await shelf(own, LOC);
  step('a second Apply adds nothing: same ids, same count', again.status === 200 && third.supplies.length === after.supplies.length, `${after.supplies.length} -> ${third.supplies.length}`);

  // ── 4. a dish, its skeleton, its recipe ──
  const c = await own('/api/owner/categories', { location_id: LOC, name: `${RUN} Start` });
  cat = c.body?.id || c.body?.category?.id;
  const p = await own('/api/owner/products', { location_id: LOC, category_id: cat, name: `${RUN} Salmon roll`, price: 900, available: true });
  dish = p.body?.id || p.body?.product?.id;
  step('a probe dish exists', !!cat && !!dish, `${c.status}/${p.status} ${dish}`);
  const list = await own(`/api/owner/products?location_id=${LOC}`);
  const products = Array.isArray(list.body) ? list.body : (list.body?.products || []);
  const sk = L.skeletons(products.filter(x => x.id === dish));
  const d = sk.drafts[0];
  step('its skeleton: salmon, rice, nori, box -- grams empty', JSON.stringify(d?.lines.map(l => l.supply)) === JSON.stringify(['pack-salmon', 'pack-rice', 'pack-nori', 'pack-box']) && d.lines.every(l => l.qty === null), JSON.stringify(d?.lines));
  const all = L.skeletons(products);
  console.log(`  info: ${all.drafts.length} of ${products.length} dishes on ${LOC} get a draft; unmatched: ${all.unmatched.map(u => u.name).join(', ') || 'none'}`);
  const grams = { 'pack-salmon': 40, 'pack-rice': 90, 'pack-nori': 1, 'pack-box': 1 };
  for (const l of d?.lines || []) l.qty = grams[l.supply];
  const rtext = L.recipesCsv([d]);
  const rdry = await csvPost('/api/owner/recipes/import', rtext);
  step('recipes dry run: one recipe, applied:false', rdry.status === 200 && rdry.body?.recipes === 1 && rdry.body?.applied === false, `${rdry.status} ${rdry.text.slice(0, 160)}`);
  schema('the recipes dry run validates', rdry.body, C.response_schema);
  const still = (await own(`/api/owner/products?id=${encodeURIComponent(dish)}`)).body;
  const bom0 = (Array.isArray(still) ? still[0] : still)?.bom || [];
  step('the dry run wrote no recipe', bom0.length === 0, JSON.stringify(bom0).slice(0, 120));
  const rap = await csvPost('/api/owner/recipes/import?apply=1', rtext);
  step('recipes Apply writes it', rap.status === 200 && rap.body?.written >= 1, `${rap.status} written=${rap.body?.written}`);
  const now = (await own(`/api/owner/products?id=${encodeURIComponent(dish)}`)).body;
  const bom1 = (Array.isArray(now) ? now[0] : now)?.bom || [];
  step('the dish holds four lines with the grams typed', bom1.length === 4 && bom1.find(l => l.supply === 'pack-salmon')?.qty === 40, JSON.stringify(bom1.map(l => [l.supply, l.qty])));
  const pub = dishes(await menu()).find(x => x.id === dish);
  // 40 g salmon x 0.9 + 90 g rice x 2.2 + one nori sheet of 3 g = 36 + 198 + 3.
  step('the storefront shows what the recipe derives (weight 237 g)', pub?.weightG === 237 && pub?.nutritionDerived === true, `weightG=${pub?.weightG} derived=${pub?.nutritionDerived}`);
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  // ── 5. put the venue back ──
  if (dish) step('the probe dish is deleted', (await own(`/api/owner/products/${dish}/delete`, { location_id: LOC })).status === 200);
  if (cat) step('its category is deleted', (await own(`/api/owner/categories/${cat}/delete`, { location_id: LOC })).status === 200);
  if (made.length) {
    const del = await own('/api/owner/supplies/delete', { ids: made, location_id: LOC, confirmUses: true });
    step(`the ${made.length} pack supplies this run added are deleted`, del.status === 200, `${del.status} ${del.text.slice(0, 120)}`);
  }
  verdict();
}
