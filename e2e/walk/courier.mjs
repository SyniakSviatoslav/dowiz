// COURIER (courier app, QA_COURIER on the sushi-durres host) -- the per-role live walk.
//   node courier.mjs [stage]  one run (stages shift, assign, run, close; a launch the process cap refuses
//                             exits 3 and prints the stage to resume from): the courier signs in (shift on); the OWNER assigns the READY TEST
//                             delivery D from the console; the courier takes the offer, picks it up, opens
//                             "Delivered" as far as the cash question and backs out, then "Refused at the
//                             door"; the owner records the money back (COMPENSATED_REFUND).
// WHY NOT DELIVERED: the order machine has no exit from DELIVERED (order_machine.rs allowed_next), so a
// TEST order delivered would stand as a real sale for ever. The delivered screen is walked up to its
// last tap; the write is the one step left out on purpose.
// FOUR SHORT CHROMIUMS, ONE AT A TIME: a second context in the --single-process Chromium this box needs
// kills the first (measured 2026-09-24: newPage on context 2 -> "Target page, context or browser has been
// closed"). The courier's session crosses launches in a storageState file.
// PASS/FAIL/INFO per step; exit code = FAILs.
import { HOST, OUT, creds, watch, csp, browser, api, step, end, sst, ssave, heal503, LOC, consoleIn, ownerToken, ownerOrder } from './_lib.mjs';
import fs from 'node:fs';
import { devices } from 'playwright';

