// F12 THE KITCHEN'S ORDER RAIL OFFLINE (S2b, AC-LV-LOC-05) on the QA hub (qa-durres ONLY).
//
//   standalone:  FLOWS_HOST=https://qa-durres.dowiz.org node e2e/flows/f12-offline-kitchen.mjs
//   from run.mjs: import f12 from './f12-offline-kitchen.mjs' -> await f12(ctx, log)
//   exit: 0 GREEN, 1 RED (a check failed), 2 COULD NOT RUN (named on stdout)
//
// WHAT IT DRIVES, in Chromium on the deployed board (/room/canvas/), with the room's service worker:
//   1. online: a TEST order is placed; the board page is opened signed in as the owner (staff_at
//      takes an owner token for Cap::Advance) and the SW is made to control it;
//   2. context.setOffline(true): the SHIPPED tablet.js + tablet.wasm, served by the SW from its
//      cache, take three taps on the ticket -- seen, CONFIRMED, PREPARING -- each decided by the
//      order machine in the module and kept in IndexedDB; an illegal tap is refused by name;
//   3. the page is reloaded STILL OFFLINE and the queue is read back from IndexedDB (3 taps);
//   4. meanwhile the owner confirms the same order online (the hub moved without the tablet);
//   5. setOffline(false): one sync (POST /api/staff/orders/glue). The law sent == merged + held,
//      3 merged (the CONFIRMED tap is Done: the hub got there), the order is PREPARING with
//      kitchen.seen on the owner's API, and the decision strip names no decision for it.
// The TEST order is ended by the refund route whatever happens.
//
// SCRIPT GAP (declared): the taps go through the tablet module the board loads, not through
// canvas pixels -- the board's intents 1/2 reach the module only once board.js routes them there
// (LV-LOC-B hand-back). The module, the SW cache, IndexedDB and the route are the shipped ones.

import { HOST, LOC, RUN, Fail, api, owner, ownerOrder } from './lib.mjs';
import { launch, until, go } from './page.mjs';

const TAPS = [['', 'seen'], ['CONFIRMED', 'confirm'], ['PREPARING', 'cook']];

async function placeTest() {
  const body = { items: [{ product_id: 'qa-water', modifier_ids: [], quantity: 1 }], contact: { name: `${RUN} guest`, phone: '+355690000019' },
    fulfilment: { kind: 'delivery', address: { line: `${RUN} Rruga Isuf Ferra 1, Durres`, note: null, lat_udeg: 41323100, lon_udeg: 19441400 } }, payment: 'cash', locale: 'en' };
  const r = await api(`/api/public/locations/${LOC}/orders`, { method: 'POST', body });
  if (r.status !== 200 || !r.body?.id) throw new Fail(`placing a TEST order: ${r.status} ${r.text.slice(0, 140)}`);
  return r.body.id;
}

async function end(id, token) {
  const o = await ownerOrder(id);
  if (!o || ['DELIVERED', 'CANCELLED', 'REJECTED', 'PICKED_UP', 'COMPENSATED_REFUND'].includes(o.status)) return;
  if (o.status === 'PENDING') { await api(`/api/owner/orders/${id}/action`, { method: 'POST', token, body: { action: 'reject', location_id: LOC } }); return; }
  await api(`/api/staff/orders/${id}/refund`, { method: 'POST', token, headers: { 'idempotency-key': `${RUN}-f12-${id}` },
    body: { location_id: LOC, reason: 'venue_cancelled', note: 'f12 cleanup' } });
}

