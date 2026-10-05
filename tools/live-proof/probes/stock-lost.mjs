// LIVE PROBE stock.refused.v1 + catalog.modifier_bom.v1 (W-LOST, P8 A13/R13)
// on qa-durres -- main runs it after the deploy; a lane never runs it against
// production. WRITTEN, NOT RUN by the lane.
//
//   node tools/live-proof/probes/stock-lost.mjs
//
// The spec's LIVE text: "count a QA supply to 30 g, order a dish needing 40 g,
// expect 409 + a refused row in /api/owner/analytics/kitchen; then add an
// option with 20 g of the supply and assert the reservation."
//
// A13. 1. a QA-only supply `qa-lost` and dish `qa-lost-roll` (40 g of it);
//      2. the supply COUNTED to 30 g; 3. the dish ordered: 409 naming the
//      supply; 4. the kitchen numbers carry the `lost` block (schema), with
//      qa-lost-roll today, revenue in integer lek; 5. a second refusal at once
//      does not add a row (one per supply per 10 minutes); 6. the shelf still
//      reads 30 g; the dish goes back off sale.
// R13. 7. a QA dish WITH an option (from the public menu); its option-bom
//      GET validates; 20 g of qa-rice set on its first option; 8. the dish
//      ordered with and without the option: the reserved qa-rice differs by
//      20 g; 9. the option recipe cleared, the orders closed.
//      NO OWNER ROUTE CREATES MODIFIER GROUPS (2026-10-05): when no QA dish
//      has an option, steps 7-9 are NEEDS-KEY (a fixture), never a pass.
import * as lib from '../../../e2e/flows/lib.mjs';
import { LOC, RUN, own, reporter, contract, shelf, read } from './stock-lib.mjs';
import { payload, close } from './_order.mjs';

const CR = contract('stock-refused'), CB = contract('modifier-bom');
const { step, schema, verdict } = reporter('stock-lost');
const SUP = 'qa-lost', DISH = 'qa-lost-roll';
const must = (ok, why) => { if (!ok) throw new Error(why); };
const q = `?location_id=${LOC}`;
const order = items => lib.api(`/api/public/locations/${LOC}/orders`, { method: 'POST', body: payload(RUN, { items }) });
const kitchen = async () => (await read(own, `/api/owner/analytics/kitchen?days=1&location_id=${LOC}`));
const level = async id => (await shelf(own, LOC)).supplies.find(s => s.id === id);
const placed = [];

