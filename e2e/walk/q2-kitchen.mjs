// Q2 KITCHEN on the QA hub -- the kitchen member (invited by q1-owner.mjs) signs in to /admin, sees only
// its tabs, and works a live storefront order: seen, New -> Preparing -> Ready; rejects a second one with a
// reason; 86es a dish and the storefront shows it sold out. No customer name, phone or street anywhere on
// the kitchen's screens, its API reads or its socket frames.
//   HOST=https://qa-durres.dowiz.org LOC=qa-durres OUT=<dir> slot.sh qa node e2e/walk/q2-kitchen.mjs
import { qaOwnerToken, storeReady } from './_qa.mjs';
import { HOST, OUT, LOC, browser, watch, csp, issues, step, end, api, sst, ssave, storeIn, dismiss, ownerToken } from './_lib.mjs';

if (!/qa-durres/.test(HOST)) { console.log('Q2 writes: QA hub only'); process.exit(2); }
const S = sst();
const K = S.qaKitchen;
if (!K?.email) { console.log('no QA kitchen in suite state: run q1-owner.mjs phone first'); process.exit(2); }
const shot = n => `${OUT}/q2-${n}.png`;
// The customer's words, which must never reach the kitchen.
const GUEST = { name: 'QA Guest Arbenita', phone: '+355 69 000 0099', street: 'Rruga QAtestore', house: '77' };
const PII = [GUEST.name, 'Arbenita', '000 0099', '0000099', 'QAtestore'];
const leaks = txt => PII.filter(w => String(txt).includes(w));
const UA = 'Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36';
const phone = { viewport: { width: 390, height: 844 }, serviceWorkers: 'block', userAgent: UA };
const ownerOrder = async (tok, id) => ((await api(`/api/owner/orders?location_id=${LOC}`, { token: tok })).body.orders || []).find(o => o.id === id);
const since = n => issues.slice(n).filter(i => !/http 503/.test(i));
/// The platform answers 503 "exceeded resource limits" to a share of requests (Free-plan CPU cap, worst on
/// the password hash of a login). On the QA hub a repeated write is harmless, so a step is re-asked.
async function again(fn, n = 6){ let r; for (let i = 0; i < n; i++) { r = await fn(); if (r.status !== 503) return r; } return r; }

