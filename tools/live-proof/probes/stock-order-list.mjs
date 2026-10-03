// LIVE PROBE stock.suppliers + stock.order_list (W-STOCK P5) on qa-durres --
// main runs it after the deploy; a lane never runs it against production.
//
//   node tools/live-proof/probes/stock-order-list.mjs
//
// 1. a supplier card (Monday and Thursday, a day's lead) through the stock
//    door; read back in supplierCards; a card that is not one is refused;
// 2. a supply bought in 5 kg sacks from that supplier: 10 kg in, 2.8 kg used
//    today (written off) -> 2800 g a day; par 2800 x (1 + 4) = 14 000;
//    7 200 free -> 6 800 -> two sacks = 10 000 suggested;
// 3. the order sent (POST ordered): on its way 10 000, nothing more suggested;
// 4. the delivery of 10 000 closes it: on its way 0;
// 5. clean-up: the card gone, the supply deleted.
import { LOC, RUN, own, reporter, contract, shelf } from './stock-lib.mjs';

const CS = contract('stock-suppliers'), CO = contract('stock-order-list');
const { step, schema, verdict } = reporter('stock-order-list');
const ITEM = `${RUN.toLowerCase()}-ol-rice`;
const NAME = `${RUN} Sea`;
let card = null, made = false;
const line = async () => {
  const s = await shelf(own, LOC);
  const g = (s.body?.orderList?.groups || []).find(x => x.supplier?.id === card);
  return { s, g, l: g?.lines?.find(x => x.id === ITEM) };
};

try {
  // ── 1. the card ──
  const bad = await own('/api/owner/stock/supplier', { card: { name: NAME, days: [8] } });
  step('a card with day 8 is refused (400) and writes nothing', bad.status === 400, `${bad.status} ${bad.text.slice(0, 100)}`);
  const body = { card: { name: NAME, phone: '+355 69 000 0000', days: [4, 1], leadDays: 1, cutoff: '18:00', lang: 'sq' } };
  schema('the request matches the contract', body, CS.request_schema);
  const c = await own('/api/owner/stock/supplier', body);
  card = c.body?.card?.id || null;
  step('the card is written', c.status === 200 && !!card, `${c.status} ${c.text.slice(0, 160)}`);
  schema('the answer validates against stock.suppliers', c.body, CS.response_schema);
  step('days sorted, the id minted from the name', JSON.stringify(c.body?.card?.days) === '[1,4]' && card === NAME.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, ''), JSON.stringify(c.body?.card));

  // ── 2. a supply, its use, its suggestion ──
  const s = await own('/api/owner/supplies', { id: ITEM, name: `${RUN} OL rice`, unit: 'g', kind: 'food_ingredient', supplier: NAME, packs: [{ name: '5 kg', qty: 5000 }], location_id: LOC });
  made = s.status === 200;
  step('a supply bought in 5 kg sacks from that supplier', made, `${s.status} ${s.text.slice(0, 100)}`);
  step('10 kg delivered', (await own('/api/owner/stock/received', { item: ITEM, qty: 10000 })).status === 200);
  step('2.8 kg used today (written off)', (await own('/api/owner/stock/wasted', { item: ITEM, qty: 2800, reason: 'spoiled' })).status === 200);
  let { s: st, g, l } = await line();
  schema('GET /api/owner/stock validates against stock.order_list', st.body, CO.response_schema);
  step('the card is listed in supplierCards', (st.body?.supplierCards || []).some(x => x.id === card));
  step('it is grouped under its card', !!g && !!l, JSON.stringify(g?.supplier));
  step('2800 a day, par 14 000, two sacks suggested', l?.adu === 2800 && l?.par === 14000 && l?.available === 7200 && l?.suggest === 10000,
    JSON.stringify(l && { adu: l.adu, par: l.par, available: l.available, suggest: l.suggest, lead: l.leadDays, cycle: l.cycleDays }));

  // ── 3. sent ──
  const send = { card: { supplier: card, lines: [{ item: ITEM, qty: 10000 }] } };
  schema('the order sent matches the contract', send, CO.request_schema);
  const o = await own('/api/owner/stock/ordered', send);
  step('the order is marked as sent', o.status === 200 && /^po_\d+$/.test(o.body?.po || ''), `${o.status} ${o.text.slice(0, 120)}`);
  schema('the answer validates', o.body, CO.ordered_response_schema);
  ({ l } = await line());
  step('on its way 10 000, nothing more suggested', l?.onOrder === 10000 && l?.suggest === 0, JSON.stringify(l && { onOrder: l.onOrder, suggest: l.suggest }));

  // ── 4. delivered ──
  step('the delivery is recorded', (await own('/api/owner/stock/received', { item: ITEM, qty: 10000, supplier: NAME })).status === 200);
  ({ l } = await line());
  step('the delivery closes the order: on its way 0', l?.onOrder === 0, JSON.stringify(l && { onOrder: l.onOrder, suggest: l.suggest }));
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  if (card) {
    const gone = await own('/api/owner/stock/supplier', { card: { id: card, name: NAME, gone: true } });
    const left = ((await shelf(own, LOC)).body?.supplierCards || []).some(x => x.id === card);
    step('the card is taken off the list', gone.status === 200 && !left);
  }
  if (made) step('the probe supply is deleted', (await own('/api/owner/supplies/delete', { ids: [ITEM], location_id: LOC, confirmUses: true })).status === 200);
  verdict();
}
