// E: storefront -- place TEST order C1 as a guest (UI), then replay its body
// with two more TEST phones via the same public route (API).
import { HOST, OUT, say, watch, csp, browser, api, save, st, finish } from './_lib.mjs';
import { devices } from 'playwright';
const b = await browser();
const made = [];
try {
  const ctx = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
  const c = await ctx.newPage(); watch(c, 'store');
  let placed = null, reqBody = null, reqHeaders = null;
  c.on('request', r => { if (/\/orders$/.test(r.url()) && r.method() === 'POST') { reqBody = r.postData(); reqHeaders = r.headers(); } });
  c.on('response', async r => { if (/\/orders$/.test(r.url()) && r.request().method() === 'POST') { try { placed = { status: r.status(), body: await r.json() }; } catch { placed = { status: r.status() }; } } });
  await c.goto(`${HOST}/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await c.waitForSelector('.card', { timeout: 60000 });
  const dismiss = async () => { for (let i = 0; i < 6; i++) { const n = await c.evaluate(() => document.getElementById('sheet')?.dataset.name || ''); if (!n) return; const l = await c.$('#insLater'); if (l) await l.click().catch(() => {}); else await c.evaluate(() => document.getElementById('scrim')?.click()); await c.waitForTimeout(600); } };
  await c.waitForTimeout(3500); await dismiss();
  await (await c.$('.card')).click(); await c.waitForTimeout(1200);
  await c.click('#dadd').catch(e => say(false, 'dish add', e.message.slice(0, 80)));
  await c.waitForTimeout(700); await c.keyboard.press('Escape'); await c.waitForTimeout(500);
  await c.click('#cartPill').catch(async () => { await dismiss(); await c.click('#cartPill'); });
  await c.waitForTimeout(900); await c.click('#toCheckout'); await c.waitForTimeout(1500);
  for (const [sel, val] of [['#f-street', 'Rruga Taulantia'], ['#f-house', '12'], ['#f-name', 'TEST walk C1'], ['#f-phone', '+355 69 000 0011'], ['#f-note', 'TEST - mos e pergatit, prove']]) {
    const el = await c.$(sel); if (el) await el.fill(val); else say(false, `checkout field ${sel}`, 'missing');
  }
  const cash = await c.$('[data-pay="cash"], #pays [data-p="cash"]'); if (cash) await cash.click();
  await c.waitForTimeout(400);
  await c.click('#place');
  for (let i = 0; i < 40 && !placed; i++) await c.waitForTimeout(500);
  say(placed?.status === 200, 'storefront TEST order C1 placed', `${placed?.status} ${placed?.body?.id} ${placed?.body?.status}`);
  if (placed?.body?.id) made.push({ id: placed.body.id, key: placed.body.access_token, phone: '+355 69 000 0011' });
  await c.screenshot({ path: `${OUT}/e1-placed.png` });
  await csp(c, 'store');
  // replay
  if (reqBody) {
    for (const [phone, name] of [['069 000 0011', 'TEST walk C2'], ['+355 69 000 0012', 'TEST walk C3']]) {
      const body = JSON.parse(reqBody);
      body.contact = { ...(body.contact || {}), name, phone };
      for (const k of ['idempotency_key', 'idempotencyKey', 'request_id', 'requestId', 'client_id']) if (k in body) body[k] = `${body[k]}-${phone.replace(/\D/g, '')}`;
      const h = { 'idempotency-key': crypto.randomUUID() };
      const r = await api(`/api/public/locations/sushi-durres/orders`, { method: 'POST', body, headers: h });
      say(r.status === 200, `API TEST order ${name}`, `${r.status} ${r.body?.id || JSON.stringify(r.body).slice(0, 200)}`);
      if (r.body?.id) made.push({ id: r.body.id, key: r.body.access_token, phone });
    }
    say(null, 'placement body keys', Object.keys(JSON.parse(reqBody)).join(','));
  }
} finally { save({ storeOrders: made }); await b.close(); finish(); }
