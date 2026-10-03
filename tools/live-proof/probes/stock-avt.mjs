// LIVE PROBE analytics.avt + telegram.stock_digest (W-STOCK P4) on qa-durres --
// main runs it after the deploy; a lane never runs it against production.
//
//   node tools/live-proof/probes/stock-avt.mjs
//   LIVE_TG=1 node ...   (when qa-durres has a TEST bot and a group that hears stock.digest)
//
// The research row's own story, on a supply and a dish made for the run:
//   count 1000 -> receive 500 (2 lek a gram) -> sell 2 x 40 g (a real order,
//   PREPARING draws the recipe) -> waste 20 -> count 1300
//   => unexplained = 1000 + 500 - 1300 - 80 - 20 = 100 g, worth 200 lek,
// read from GET /api/owner/analytics/kitchen?days=1 and validated against the
// contract. The order is closed (refund from PREPARING, as the flows' F4 does)
// and the dish, category and supply are deleted.
import { LOC, RUN, api, own, owner, menu, reporter, contract, shelf, read } from './stock-lib.mjs';

const C = contract('stock-avt');
const { step, schema, verdict } = reporter('stock-avt');
const ITEM = `${RUN.toLowerCase()}-avt-salmon`;
let cat = null, dish = null, order = null, made = false, pickupWas = null;
const move = (kind, body) => own(`/api/owner/stock/${kind}`, body);