export default async function f12(_ctx, log = console.log) {
  const token = await owner();
  const id = await placeTest();
  log(`F12 order ${id}`);
  const browser = await launch();
  try {
    const context = await browser.newContext({ viewport: { width: 1024, height: 768 }, serviceWorkers: 'allow', reducedMotion: 'reduce' });
    const session = { jwt: token, staff: { id: 'owner', locationId: LOC, role: 'owner', caps: 'advance,take_orders', expiresMs: Date.now() + 3600_000 } };
    await context.addInitScript(s => { try { if (!localStorage.getItem('dw_room_session')) localStorage.setItem('dw_room_session', s); } catch {} }, JSON.stringify(session));
    const page = await context.newPage();
    const uncaught = [];
    page.on('pageerror', e => uncaught.push(String(e.message).split('\n')[0].slice(0, 160)));
    await go(page, `${HOST}/room/canvas/`);
    if (!await until(page, () => navigator.serviceWorker?.ready.then(() => true), null, 30000)) throw new Fail('the room service worker never became ready');
    await page.reload({ waitUntil: 'domcontentloaded' });
    if (!await until(page, () => !!navigator.serviceWorker?.controller, null, 20000)) throw new Fail('the room service worker does not control the board');
    const board = await api(`/api/staff/kitchen?location_id=${LOC}`, { token });
    if (board.status !== 200 || !(board.body?.orders || []).some(o => o.id === id)) throw new Fail(`the kitchen board does not list ${id}: ${board.status}`);

    // ── OFFLINE: the shipped module, from the SW cache, takes the taps ──
    await context.setOffline(true);
    const took = await page.evaluate(async ({ loc, orders, id, taps }) => {
      const t = await import('/room/canvas/tablet.js');
      const inst = await WebAssembly.instantiateStreaming(fetch('/room/canvas/tablet.wasm'), {});
      const tab = t.tablet({ mod: t.surface(inst.instance), store: t.idbStore(`${loc}:kitchen`), post: async () => { throw Object.assign(new Error('offline'), { offline: true }); }, loc });
      await tab.orderRail(orders);
      const out = [];
      for (const [next] of taps) out.push(await tab.order(id, next));
      let refused = '';
      try { await tab.order(id, 'CONFIRMED'); } catch (e) { refused = `${e.code} ${e.message}`; }
      return { out, refused, pending: await tab.ordersPending() };
    }, { loc: LOC, orders: board.body.orders, id, taps: TAPS });
    log(`F12 offline taps ${JSON.stringify(took)}`);
    if (took.pending !== 3 || !took.out.every(Boolean)) throw new Fail(`offline: 3 taps queued expected, got ${JSON.stringify(took)}`);
    if (!/^46 IllegalTransitionError/.test(took.refused)) throw new Fail(`offline: PREPARING -> CONFIRMED was not refused by the order machine: ${took.refused}`);

    // ── RELOAD, STILL OFFLINE: the queue survives in IndexedDB ──
    await page.reload({ waitUntil: 'domcontentloaded' }).catch(e => { throw new Fail(`offline reload: ${String(e.message).split('\n')[0]}`); });
    const kept = await page.evaluate(async loc => {
      const t = await import('/room/canvas/tablet.js');
      const inst = await WebAssembly.instantiateStreaming(fetch('/room/canvas/tablet.wasm'), {});
      return t.tablet({ mod: t.surface(inst.instance), store: t.idbStore(`${loc}:kitchen`), post: null, loc }).ordersPending();
    }, LOC);
    if (kept !== 3) throw new Fail(`after an offline reload the queue holds ${kept}, not 3`);

    // ── THE HUB MOVES WITHOUT THE TABLET ──
    const c = await api(`/api/owner/orders/${id}/action`, { method: 'POST', token, body: { action: 'confirm', location_id: LOC } });
    if (c.status !== 200) throw new Fail(`owner confirm ${c.status} ${c.text.slice(0, 120)}`);

    // ── ONLINE: one sync ──
    await context.setOffline(false);
    const reply = await page.evaluate(async ({ loc, jwt }) => {
      const t = await import('/room/canvas/tablet.js');
      const inst = await WebAssembly.instantiateStreaming(fetch('/room/canvas/tablet.wasm'), {});
      const post = async (p, body) => {
        const r = await fetch(`/api${p}`, { method: 'POST', headers: { 'content-type': 'application/json', authorization: `Bearer ${jwt}` }, body: JSON.stringify(body) });
        const j = await r.json().catch(() => ({}));
        if (!r.ok) throw Object.assign(new Error(`${r.status} ${JSON.stringify(j).slice(0, 120)}`), { status: r.status });
        return j;
      };
      const tab = t.tablet({ mod: t.surface(inst.instance), store: t.idbStore(`${loc}:kitchen`), post, loc });
      const r = await tab.syncOrders();
      return { r, left: await tab.ordersPending() };
    }, { loc: LOC, jwt: token });
    log(`F12 sync ${JSON.stringify(reply)}`);
    const r = reply.r;
    if (r.sent !== 3 || r.sent !== r.merged + r.held.length) throw new Fail(`the law: ${JSON.stringify(r)}`);
    if (r.merged !== 3 || r.applied !== 2) throw new Fail(`3 merged (the CONFIRMED tap already done by the hub), 2 applied: ${JSON.stringify(r)}`);
    if (reply.left !== 0) throw new Fail(`after the sync ${reply.left} taps still wait`);
    const o = await ownerOrder(id);
    if (o?.status !== 'PREPARING' || !o?.kitchen?.seen) throw new Fail(`the owner's order: ${o?.status} seen=${JSON.stringify(o?.kitchen?.seen || null)}`);
    const d = await api(`/api/staff/stock/decisions?location_id=${LOC}`, { token });
    if ((d.body?.open || []).some(x => x.order === id)) throw new Fail(`a decision was left for a clean rebase: ${d.text.slice(0, 160)}`);
    if (uncaught.length) throw new Fail(`uncaught page errors: ${uncaught.join(' | ')}`);
    log('F12 GREEN: 3 offline taps, offline reload kept them, hub moved meanwhile, sync merged 3 held 0, order PREPARING + seen');
    return 0;
  } finally {
    await browser.close().catch(() => {});
    await end(id, token).catch(e => log(`F12 cleanup of ${id}: ${e.message}`));
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try { process.exit(await f12({})); } catch (e) {
    console.log(`F12 ${e instanceof Fail ? 'RED' : 'COULD NOT RUN'}: ${e.message}`);
    process.exit(e instanceof Fail ? 1 : 2);
  }
}
