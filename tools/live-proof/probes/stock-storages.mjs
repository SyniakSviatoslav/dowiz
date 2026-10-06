// LIVE PROBE stock.storages 1.1.0 + stock.haccp_export 1.1.0 (W-STORE, P12/P13) on
// qa-durres -- main runs it after the deploy; a lane never runs it against
// production. WRITTEN, NOT RUN by the lane.
//
//   node tools/live-proof/probes/stock-storages.mjs
//
// 1. 2000 g of QA Salmon received INTO THE FREEZER with a lot that expires
//    first (so FEFO draws it) and a supplier's treatment paper;
// 2. 500 g moved freezer -> kitchen: the freezer +1500, the kitchen +500 over
//    the start, the next sale draws from the kitchen; 10x too much is a 409;
// 3. a QA Salmon roll sold (placed, confirmed, preparing, ready): the freezer
//    is untouched -- the sale drew from the storage that last received;
// 4. a storage card: named, refused archiving while it holds stock (409),
//    archived once emptied;
// 5. an in-house freezing record of the lot: -20 C 23 h meets no rule (still
//    written), -20 C 24 h meets -20C/24h;
// 6. the three CSV files for the venue's today: the order is found under the
//    lot (lots), the lot under the order (orders), both freezing rows and the
//    supplier paper in the freezing log; without a token, 401.
// W-STORE2 (stock.storages 1.1.0, stock.haccp_export 1.1.0):
// 7. the KITCHEN station (the QA roll has no `station`, so it is the
//    kitchen's) bound to the freezer: listed on the freezer's `stations`, the
//    next roll draws from the FREEZER although the kitchen received last;
//    unbound again (also in `finally`), the next roll draws from the kitchen.
//    A binding to an unknown or archived storage is a 400 naming it;
// 8. a freezing record with a START 20 h ago (venue time, Europe/Tirane) typed
//    as 24 h meets no rule; one started 25 h ago with no hours meets -20C/24h;
//    started 30 h ago and ended 10 h ago meets no rule (20 h);
// 9. the freezing CSV has the 1.1.0 header (by_name, started appended) and the
//    started row's last cell is a local minute.
import * as lib from '../../../e2e/flows/lib.mjs';
import { LOC, RUN, own, reporter, contract, shelf } from './stock-lib.mjs';
import { place, close } from './_order.mjs';

const CS = contract('stock-storages'), CH = contract('haccp-export');
const { step, schema, verdict } = reporter('stock-storages');
const ITEM = 'qa-salmon';
const LOT = `${RUN}-L1`;
const CELLAR = `${RUN} cellar`;
const must = (ok, why) => { if (!ok) throw new Error(why); };
const levels = async () => {
  const s = await shelf(own, LOC);
  const row = s.supplies.find(x => x.id === ITEM);
  return { s, row, at: id => row?.byStore?.stores?.[id] || 0, home: row?.byStore?.home };
};
let order = null, cellarId = null, order2 = null, order3 = null, bound = false;
/// `yyyy-mm-ddThh:mm` in the venue's zone, `h` hours before now.
const localAgo = h => {
  const p = Object.fromEntries(new Intl.DateTimeFormat('en-CA', { timeZone: 'Europe/Tirane', year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hourCycle: 'h23' })
    .formatToParts(new Date(Date.now() - h * 3600000)).map(x => [x.type, x.value]));
  return `${p.year}-${p.month}-${p.day}T${p.hour}:${p.minute}`;
};
const sellRoll = async () => {
  const id = (await place({ lib, run: RUN, must }, { items: [{ product_id: 'qa-salmon-roll', modifier_ids: [], quantity: 1 }] })).id;
  for (const a of ['confirm', 'preparing', 'ready']) {
    const x = await own(`/api/owner/orders/${id}/action`, { action: a, location_id: LOC });
    must(x.status === 200, `${a} ${x.status} ${x.text.slice(0, 100)}`);
  }
  return id;
};