try {
  // ── the supply and the dish ──
  const s = await own('/api/owner/supplies', { id: ITEM, name: `${RUN} AvT salmon`, unit: 'g', kind: 'food_ingredient', category: 'LIVEPROOF', kcalPer100: 208, proteinPer100: 20, fatPer100: 13, carbsPer100: 0, location_id: LOC });
  made = s.status === 200;
  step('a probe supply exists', made, `${s.status} ${s.text.slice(0, 100)}`);
  const c = await own('/api/owner/categories', { location_id: LOC, name: `${RUN} AvT` });
  cat = c.body?.id || c.body?.category?.id;
  const p = await own('/api/owner/products', { location_id: LOC, category_id: cat, name: `${RUN} AvT roll`, price: 1000, available: true });
  dish = p.body?.id || p.body?.product?.id;
  const b = await own(`/api/owner/products/${dish}`, { location_id: LOC, bom: [{ supply: ITEM, qty: 40 }] });
  step('a probe dish with a 40 g recipe', !!dish && b.status === 200, `${p.status}/${b.status} ${dish}`);

  // ── the story ──
  const c1 = await move('count', { lines: [{ item: ITEM, observed: 1000 }] });
  step('count 1000', c1.status === 200, c1.text.slice(0, 120));
  const r1 = await move('received', { item: ITEM, qty: 500, unitCost: 2000, per: 1000, supplier: `${RUN} Sea` });
  step('receive 500 at 2 lek a gram', r1.status === 200, r1.text.slice(0, 120));
  const m0 = await menu();
  pickupWas = !!m0.location?.pickup;
  if (!pickupWas) await own('/api/owner/location', { location_id: LOC, pickup: true });
  const o = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', headers: { 'idempotency-key': `${RUN}-avt` },
    body: { items: [{ product_id: dish, quantity: 2 }], contact: { name: `${RUN} avt`, phone: '' }, fulfilment: { kind: 'pickup', note: 'LIVE PROOF - do not cook' }, payment: 'cash' } });
  order = o.body?.id || o.body?.order?.id || null;
  step('an order of two is placed', o.status < 400 && !!order, `${o.status} ${o.text.slice(0, 160)}`);
  for (const action of ['confirm', 'preparing']) {
    const a = await own(`/api/owner/orders/${order}/action`, { action, location_id: LOC });
    step(`order ${action}`, a.status === 200, `${a.status} ${a.text.slice(0, 100)}`);
  }
  const w = await move('wasted', { item: ITEM, qty: 20, reason: 'spoiled' });
  step('waste 20 (spoiled)', w.status === 200, w.text.slice(0, 120));
  const before = (await shelf(own, LOC)).supplies.find(x => x.id === ITEM);
  step('the ledger expects 1400 before the count', before?.onHand === 1400, JSON.stringify({ onHand: before?.onHand, reserved: before?.reserved }));
  const countAt = Date.now();
  const c2 = await move('count', { lines: [{ item: ITEM, observed: 1300 }] });
  step('count 1300 (drift -100)', c2.status === 200 && c2.body?.lines?.[0]?.drift === -100, c2.text.slice(0, 160));

  // ── the AvT line ──
  const k = await read(own, '/api/owner/analytics/kitchen?days=1');
  step('the kitchen numbers answer', k.status === 200, `${k.status} ${k.text.slice(0, 100)}`);
  schema('the answer validates against analytics.avt', k.body, C.response_schema);
  const row = (k.body?.avt?.rows || []).find(r => r.id === ITEM);
  step('the AvT line: 1000 + 500 - 1300 - 80 sold - 20 wasted = 100 g unexplained',
    row && row.opening === 1000 && row.received === 500 && row.closing === 1300 && row.sold === 80 && row.wasted === 20 && row.unexplained === 100,
    JSON.stringify(row && { opening: row.opening, received: row.received, sold: row.sold, wasted: row.wasted, closing: row.closing, unexplained: row.unexplained }));
  step('valued at the average of the count: 200 lek', row?.value === 200, `value=${row?.value} revenuePm=${row?.revenuePm} flags=${JSON.stringify(row?.flags)}`);
  step('it leads the top losses', (k.body?.avt?.top || []).includes(ITEM), JSON.stringify(k.body?.avt?.top));
  step('its records end with the closing count', row?.records?.[row.records.length - 1]?.kind === 'stocktake', JSON.stringify(row?.records?.map(r => r.kind)));

  // ── the digest (a real external service: NEEDS-KEY until a QA bot and group exist) ──
  if (!process.env.LIVE_TG) step('digest: stock.digest read back from the QA Telegram group', 'NEEDS-KEY', 'no TEST bot/group on qa-durres (operator ask, same as W-LIVE row 5); set LIVE_TG=1 once it exists');
  else {
    let seen = false;
    for (let i = 0; i < 4 && !seen; i++) {
      await new Promise(r => setTimeout(r, 30_000)); // the outbox drains once a minute
      const tg = await own('/api/owner/telegram');
      seen = JSON.stringify(tg.body || {}).match(/"lastOk(?:At)?":\s*(\d+)/g)?.some(m => Number(m.split(':')[1]) >= countAt) || false;
    }
    step('digest: a group was sent to after the closing count (lastOk)', seen, 'consumer read-back of the text needs the message_id kept (W-LIVE F-D)');
  }
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  if (order) {
    const r = await api(`/api/staff/orders/${order}/refund`, { method: 'POST', token: await owner(), headers: { 'idempotency-key': `${RUN}-avt-refund` },
      body: { location_id: LOC, reason: 'venue_cancelled', note: 'LIVE PROOF stock-avt cleanup' } });
    step('the order is closed (refund from PREPARING)', r.status === 200, `${r.status} ${r.text.slice(0, 100)}`);
  }
  if (pickupWas === false) step('pickup is switched back off', (await own('/api/owner/location', { location_id: LOC, pickup: false })).status === 200);
  if (dish) step('the probe dish is deleted', (await own(`/api/owner/products/${dish}/delete`, { location_id: LOC })).status === 200);
  if (cat) step('its category is deleted', (await own(`/api/owner/categories/${cat}/delete`, { location_id: LOC })).status === 200);
  if (made) step('the probe supply is deleted', (await own('/api/owner/supplies/delete', { ids: [ITEM], location_id: LOC, confirmUses: true })).status === 200);
  verdict();
}
