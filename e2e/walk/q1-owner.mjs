// Q1 OWNER on the QA hub -- every console tab and every More tile opens with no console error and no
// failed request; a menu edit round trip; a kitchen member invited through the Staff sheet.
//   HOST=https://qa-durres.dowiz.org LOC=qa-durres OUT=<dir> slot.sh qa node e2e/walk/q1-owner.mjs phone|desktop
// One browser, closed at the end. PASS/FAIL/INFO per step; exit code = FAILs.
import { HOST, OUT, LOC, browser, consoleIn, csp, issues, step, end, api, ssave, creds } from './_lib.mjs';

const VIEW = process.argv[2] || 'phone';
const vp = VIEW === 'desktop' ? { width: 1366, height: 900 } : { width: 390, height: 844 };
if (!/qa-durres/.test(HOST)) { console.log('Q1 writes: run it on the QA hub only (HOST=https://qa-durres.dowiz.org)'); process.exit(2); }
const shot = n => `${OUT}/q1-${VIEW}-${n}.png`;
/// The issues a step added, so each tab is judged by its own errors.
const since = n => issues.slice(n).filter(i => !/http 503/.test(i));

let b = null;
try {
  b = await browser();
  const ctx = await b.newContext({ viewport: vp, serviceWorkers: 'block', userAgent: 'Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Mobile Safari/537.36' });
  let n0 = issues.length;
  const o = await consoleIn(ctx, `q1-${VIEW}`);
  const loc = await o.evaluate(() => localStorage.getItem('dw_loc'));
  const venue = await o.$eval('#top', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
  step(loc === LOC, 'owner signs in on the QA host and the console opens THIS venue', `dw_loc=${loc} header="${venue.slice(0, 80)}"`);
  await o.screenshot({ path: shot('0-login') });
  step(since(n0).length === 0, 'sign-in: no console error, no failed request', since(n0).join(' | '));

  // ── every tab ──
  const tabs = await o.$$eval('#nav [data-tab]', els => els.map(e => e.dataset.tab));
  step(tabs.length >= 5, 'the owner sees the full bar', tabs.join(','));
  // ONLY_EDIT=1 skips the tab/tile sweep (re-running the round trip alone).
  for (const tab of process.env.ONLY_EDIT ? [] : tabs) {
    n0 = issues.length;
    await o.click(`#nav [data-tab="${tab}"]`); await o.waitForTimeout(3000);
    const txt = await o.$eval('#app', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    await o.screenshot({ path: shot(`tab-${tab}`) });
    await csp(o, `tab-${tab}`);
    const bad = since(n0);
    step(bad.length === 0 && txt.length > 20 && !/HTTP [45]\d\d|undefined|NaN|\[object/.test(txt), `tab ${tab} opens clean`, bad.length ? bad.join(' | ') : txt.slice(0, 140));
  }

  // ── every More tile ──
  await o.click('#nav [data-tab="more"]'); await o.waitForTimeout(2000);
  const keys = await o.$$eval('[data-open]', els => els.map(e => e.dataset.open));
  step(null, `${keys.length} More tiles`, keys.join(','));
  for (const key of process.env.ONLY_EDIT ? [] : keys) {
    n0 = issues.length;
    await o.click('#nav [data-tab="more"]').catch(() => {}); await o.waitForTimeout(1200);
    await o.click(`[data-open="${key}"]`).catch(e => issues.push(`tile ${key} click ${e.message.slice(0, 80)}`));
    await o.waitForTimeout(3000);
    const txt = await o.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    await o.screenshot({ path: shot(`tile-${key}`) });
    await csp(o, `tile-${key}`);
    const bad = since(n0);
    step(bad.length === 0 && txt.length > 10 && !/HTTP [45]\d\d|undefined|NaN|\[object/.test(txt), `tile ${key} opens clean`, bad.length ? bad.join(' | ') : txt.slice(0, 140));
    await o.keyboard.press('Escape'); await o.waitForTimeout(600);
  }

  if (VIEW === 'phone') {
    // ── menu edit round trip: change a price in the dish sheet, read it back, put it back ──
    const tok = (await api('/api/auth/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD, location_id: LOC } })).body.access_token;
    const read = async () => ((await api(`/api/owner/products?location_id=${LOC}`, { token: tok })).body.products || []).find(p => p.id === 'qa-tea');
    const before = (await read())?.price;
    const next = before === 250 ? 260 : 250;
    await o.click('#nav [data-tab="menu"]'); await o.waitForTimeout(2500);
    // A category folds: tap its header to open it, as an owner would.
    if (!(await o.isVisible('[data-p="qa-tea"]'))) { await o.click('[data-cat="qa-drinks"]'); await o.waitForTimeout(600); }
    await o.click('[data-p="qa-tea"]'); await o.waitForTimeout(1500);
    await o.fill('#d-price', String(next));
    await o.click('#dSave'); await o.waitForTimeout(3000);
    await o.screenshot({ path: shot('menu-saved') });
    const after = (await read())?.price;
    step(after === next, 'menu edit: the price typed in the dish sheet is the price the hub holds', `${before} -> typed ${next} -> hub ${after}`);
    const pub = await api(`/api/public/locations/${LOC}/menu?fresh=1`);
    const onStore = (pub.body.categories || []).flatMap(c => c.products).find(p => p.id === 'qa-tea')?.price;
    step(onStore === next, 'menu edit: the storefront shows the new price', `${onStore}`);
    await o.click('#nav [data-tab="menu"]'); await o.waitForTimeout(2000);
    if (!(await o.isVisible('[data-p="qa-tea"]'))) { await o.click('[data-cat="qa-drinks"]'); await o.waitForTimeout(600); }
    const rowTxt = await o.$eval('[data-p="qa-tea"]', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    step(rowTxt.includes(String(next)), 'menu edit: the menu row redraws with the new price', rowTxt);

    // ── staff: invite a QA kitchen member ──
    n0 = issues.length;
    await o.click('#nav [data-tab="more"]'); await o.waitForTimeout(1200);
    await o.click('[data-open="staff"]'); await o.waitForTimeout(2500);
    await o.click('#invite'); await o.waitForTimeout(1200);
    const stamp = Date.now().toString(36);
    const email = `qa-kitchen-${stamp}@example.com`;
    await o.fill('#i-email', email); await o.fill('#i-name', 'QA kitchen');
    const roles = await o.$$eval('#i-role option', els => els.map(e => e.value));
    await o.selectOption('#i-role', 'kitchen');
    await o.click('#iGo'); await o.waitForTimeout(3000);
    const code = await o.$eval('#iCode', e => e.textContent.trim()).catch(() => '');
    await o.screenshot({ path: shot('invite-kitchen') });
    step(!!code && since(n0).length === 0, 'invite a kitchen member: the sheet shows a one-time code', `roles offered=${roles} code ${code ? code.length + ' chars' : 'MISSING'} ${since(n0).join(' | ')}`);
    const st = await api(`/api/owner/staff?location_id=${LOC}`, { token: tok });
    const inv = (st.body.invites || []).find(i => i.email === email || i.name === 'QA kitchen');
    step(inv?.role === 'kitchen', 'API: the invite waits with role kitchen', JSON.stringify(inv || st.body).slice(0, 200));
    ssave({ qaKitchen: { email, code, password: `Qa-${stamp}-kitchen-pass!` } });
  }
} catch (e) {
  step(false, 'Q1 threw', e.stack?.slice(0, 400));
}
await end(b);
