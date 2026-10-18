// F7 COURIER, the courier app on a phone, the three deliveries F6 made READY:
//   A  cash with a checkout tip; first the courier types MORE than is owed (a tip at the door): the hub's
//      answer is recorded, then the exact cash. API: DELIVERED, cash_collected = total, the tip on the order.
//   B  no tip; the phone goes OFFLINE between picked-up and delivered (F10 c): the tap is queued, the
//      order stays IN_DELIVERY in the API, the app says so; back online the queue drains, the API is
//      DELIVERED exactly once (the shift's delivery count moves by one) and the app agrees.
//   D  "refused at the door" (with a note): money back recorded -- the order leaves DELIVERED's road for
//      the refund one, cash_collected 0.
// Then the courier's earnings (tips apart from the float) and C, the pickup, "collected" in the console.
import { HOST, LOC, S, save, Fail, api, statusOf, ownerOrder } from './lib.mjs';
import { launch, go, PHONE, routeLocal, localServed, guard, clean, inspect, shot, until } from './page.mjs';
import { C, softFail, endSoft, info, courierLogin, openConsole, openTab, ownerRow } from './cycle-lib.mjs';

const earnings = async jwt => (await api('/api/courier/earnings', { token: jwt })).body || {};

async function take(k, bad, x) {
  const st = await statusOf(x.id);
  if (!softFail(bad, st === 'READY', `${x.key}: READY for the courier (API ${st})`)) return false;
  await k.waitForSelector(`[data-sel="${x.id}"]`, { timeout: 45000 }).catch(() => {});
  if (!softFail(bad, !!(await k.$(`[data-sel="${x.id}"]`)), `${x.key}: the READY delivery is in the courier's pool`)) return false;
  await k.click(`[data-sel="${x.id}"]`); await k.waitForTimeout(500);
  await k.click('#take');
  if (!softFail(bad, await k.waitForSelector('#pick', { timeout: 60000 }).then(() => true, () => false), `${x.key}: take -> the order is in hand`)) return false;
  softFail(bad, (await ownerOrder(x.id))?.courier_id === C().courierId, `${x.key}: API courier_id is this courier`);
  await k.click('#pick');
  await k.waitForSelector('#done', { timeout: 60000 }).catch(() => {});
  const s = await statusOf(x.id);
  return softFail(bad, s === 'IN_DELIVERY', `${x.key}: picked up: API ${s}, want IN_DELIVERY`);
}
const toastText = k => k.evaluate(() => document.getElementById('toast')?.innerText.trim() || '');

