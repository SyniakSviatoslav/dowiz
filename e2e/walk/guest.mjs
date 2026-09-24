// GUEST (storefront, phone viewport) -- the per-role live walk, one phase per run:
//   node guest.mjs browse      the menu in sq / en / uk (untranslated keys, dish names, nav)
//   node guest.mjs orders      three TEST orders: D delivery (for the courier), X delivery (the owner rejects),
//                              P pickup; the tracking sheet and the order read back with the guest's own token
//   node guest.mjs book        book a table without an account (B1 on table 92, B2 any table); the share link
//   node guest.mjs bookcancel  open B1's share link on ANOTHER phone, see it, cancel it
//   node guest.mjs qr          open table 91's QR link, place a round, see the table line and the sitting's bill
// PASS/FAIL/INFO per step with the API read-back; exit code = FAILs.
import { HOST, OUT, watch, csp, browser, api, step, end, sst, ssave, LOC, storeIn, dismiss } from './_lib.mjs';
import { devices } from 'playwright';

const PHASE = process.argv[2] || 'browse';
const S = sst();
const b = await browser();
const phone = { ...devices['iPhone 14 Pro'], serviceWorkers: 'block', reducedMotion: 'reduce' };
const orderOf = async (id, key) => api(`/api/order/${encodeURIComponent(id)}`, { headers: { authorization: 'Bearer ' + key } });

/// One order through the real checkout. Answers {status, body} of the POST.
async function checkout(c, { how = 'delivery', name, tel, note = 'TEST walk - mos e pergatit, prove' }) {
  let placed = null;
  const h = async r => { if (/\/orders$/.test(r.url()) && r.request().method() === 'POST') { try { placed = { status: r.status(), body: await r.json() }; } catch { placed = { status: r.status() }; } } };
  c.on('response', h);
  await c.evaluate(() => document.getElementById('scrim')?.click()).catch(() => {}); await c.waitForTimeout(500);
  await dismiss(c, []);
  await c.click('.card .card-hit, .card'); await c.waitForTimeout(1200);
  await c.click('#dadd').catch(e => step(false, 'dish sheet Add', e.message.slice(0, 80)));
  await c.waitForTimeout(700); await c.keyboard.press('Escape'); await c.waitForTimeout(500);
  await c.click('#cartPill').catch(async () => { await dismiss(c); await c.click('[data-tab="cart"]'); });
  await c.waitForTimeout(900);
  const cartText = await c.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
  await c.click('#toCheckout'); await c.waitForTimeout(1500);
  if (how === 'pickup') { await c.click('[data-how="pickup"]').catch(e => step(false, 'pickup switch', e.message.slice(0, 80))); await c.waitForTimeout(800); }
  const fields = how === 'delivery' ? [['#f-street', 'Rruga Taulantia'], ['#f-house', '12']] : [];
  for (const [sel, val] of [...fields, ['#f-name', name], ['#f-phone', tel], ['#f-note', note]]) {
    const el = await c.$(sel);
    if (el && await el.isVisible()) await el.fill(val); else if (sel !== '#f-note') step(false, `checkout field ${sel}`, 'missing');
  }
  const rails = await c.$$eval('[data-pay]', els => els.map(e => e.dataset.pay));
  const cash = await c.$('[data-pay="cash"]'); if (cash) await cash.click();
  await c.waitForTimeout(400);
  const checkoutText = await c.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
  await c.click('#place');
  for (let i = 0; i < 40 && !placed; i++) await c.waitForTimeout(500);
  c.off('response', h);
  if (!placed) step(false, 'no POST /orders seen', `f-err=${await c.$eval('#f-err', e => e.innerText).catch(() => '')} toast=${await c.$eval('.toast, #toast', e => e.textContent).catch(() => '')}`);
  return { placed, rails, cartText, checkoutText };
}