let b = null;
try {
  // ── the invite becomes a member (the room app's claim route; kitchen.mjs walks that screen) ──
  if (!S.qaKitchenClaimed) {
    // A claim that answered 503 "exceeded resource limits" may or may not have landed: try again, and
    // judge by whether the member can sign in.
    for (let i = 0; i < 3; i++) {
      const c = await api('/api/staff/claim', { method: 'POST', body: { email: K.email, code: K.code, password: K.password } });
      step(null, `claim try ${i + 1}`, `${c.status} ${JSON.stringify(c.body).replace(/"(jwt|token)":"[^"]+"/g, '"$1":"..."').slice(0, 120)}`);
      if (c.status !== 503) break;
    }
    const l = await api('/api/staff/login', { method: 'POST', body: { email: K.email, password: K.password } });
    step(l.status === 200, 'the QA kitchen claimed its invite and can sign in', `${l.status} role=${l.body?.staff?.role || ''}`);
    if (l.status === 200) ssave({ qaKitchenClaimed: true });
  }
  let own = null; for (let i = 0; i < 4 && !own; i++) own = await ownerToken();
  b = await browser();

  // ── a live order through the storefront's own checkout ──
  // ONE context for every page: a second context in --single-process mode ends the browser.
  const cctx = await b.newContext(phone);
  const { c, taps } = await storeReady(cctx, 'q2-store');
  if (taps) step(null, `the storefront showed "menu did not load" (platform 503) and needed ${taps} retry tap(s)`);
  let placed = null, reqBody = null;
  c.on('request', r => { if (/\/orders$/.test(r.url()) && r.method() === 'POST') reqBody = r.postData(); });
  c.on('response', async r => { if (/\/orders$/.test(r.url()) && r.request().method() === 'POST') { try { placed = { status: r.status(), body: await r.json() }; } catch { placed = { status: r.status() }; } } });
  await c.click('.card[data-p="qa-salmon-roll"] .card-hit'); await c.waitForTimeout(1200);
  await c.click('#dadd'); await c.waitForTimeout(700); await c.keyboard.press('Escape'); await c.waitForTimeout(500);
  await c.click('#cartPill').catch(async () => { await dismiss(c); await c.click('[data-tab="cart"]'); });
  await c.waitForTimeout(900); await c.click('#toCheckout'); await c.waitForTimeout(1500);
  for (const [sel, val] of [['#f-street', GUEST.street], ['#f-house', GUEST.house], ['#f-name', GUEST.name], ['#f-phone', GUEST.phone], ['#f-note', 'QA walk: extra ginger']]) {
    const el = await c.$(sel); if (el && await el.isVisible()) await el.fill(val); else if (sel !== '#f-note') step(false, `checkout field ${sel}`, 'missing');
  }
  const cash = await c.$('[data-pay="cash"]'); if (cash) await cash.click();
  await c.waitForTimeout(400); await c.click('#place');
  for (let i = 0; i < 40 && !placed; i++) await c.waitForTimeout(500);
  await c.screenshot({ path: shot('0-store-placed') });
  const O1 = placed?.body?.id;
  step(placed?.status === 200 && !!O1, 'storefront order placed on the QA hub (delivery, cash)', `${placed?.status} ${O1} ${placed?.body?.status}`);
  await csp(c, 'q2-store');
  // A second order for the reject: the same body, a new idempotency key.
  let O2 = null;
  if (reqBody) {
    const body = JSON.parse(reqBody);
    for (const k of ['idempotency_key', 'idempotencyKey', 'request_id', 'requestId', 'client_id']) if (k in body) body[k] = `${body[k]}-2`;
    const r = await again(() => api(`/api/public/locations/${LOC}/orders`, { method: 'POST', body, headers: { 'idempotency-key': crypto.randomUUID() } }));
    O2 = r.body?.id;
    step(r.status === 200 && !!O2, 'a second order (for the reject)', `${r.status} ${O2 || JSON.stringify(r.body).slice(0, 160)}`);
  }
  ssave({ q2: { O1, O2 }, orderBody: reqBody });
  // Nothing is closed until the end: with --single-process, closing the last page ends the browser.


  // ── the kitchen signs in to the hub ──
  const kctx = cctx;
  const k = await kctx.newPage(); watch(k, 'q2-kitchen');
  const frames = [];
  k.on('websocket', ws => ws.on('framereceived', f => frames.push(String(f.payload).slice(0, 4000))));
  const reads = [];
  k.on('response', async r => { if (/\/api\//.test(r.url()) && r.request().method() === 'GET') { try { reads.push(`${r.url()} ${await r.text()}`); } catch {} } });
  let n0 = issues.length;
  await k.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await k.waitForSelector('#e', { timeout: 40000 });
  await k.fill('#e', K.email); await k.fill('#p', K.password); await k.click('#go');
  const inOk = await k.waitForSelector('#nav:not([hidden])', { timeout: 30000 }).then(() => true).catch(() => false);
  await k.waitForTimeout(3500);
  const loginErr = await k.$eval('#app .ui-alert, #app [role="alert"]', e => e.innerText.trim()).catch(() => '');
  step(inOk, 'the kitchen signs in through the /admin form', inOk ? 'landed' : `stayed on the form: "${loginErr}"`);
  if (!inOk) {
    // DEFECT (fixed in admin/signin.js, app.js line handed back): the form reads the owner door's plain-text
    // refusal with r.json() and never asks the staff door. To walk the rest of the kitchen's day, the session
    // the fixed form would store is stored by hand: the staff door's own token, from the same credentials.
    await k.screenshot({ path: shot('1-login-refused') });
    const sl = await again(() => api('/api/staff/login', { method: 'POST', body: { email: K.email, password: K.password } }));
    await k.evaluate(([tk, loc]) => { sessionStorage.setItem('dw_at', tk); localStorage.setItem('dw_loc', loc); }, [sl.body.jwt, sl.body.staff.locationId]);
    await k.reload({ waitUntil: 'domcontentloaded' });
    await k.waitForSelector('#nav:not([hidden])', { timeout: 30000 }).catch(() => {});
    await k.waitForTimeout(3500);
    n0 = issues.length;
  }
  const tabs = await k.$$eval('#nav [data-tab]', els => els.map(e => e.dataset.tab));
  step(tabs.includes('kitchen') && !tabs.some(t => ['orders', 'couriers', 'more'].includes(t)), 'the kitchen console shows only its tabs', `tabs=${tabs.join(',')}`);
  const bad401 = issues.slice(n0).filter(i => /http 40[13]/.test(i));
  step(bad401.length === 0, 'the kitchen console makes no request it is refused', bad401.join(' | '));
  await k.click('#nav [data-tab="kitchen"]').catch(() => {}); await k.waitForTimeout(3000);
  await k.screenshot({ path: shot('1-board'), fullPage: true });
  const onBoard = async id => k.$(`[data-o="${id}"]`).then(Boolean);
  step(await onBoard(O1), 'the board shows the storefront order', `${O1}`);

  // ── seen ──
  await k.click(`[data-seen="${O1}"]`).catch(e => step(false, 'seen tap', e.message.slice(0, 80)));
  await k.waitForTimeout(2500);
  let o = await ownerOrder(own, O1);
  step(!!o?.kitchen?.seen || !!o?.kitchen?.seenAtMs || !!o?.kitchen?.seen_at_ms, '"seen" tap: the owner reads the order as seen by the kitchen', JSON.stringify(o?.kitchen || null));

  // ── bump New -> Preparing -> Ready ──
  for (const want of ['CONFIRMED', 'PREPARING', 'READY']) {
    n0 = issues.length;
    await k.click(`[data-bump="${O1}"]`).catch(e => step(false, `bump to ${want}`, e.message.slice(0, 80)));
    await k.waitForTimeout(3000);
    o = await ownerOrder(own, O1);
    step(o?.status === want, `bump: the order is ${want} (owner read-back)`, `${o?.status} ${since(n0).join(' | ')}`);
    await k.screenshot({ path: shot(`2-bump-${want.toLowerCase()}`), fullPage: true });
  }
  const readyCol = await k.$eval(`.kds-col-ready [data-o="${O1}"]`, () => true).catch(() => false);
  step(readyCol, 'the ticket stands in the Ready column', '');

  // ── reject O2 with a reason ──
  if (O2) {
    await k.click(`[data-stop="${O2}"]`).catch(e => step(false, 'reject button', e.message.slice(0, 80)));
    await k.waitForTimeout(1200);
    await k.fill('#kdsWhy', 'QA walk: out of salmon');
    await k.screenshot({ path: shot('3-reject-reason') });
    await k.click('[data-ui-close="yes"]'); await k.waitForTimeout(3000);
    o = await ownerOrder(own, O2);
    step(o?.status === 'REJECTED' && o?.rejection_reason === 'QA walk: out of salmon', 'reject with a reason: REJECTED, and the reason is stored', `${o?.status} rejection_reason=${JSON.stringify(o?.rejection_reason)}`);
  }

  // ── 86 a dish ──
  await k.click('#nav [data-tab="kitchen"]'); await k.waitForTimeout(2000);
  n0 = issues.length;
  await k.click('[data-off="qa-veg-roll"]').catch(e => step(false, '86 button', e.message.slice(0, 80)));
  await k.waitForTimeout(3000);
  await k.screenshot({ path: shot('4-86'), fullPage: true });
  const pub = await again(() => api(`/api/public/locations/${LOC}/menu?fresh=1`));
  const veg = (pub.body.categories || []).flatMap(x => x.products).find(p => p.id === 'qa-veg-roll');
  step(veg && veg.available === false, '86: the hub holds the dish off sale', `${JSON.stringify(veg && { available: veg.available })} ${since(n0).join(' | ')}`);
  const backOn = await k.$('[data-on="qa-veg-roll"]').then(Boolean);
  step(backOn, '86: the stop list redraws with "back on" for the dish', backOn ? '' : 'the row still offers 86: the console re-read the menu from the BROWSER cache (fresh=1 answered max-age=30); fixed in storefront.rs menu_cache_control');
  const sctx = cctx;
  // A customer's menu is edge-cached for thirty seconds by design: look after the window.
  await k.waitForTimeout(35000);
  const { c: s2 } = await storeReady(sctx, 'q2-store-86');
  await s2.waitForTimeout(2000);
  const card = await s2.$eval('.card[data-p="qa-veg-roll"]', e => ({ cls: e.className, txt: e.innerText.replace(/\s+/g, ' ') })).catch(() => null);
  await s2.screenshot({ path: shot('5-store-86'), fullPage: true });
  step(!card || /sold-out/.test(card.cls), '86: the storefront shows the dish sold out (or not at all)', JSON.stringify(card));

  await k.bringToFront();
  if (await k.$('[data-on="qa-veg-roll"]')) { await k.click('[data-on="qa-veg-roll"]'); await k.waitForTimeout(2500); }
  else { const ot = await qaOwnerToken(); await api('/api/owner/products/qa-veg-roll', { method: 'POST', token: ot, body: { location_id: LOC, available: true } }); step(null, '86 undone through the API (the screen offered no back-on)'); }
  const pub2 = await again(() => api(`/api/public/locations/${LOC}/menu?fresh=1`));
  step((pub2.body.categories || []).flatMap(x => x.products).find(p => p.id === 'qa-veg-roll')?.available === true, '86 undone: the dish is back on sale', '');

  // ── no customer data anywhere the kitchen can see ──
  const screenTxt = await k.evaluate(() => document.body.innerText);
  step(leaks(screenTxt).length === 0, 'no customer name/phone/street on the kitchen screen', leaks(screenTxt).join(','));
  const readLeaks = reads.filter(r => leaks(r).length);
  step(readLeaks.length === 0, `no customer data in the kitchen's ${reads.length} API reads`, readLeaks.map(r => r.slice(0, 160)).join(' | '));
  const frameLeaks = frames.filter(f => leaks(f).length);
  step(frameLeaks.length === 0, `no customer data in the kitchen's ${frames.length} socket frames`, frameLeaks.map(f => f.slice(0, 200)).join(' | '));
  step(null, 'socket frames seen', frames.slice(0, 3).map(f => f.slice(0, 160)).join(' | '));
  await csp(k, 'q2-kitchen');
} catch (e) {
  step(false, 'Q2 threw', e.stack?.slice(0, 400));
}
await end(b);