export async function f7(log) {
  const bad = [];
  const gs = C().guests || [];
  const A = gs.find(x => x.key === 'A' && x.id), B = gs.find(x => x.key === 'B' && x.id), D = gs.find(x => x.key === 'D' && x.id);
  if (!A && !B && !D) throw new Fail('not run: F5 placed no delivery order');
  const cr = await courierLogin();
  C().courierId = cr.id; save();
  const e0 = await earnings(cr.jwt);
  const b = await launch();
  try {
    const ctx = await b.newContext({ ...PHONE, permissions: ['geolocation'], geolocation: { latitude: 41.3225, longitude: 19.445 } });
    await routeLocal(ctx);
    const k = await ctx.newPage(); const g = guard(k, 'courier');
    g.expect('POST', /\/deliver$/, 400);   // A's door tip: the hub's refusal is the thing under test
    k.on('dialog', d => d.accept());
    await go(k, `${HOST}/courier/`, g);
    await k.waitForSelector('#em', { timeout: 60000 });
    await k.fill('#em', cr.phone); await k.fill('#pw', cr.password); await k.click('#go');
    await until(k, () => !document.getElementById('em'), null, 30000);
    await k.waitForTimeout(2500);
    const skip = await k.$('[data-gd-act="skip"]'); if (skip) { await skip.click(); await k.waitForTimeout(800); }
    if (await k.$('#openShift')) {
      if (S.restore.courierShift === undefined) { S.restore.courierShift = 'opened-by-F7'; save(); }
      await k.click('#openShift');
      softFail(bad, await until(k, () => !document.getElementById('openShift'), null, 20000), 'the courier is on shift');
    }

    // ── A: a door tip first, then the exact cash ──
    if (A && await take(k, bad, A)) {
      const o = await ownerOrder(A.id);
      await k.click('#done');
      if (softFail(bad, await k.waitForSelector('#got', { timeout: 10000 }).then(() => true, () => false), 'A: the app asks how much cash was taken')) {
        const prefill = Number(await k.$eval('#got', e => e.value));
        softFail(bad, prefill === o.total, `A: the cash field starts at the order's total ${o.total} (shows ${prefill})`);
        await k.fill('#got', String(o.total + 300));
        await k.click('#confirm'); await k.waitForTimeout(3000);
        const said = await toastText(k), s1 = await statusOf(A.id);
        C().doorTip = { typed: o.total + 300, due: o.total, said, api: s1 }; save();
        softFail(bad, s1 === 'IN_DELIVERY' && said.length > 0, `A: a door tip (${o.total + 300} on ${o.total}) is answered in words, nothing recorded: app "${said.slice(0, 90)}", API ${s1}`);
        info(`A: the hub records no cash above what is owed (courier/cash.rs handover): a tip handed over at the door cannot be entered -- product gap, by design`);
        await k.fill('#got', String(o.total));
        await k.click('#confirm');
      }
      await until(k, () => !document.getElementById('done') && !document.getElementById('got'), null, 60000);
      let s = null; for (let i = 0; i < 5 && s !== 'DELIVERED'; i++) { s = await statusOf(A.id); if (s !== 'DELIVERED') await k.waitForTimeout(2000); }
      const a = await ownerOrder(A.id);
      softFail(bad, s === 'DELIVERED' && a?.cash_collected === a?.total && a?.tip === A.tip,
        `A: delivered with cash: API ${s}, cash_collected ${a?.cash_collected} of ${a?.total}, tip on the order ${a?.tip} (asked ${A.tip})`);
    }

    // ── B: offline between picked-up and delivered (F10 c) ──
    if (B && await take(k, bad, B)) {
      const before = (await earnings(cr.jwt)).today?.deliveries ?? null;
      const seen = g.bad.length;
      await ctx.setOffline(true);
      await k.click('#done');
      if (await k.waitForSelector('#got', { timeout: 10000 }).then(() => true, () => false)) await k.click('#confirm');
      await k.waitForTimeout(3000);
      const held = await k.evaluate(() => ({ outbox: !document.getElementById('outboxTag')?.hidden, n: document.getElementById('outboxText')?.innerText || '',
        done: !!document.getElementById('done'), text: document.getElementById('app')?.innerText.replace(/\s+/g, ' ').slice(0, 160) }));
      const sOff = await statusOf(B.id);
      const f1 = await shot(k, 'F7-B-offline-queued');
      const c1 = softFail(bad, held.outbox && !held.done && sOff === 'IN_DELIVERY',
        `F10c B offline: the tap is QUEUED and shown as such (outbox "${held.n}", deliver button gone: ${!held.done}), API still ${sOff} [${f1}]`);
      await ctx.setOffline(false);
      const dropped = g.bad.splice(seen);
      if (dropped.length) info(`F10c: ${dropped.length} request(s) failed while the phone was offline (expected): ${dropped[0].slice(0, 100)}`);
      let s = null; for (let i = 0; i < 12 && s !== 'DELIVERED'; i++) { await k.waitForTimeout(2500); s = await statusOf(B.id); }
      await until(k, () => document.getElementById('outboxTag')?.hidden !== false, null, 20000);
      const after = (await earnings(cr.jwt)).today?.deliveries ?? null;
      const appNow = await k.evaluate(() => ({ outbox: !document.getElementById('outboxTag')?.hidden, done: !!document.getElementById('done') }));
      const bo = await ownerOrder(B.id);
      const c2 = softFail(bad, s === 'DELIVERED' && before != null && after === before + 1 && !appNow.outbox && !appNow.done && bo?.cash_collected === bo?.total,
        `F10c B back online: the queue drained, API ${s} exactly once (shift deliveries ${before} -> ${after}), cash ${bo?.cash_collected}/${bo?.total}, app: outbox empty ${!appNow.outbox}, run closed ${!appNow.done}`);
      C().f10c = { ok: c1 && c2, queued: held, apiOffline: sOff, apiAfter: s, deliveries: [before, after], shot: f1 }; save();
    }

    // ── D: refused at the door ──
    if (D && await take(k, bad, D)) {
      await k.click('#refused');
      await k.waitForSelector('#rnote', { timeout: 10000 });
      await k.fill('#rnote', 'QA flows gate: nobody opened the door');
      await k.click('#rgo');
      let o = null; for (let i = 0; i < 6; i++) { await k.waitForTimeout(2000); o = await ownerOrder(D.id); if (o && o.status !== 'IN_DELIVERY') break; }
      const ok = ['REFUNDING', 'COMPENSATED_REFUND'].includes(o?.status) && o?.refund?.reason === 'refused_at_door' && !(o?.cash_collected > 0);
      C().refusedAtDoor = { status: o?.status, refund: o?.refund || null, cash: o?.cash_collected ?? null }; save();
      softFail(bad, ok, `D refused at the door: money back recorded: API ${o?.status}, refund ${JSON.stringify(o?.refund || null).slice(0, 140)}, cash_collected ${o?.cash_collected}`);
    }

    const e1 = await earnings(cr.jwt);
    const dTips = (e1.today?.tips ?? 0) - (e0.today?.tips ?? 0);
    softFail(bad, dTips === (A ? A.tip : 0), `courier earnings: today's tips moved by ${dTips} (A's checkout tip ${A?.tip ?? 0}), kept apart from the cash ${e0.today?.cash} -> ${e1.today?.cash}`);
    clean(g, 'F7 courier');
    await inspect(k, 'F7 courier');
    await shot(k, 'F7-courier-after');
    localServed(ctx, 'F7');
  } catch (e) {
    { const pg = b.contexts()[0]?.pages()[0]; if (pg) e.shot = await shot(pg, 'F7-fail'); }
    throw e;
  } finally { await b.close().catch(() => {}); }

  // ── C: the pickup, "collected" from the console ──
  const cx = gs.find(x => x.key === 'C' && x.id);
  if (cx) {
    const ob = await launch();
    try {
      const { p, g } = await openConsole(ob, 'owner-F7');
      await openTab(p, 'orders');
      if (softFail(bad, await ownerRow(p, cx.id), 'C: the console lists the READY pickup')) {
        const c = await p.$(`.orow [data-act="collected"][data-o="${cx.id}"]`);
        if (softFail(bad, !!c, 'C: the READY pickup offers "Collected" on its row')) await c.click();
        let s = null; for (let i = 0; i < 6 && s !== 'PICKED_UP'; i++) { await p.waitForTimeout(1500); s = (await ownerOrder(cx.id))?.status; }
        softFail(bad, s === 'PICKED_UP', `C: collected: API ${s}, want PICKED_UP`);
      }
      clean(g, 'F7 owner');
      await inspect(p, 'F7 owner');
    } finally { await ob.close().catch(() => {}); }
  }
  endSoft(bad, 'F7');
}
