import { chromium, makePage, wakeStaging, mockAuth, setToken, recorder, BASE, MOBILE, DESKTOP } from './_harness.mjs';

const rec = recorder();

async function courier(browser, device, token) {
  const F = (s, a, e) => rec.add(`COURIER[${device.name}]`, a, s, e);
  const h = await makePage(browser, device);
  const { page } = h;
  try {
    await setToken(page, token);
    await page.goto(`${BASE}/courier`, { waitUntil: 'domcontentloaded', timeout: 45000 }).catch(() => {});
    await page.waitForTimeout(3000);
    const txt = (await page.locator('body').innerText().catch(() => '')) || '';
    const ok = txt.length > 30 && !/something went wrong|application error|cannot read/i.test(txt);
    F(ok ? 'PASS' : 'FAIL', '/courier task list renders', `len=${txt.length} url=${page.url()}`);
    F(/task|delivery|porosi|no tasks|shift|earnings|online|offline/i.test(txt) ? 'PASS' : 'WARN', 'courier home shows task/shift content', `sample="${txt.slice(0,80).replace(/\n/g,' ')}"`);
    await h.shot('courier-home');

    // shift toggle
    const shiftToggle = await page.getByRole('button', { name: /shift|online|offline|go online|start/i }).count();
    F(shiftToggle > 0 ? 'PASS' : 'WARN', 'shift toggle present', `toggles=${shiftToggle}`);

    for (const sub of ['earnings', 'history', 'shift']) {
      await page.goto(`${BASE}/courier/${sub}`, { waitUntil: 'domcontentloaded' }).catch(() => {});
      await page.waitForTimeout(2200);
      const st = (await page.locator('body').innerText().catch(() => '')) || '';
      const r = st.length > 20 && !/something went wrong|cannot read|application error/i.test(st);
      F(r ? 'PASS' : 'FAIL', `/courier/${sub} renders`, `len=${st.length}`);
      await h.shot(`courier-${sub}`);
    }

    F(h.pageErrors.length === 0 ? 'PASS' : 'FAIL', 'no uncaught page errors (courier)', [...new Set(h.pageErrors)].slice(0,5).join(' | ') || 'none');
    F(h.netFailures.length === 0 ? 'PASS' : 'WARN', 'failed network reqs (courier)', [...new Set(h.netFailures)].slice(0,8).join(' | ') || 'none');
  } catch (e) {
    F('FAIL', 'courier spec crashed', String(e).slice(0, 200));
    await h.shot('courier-crash');
  } finally {
    await h.teardown();
  }
}

async function crosscut(browser, device) {
  const F = (s, a, e) => rec.add(`CROSSCUT[${device.name}]`, a, s, e);
  const h = await makePage(browser, device);
  const { page } = h;
  try {
    // 404 page
    await page.goto(`${BASE}/this-route-does-not-exist-xyz`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await page.waitForTimeout(1500);
    const t404 = await page.locator('body').innerText().catch(() => '');
    F(/404|not found|nuk u gjet/i.test(t404) ? 'PASS' : 'FAIL', '404 page shown for unknown route', `sample="${t404.slice(0,60).replace(/\n/g,' ')}"`);
    await h.shot('cc-404');

    // bad slug storefront
    await page.goto(`${BASE}/s/zzz-nope`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await page.waitForTimeout(2500);
    const badSlug = await page.locator('body').innerText().catch(() => '');
    const handled = /not found|unavailable|menu unavailable|failed|error|nuk|404|empty/i.test(badSlug) || badSlug.length < 200;
    F(handled ? 'PASS' : 'FAIL', 'bad slug /s/zzz-nope shows error/empty (not crash)', `len=${badSlug.length} sample="${badSlug.slice(0,80).replace(/\n/g,' ')}"`);
    F(h.pageErrors.length === 0 ? 'PASS' : 'WARN', 'bad slug no uncaught page error', h.pageErrors.slice(0,3).join(' | ') || 'none');
    await h.shot('cc-badslug');

    // PWA manifest
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await page.waitForTimeout(1500);
    const manifestHref = await page.locator('link[rel="manifest"]').getAttribute('href').catch(() => null);
    F(manifestHref ? 'PASS' : 'WARN', 'PWA manifest link present', `href=${manifestHref}`);
    if (manifestHref) {
      const murl = manifestHref.startsWith('http') ? manifestHref : `${BASE}${manifestHref}`;
      const mr = await fetch(murl).catch(() => ({ status: 'ERR' }));
      F(mr.status === 200 ? 'PASS' : 'FAIL', 'manifest fetches 200', `${murl} -> ${mr.status}`);
    }

    // 44px tap targets (sample buttons on storefront)
    const small = await page.evaluate(() => {
      const btns = [...document.querySelectorAll('button, a[href], [role="button"]')];
      let tooSmall = 0, total = 0;
      for (const b of btns) {
        const r = b.getBoundingClientRect();
        if (r.width === 0 || r.height === 0) continue;
        total++;
        if (r.height < 44 && r.width < 44) tooSmall++;
      }
      return { tooSmall, total };
    });
    F(small.total > 0 && small.tooSmall / small.total < 0.4 ? 'PASS' : 'WARN', '44px tap targets (storefront)', `tooSmall=${small.tooSmall}/${small.total}`);

    // focus-visible: tab and check outline
    await page.keyboard.press('Tab').catch(() => {});
    const focusOk = await page.evaluate(() => {
      const el = document.activeElement;
      if (!el || el === document.body) return false;
      const s = getComputedStyle(el);
      return s.outlineStyle !== 'none' || s.boxShadow !== 'none' || el.matches(':focus-visible');
    }).catch(() => false);
    F(focusOk ? 'PASS' : 'WARN', 'focus-visible indicator on tab', `focusable=${focusOk}`);

    // reduced motion respected (no JS error when emulated)
    await page.emulateMedia({ reducedMotion: 'reduce' }).catch(() => {});
    await page.reload({ waitUntil: 'domcontentloaded' }).catch(() => {});
    await page.waitForTimeout(2000);
    const rmText = await page.locator('body').innerText().catch(() => '');
    F(rmText.length > 40 ? 'PASS' : 'FAIL', 'page works under reduced-motion', `len=${rmText.length}`);
    await h.shot('cc-reduced-motion');

    F(h.pageErrors.length === 0 ? 'PASS' : 'WARN', 'no uncaught page errors (crosscut)', h.pageErrors.slice(0,4).join(' | ') || 'none');
  } catch (e) {
    F('FAIL', 'crosscut spec crashed', String(e).slice(0, 200));
    await h.shot('cc-crash');
  } finally {
    await h.teardown();
  }
}

(async () => {
  await wakeStaging();
  const token = await mockAuth('courier@demo.com', 'courier');
  console.log('[auth] courier token len', token.length);
  const browser = await chromium.launch();
  await courier(browser, MOBILE, token);
  await courier(browser, DESKTOP, token);
  await crosscut(browser, MOBILE);
  await crosscut(browser, DESKTOP);
  await browser.close();
  rec.dump();
})();
