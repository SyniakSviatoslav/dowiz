// LIVE PROBE sense (W-SENSE, 2026-10-04) on qa-durres -- main runs it AFTER the deploy; a lane never
// runs it against production.
//
//   node tools/live-proof/probes/feature-sense.mjs      (FLOWS_HOST may name another qa- hub)
//
// 1. the owner saves a sense on qa-water; the public menu (fresh) serves it, schema-valid, version moved;
// 2. out-of-vocabulary / out-of-range values are refused 400;
// 3. Suggest answers a draft and writes nothing;
// 4. the context route answers the VENUE's moment; a guest coordinate in its query is refused 400;
// 5. a TEST order with a phone and taste_sync.sense -> the guest's view carries the sense; the
//    builder counts a smoky guest; an allergen key is refused;
// 6. a taste offer is filed as a campaign with the server's words at the public price; a code is refused;
// 7. qa-water's sense is restored.
// Every TEST order is ended. A step that cannot run is a FAIL, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { place, close, sweep } from './_order.mjs';

const C = contract('sense');
const { step, schema, verdict } = reporter('feature-sense');
const run = `LIVE-${lib.RUN}`;
const must = (ok, why) => { if (!ok) throw new Error(why); };
const DISH = 'qa-water';
const SENSE = { taste: { salty: 1, sweet: 0 }, texture: { crispy: 2 }, aroma: { smoky: 3, citrus: 1 } };
const tail = Number.parseInt(lib.RUN.replace(/\D/g, '').slice(-2) || '0', 10) % 30;
const phone = `+3556900008${String(tail).padStart(2, '0')}`;

const menu = async () => lib.api(`/api/public/locations/${lib.LOC}/menu?fresh=1`);
const dishOf = m => (m.body?.categories || []).flatMap(c => c.products || []).find(p => p.id === DISH);
const edit = body => lib.own(`/api/owner/products/${DISH}?location_id=${lib.LOC}`, { location_id: lib.LOC, ...body });
const orders = [];
let before = null;

try {
  await sweep(lib);
  const m0 = await menu();
  must(m0.status === 200, `menu ${m0.status}`);
  const d0 = dishOf(m0);
  must(d0, `${DISH} is not on the qa menu`);
  before = d0.sense ?? null;

  // 1. saved and served
  const e = await edit({ sense: SENSE });
  step('the owner saves taste, texture and aroma', e.status === 200, `${e.status} ${e.text}`);
  const m1 = await menu();
  const d1 = dishOf(m1);
  schema('the served sense matches its schema', d1?.sense ?? null, C.sense_schema);
  step('the storefront serves it as saved', d1?.sense?.aroma?.smoky === 3 && d1?.sense?.taste?.sweet === 0 && d1?.sense?.taste?.bitter === undefined, JSON.stringify(d1?.sense));
  step('the menu version moved', (m1.body?.location?.menuVersion || 0) > (m0.body?.location?.menuVersion || 0), `${m0.body?.location?.menuVersion} -> ${m1.body?.location?.menuVersion}`);

  // 2. the closed vocabulary
  for (const [bad, what] of [[{ taste: { spicy: 6 } }, 'a 6 on an axis'], [{ taste: { richness: 1 } }, 'an unknown axis'], [{ aroma: { gluten: 1 } }, 'an allergen as an aroma']]) {
    const r = await edit({ sense: bad });
    step(`${what} is refused 400`, r.status === 400, `${r.status} ${r.text}`);
  }

  // 3. a draft, never written
  const s = await lib.own(`/api/owner/products/${DISH}/sense/suggest?location_id=${lib.LOC}`, { location_id: lib.LOC });
  schema('suggest matches its schema', s.body, C.suggest_response_schema);
  const m3 = await menu();
  step('suggest wrote nothing', s.status === 200 && s.body?.saved === false && m3.body?.location?.menuVersion === m1.body?.location?.menuVersion, `${s.status} ${s.text}`);

  // 4. the venue's moment, never the guest's place
  const c = await lib.api(`/api/public/locations/${lib.LOC}/context`);
  schema('context matches its schema', c.body, C.context_response_schema);
  step(`context answers the venue's moment (weather: ${c.body?.weather})`, c.status === 200 && c.body?.source?.at === 'venue', `${c.status} ${c.text}`);
  const cq = await lib.api(`/api/public/locations/${lib.LOC}/context?lat=48.1&lon=11.5`);
  step('a guest coordinate is refused 400', cq.status === 400, `${cq.status} ${cq.text}`);

  // 5. the guest on the same axes
  const o = await place({ lib, run, must }, { items: [{ product_id: DISH, modifier_ids: [], quantity: 2 }],
    taste_sync: { v: 1, tags: {}, cats: {}, sense: { 'a:smoky': 1000, 'x:crispy': 600 } } }, phone);
  orders.push(o.id);
  const v = await lib.api(`/api/order/${encodeURIComponent(o.id)}/taste`, { token: o.token });
  step('the guest sees their taste on the dish axes', v.status === 200 && (v.body?.taste?.sense || []).some(r => r.key === 'a:smoky') && Array.isArray(v.body?.taste?.because), `${v.status} ${v.text}`);
  const b = await lib.own(`/api/owner/customers/taste/builder?location_id=${lib.LOC}&key=a:smoky&min=300`);
  schema('builder matches its schema', b.body, C.builder_response_schema);
  step('the builder counts the smoky guest and plans the dish', b.status === 200 && b.body?.count >= 1 && (b.body?.plan?.dishes || []).some(d => d.id === DISH), `${b.status} ${b.text}`);
  const ba = await lib.own(`/api/owner/customers/taste/builder?location_id=${lib.LOC}&key=allergen:fish`);
  step('an allergen is not a segment key (400)', ba.status === 400, `${ba.status} ${ba.text}`);

  // 6. the personalised offer, at the public price
  const seg = { kind: 'taste', filter: { key: 'a:smoky', min: 300 }, dish: DISH, lang: 'en' };
  const coded = await lib.own(`/api/owner/campaigns?location_id=${lib.LOC}`, { name: `${run} smoky`, text: 'x', segment: seg, promo: 'LIVE10' });
  step('a taste offer with a code is refused 400', coded.status === 400, `${coded.status} ${coded.text}`);
  const off = await lib.own(`/api/owner/campaigns?location_id=${lib.LOC}`, { name: `${run} smoky`, text: 'Only 1 lek!', segment: seg });
  const text = off.body?.campaign?.text || '';
  step('the offer is labelled and carries the public price', off.status === 200 && text.startsWith('Personalised offer') && text.includes(`${d1.price} `) && !text.includes('1 lek'), `${off.status} ${text}`);
} catch (e) {
  step('the probe ran to the end', false, e.stack || String(e));
} finally {
  try {
    const r = await edit({ sense: before || {} });
    step('qa-water\'s sense is restored', r.status === 200, `${r.status} ${r.text}`);
  } catch (e) { step('qa-water\'s sense is restored', false, String(e.message || e)); }
  for (const id of orders) {
    try { await close({ lib, run, must }, id); } catch (e) { step(`ending TEST order ${id}`, false, String(e.message || e)); }
  }
}
verdict();
