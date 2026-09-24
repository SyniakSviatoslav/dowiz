// A: room app -- sign in, Floor, Open a table (TEST), pay by card with a tip.
import { HOST, OUT, creds, say, watch, csp, browser, api, staffToken, save, st, finish } from './_lib.mjs';
const TABLE = process.env.TABLE || 'TEST-W1';
const ITEM = process.env.ITEM || 'item-78';
const TIP = process.env.TIP || '50';
const b = await browser();
try {
  const ctx = await b.newContext({ viewport: { width: 400, height: 860 }, serviceWorkers: 'block' });
  const p = await ctx.newPage(); watch(p, 'room');
  let placed = null, paid = null;
  p.on('response', async r => {
    if (/\/api\/public\/locations\/[^/]+\/orders$/.test(r.url()) && r.request().method() === 'POST') { try { placed = { status: r.status(), body: await r.json() }; } catch {} }
    if (/\/api\/staff\/orders\/[^/]+\/pay$/.test(r.url())) { try { paid = { status: r.status(), body: await r.json() }; } catch { paid = { status: r.status() }; } }
  });
  await p.goto(`${HOST}/room/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForSelector('form[data-form="login"]', { timeout: 40000 });
  await p.fill('input[name="email"]', creds.OWNER_EMAIL);
  await p.fill('input[name="password"]', creds.OWNER_PASSWORD);
  await p.click('form[data-form="login"] button[type="submit"]');
  await p.waitForSelector('[data-act="signout"]', { timeout: 40000 }).catch(() => {});
  await p.waitForTimeout(2500);
  const bar = await p.$$eval('.bar [data-act]', els => els.map(e => e.dataset.act + ':' + e.textContent.trim()));
  say(bar.length > 0, 'room signs in', bar.join(' | '));
  await csp(p, 'room-home');
  await p.screenshot({ path: `${OUT}/a1-room.png` });

  // FLOOR before
  await p.click('[data-act="floor"]');
  await p.waitForTimeout(3500);
  const fl0 = await p.evaluate(() => ({ text: document.querySelector('#app').innerText.slice(0, 600), zones: document.querySelectorAll('.floor-zone').length, unplaced: [...document.querySelectorAll('[data-act="pickTable"], .tables .card')].map(e => e.innerText.replace(/\s+/g, ' ')) }));
  say(null, 'floor before', JSON.stringify(fl0));
  await p.screenshot({ path: `${OUT}/a2-floor0.png` });
  await csp(p, 'room-floor');
  await p.click('[data-act="back"]'); await p.waitForTimeout(800);

  // OPEN A TABLE
  await p.click('[data-act="open"]'); await p.waitForTimeout(2000);
  await p.fill('input[data-in="table"]', TABLE);
  const pick = await p.$(`[data-act="pick"][data-id="${ITEM}"]`);
  say(!!pick, 'open-table picker lists the dish', ITEM);
  await pick?.click(); await p.waitForTimeout(500);
  await p.screenshot({ path: `${OUT}/a3-open.png` });
  await p.click('[data-act="send"]');
  for (let i = 0; i < 30 && !placed; i++) await p.waitForTimeout(500);
  say(placed?.status === 200 || placed?.status === 201, 'Open a table places a round', JSON.stringify(placed).slice(0, 300));
  await p.waitForTimeout(3000);
  const id = placed?.body?.id;
  save({ roomOrder: id, roomSitting: placed?.body?.sitting_id, roomTable: TABLE });
  const view = await p.evaluate(() => document.querySelector('#app').innerText.slice(0, 500));
  say(null, 'after placing, screen shows', view.replace(/\s+/g, ' ').slice(0, 300));
  await p.screenshot({ path: `${OUT}/a4-round.png` });

  // PAY with a tip
  let payBtn = await p.$('[data-act="pay"]');
  if (!payBtn) {
    // go to the round from the room list
    await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(1500);
    const card = await p.$(`[data-act="sit"]:has-text("${TABLE}")`);
    say(!!card, 'room list shows the new table', TABLE);
    await card?.click(); await p.waitForTimeout(1200);
    const r = await p.$('[data-act="round"]'); if (r) { await r.click(); await p.waitForTimeout(800); }
    payBtn = await p.$('[data-act="pay"]');
  }
  say(!!payBtn, 'round offers Take payment');
  await payBtn?.click(); await p.waitForTimeout(1200);
  await p.click('[data-act="method"][data-v="card"]').catch(e => say(false, 'card method button', e.message.slice(0, 80)));
  await p.waitForTimeout(400);
  await p.fill('input[data-in="tip"]', TIP);
  await p.waitForTimeout(300);
  const amt = await p.inputValue('input[data-in="amount"]');
  say(null, 'pay form amount prefilled', amt);
  await p.screenshot({ path: `${OUT}/a5-pay.png` });
  await p.click('form[data-form="pay"] button[type="submit"]');
  for (let i = 0; i < 30 && !paid; i++) await p.waitForTimeout(500);
  say(paid?.status === 200, 'payment with tip lands', JSON.stringify(paid).slice(0, 400));
  await p.waitForTimeout(2000);
  const note = await p.evaluate(() => document.querySelector('#app').innerText.slice(0, 300));
  say(null, 'pay screen after', note.replace(/\s+/g, ' '));
  await p.screenshot({ path: `${OUT}/a6-paid.png` });
  await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(600);
  await p.click('[data-act="back"]').catch(() => {}); await p.waitForTimeout(600);

  // FLOOR after pay (expect ordering: the round is still PENDING)
  await p.click('[data-act="floor"]').catch(() => {}); await p.waitForTimeout(3500);
  const fl1 = await p.evaluate(() => ({ zones: document.querySelectorAll('.floor-zone').length, unplaced: [...document.querySelectorAll('.tables .card')].map(e => e.className + ' ' + e.innerText.replace(/\s+/g, ' ')) }));
  say(null, 'floor after pay', JSON.stringify(fl1));
  await p.screenshot({ path: `${OUT}/a7-floor1.png` });
  await csp(p, 'room-floor-after');
  // read back
  const tok = await staffToken();
  const o = await api(`/api/staff/room?location_id=sushi-durres`, { token: tok });
  const sit = (o.body.sittings || []).find(s => s.sitting_id === placed?.body?.sitting_id);
  say(null, 'API room sitting', JSON.stringify(sit).slice(0, 900));
  const f = await api(`/api/staff/floor?location_id=sushi-durres`, { token: tok });
  say(null, 'API floor', JSON.stringify(f.body).slice(0, 600));
} finally { await b.close(); finish(); }
