// WAITER (room app) -- the per-role live walk, one phase per run. The waiter is a
// TEST staff member the owner invited from the console (owner.mjs setup).
//   node waiter.mjs claim     claim the invite in the room app, the Floor with the TEST plan, open table 91, place R1
//   node waiter.mjs confirm   the guest's QR round (R2) waits in the waiter's sitting; confirm it
//   node waiter.mjs pay       pay R1 and R2 by card with a tip; the till's tips with no till open; the Floor's
//                             to-clear table; Cleared; the header never says "0 queued"
//   node waiter.mjs tips      the till screen's tips as the OWNER (a waiter preset holds no till)
// PASS/FAIL/INFO per step with the API read-back; exit code = FAILs.
import { HOST, OUT, creds, watch, csp, browser, api, step, end, sst, ssave, LOC, roomIn, staffLogin, staffToken } from './_lib.mjs';

const PHASE = process.argv[2] || 'claim';
const S = sst();
const W = S.waiter;
if (!W?.email) { console.log('no TEST waiter in suite state: run owner.mjs setup first'); process.exit(2); }
const b = await browser();
const ctx = await b.newContext({ viewport: { width: 400, height: 860 }, serviceWorkers: 'block', reducedMotion: 'reduce' });
const text = p => p.evaluate(() => document.querySelector('#app').innerText.replace(/\s+/g, ' ').slice(0, 500));
const queuedTag = p => p.evaluate(() => ({ hidden: document.getElementById('outboxTag')?.hidden, text: document.getElementById('outboxText')?.textContent }));
const tok = async () => (await staffLogin(W.email, W.password)).jwt;
const room = async () => (await api(`/api/staff/room?location_id=${LOC}`, { token: await tok() })).body;
const roundOf = async id => { for (const s of (await room()).sittings || []) for (const r of s.rounds || []) if (r.id === id) return { ...r, sitting: s }; return null; };

/// The till screen's tips section, whoever holds the till; answers nothing, prints PASS/FAIL.
async function tillTips(tp, who) {
  await tp.click('[data-act="till"]').catch(e => step(false, 'till button', e.message.slice(0, 80))); await tp.waitForTimeout(4500);
  const till = await tp.evaluate(() => ({ tips: document.querySelector('[data-tips]')?.innerText.replace(/\s+/g, ' ') || null, openForm: !!document.querySelector('form[data-form="open"]'), text: document.querySelector('#app').innerText.replace(/\s+/g, ' ').slice(0, 300) }));
  step(!!till.tips && /TEST walk waiter/.test(till.tips) && /80/.test(till.tips), `till screen (${who}) lists the TEST waiter's 80 in tips${till.openForm ? ' with no till open' : ''}`, till);
  await tp.screenshot({ path: `${OUT}/w7-till-${who}.png`, fullPage: true });
  const tips = await api(`/api/staff/till/tips?location_id=${LOC}&from_ms=${Date.now() - 3 * 3600e3}`, { token: await staffToken() });
  step(tips.status === 200, 'API tips (owner staff token)', JSON.stringify(tips.body).slice(0, 400));
  await csp(tp, 'room-till');
  await tp.click('[data-act="back"]').catch(() => {});
}