const S = sst();
const D = process.env.ORDER || S.orders?.D;
const CPHONE = creds.QA_COURIER_PHONE, CPASS = creds.QA_COURIER_PASSWORD;
const tok = await ownerToken();
const sOf = async () => (await ownerOrder(tok, D)) || {};
/// The courier's own token. Its login answered 503 "Worker exceeded resource limits" on 7 of 12 tries
/// (2026-09-24), so it is asked up to 8 times -- each 503 printed by api().
async function courierJwt() {
  for (let i = 0; i < 8; i++) { const r = await api('/api/courier/auth/login', { method: 'POST', body: { phone: CPHONE, password: CPASS } }); if (r.body?.jwt) return r.body.jwt; }
  step(false, 'courier API login', 'no jwt after 8 tries'); return null;
}
let b = null;
try {
  const lg = await api('/api/courier/auth/login', { method: 'POST', body: { phone: CPHONE, password: CPASS } });
  const cid = lg.body?.courier?.id;
  step(lg.status === 200 && lg.body?.courier?.locationId === LOC, 'QA courier belongs to this venue (host decides)', `${lg.status} at=${lg.body?.courier?.locationId} id=${cid}`);
  const o0 = await sOf();
  step(process.argv[2] && process.argv[2] !== 'shift' && process.argv[2] !== 'assign' ? null : o0.status === 'READY' && o0.fulfilment?.kind === 'delivery', 'TEST D is a READY delivery', `${o0.status} ${o0.fulfilment?.kind} payment=${JSON.stringify(o0.payment ?? o0.payment_method ?? null)}`);
  const PHONE = { ...devices['Pixel 7'], serviceWorkers: 'block', reducedMotion: 'reduce', permissions: ['geolocation'], geolocation: { latitude: 41.3225, longitude: 19.4450 } };
  const SESSION = `${OUT}/courier-state.json`;
  const dialogs = [], taps = [];
  const phone = async () => {   // the courier's phone, its session carried over from the last launch
    b = await browser();
    const cctx = await b.newContext({ ...PHONE, ...(fs.existsSync(SESSION) ? { storageState: SESSION } : {}) });
    const k = await cctx.newPage(); watch(k, 'courier');
    k.on('dialog', async d => { dialogs.push(d.message()); await d.accept(); });
    k.on('response', async r => { if (/\/api\/courier\/orders\//.test(r.url()) && r.request().method() === 'POST') taps.push({ status: r.status(), url: r.url().replace(HOST, ''), body: (await r.text().catch(() => '')).slice(0, 200) }); });
    await k.goto(`${HOST}/courier/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
    await k.waitForTimeout(3500);
    await heal503(k);
    return { k, cctx };
  };
  const shut = async () => { try { await b?.close(); } catch {} b = null; };
  const consoleNow = async () => { b = await browser(); const octx = await b.newContext({ viewport: { width: 420, height: 900 }, serviceWorkers: 'block' }); return consoleIn(octx); };

  const STAGES = ['shift', 'assign', 'run', 'close'];
  const from = STAGES.indexOf(process.argv[2] || 'shift');
  const at = st => { const go = STAGES.indexOf(st) >= from; if (go) { console.log(`-- stage ${st} (resume: node courier.mjs ${st})`); ssave({ courierStage: st }); } return go; };
  let k, cctx, o;
  // 1. the courier's phone: signed in and on shift before the owner assigns
  if (at('shift')) {
  try { fs.unlinkSync(SESSION); } catch {}
  ({ k, cctx } = await phone());
  await k.waitForSelector('#em', { timeout: 40000 });
  await k.fill('#em', CPHONE); await k.fill('#pw', CPASS); await k.click('#go');
  await k.waitForTimeout(4500);
  step(!(await k.$('#em')), 'courier signs in');
  const shift = await k.$('#openShift'); if (shift) { await shift.click(); await k.waitForTimeout(3500); }
  step(!(await k.$('#openShift')), 'courier is on shift', shift ? 'opened now' : 'was already on');
  await k.screenshot({ path: `${OUT}/c1-courier.png` });
  await csp(k, 'courier');
  await cctx.storageState({ path: SESSION });
  await shut();
  }

  // 2. the owner assigns D from the console
  if (at('assign')) {
  o = await consoleNow();
  await o.click('#nav [data-tab="orders"]'); await o.waitForTimeout(3000);
  // the row offers Assign only while it has a next kitchen step (admin/orders.js row(): `step && ...`);
  // at READY the order's own sheet carries it
  let asg = await o.$(`.orow [data-assign="${D}"]`), where = 'row';
  if (!asg) { await o.click(`.orow[data-o="${D}"]`).catch(() => {}); await o.waitForTimeout(1500); asg = await o.$(`#sheetIn [data-assign="${D}"]`); where = 'order sheet'; }
  step(!!asg, 'console offers Assign on the READY delivery', where);
  await asg?.click(); await o.waitForTimeout(1500);
  const choices = await o.$$eval('#sheetIn [data-c]', els => els.map(e => e.dataset.c + ':' + e.innerText.replace(/\s+/g, ' ')));
  step(choices.some(c => c.startsWith(cid)), 'Assign lists the QA courier', choices.join(' | '));
  let assigned = null;
  o.on('response', async r => { if (/\/assign$/.test(r.url())) assigned = { status: r.status(), body: (await r.text().catch(() => '')).slice(0, 200) }; });
  await o.click(`#sheetIn [data-c="${cid}"]`).catch(async () => { await o.click('#sheetIn details summary'); await o.click(`#sheetIn [data-c="${cid}"]`); });
  for (let i = 0; i < 20 && !assigned; i++) await o.waitForTimeout(500);
  const o1 = await sOf();
  step(assigned?.status === 200 && o1.courier_id === cid, 'assign lands (API courier_id)', `${JSON.stringify(assigned)} courier_id=${o1.courier_id} status=${o1.status}`);
  await o.screenshot({ path: `${OUT}/c2-assigned.png` });
  await csp(o, 'console-assign');
  await shut();
  }

  // 3. the courier's phone again: take the offer, pick up, the cash question, refused at the door
  if (at('run')) {
  ({ k, cctx } = await phone());
  step(!(await k.$('#em')), 'the courier session survives a relaunch');
  const offer = await k.waitForSelector('#takeOffer', { timeout: 60000 }).catch(() => null);
  step(!!offer, 'the courier sees the offer (#takeOffer)', await k.evaluate(() => document.querySelector('#app')?.innerText.replace(/\s+/g, ' ').slice(0, 200)));
  await k.screenshot({ path: `${OUT}/c3-offer.png` });
  await offer?.click(); await k.waitForTimeout(3500);
  let pick = await k.waitForSelector('#pick', { timeout: 30000 }).catch(() => null);
  step(!!pick, 'the courier has D in hand (Picked up offered)', JSON.stringify(taps.slice(-1)));
  if (!pick && /another courier took/.test(JSON.stringify(taps.slice(-1)))) {
    // DEFECT (2026-09-24): an OWNER-assigned order is shown to its courier as an offer, and Take answers
    // 409 "another courier took this order" -- accept() refuses whenever an assignment row exists, even
    // when it names this very courier (courier.rs accept: `if asg_of(t, &oid).is_some()`). The walk goes
    // on through the courier's own API routes so the order can still be ended from the door.
    const cj = await courierJwt();
    const pu = await api(`/api/courier/orders/${D}/pickup`, { method: 'POST', token: cj, body: {}, headers: { 'idempotency-key': 'walk-pu-' + Date.now() } });
    step(pu.status === 200, 'fallback: pickup through the courier API (the app has no way past the offer)', `${pu.status} ${JSON.stringify(pu.body).slice(0, 160)}`);
    await k.reload({ waitUntil: 'domcontentloaded' }); await k.waitForTimeout(4500); await heal503(k);
    step(null, 'courier screen after the API pickup', await k.evaluate(() => document.querySelector('#app')?.innerText.replace(/\s+/g, ' ').slice(0, 200)));
  } else { await pick?.click(); await k.waitForTimeout(4000); }
  const o2 = await sOf();
  step(o2.status === 'IN_DELIVERY', 'pickup -> IN_DELIVERY (API)', `${o2.status} ${JSON.stringify(taps.slice(-1))}`);
  await k.screenshot({ path: `${OUT}/c4-picked.png` });
  // "Delivered" as far as the cash question -- only when the order is cash (else the tap would deliver)
  const cash = /cash/.test(JSON.stringify([o2.payment ?? '', o2.payments ?? '', o2.payment_method ?? '', o2.cash_pay_with ?? '']));
  if (cash && await k.$('#done')) {
    await k.click('#done'); await k.waitForTimeout(1500);
    const got = await k.$('#got');
    step(!!got, 'Delivered asks how much cash was taken (nothing written yet)', `dialogs=${JSON.stringify(dialogs)} got=${await got?.inputValue()}`);
    await k.screenshot({ path: `${OUT}/c5-cash.png` });
    await k.click('#back').catch(() => {}); await k.waitForTimeout(1200);
    step((await sOf()).status === 'IN_DELIVERY', 'backing out of the cash question writes nothing', (await sOf()).status);
  } else step(null, 'Delivered screen skipped', `payment=${JSON.stringify(o2.payment)} -- a non-cash tap would deliver`);
  if (await k.$('#refused')) {
    await k.click('#refused'); await k.waitForTimeout(1000);
    await k.fill('#rnote', 'TEST walk: not a real delivery').catch(() => {});
    await k.click('#rgo').catch(() => {}); await k.waitForTimeout(4500);
  } else {
    step(false, 'Refused at the door button on the courier screen', await k.evaluate(() => document.querySelector('#app')?.innerText.replace(/\s+/g, ' ').slice(0, 160)));
    const rf = await api(`/api/courier/orders/${D}/refused`, { method: 'POST', token: await courierJwt(), body: { note: 'TEST walk: not a real delivery' }, headers: { 'idempotency-key': 'walk-rf-' + Date.now() } });
    step(rf.status === 200, 'fallback: refused at the door through the courier API', `${rf.status} ${JSON.stringify(rf.body).slice(0, 160)}`);
  }
  const o3 = await sOf();
  step(o3.status === 'REFUNDING', 'refused at the door -> REFUNDING (API)', `${o3.status} refund=${JSON.stringify(o3.refund || null).slice(0, 200)} ${JSON.stringify(taps.slice(-1))}`);
  await k.screenshot({ path: `${OUT}/c6-refused.png` });
  step(null, 'courier screen after', await k.evaluate(() => document.querySelector('#app')?.innerText.replace(/\s+/g, ' ').slice(0, 200)));
  await csp(k, 'courier-run');
  await shut();
  }

  // 4. the owner closes it: money back
  if (at('close')) {
  o = await consoleNow();
  await o.click('#nav [data-tab="orders"]'); await o.waitForTimeout(2500);
  await o.click(`.orow[data-o="${D}"]`).catch(e => step(false, 'D row on the console', e.message.slice(0, 80)));
  await o.waitForTimeout(1500);
  step(null, 'console order sheet for D', (await o.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '')).slice(0, 300));
  const mb = await o.$(`[data-moneyback="${D}"]`);
  step(!!mb, 'console offers Money handed back on REFUNDING');
  await mb?.click(); await o.waitForTimeout(4000);
  const o4 = await sOf();
  step(o4.status === 'COMPENSATED_REFUND', 'owner closes D -> COMPENSATED_REFUND', o4.status);
  await o.screenshot({ path: `${OUT}/c7-closed.png` });
  await csp(o, 'console-courier');
  ssave({ courierStage: 'done' });
  }
} catch (e) {
  step(false, 'courier run threw', e.stack?.slice(0, 400));
}
if (sst().courierStage === 'done') { try { fs.unlinkSync(`${OUT}/courier-state.json`); } catch {} }
await end(b);
