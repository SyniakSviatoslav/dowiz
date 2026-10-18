// F8 IN-VENUE: the floor plan (tables 91 and 92 in a zone of this run, ADDED to whatever plan the
// venue has and put back after), the till opened by a counter-manager, and the room app on a waiter's
// phone (both staff invited and claimed through the product):
//   R1 table 91, the waiter's round      -> paid by CARD in ALL with a TIP
//   R2 table 91, the guest's QR round     -> confirmed by the waiter, paid in CASH IN EUR at a typed rate
//                                            (rate_ppm, change right: what came off the bill is the hub's)
//   R3 table 92, the waiter's round placed while the phone is OFFLINE (F10 d) -> CARD, NO tip
// then the till: tips by person, cash per currency (src/command/till/fold.rs), and both tables cleared.
import { HOST, LOC, RUN, S, save, Fail, api, own, menu, dishes, opened, ownerOrder } from './lib.mjs';
import { launch, go, PHONE, routeLocal, guard, clean, shot, until } from './page.mjs';
import { C, softFail, endSoft, info, randPhone, checkoutUI, makeStaff, staffJwt, roomIn } from './cycle-lib.mjs';

const lastPay = o => (Array.isArray(o?.payments) && o.payments.length ? o.payments[o.payments.length - 1] : null);
const roomRounds = async jwt => {
  const r = await api(`/api/staff/room?location_id=${LOC}`, { token: jwt });
  return (r.body?.sittings || []).flatMap(s => (s.rounds || []).map(x => ({ ...x, sitting_id: x.sitting_id || s.sitting_id, table: s.table })));
};

/// Place a round from the room app's "open table" screen. Answers the POST's {status, body} or null.
async function openRound(p, table, n) {
  await p.click('.bar [data-act="open"]');
  await p.waitForSelector('[data-in="table"]', { timeout: 15000 });
  await p.fill('[data-in="table"]', table);
  await p.waitForSelector('[data-act="pick"]:not([disabled])', { timeout: 20000 }).catch(() => {});   // the menu arrives after the screen
  const picks = await p.$$('[data-act="pick"]:not([disabled])');
  if (!picks.length) return { none: true };
  for (let i = 0; i < n; i++) await (await p.$$('[data-act="pick"]:not([disabled])'))[i % picks.length].click();
  let placed = null;
  const h = async r => { if (/\/api\/public\/locations\/[^/]+\/orders$/.test(r.url()) && r.request().method() === 'POST') { try { placed = { status: r.status(), body: await r.json() }; } catch { placed = { status: r.status() }; } } };
  p.on('response', h);
  await p.click('[data-act="send"]');
  for (let i = 0; i < 80 && !placed; i++) await p.waitForTimeout(500);
  p.off('response', h);
  if (!placed) placed = { status: null, toast: await p.evaluate(() => document.getElementById('toast')?.innerText.trim() || '') };
  return placed;
}
/// From the room list into one round of one sitting.
async function intoRound(p, sitting, round) {
  for (let i = 0; i < 3 && await p.$('[data-act="back"]'); i++) { await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(600); }
  await p.click('.bar [data-act="refresh"]').catch(() => {}); await p.waitForTimeout(1500);
  await p.click(`[data-act="sit"][data-id="${sitting}"]`); await p.waitForTimeout(1500);
  // a sitting with ONE round opens that round straight away (room/app.js); with more, the round is picked
  const r = await p.$(`[data-act="round"][data-id="${round}"]`);
  if (r) { await r.click(); await p.waitForTimeout(1000); }
}
/// Pay one round in the room app's pay screen (room/pay.js). o: {method, currency?, rate?, tip?}
async function payRound(p, o) {
  await p.click('[data-act="pay"]'); await p.waitForSelector('form[data-form="pay"]', { timeout: 10000 });
  if (o.currency) { await p.click(`[data-act="currency"][data-v="${o.currency}"]`); await p.waitForTimeout(400); }
  await p.click(`[data-act="method"][data-v="${o.method}"]`); await p.waitForTimeout(400);
  if (o.rate) { await p.fill('[data-in="rate"]', o.rate); await p.waitForTimeout(300); await p.click('[data-act="fill"]'); await p.waitForTimeout(400); }
  if (o.tip) { await p.fill('[data-in="tip"]', String(o.tip)); await p.waitForTimeout(300); }
  const amount = await p.$eval('[data-in="amount"]', e => e.value).catch(() => null);
  await p.click('form[data-form="pay"] button[type="submit"]');
  await p.waitForTimeout(3500);
  const note = await p.evaluate(() => [...document.querySelectorAll('.ui-alert')].filter(e => e.checkVisibility()).map(e => e.innerText.trim()).join(' / ') || document.getElementById('toast')?.innerText.trim() || '');
  return { amount, note };
}