try {
  if (PHASE === 'tips') {   // the owner in the room app: the till screen's tips, with no till open
    const { p: tp, answer: oa } = await roomIn(ctx, creds.OWNER_EMAIL, creds.OWNER_PASSWORD, null, 'room-owner');
    step(oa?.status === 200, 'the owner signs in to the room app', String(oa?.status));
    await tillTips(tp, 'owner');
    await end(b);
  }
  const { p, answer } = await roomIn(ctx, W.email, W.password, PHASE === 'claim' ? W.code : null);
  step(answer?.status === 200, PHASE === 'claim' ? 'the TEST waiter claims the invite in the room app' : 'the TEST waiter signs in', JSON.stringify(answer));
  const bar = await p.$$eval('.bar [data-act], .bar .chip', els => els.map(e => (e.dataset.act || 'chip') + ':' + e.textContent.trim()));
  step(bar.some(x => /^open:/.test(x)) && bar.some(x => /^floor:/.test(x)), 'waiter bar: Open a table, Floor', bar.join(' | '));
  let q = await queuedTag(p);
  step(q.hidden === true, 'header does not show a queue', JSON.stringify(q));
  await p.screenshot({ path: `${OUT}/w1-room-${PHASE}.png` });

  if (PHASE === 'claim') {
    await p.click('[data-act="floor"]'); await p.waitForTimeout(4000);
    const fl = await p.evaluate(() => ({ zones: [...document.querySelectorAll('.floor-zone')].map(z => ({ name: z.querySelector('h3')?.textContent, tables: [...z.querySelectorAll('g.ft')].map(g => g.className.baseVal.replace(/\s+/g, ' ') + ' | ' + g.getAttribute('aria-label')) })) }));
    step(fl.zones.some(z => z.name === 'TEST walk' && z.tables.length === 2), 'Floor draws the TEST room with 91 and 92', fl);
    await p.screenshot({ path: `${OUT}/w2-floor.png`, fullPage: true });
    const apiFl = await api(`/api/staff/floor?location_id=${LOC}`, { token: await tok() });
    step(apiFl.status === 200, 'API floor with the waiter token', JSON.stringify(apiFl.body).slice(0, 500));
    await csp(p, 'room-floor');
    await p.click('[data-act="back"]'); await p.waitForTimeout(800);
    // open table 91 with one dish
    let placed = null;
    p.on('response', async r => { if (/\/api\/public\/locations\/[^/]+\/orders$/.test(r.url()) && r.request().method() === 'POST') { try { placed = { status: r.status(), body: await r.json() }; } catch { placed = { status: r.status() }; } } });
    await p.click('[data-act="open"]'); await p.waitForTimeout(2500);
    const table = `${S.zone}:91`;
    await p.fill('input[data-in="table"]', table);
    const pick = await p.$('[data-act="pick"]:not([disabled])');
    step(!!pick, 'open-table menu lists dishes');
    await pick?.click(); await p.waitForTimeout(500);
    await p.screenshot({ path: `${OUT}/w3-open.png` });
    await p.click('[data-act="send"]');
    for (let i = 0; i < 30 && !placed; i++) await p.waitForTimeout(500);
    const o = placed?.body || {};
    step(placed?.status === 200 && o.fulfilment?.table === table, `Open a table places R1 on ${table}`, `${placed?.status} id=${o.id} status=${o.status} table=${o.fulfilment?.table} sitting=${o.sitting_id} placed_by=${o.placed_by}`);
    ssave({ r1: o.id, roomSitting: o.sitting_id });
    await p.waitForTimeout(3000);
    step(null, 'screen after placing', await text(p));
    await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(800);
    await p.click('[data-act="floor"]').catch(() => {}); await p.waitForTimeout(4000);
    const t91 = await p.$$eval('g.ft', els => els.map(g => g.className.baseVal.replace(/\s+/g, ' ') + ' | ' + g.getAttribute('aria-label')).filter(x => /91/.test(x)));
    step(t91.some(x => !/fs-free/.test(x)), 'Floor: table 91 is taken now', t91.join(' / '));
    await p.screenshot({ path: `${OUT}/w4-floor-91.png`, fullPage: true });
    const r = await roundOf(o.id);
    step(r?.status === 'PENDING', 'API room: R1 PENDING in its sitting', JSON.stringify(r && { status: r.status, table: r.sitting.table, total: r.total }));
  }

  if (PHASE === 'confirm') {
    const cards = await p.$$eval('[data-act="sit"]', els => els.map(e => ({ id: e.dataset.id, text: e.innerText.replace(/\s+/g, ' ') })));
    const mine = cards.find(c => c.id === S.roomSitting);
    step(!!mine && /2/.test(mine.text), 'room list shows table 91 with two rounds and a guest waiting', JSON.stringify(mine));
    await p.click(`[data-act="sit"][data-id="${S.roomSitting}"]`); await p.waitForTimeout(1200);
    const rounds = await p.$$eval('[data-act="round"]', els => els.map(e => e.dataset.id));
    step(rounds.includes(S.r1) && rounds.includes(S.r2), 'the sitting lists R1 and the guest\'s R2', rounds.join(','));
    await p.click(`[data-act="round"][data-id="${S.r2}"]`); await p.waitForTimeout(1200);
    const gbar = await p.$eval('.guest-round', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    step(!!gbar, 'guest round shows Confirm / Reject', gbar);
    await p.screenshot({ path: `${OUT}/w5-guest-round.png` });
    let ans = null;
    p.on('response', async r => { if (/\/guest$/.test(r.url())) ans = { status: r.status(), body: (await r.text().catch(() => '')).slice(0, 200) }; });
    await p.click('[data-act="guestConfirm"]');
    for (let i = 0; i < 20 && !ans; i++) await p.waitForTimeout(500);
    await p.waitForTimeout(2000);
    const r = await roundOf(S.r2);
    step(ans?.status === 200 && r?.status === 'CONFIRMED', 'Confirm lands: R2 CONFIRMED (API)', `${JSON.stringify(ans)} api=${r?.status}`);
    q = await queuedTag(p);
    step(q.hidden === true, 'header after the confirm: no queue', JSON.stringify(q));
  }

  if (PHASE === 'pay') {
    const paid = [];
    p.on('response', async r => { if (/\/api\/staff\/orders\/[^/]+\/pay$/.test(r.url())) { let body; try { body = await r.json(); } catch { body = null; } paid.push({ status: r.status(), payment: body?.payments?.slice?.(-1)?.[0] || body?.payment || null, paid: body?.payment_status }); } });
    for (const [id, tip] of [[S.r1, '50'], [S.r2, '30']]) {
      const before = await roundOf(id);
      if (before?.payment_status === 'paid') { step(null, `${id.slice(0, 8)} already paid by an earlier run`, JSON.stringify(before.payments || []).slice(0, 200)); continue; }
      await p.click(`[data-act="sit"][data-id="${S.roomSitting}"]`).catch(() => {}); await p.waitForTimeout(1200);
      await p.click(`[data-act="round"][data-id="${id}"]`).catch(() => {}); await p.waitForTimeout(1200);
      const payBtn = await p.$('[data-act="pay"]');
      step(!!payBtn, `${id.slice(0, 8)} offers Take payment`);
      await payBtn?.click(); await p.waitForTimeout(1200);
      await p.click('[data-act="method"][data-v="card"]').catch(e => step(false, 'card method button', e.message.slice(0, 80)));
      await p.waitForTimeout(400);
      await p.fill('input[data-in="tip"]', tip); await p.waitForTimeout(300);
      const amt = await p.inputValue('input[data-in="amount"]').catch(() => '');
      await p.screenshot({ path: `${OUT}/w6-pay-${id.slice(0, 8)}.png` });
      const n = paid.length;
      await p.click('form[data-form="pay"] button[type="submit"]');
      for (let i = 0; i < 30 && paid.length === n; i++) await p.waitForTimeout(500);
      await p.waitForTimeout(1500);
      const r = await roundOf(id);
      step(paid[n]?.status === 200 && r?.payment_status === 'paid', `card + tip ${tip} on ${id.slice(0, 8)} lands`, `amount=${amt} resp=${JSON.stringify(paid[n])} api=${r?.payment_status} ${JSON.stringify(r?.payments || []).slice(0, 200)}`);
      await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(600);
      await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(600);
    }
    // the till's tips, with no till open. The till is a capability (room/logic.js canTill); a waiter
    // preset without it gets no Till button -- then the OWNER's room session is the one that looks.
    const hasTill = !!(await p.$('[data-act="till"]'));
    step(null, 'waiter preset has a Till button', String(hasTill));
    const wt = await api(`/api/staff/till/tips?location_id=${LOC}&from_ms=${Date.now() - 3 * 3600e3}`, { token: await tok() });
    step(null, 'API tips with the WAITER token', `${wt.status} ${JSON.stringify(wt.body).slice(0, 200)}`);
    if (hasTill) await tillTips(p, 'waiter');
    else step(null, 'the till tips are checked by `node waiter.mjs tips` (the owner\'s room session)', 'a second browser context crashes the single-process Chromium on this box');
    await p.waitForTimeout(600);
    // the Floor: 91 is to clear once every round is paid and served (READY)
    let cleared = null;
    p.on('response', async r => { if (/\/floor\/[^/]+\/cleared$/.test(r.url())) cleared = { status: r.status(), body: (await r.text().catch(() => '')).slice(0, 200) }; });
    await p.click('[data-act="floor"]'); await p.waitForTimeout(4000);
    const g = await p.$(`g.ft[data-id="${S.roomSitting}"]`);
    const cls = await g?.evaluate(e => e.className.baseVal + ' | ' + e.getAttribute('aria-label'));
    step(!!g && /fs-dirty|tap/.test(cls || ''), 'Floor: table 91 is to-clear and tappable', cls || 'not tappable');
    // 91 and 92 overlap on this plan (the editor placed 92 over 91 and raised no issue): the
    // centre of 91 is under 92's circle, so the tap goes to 91's right edge.
    const bb = await g?.boundingBox();
    await p.locator(`g.ft[data-id="${S.roomSitting}"]`).click({ position: { x: Math.max(1, (bb?.width || 10) - 4), y: (bb?.height || 10) / 2 } }).catch(e => step(false, 'tap 91', e.message.slice(0, 160)));
    await p.waitForTimeout(800);
    const btn = await p.$(`[data-act="tableCleared"][data-id="${S.roomSitting}"]`);
    step(!!btn, 'tapping it offers Cleared');
    await p.screenshot({ path: `${OUT}/w8-clear.png`, fullPage: true });
    await btn?.click();
    for (let i = 0; i < 20 && !cleared; i++) await p.waitForTimeout(500);
    await p.waitForTimeout(2500);
    const f = await api(`/api/staff/floor?location_id=${LOC}`, { token: await tok() });
    const t91 = (f.body.zones || []).flatMap(z => z.tables || []).find(t => t.n === 91);
    step(cleared?.status === 200 && t91?.state === 'free', 'Cleared lands; API floor: 91 free', `${JSON.stringify(cleared)} t91=${JSON.stringify(t91)}`);
    await p.screenshot({ path: `${OUT}/w9-free.png`, fullPage: true });
    q = await queuedTag(p);
    step(q.hidden === true && !/0 /.test(q.hidden ? '' : q.text || ''), 'header at the end: no "0 queued"', JSON.stringify(q));
    await csp(p, 'room-floor');
  }
  await csp(p, `room-${PHASE}`);
} catch (e) {
  step(false, `phase ${PHASE} threw`, e.stack?.slice(0, 400));
}
await end(b);