try {
  if (PHASE === 'browse') {
    const ctx = await b.newContext(phone);
    const c = await storeIn(ctx);
    for (const l of ['sq', 'en', 'uk']) {
      await c.click('#langBtn'); await c.waitForTimeout(900);
      await c.click(`[data-l="${l}"]`); await c.waitForTimeout(1800);
      await c.evaluate(() => document.getElementById('scrim')?.click()); await c.waitForTimeout(700);
      const r = await c.evaluate(() => {
        const bad = [...document.querySelectorAll('[data-t]')].filter(e => e.offsetParent !== null && (!e.textContent.trim() || e.textContent.trim() === e.dataset.t && e.dataset.t !== 'kcal')).map(e => e.dataset.t);
        return { lang: document.documentElement.lang, cards: document.querySelectorAll('.card').length, dish: [...document.querySelectorAll('.card-name')].slice(0, 3).map(e => e.textContent),
          nav: [...document.querySelectorAll('#nav .tab-l')].map(e => e.textContent), bad: [...new Set(bad)].slice(0, 12) };
      });
      step(r.lang === l && r.cards > 0 && r.bad.length === 0, `menu in ${l}`, r);
      await c.screenshot({ path: `${OUT}/g1-menu-${l}.png` });
      await csp(c, `store-${l}`);
    }
    // a dish sheet opens and closes
    await c.click('.card .card-hit'); await c.waitForTimeout(1200);
    const dish = await c.evaluate(() => ({ sheet: document.getElementById('sheet')?.dataset.name, add: !!document.getElementById('dadd'), text: document.getElementById('sheetIn')?.innerText.replace(/\s+/g, ' ').slice(0, 160) }));
    step(!!dish.add, 'dish sheet opens with Add', dish);
    await c.keyboard.press('Escape');
    // back to Albanian (the venue's own) for the next phases
    await c.click('#langBtn'); await c.waitForTimeout(700); await c.click('[data-l="sq"]'); await c.waitForTimeout(800);
    const menu = await api(`/api/public/locations/${LOC}/menu`);
    step(menu.status === 200, 'API menu', `${(menu.body.categories || []).length} categories, payments=${JSON.stringify(menu.body.location?.payments)}`);
  }

  if (PHASE === 'orders') {
    const ctx = await b.newContext(phone);
    const c = await storeIn(ctx);
    const made = { ...(S.made || {}) };
    const ONLY = (process.env.ONLY || 'D,X,P').split(',');
    for (const [k, how, name, tel] of [['D', 'delivery', 'TEST walk D', '+355 69 000 0021'], ['X', 'delivery', 'TEST walk X', '069 000 0021'], ['P', 'pickup', 'TEST walk P', '+355 69 000 0022']].filter(x => ONLY.includes(x[0]))) {
      const { placed, rails } = await checkout(c, { how, name, tel });
      const id = placed?.body?.id, key = placed?.body?.access_token;
      step(placed?.status === 200 && !!id, `TEST ${k} (${how}) placed from the checkout`, `${placed?.status} ${id} ${placed?.body?.status} rails=${rails}`);
      await c.waitForTimeout(2500);
      const tr = await c.evaluate(() => ({ sheet: document.getElementById('sheet')?.dataset.name, title: document.querySelector('.ep-title')?.innerText || '', text: document.getElementById('sheetIn')?.innerText.replace(/\s+/g, ' ').slice(0, 220) }));
      step(tr.sheet === 'track' && !!tr.title, `tracking sheet opens for ${k}`, tr);
      if (id) {
        const rb = await orderOf(id, key);
        step(rb.status === 200 && rb.body?.status === 'PENDING', `API read-back ${k} with the guest token`, `${rb.status} ${rb.body?.status} ${rb.body?.fulfilment?.kind} total=${rb.body?.total}`);
        made[k] = { id, key };
        ssave({ made });
      }
      await c.screenshot({ path: `${OUT}/g2-track-${k}.png` });
      await c.keyboard.press('Escape'); await c.waitForTimeout(800);
    }
    ssave({ orders: { D: made.D?.id, X: made.X?.id, P: made.P?.id }, orderKeys: { D: made.D?.key, X: made.X?.key, P: made.P?.key } });
    // the orders tab lists them
    await c.click('[data-tab="orders"]'); await c.waitForTimeout(2500);
    const hist = await c.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    const short = Object.entries(made).filter(([k]) => ONLY.includes(k)).map(([, m]) => m.id.slice(0, 8));   // this phone's own orders
    step(short.every(s => hist.includes(s)), 'Orders tab lists this phone\'s TEST orders', hist.slice(0, 300));
    await csp(c, 'store-orders');
  }

  if (PHASE === 'book') {
    const ctx = await b.newContext({ ...phone, permissions: ['clipboard-read', 'clipboard-write'] });
    const c = await storeIn(ctx);
    const posts = [];
    c.on('response', async r => { if (/\/reservations$/.test(r.url()) && r.request().method() === 'POST') posts.push({ status: r.status(), body: await r.text().catch(() => ''), auth: !!r.request().headers()['authorization'] }); });
    let prompted = null; c.on('dialog', async d => { prompted = d.defaultValue(); await d.accept(); });
    const openBook = async () => {
      await c.evaluate(() => document.getElementById('scrim')?.click()).catch(() => {}); await c.waitForTimeout(500);
      await c.click('[data-tab="book"]'); await c.waitForTimeout(1500);
      let times = await c.$$('[data-min]');
      if (!times.length) { const d = await c.$('[data-day]:not(.on)'); await d?.click(); await c.waitForTimeout(1200); times = await c.$$('[data-min]'); }
      const day = await c.$eval('.bk-day.on', e => +e.dataset.day).catch(() => 0);
      await times[0]?.click(); await c.waitForTimeout(3500);
      return day;
    };
    const day = await openBook();
    const plan = await c.evaluate(() => ({ tabs: [...document.querySelectorAll('.bk-tab')].map(e => e.textContent), tables: [...document.querySelectorAll('.bk-t')].map(e => e.getAttribute('aria-label')) }));
    step(plan.tabs.includes('TEST walk') && plan.tables.length === 2, 'booking page shows the TEST room and its two tables', plan);
    await c.screenshot({ path: `${OUT}/g3-book-plan.png` });
    await c.click('.bk-t[data-n="92"]'); await c.waitForTimeout(700);
    await c.fill('#bk-name', 'TEST walk B1'); await c.fill('#bk-phone', '+355 69 000 0031');
    const foot = await c.$eval('.bk-foot', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    await c.click('#bkGo'); await c.waitForTimeout(4500);
    const p1 = posts[0]; let d1 = {}; try { d1 = JSON.parse(p1?.body || '{}'); } catch {}
    step(p1?.status === 200 && !!d1.id && !p1.auth, 'B1 booked without an account (no bearer)', `${p1?.status} ${p1?.body?.replace(/"access_token":"[^"]+"/, '"access_token":"…"').slice(0, 220)} chosen="${foot}"`);
    const mine = await c.evaluate(() => ({ sheet: document.getElementById('sheet')?.dataset.name, text: document.getElementById('sheetIn')?.innerText.replace(/\s+/g, ' ').slice(0, 300) }));
    step(mine.sheet === 'bookMine' && /92/.test(mine.text), 'My bookings opens with B1 on table 92', mine);
    await c.click(`[data-share="${d1.id}"]`).catch(e => step(false, 'share button', e.message.slice(0, 80))); await c.waitForTimeout(1200);
    let link = prompted || await c.evaluate(() => navigator.clipboard.readText()).catch(() => '');
    step(/#rsv=/.test(link), 'Share gives a #rsv= link', link.replace(/t=[^&]+/, 't=…'));
    await c.screenshot({ path: `${OUT}/g4-mine.png` });
    const rb1 = await api(`/api/public/locations/${LOC}/reservations/${d1.id}`, { headers: { authorization: 'Bearer ' + d1.access_token } });
    step(rb1.status === 200 && rb1.body.status === 'REQUESTED' && rb1.body.tableN === 92, 'API read-back B1 with its own token', `${rb1.status} ${JSON.stringify(rb1.body).slice(0, 250)}`);
    // B2: any table
    await openBook();
    await c.fill('#bk-name', 'TEST walk B2'); await c.fill('#bk-phone', '+355 69 000 0032');
    await c.click('#bkGo'); await c.waitForTimeout(4500);
    const p2 = posts[1]; let d2 = {}; try { d2 = JSON.parse(p2?.body || '{}'); } catch {}
    step(p2?.status === 200 && !!d2.id, 'B2 (any table) booked', `${p2?.status} ${p2?.body?.replace(/"access_token":"[^"]+"/, '"access_token":"…"').slice(0, 200)}`);
    const n = await c.$$eval('.bk-mine', els => els.length).catch(() => 0);
    step(n >= 2, 'My bookings lists both', `${n} cards`);
    ssave({ bookDay: day, b1: { id: d1.id, t: d1.access_token, slotMin: rb1.body?.slotMin, link }, b2: { id: d2.id, t: d2.access_token } });
    await csp(c, 'store-book');
  }

  if (PHASE === 'bookcancel') {
    const ctx = await b.newContext(phone);   // another phone: nothing in its storage
    const url = S.b1.link.replace(/^https?:\/\/[^/]+/, '');
    const c = await storeIn(ctx, url, 'store-link');
    await c.waitForTimeout(2500);
    const mine = await c.evaluate(() => ({ sheet: document.getElementById('sheet')?.dataset.name, text: document.getElementById('sheetIn')?.innerText.replace(/\s+/g, ' ').slice(0, 300), hash: location.hash }));
    step(mine.sheet === 'bookMine' && /92/.test(mine.text) && !mine.hash, 'the share link opens My bookings on another phone (token cleared from the bar)', mine);
    const cancel = await c.$(`[data-cancel="${S.b1.id}"]`);
    step(!!cancel, 'B1 (CONFIRMED by the venue) offers Cancel to the guest');
    await cancel?.click(); await c.waitForTimeout(500);
    const armed = await c.$eval(`[data-cancel="${S.b1.id}"]`, e => e.innerText).catch(() => '');
    await c.click(`[data-cancel="${S.b1.id}"]`).catch(() => {}); await c.waitForTimeout(4000);
    const after = await c.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    const rb = await api(`/api/public/locations/${LOC}/reservations/${S.b1.id}`, { headers: { authorization: 'Bearer ' + S.b1.t } });
    step(rb.body?.status === 'CANCELLED_BY_GUEST', 'guest cancel -> CANCELLED_BY_GUEST (API)', `armed="${armed}" screen="${after.slice(0, 160)}" api=${rb.status} ${rb.body?.status}`);
    const rb2 = await api(`/api/public/locations/${LOC}/reservations/${S.b2.id}`, { headers: { authorization: 'Bearer ' + S.b2.t } });
    step(null, 'B2 as the guest reads it', `${rb2.status} ${rb2.body?.status} ${rb2.body?.reason || ''}`);
    await c.screenshot({ path: `${OUT}/g5-cancelled.png` });
    await csp(c, 'store-link');
  }

  if (PHASE === 'qr') {
    const ctx = await b.newContext(phone);
    const url = S.qr91.replace(/^https?:\/\/[^/]+/, '');
    const c = await storeIn(ctx, url, 'store-qr');
    const { placed, rails, cartText, checkoutText } = await checkout(c, { how: 'table', name: 'TEST walk QR', tel: '+355 69 000 0041', note: 'TEST walk QR round' });
    step(/TEST walk|91/.test(checkoutText) && /salla-1 91|91/.test(checkoutText), 'checkout shows the table line', checkoutText.slice(0, 260));
    step(rails.join() === 'cash', 'a table round offers only pay-at-the-table', rails.join());
    const o = placed?.body || {};
    step(placed?.status === 200 && o.placed_by === 'guest' && o.status === 'PENDING' && o.sitting_id === S.roomSitting, 'QR round lands PENDING, placed by the guest, in the waiter\'s sitting', `${placed?.status} id=${o.id} placed_by=${o.placed_by} sitting=${o.sitting_id} (waiter's ${S.roomSitting}) table=${o.fulfilment?.table}`);
    ssave({ r2: o.id, r2key: o.access_token });
    await c.waitForTimeout(6000);
    const bill = await c.evaluate(() => ({ sheet: document.getElementById('sheet')?.dataset.name, bill: document.getElementById('tableBill')?.innerText.replace(/\s+/g, ' ') || '(no #tableBill)' }));
    step(/#1/.test(bill.bill) && /#2/.test(bill.bill), 'tracking sheet shows the sitting bill with both rounds', bill);
    await c.screenshot({ path: `${OUT}/g6-qr-bill.png`, fullPage: true });
    if (o.id) {
      const sb = await api(`/api/order/${o.id}/sitting`, { headers: { authorization: 'Bearer ' + o.access_token } });
      step(sb.status === 200 && (sb.body.rounds || []).length === 2, 'API sitting bill with the guest token', `${sb.status} ${JSON.stringify(sb.body).slice(0, 400)}`);
      const leak = JSON.stringify(sb.body);
      step(!/TEST walk|\+355|"phone"|"contact"|"contactName"/.test(leak), 'the bill carries no guest name or phone', leak.match(/TEST walk|\+355|"phone"|"contact"|"contactName"/)?.[0] || 'none of: TEST walk, +355, phone, contact');   // an item's "name" is the dish
    }
    await csp(c, 'store-qr');
  }
} catch (e) {
  step(false, `phase ${PHASE} threw`, e.stack?.slice(0, 400));
}
await end(b);