try {
  // ── 1. into the freezer ──
  const s0 = await levels();
  step('the QA shelf has QA Salmon (e2e/walk/qa-setup.mjs)', !!s0.row, `${s0.s.status}`);
  schema('GET /api/owner/stock validates against stock.storages (storages, byStore)', s0.s.body, CS.response_schema);
  const recv = { item: ITEM, qty: 2000, store: 'freezer', lot: LOT, expiry: '2000-01-02', treated: `${RUN}-CERT` };
  schema('the delivery matches the contract', recv, CS.request_schema);
  const r = await own('/api/owner/stock/received', recv);
  step('2000 g received into the freezer', r.status === 200, `${r.status} ${r.text.slice(0, 120)}`);

  // ── 2. a transfer ──
  const mv = { item: ITEM, qty: 500, from: 'freezer', to: 'kitchen' };
  schema('the transfer matches the contract', mv, CS.request_schema);
  const m = await own('/api/owner/stock/moved', mv);
  step('500 g moved freezer -> kitchen', m.status === 200, `${m.status} ${m.text.slice(0, 120)}`);
  schema('the transfer answer validates', m.body, CS.moved_response_schema);
  const s1 = await levels();
  step('freezer +1500, kitchen +500, next sale from the kitchen', s1.at('freezer') - s0.at('freezer') === 1500 && s1.at('kitchen') - s0.at('kitchen') === 500 && s1.home === 'kitchen',
    JSON.stringify({ before: s0.row?.byStore, after: s1.row?.byStore }));
  step('the shelf total moved by the delivery only', s1.row?.onHand - s0.row?.onHand === 2000, `${s0.row?.onHand} -> ${s1.row?.onHand}`);
  const big = await own('/api/owner/stock/moved', { ...mv, qty: 10 * (s1.at('freezer') + 1) });
  step('moving more than the freezer holds is refused (409)', big.status === 409, `${big.status} ${big.text.slice(0, 100)}`);

  // ── 3. a sale ──
  order = (await place({ lib, run: RUN, must }, { items: [{ product_id: 'qa-salmon-roll', modifier_ids: [], quantity: 1 }] })).id;
  for (const a of ['confirm', 'preparing', 'ready']) {
    const x = await own(`/api/owner/orders/${order}/action`, { action: a, location_id: LOC });
    must(x.status === 200, `${a} ${x.status} ${x.text.slice(0, 100)}`);
  }
  const s2 = await levels();
  step('the sale left the freezer untouched (drew from the kitchen)', s2.at('freezer') === s1.at('freezer') && s1.at('kitchen') - s2.at('kitchen') >= 0,
    JSON.stringify({ freezer: [s1.at('freezer'), s2.at('freezer')], kitchen: [s1.at('kitchen'), s2.at('kitchen')] }));
  const sum = Object.values(s2.row?.byStore?.stores || {}).reduce((a, q) => a + q, 0);
  step('the storages sum to the shelf', sum === s2.row?.onHand, `${sum} vs ${s2.row?.onHand}`);

  // ── 4. a storage card ──
  const c = await own('/api/owner/stock/storage', { card: { name: CELLAR } });
  cellarId = c.body?.id || null;
  step('a storage is named', c.status === 200 && !!cellarId, `${c.status} ${c.text.slice(0, 100)}`);
  schema('the card answer validates', c.body, CS.storage_response_schema);
  await own('/api/owner/stock/moved', { item: ITEM, qty: 1, from: 'kitchen', to: cellarId });
  const no = await own('/api/owner/stock/storage', { card: { id: cellarId, name: CELLAR, archived: true } });
  step('a storage holding stock is not archived (409)', no.status === 409, `${no.status} ${no.text.slice(0, 100)}`);
  await own('/api/owner/stock/moved', { item: ITEM, qty: 1, from: cellarId, to: 'kitchen' });
  const yes = await own('/api/owner/stock/storage', { card: { id: cellarId, name: CELLAR, archived: true } });
  step('emptied, it is archived', yes.status === 200 && yes.body?.archived === true, `${yes.status} ${yes.text.slice(0, 100)}`);

  // ── 5. freezing ──
  const f23 = { item: ITEM, lot: LOT, hours: 23, tempC: -20 }, f24 = { ...f23, hours: 24 };
  schema('the freezing record matches the contract', f24, CH.request_schema);
  const a = await own('/api/owner/stock/frozen', f23), b = await own('/api/owner/stock/frozen', f24);
  step('-20 C 23 h is recorded and meets no rule', a.status === 200 && a.body?.rule === null, `${a.status} ${a.text.slice(0, 100)}`);
  step('-20 C 24 h meets -20C/24h', b.status === 200 && b.body?.rule === '-20C/24h', `${b.status} ${b.text.slice(0, 100)}`);
  schema('the freezing answer validates', b.body, CH.frozen_response_schema);

  // ── 6. the export ──
  const day = s2.s.body?.today;
  const csv = async kind => (await own(`/api/owner/stock/haccp?kind=${kind}&from=${day}&to=${day}&location_id=${LOC}`));
  const lots = await csv('lots'), ords = await csv('orders'), frz = await csv('freezing');
  const rows = x => String(x.body || '').trim().split('\n');
  step('lots.csv has the contract header', lots.status === 200 && rows(lots)[0] === CH.csv_headers.lots.join(','), `${lots.status} ${rows(lots)[0]}`);
  step('the order is found under its lot', rows(lots).some(l => l.startsWith(`${ITEM},`) && l.includes(`,${LOT},${day},${order},`)), rows(lots).filter(l => l.includes(LOT)).join(' | ').slice(0, 300));
  step('the lot is found under the order', ords.status === 200 && rows(ords).some(l => l.startsWith(`${order},${day},${ITEM},`) && l.includes(`,${LOT},`)), rows(ords).filter(l => l.includes(order)).join(' | ').slice(0, 300));
  step('the freezing log has both records and the supplier paper', frz.status === 200 && rows(frz).filter(l => l.includes(`,${LOT},in_house,`)).length >= 2
    && rows(frz).some(l => l.includes(`,${LOT},in_house,24,-20,-20C/24h,`)) && rows(frz).some(l => l.includes(`,${LOT},supplier,`) && l.includes(`${RUN}-CERT`)), rows(frz).filter(l => l.includes(LOT)).join(' | ').slice(0, 300));
  const anon = await lib.api(`/api/owner/stock/haccp?kind=lots&from=${day}&to=${day}`);
  step('without a token the export is refused (401)', anon.status === 401, `${anon.status}`);
  const badRange = await csv('lots').then(() => own(`/api/owner/stock/haccp?kind=lots&from=${day}&to=2000-01-01&location_id=${LOC}`));
  step('a range that ends before it starts is a 400', badRange.status === 400, `${badRange.status} ${badRange.text.slice(0, 80)}`);

  // ── 7. W-STORE2: a station bound to a storage ──
  const bind = { station: 'kitchen', store: 'freezer' };
  schema('the binding matches the contract', bind, CS.request_schema);
  const bd = await own('/api/owner/stock/bound', bind);
  bound = bd.status === 200;
  step('the kitchen station is bound to the freezer', bound, `${bd.status} ${bd.text.slice(0, 100)}`);
  schema('the binding answer validates', bd.body, CS.bound_response_schema);
  const s3 = await levels();
  schema('GET /api/owner/stock (1.1.0, storages[].stations) validates', s3.s.body, CS.response_schema);
  step('the freezer lists the kitchen station', (s3.s.body?.storages || []).find(x => x.id === 'freezer')?.stations?.includes('kitchen'), JSON.stringify(s3.s.body?.storages));
  order2 = await sellRoll();
  const s4 = await levels();
  step('the bound roll drew from the FREEZER, the kitchen untouched', s3.at('freezer') - s4.at('freezer') > 0 && s4.at('kitchen') === s3.at('kitchen'),
    JSON.stringify({ freezer: [s3.at('freezer'), s4.at('freezer')], kitchen: [s3.at('kitchen'), s4.at('kitchen')] }));
  const un = await own('/api/owner/stock/bound', { station: 'kitchen', store: '' });
  bound = un.status !== 200;
  step('unbound', un.status === 200, `${un.status} ${un.text.slice(0, 100)}`);
  order3 = await sellRoll();
  const s5 = await levels();
  step('unbound, the roll draws from the kitchen again', s5.at('freezer') === s4.at('freezer') && s4.at('kitchen') - s5.at('kitchen') > 0,
    JSON.stringify({ freezer: [s4.at('freezer'), s5.at('freezer')], kitchen: [s4.at('kitchen'), s5.at('kitchen')] }));
  const sum5 = Object.values(s5.row?.byStore?.stores || {}).reduce((a, q) => a + q, 0);
  step('the storages still sum to the shelf', sum5 === s5.row?.onHand, `${sum5} vs ${s5.row?.onHand}`);
  const ghost = await own('/api/owner/stock/bound', { station: 'bar', store: `${RUN}-attic` });
  step('a binding to an unknown storage is a 400 naming it', ghost.status === 400 && ghost.text.includes(`${RUN}-attic`), `${ghost.status} ${ghost.text.slice(0, 100)}`);
  const arch = cellarId ? await own('/api/owner/stock/bound', { station: 'bar', store: cellarId }) : { status: 0, text: 'no cellar' };
  step('a binding to an archived storage is a 400 naming it', arch.status === 400 && arch.text.includes(String(cellarId)), `${arch.status} ${arch.text.slice(0, 100)}`);

  // ── 8. W-STORE2: a freezing record with its start ──
  const f20 = { item: ITEM, lot: LOT, hours: 24, tempC: -20, started: localAgo(20) };
  schema('the started record matches the contract', f20, CH.request_schema);
  const c20 = await own('/api/owner/stock/frozen', f20);
  step('started 20 h ago, typed 24 h: meets no rule (decided by the start)', c20.status === 200 && c20.body?.rule === null && c20.body?.hours === 20 && c20.body?.typedHours === 24, `${c20.status} ${c20.text.slice(0, 160)}`);
  schema('the started answer validates', c20.body, CH.frozen_response_schema);
  const c25 = await own('/api/owner/stock/frozen', { item: ITEM, lot: LOT, tempC: -20, started: localAgo(25) });
  step('started 25 h ago, no hours: meets -20C/24h', c25.status === 200 && c25.body?.rule === '-20C/24h' && c25.body?.hours === 25, `${c25.status} ${c25.text.slice(0, 160)}`);
  const fe = { item: ITEM, lot: LOT, tempC: -20, started: localAgo(30), ended: localAgo(10) };
  schema('the started+ended record matches the contract', fe, CH.request_schema);
  const ce = await own('/api/owner/stock/frozen', fe);
  step('started 30 h ago, ended 10 h ago: 20 h, meets no rule (a late record adds no hours)', ce.status === 200 && ce.body?.hours === 20 && ce.body?.rule === null && Number.isInteger(ce.body?.ended), `${ce.status} ${ce.text.slice(0, 160)}`);
  const bad = await own('/api/owner/stock/frozen', { item: ITEM, lot: LOT, tempC: -20, started: 'soon' });
  step('a start that is not a local time is a 400', bad.status === 400, `${bad.status} ${bad.text.slice(0, 100)}`);

  // ── 9. W-STORE2: the CSV's name and start columns ──
  const frz2 = await csv('freezing');
  step('freezing.csv has the 1.1.0 header (by_name, started, ended appended)', frz2.status === 200 && rows(frz2)[0] === CH.csv_headers.freezing.join(','), `${frz2.status} ${rows(frz2)[0]}`);
  step('the started rows end in a local minute', rows(frz2).filter(l => l.includes(`,${LOT},in_house,`) && /,\d{4}-\d{2}-\d{2} \d{2}:\d{2},(\d{4}-\d{2}-\d{2} \d{2}:\d{2})?$/.test(l)).length >= 2,
    rows(frz2).filter(l => l.includes(LOT)).join(' | ').slice(0, 400));
} catch (e) {
  step('the probe ran to the end', false, e.stack?.slice(0, 300));
} finally {
  if (bound) { const u = await own('/api/owner/stock/bound', { station: 'kitchen', store: '' }).catch(e => ({ status: String(e) })); step('the kitchen station is left unbound', u.status === 200, `${u.status}`); }
  for (const o of [order, order2, order3].filter(Boolean)) { try { await close({ lib, run: RUN, must }, o); step(`the TEST order ${o} is closed`, true); } catch (e) { step(`the TEST order ${o} is closed`, false, e.message); } }
  verdict();
}