try {
  // ── A13 ──
  const s = await own(`/api/owner/supplies${q}`, { id: SUP, name: 'QA lost-sale supply', unit: 'g', kind: 'food_ingredient' });
  step('the QA supply qa-lost exists', s.status === 200, `${s.status} ${s.text.slice(0, 100)}`);
  const d = await own('/api/owner/products', { location_id: LOC, category_id: 'qa-rolls', id: DISH, name: 'QA lost-sale roll', price: 600, available: false });
  console.log(`  .. create ${DISH}: ${d.status} (a second run finds it there; the next step proves it)`);
  const on = await own(`/api/owner/products/${DISH}`, { location_id: LOC, allergens: [], available: true, bom: [{ supply: SUP, qty: 40 }] });
  step('40 g of qa-lost per roll, on sale', on.status === 200, `${on.status} ${on.text.slice(0, 100)}`);
  const c = await own(`/api/owner/stock/stocktake${q}`, { item: SUP, observed: 30 });
  step('qa-lost counted to 30 g', c.status === 200, `${c.status} ${c.text.slice(0, 100)}`);
  const k0 = await kitchen();
  const rows0 = k0.body?.lost?.dishes?.find(x => x.id === DISH)?.rows || 0;

  const r = await order([{ product_id: DISH, modifier_ids: [], quantity: 1 }]);
  if (r.status === 200 && r.body?.id) placed.push(r.body.id);
  step('ordering a dish needing 40 g of 30 g is refused (409) naming the supply', r.status === 409 && r.text.includes(SUP), `${r.status} ${r.text.slice(0, 140)}`);
  const k1 = await kitchen();
  step('the kitchen numbers answer', k1.status === 200, `${k1.status}`);
  schema('GET /api/owner/analytics/kitchen `lost` validates against stock.refused.v1', k1.body, CR.response_schema);
  const row = k1.body?.lost?.dishes?.find(x => x.id === DISH);
  // A run within 10 minutes of the last one finds the window still closed: the
  // row then exists from that run, and is not added again.
  step('a refused row for qa-lost-roll today', !!row && row.rows >= 1 && (row.rows === rows0 + 1 || rows0 >= 1), JSON.stringify({ before: rows0, row }));
  step('its revenue is integer lek (600 a portion)', Number.isInteger(row?.revenue) && row.revenue % 600 === 0 && row.revenue > 0, `${row?.revenue}`);
  const again = await order([{ product_id: DISH, modifier_ids: [], quantity: 1 }]);
  const k2 = await kitchen();
  const rows2 = k2.body?.lost?.dishes?.find(x => x.id === DISH)?.rows || 0;
  step('a second refusal at once adds no row (1 per supply per 10 min)', again.status === 409 && rows2 === row?.rows, `${again.status} rows ${row?.rows} -> ${rows2}`);
  step('the shelf never moved: qa-lost still 30 g', (await level(SUP))?.onHand === 30, JSON.stringify(await level(SUP)));
  const text = JSON.stringify(k2.body?.lost || {});
  step('no personal data in the block', !/355690000019|guest|phone/.test(text), text.slice(0, 200));

  // ── R13 ──
  const menu = await lib.api(`/api/public/locations/${LOC}/menu?fresh=1`);
  const withOpt = (menu.body?.categories || []).flatMap(x => x.products || []).find(p => (p.modifierGroups || []).some(g => (g.options || []).length));
  if (!withOpt) {
    step('a QA dish with an option to give a recipe (no owner route creates modifier groups)', 'NEEDS-KEY');
  } else {
    step('the public menu shows no option recipe', !JSON.stringify(withOpt).includes('optionBom'), withOpt.id);
    const g = withOpt.modifierGroups.find(x => (x.options || []).length);
    const opt = g.options[0].id;
    const v = await own(`/api/owner/products/${encodeURIComponent(withOpt.id)}/option-bom?location_id=${LOC}`);
    step('GET option-bom answers', v.status === 200, `${v.status} ${v.text.slice(0, 100)}`);
    schema('the option-bom answer validates against catalog.modifier_bom.v1', v.body, CB.response_schema);
    const body = { location_id: LOC, option: opt, bom: [{ supply: 'qa-rice', qty: 20 }] };
    schema('the save matches the contract', body, CB.request_schema);
    const w = await own(`/api/owner/products/${encodeURIComponent(withOpt.id)}/option-bom`, body);
    step('20 g of qa-rice set on the option', w.status === 200 && w.body?.options?.find(o => o.id === opt)?.bom?.[0]?.qty === 20, `${w.status} ${w.text.slice(0, 120)}`);
    const bad = await own(`/api/owner/products/${encodeURIComponent(withOpt.id)}/option-bom`, { ...body, option: `${RUN}-nope` });
    step('an option the dish does not have is a 400', bad.status === 400, `${bad.status}`);
    // The twin order differs ONLY by the option: a required group takes its other option instead.
    const alt = g.min >= 1 ? g.options.find(o => o.id !== opt)?.id : null;
    const required = (withOpt.modifierGroups || []).filter(x => (x.min || 0) >= 1 && x !== g).map(x => x.options[0].id);
    if (g.min >= 1 && !alt) {
      step('a twin order without the option (its group is required and has one option)', 'NEEDS-KEY');
    } else {
      const r0 = (await level('qa-rice'))?.reserved || 0;
      const a = await order([{ product_id: withOpt.id, modifier_ids: [...required, ...(alt ? [alt] : [])], quantity: 1 }]);
      if (a.status === 200) placed.push(a.body.id);
      const r1 = (await level('qa-rice'))?.reserved || 0;
      const b = await order([{ product_id: withOpt.id, modifier_ids: [...required, opt], quantity: 1 }]);
      if (b.status === 200) placed.push(b.body.id);
      const r2 = (await level('qa-rice'))?.reserved || 0;
      step('with the option the order reserves 20 g more qa-rice than its twin', a.status === 200 && b.status === 200 && (r2 - r1) - (r1 - r0) === 20,
        JSON.stringify({ a: a.status, b: b.status, reserved: [r0, r1, r2] }));
    }
    const clear = await own(`/api/owner/products/${encodeURIComponent(withOpt.id)}/option-bom`, { ...body, bom: [] });
    step('the option recipe cleared', clear.status === 200, `${clear.status}`);
  }
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  try { await own(`/api/owner/products/${DISH}`, { location_id: LOC, available: false, unavailable_note: 'live-proof' }); } catch { /* reported by the next run's step 1 */ }
  for (const id of placed) { try { await close({ lib, run: RUN, must }, id); step(`the TEST order ${id} is closed`, true); } catch (e) { step(`the TEST order ${id} is closed`, false, e.message); } }
  verdict();
}