export async function f8(log) {
  const bad = [];
  const w = await makeStaff('waiter');
  const cm = await makeStaff('counter-manager');
  log(`waiter ${w.email} and counter-manager ${cm.email} invited and claimed through the product`);
  const zone = `flows-room-${RUN.slice(6).toLowerCase()}`;
  const prev = await own(`/api/owner/floorplan?location_id=${LOC}`);
  const prevZones = Array.isArray(prev.body?.zones) ? prev.body.zones : [];
  if (S.restore.floorplan === undefined) { S.restore.floorplan = prevZones; save(); }
  const plan = await own('/api/owner/floorplan', { zones: [...prevZones.filter(z => !String(z.id).startsWith('flows-room-')), { id: zone, name: `${RUN} room`, tables: [
    { n: 91, x: 60, y: 60, w: 60, h: 60, seats: 4, shape: 'rect' }, { n: 92, x: 180, y: 60, w: 60, h: 60, seats: 4, shape: 'rect' }] }] });
  if (!softFail(bad, plan.status === 200, `the floor plan takes tables 91 and 92 beside the venue's ${prevZones.length} zone(s): ${plan.status} ${plan.text.slice(0, 120)}`)) throw new Fail(bad[0]);
  C().zone = zone; save();
  const qr = await own(`/api/owner/tables/qr?location_id=${LOC}`);
  const q91 = (qr.body?.tables || []).find(t => t.n === 91 && (t.zone === zone || String(t.url || '').includes(zone)));
  if (!softFail(bad, !!q91?.url, `table 91 has a QR link (${q91?.url ? 'yes' : qr.text.slice(0, 120)})`)) throw new Fail(bad[0]);
  // the till, for cash (pay.rs refuses cash with no till open)
  const cmJwt = await staffJwt(cm);
  const till = await api('/api/staff/till/open', { method: 'POST', token: cmJwt, body: { location_id: LOC, float: { ALL: 0, EUR: 0 } } });
  if (till.status === 200) { S.restore.till = 'opened-by-F8'; C().tillBy = cm; save(); }
  info(`till open as the counter-manager: ${till.status} ${till.text.slice(0, 100)}`);
  const T91 = `${zone}:91`, T92 = `${zone}:92`;

  // 1) the waiter opens table 91 (R1), then table 92 OFFLINE (R3, F10 d)
  let b = await launch();
  try {
    const { ctx, p, g, ok } = await roomIn(b, w, 'room');
    if (!softFail(bad, ok, 'the waiter signs in to the room app')) throw new Fail(bad[0]);
    const r1 = await openRound(p, T91, 2);
    softFail(bad, r1?.status === 200 && r1.body?.fulfilment?.table === T91, `R1: the waiter's round on table 91: ${r1?.status} ${r1?.body?.id ?? JSON.stringify(r1).slice(0, 100)}`);
    if (r1?.body?.id) { C().r1 = r1.body.id; C().sitting91 = r1.body.sitting_id; opened(r1.body.id, null, 'F8'); save(); }
    await shot(p, 'F8-waiter-r1');
    // F10 d: offline while placing a round
    for (let i = 0; i < 3 && await p.$('[data-act="back"]'); i++) { await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(500); }
    const seen = g.bad.length;
    await ctx.setOffline(true);
    const off = await openRound(p, T92, 1);
    await p.waitForTimeout(1500);
    const said = await p.evaluate(() => document.getElementById('toast')?.innerText.trim() || '');
    const fOff = await shot(p, 'F10d-waiter-offline');
    await ctx.setOffline(false);
    const dropped = g.bad.splice(seen);
    if (dropped.length) info(`F10d: ${dropped.length} request(s) failed while offline (expected): ${dropped[0].slice(0, 90)}`);
    await p.waitForTimeout(6000);
    const at92 = async () => (await roomRounds(await staffJwt(w))).filter(r => r.table === T92 || r.fulfilment?.table === T92);
    let n92 = (await at92()).length;
    const queuedLanded = n92 > 0;
    if (!n92 && await p.$('[data-act="send"]:not([disabled])')) {        // not queued: the waiter taps send again
      const again = await openRoundSendOnly(p);
      info(`F10d: the round was not queued; send tapped again online: ${again?.status}`);
      await p.waitForTimeout(2000);
      n92 = (await at92()).length;
    }
    const r3 = (await at92())[0];
    C().f10d = { offlineAnswer: off ? (off.status ?? 'no response') : 'no response', said, queuedLanded, rounds92: n92, shot: fOff, ok: n92 === 1 }; save();
    softFail(bad, n92 === 1, `F10d waiter offline while placing: page said "${said.slice(0, 80)}"; ${queuedLanded ? 'the queued round landed by itself' : 'NOT queued (open.js places through c.api, not the outbox): an honest refusal, the waiter re-sends'}; table 92 has exactly ${n92} round(s), want 1 [${fOff}]`);
    if (r3) { C().r3 = r3.id; C().sitting92 = r3.sitting_id; opened(r3.id, null, 'F8'); save(); }
    clean(g, 'F8 waiter R1/R3');
  } catch (e) {
    { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F8-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }

  // 2) the guest's QR round on table 91 (R2)
  b = await launch();
  try {
    const ctx = await b.newContext(PHONE); await routeLocal(ctx);
    const p = await ctx.newPage(); const g = guard(p, 'store-qr');
    await go(p, `${HOST}${q91.url.replace(/^https?:\/\/[^/]+/, '')}`, g);
    await p.waitForSelector('.card', { timeout: 60000 }).catch(() => {});
    await p.waitForTimeout(2500);
    const dishId = dishes(await menu()).find(d => d.available)?.id;
    const placed = await checkoutUI(p, { how: 'table', ids: [dishId], qty: 1, name: `${RUN} guest T`, phone: randPhone() }).catch(e => { bad.push(`R2 the QR round: ${e.message}`); return null; });
    if (placed?.id) {
      opened(placed.id, placed.access_token, 'F8'); C().r2 = placed.id; save();
      softFail(bad, !C().sitting91 || placed.sitting_id === C().sitting91, `R2: the QR round lands in the waiter's sitting (${placed.sitting_id ?? 'none'} vs ${C().sitting91})`);
    }
    clean(g, 'F8 QR');
    await shot(p, 'F8-qr-round');
  } finally { await b.close().catch(() => {}); }

  // 3) the waiter: confirm R2, card+tip R1, cash EUR R2, card no tip R3, clear both tables
  b = await launch();
  try {
    const { p, g, ok } = await roomIn(b, w, 'room2');
    if (!ok) throw new Fail('the waiter could not sign in again');
    const wj = await staffJwt(w);
    if (C().r2 && C().sitting91) {
      await intoRound(p, C().sitting91, C().r2);
      await p.click('[data-act="guestConfirm"]').catch(() => {}); await p.waitForTimeout(2500);
      const o = await ownerOrder(C().r2);
      softFail(bad, o && o.status !== 'PENDING', `R2: the waiter answers the guest's round: API ${o?.status}`);
    }
    const pays = [
      [C().r1, C().sitting91, { method: 'card', tip: 50 }, 'R1 card ALL +tip 50'],
      [C().r2, C().sitting91, { method: 'cash', currency: 'EUR', rate: '100' }, 'R2 cash EUR at 1 EUR = 100 ALL'],
      [C().r3, C().sitting92, { method: 'card' }, 'R3 card, no tip'],
    ].filter(x => x[0] && x[1]);
    C().roomPays = [];
    for (const [id, sit, how, label] of pays) {
      await intoRound(p, sit, id);
      const r = await payRound(p, how);
      const o = await ownerOrder(id), lp = lastPay(o);
      const want = how.currency ? lp?.currency === how.currency && Number(lp?.rate_ppm) === 1000000 : true;
      C().roomPays.push({ label, id, typed: r.amount, note: r.note, payment_status: o?.payment_status, total: o?.total, pay: lp }); save();
      softFail(bad, o?.payment_status === 'paid' && lp?.method === how.method && (Number(lp?.tip || 0) === (how.tip || 0)) && want,
        `${label}: API ${o?.payment_status}, last payment ${JSON.stringify(lp || null).slice(0, 160)}; the screen said "${r.note.slice(0, 90)}"`);
    }
    // THE KITCHEN'S SIDE of the three rounds (its screen is F6's): to READY through the owner's own route,
    // so the tables can reach "dirty" (command/floor.rs state_of: a round still PENDING/CONFIRMED is "ordering").
    for (const id of [C().r1, C().r2, C().r3].filter(Boolean)) {
      for (const a of ['confirm', 'preparing', 'ready']) {
        const o = await ownerOrder(id); if (!o || ['READY', 'DELIVERED'].includes(o.status)) break;
        if (a === 'confirm' && o.status !== 'PENDING') continue;
        if (a === 'preparing' && o.status !== 'CONFIRMED') continue;
        const r = await own(`/api/owner/orders/${id}/action`, { action: a, location_id: LOC });
        if (r.status !== 200) { softFail(bad, false, `kitchen side: ${a} on round ${id.slice(0, 8)}: ${r.status} ${r.text.slice(0, 100)}`); break; }
      }
    }
    // the till: tips by person, the cash per currency
    const tips = await api(`/api/staff/till/tips?location_id=${LOC}`, { token: await staffJwt(cm) });
    C().tillTips = tips.body; save();
    softFail(bad, tips.status === 200 && JSON.stringify(tips.body).includes('50'), `till tips by person carry the waiter's 50 ALL: ${tips.status} ${tips.text.slice(0, 160)}`);
    // the floor: both tables cleared
    for (const [sit, n] of [[C().sitting91, 91], [C().sitting92, 92]].filter(x => x[0])) {
      for (let i = 0; i < 3 && await p.$('[data-act="back"]'); i++) { await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(500); }
      await p.click('.bar [data-act="floor"]'); await p.waitForTimeout(2500);
      await p.click(`[data-act="pickTable"][data-id="${sit}"]`).catch(() => {}); await p.waitForTimeout(800);
      await p.click(`[data-act="tableCleared"][data-id="${sit}"]`).catch(() => {}); await p.waitForTimeout(2500);
      const fl = await api(`/api/staff/floor?location_id=${LOC}`, { token: wj });
      const t = (fl.body?.zones || []).flatMap(z => (z.tables || []).map(x => ({ ...x, zone: z.id }))).find(x => x.zone === zone && x.n === n);
      softFail(bad, t?.state === 'free', `table ${n} is cleared: API ${JSON.stringify(t || null).slice(0, 120)}`);
    }
    await shot(p, 'F8-cleared');
    clean(g, 'F8 waiter');
  } catch (e) {
    { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F8-waiter-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }
  await closeTill(bad);
  const back = await own('/api/owner/floorplan', { zones: S.restore.floorplan || [] });
  if (back.status === 200) { delete S.restore.floorplan; save(); }
  softFail(bad, back.status === 200, `the venue's floor plan is put back as found (${(prevZones || []).length} zone(s)): ${back.status} ${back.text.slice(0, 100)}`);
  endSoft(bad, 'F8');
}

/// The send button alone (the basket and the table are still on screen after a refused send).
async function openRoundSendOnly(p) {
  let placed = null;
  const h = async r => { if (/\/orders$/.test(r.url()) && r.request().method() === 'POST') placed = { status: r.status() }; };
  p.on('response', h); await p.click('[data-act="send"]');
  for (let i = 0; i < 30 && !placed; i++) await p.waitForTimeout(500);
  p.off('response', h); return placed;
}

/// The till this flow opened: counted as expected, closed; the Z report's cash per currency recorded.
export async function closeTill(bad) {
  if (S.restore.till !== 'opened-by-F8' || !C().tillBy) return;
  const jwt = await staffJwt(C().tillBy);
  const h = await own(`/api/owner/health?location_id=${LOC}`);
  const per = (h.body?.till?.periods || []).filter(x => !x.closed_at).slice(-1)[0];
  C().tillPeriod = per || null; save();
  const eur = (C().roomPays || []).find(x => x.pay?.currency === 'EUR');
  softFail(bad, !!per && (!eur || Number(per.cash_paid?.EUR || 0) === Number(eur.pay?.amount || 0)),
    `the open till holds the cash per currency: ${JSON.stringify(per?.cash_paid ?? null)} (EUR paid ${eur?.pay?.amount ?? 'none'})`);
  if (per?.expected) await api('/api/staff/till/count', { method: 'POST', token: jwt, body: { location_id: LOC, observed: per.expected } });
  const z = await api('/api/staff/till/close', { method: 'POST', token: jwt, body: { location_id: LOC } });
  C().tillZ = z.body; save();
  if (z.status === 200) { delete S.restore.till; save(); }
  softFail(bad, z.status === 200, `the till is counted and closed (Z report): ${z.status} ${z.text.slice(0, 160)}`);
}
