// C: room -- the paid READY table shows "to clear"; press Cleared; see it free. Then the till's tips.
import { HOST, OUT, creds, say, watch, csp, browser, api, staffToken, st, finish } from './_lib.mjs';
const S = st();
const b = await browser();
try {
  const ctx = await b.newContext({ viewport: { width: 400, height: 860 }, serviceWorkers: 'block' });
  const p = await ctx.newPage(); watch(p, 'room');
  let cleared = null;
  p.on('response', async r => { if (/\/floor\/[^/]+\/cleared$/.test(r.url())) { try { cleared = { status: r.status(), body: await r.json() }; } catch { cleared = { status: r.status() }; } } });
  await p.goto(`${HOST}/room/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForSelector('form[data-form="login"]', { timeout: 40000 });
  await p.fill('input[name="email"]', creds.OWNER_EMAIL); await p.fill('input[name="password"]', creds.OWNER_PASSWORD);
  await p.click('form[data-form="login"] button[type="submit"]');
  await p.waitForSelector('[data-act="floor"]', { timeout: 40000 }); await p.waitForTimeout(1500);
  await p.click('[data-act="floor"]'); await p.waitForTimeout(3500);
  const cards = async () => p.$$eval('.tables .card', els => els.map(e => ({ cls: e.className, text: e.innerText.replace(/\s+/g, ' '), id: e.dataset.id || null, disabled: e.disabled })));
  const c0 = await cards();
  say(c0.some(c => c.id === S.roomSitting && /fs-dirty/.test(c.cls)), 'floor shows the paid READY table as to-clear', JSON.stringify(c0));
  await p.screenshot({ path: `${OUT}/c1-dirty.png` });
  const card = await p.$(`[data-act="pickTable"][data-id="${S.roomSitting}"]`);
  if (card) { await card.click(); await p.waitForTimeout(800); }
  const btn = await p.$(`[data-act="tableCleared"][data-id="${S.roomSitting}"]`);
  say(!!btn, 'tapping it offers Cleared', await p.$eval('.floor-clear', e => e.innerText.replace(/\s+/g, ' ')).catch(() => ''));
  await p.screenshot({ path: `${OUT}/c2-pick.png` });
  await btn?.click();
  for (let i = 0; i < 20 && !cleared; i++) await p.waitForTimeout(500);
  say(cleared?.status === 200, 'Cleared lands', JSON.stringify(cleared));
  await p.waitForTimeout(2500);
  const c1 = await cards();
  say(!c1.some(c => c.id === S.roomSitting), 'the table is free (gone from the off-plan list)', JSON.stringify(c1));
  const toast = await p.$eval('#toast', e => e.hidden ? '' : e.textContent).catch(() => '');
  say(null, 'toast', toast);
  await p.screenshot({ path: `${OUT}/c3-free.png` });
  await csp(p, 'room-floor');
  const tok = await staffToken();
  const f = await api(`/api/staff/floor?location_id=sushi-durres`, { token: tok });
  say(!(f.body.unplaced || []).some(u => u.sitting_id === S.roomSitting), 'API floor: table free', JSON.stringify(f.body.unplaced));
  // TILL + tips
  await p.click('[data-act="back"]'); await p.waitForTimeout(800);
  await p.click('[data-act="till"]'); await p.waitForTimeout(3000);
  const till = await p.evaluate(() => ({ text: document.querySelector('#app').innerText.replace(/\s+/g, ' ').slice(0, 700), tips: !!document.querySelector('[data-tips]') }));
  say(till.tips, 'till screen has a tips-per-person section', JSON.stringify(till));
  await p.screenshot({ path: `${OUT}/c4-till.png`, fullPage: true });
  await csp(p, 'room-till');
  const from = Date.now() - 3 * 3600e3;
  const tips = await api(`/api/staff/till/tips?location_id=sushi-durres&from_ms=${from}`, { token: tok });
  say(null, 'API tips (last 3 h)', JSON.stringify(tips).slice(0, 500));
} finally { await b.close(); finish(); }
