import { chromium, makePage, wakeStaging, mockAuth, setToken, recorder, BASE, MOBILE, DESKTOP } from './_harness.mjs';

const rec = recorder();
const ADMIN_PAGES = [
  ['orders', /orders|porosi|dashboard|no orders|live/i],
  ['menu', /menu|categor|product|artikuj/i],
  ['couriers', /courier|invite|rider|kuriers?/i],
  ['branding', /brand|theme|logo|color|seo/i],
  ['settings', /setting|cilësim|general|hours|otp/i],
  ['crm', /crm|customer|klient/i],
  ['analytics', /analytic|revenue|orders|chart/i],
  ['promotions', /promo|discount|zbritje/i],
  ['supplies', /suppl|inventory|ingredient|furnizim/i],
  ['activation', /activ|publish|checklist|preview|go.?live/i],
  ['onboarding', /onboard|welcome|step|setup|mirë/i],
];

async function run(browser, device, token) {
  const F = (s, a, e) => rec.add(`ADMIN[${device.name}]`, a, s, e);
  const h = await makePage(browser, device);
  const { page } = h;
  try {
    await setToken(page, token);

    // /admin home (onboarding/activation split / dashboard)
    await page.goto(`${BASE}/admin`, { waitUntil: 'domcontentloaded', timeout: 45000 }).catch(() => {});
    await page.waitForTimeout(3000);
    const homeText = (await page.locator('body').innerText().catch(() => '')) || '';
    const homeOk = homeText.length > 40 && !/cannot|something went wrong|application error/i.test(homeText);
    F(homeOk ? 'PASS' : 'FAIL', '/admin home renders (not blank/error)', `len=${homeText.length} url=${page.url()}`);
    await h.shot('admin-home');

    for (const [path, re] of ADMIN_PAGES) {
      const before = h.netFailures.length;
      const respErrCountBefore = h.pageErrors.length;
      await page.goto(`${BASE}/admin/${path}`, { waitUntil: 'domcontentloaded', timeout: 45000 }).catch(() => {});
      await page.waitForTimeout(2600);
      const txt = (await page.locator('body').innerText().catch(() => '')) || '';
      const rendered = txt.length > 30 && !/something went wrong|application error|cannot read|undefined is not/i.test(txt);
      const matched = re.test(txt);
      const pageErr = h.pageErrors.length > respErrCountBefore;
      let status = rendered && !pageErr ? (matched ? 'PASS' : 'WARN') : 'FAIL';
      F(status, `/admin/${path} renders core content`, `len=${txt.length} matched=${matched} pageErr=${pageErr} url=${page.url().replace(BASE,'')}`);
      await h.shot(`admin-${path}`);
    }

    // ---- Flow 5: menu manager core action — open and try add category ----
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await page.waitForTimeout(2500);
    const addCat = page.getByRole('button', { name: /add category|new category|shto kategori|add categor/i }).first();
    const addBtns = await page.getByRole('button').count();
    F(addBtns > 0 ? 'PASS' : 'FAIL', 'menu manager has action buttons', `buttons=${addBtns} addCatFound=${await addCat.count()}`);
    // CSV / photo import presence
    const importBtn = await page.getByText(/import|csv|upload|photo|bulk/i).count();
    F(importBtn > 0 ? 'PASS' : 'WARN', 'menu CSV/photo import entrypoint present', `importEls=${importBtn}`);
    await h.shot('admin-menu-detail');

    // ---- Flow 6: orders — list + WS + status ----
    await page.goto(`${BASE}/admin/orders`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await page.waitForTimeout(2500);
    const ordersTxt = await page.locator('body').innerText().catch(() => '');
    F(/order|porosi|no orders|live|today/i.test(ordersTxt) ? 'PASS' : 'WARN', 'orders dashboard content', `len=${ordersTxt.length}`);
    await h.shot('admin-orders-detail');

    // ---- Flow 7: couriers — invite entrypoint ----
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await page.waitForTimeout(2500);
    const inviteBtn = await page.getByRole('button', { name: /invite|shto|add courier|generate/i }).count();
    F(inviteBtn > 0 ? 'PASS' : 'WARN', 'courier invite entrypoint', `inviteBtns=${inviteBtn}`);
    await h.shot('admin-couriers-detail');

    F(h.pageErrors.length === 0 ? 'PASS' : 'FAIL', 'no uncaught page errors (admin)', [...new Set(h.pageErrors)].slice(0,5).join(' | ') || 'none');
    F(h.netFailures.length === 0 ? 'PASS' : 'WARN', 'failed network reqs (admin)', [...new Set(h.netFailures)].slice(0, 10).join(' | ') || 'none');
    F(h.consoleErrors.length === 0 ? 'PASS' : 'WARN', 'console errors (admin)', [...new Set(h.consoleErrors)].slice(0, 6).join(' | ') || 'none');
  } catch (e) {
    F('FAIL', 'admin spec crashed', String(e).slice(0, 200));
    await h.shot('admin-crash');
  } finally {
    await h.teardown();
  }
}

(async () => {
  await wakeStaging();
  const token = await mockAuth('ownera@demo.com', 'owner');
  console.log('[auth] owner token len', token.length);
  const browser = await chromium.launch();
  await run(browser, DESKTOP, token);
  await run(browser, MOBILE, token);
  await browser.close();
  rec.dump();
})();
