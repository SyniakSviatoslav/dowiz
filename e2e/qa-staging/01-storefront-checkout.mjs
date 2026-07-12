import { chromium, makePage, wakeStaging, recorder, BASE, MOBILE, DESKTOP } from './_harness.mjs';

const rec = recorder();

async function run(browser, device) {
  const F = (s, a, e) => rec.add(`STOREFRONT[${device.name}]`, a, s, e);
  const h = await makePage(browser, device);
  const { page } = h;
  try {
    const resp = await page.goto(`${BASE}/s/demo`, { waitUntil: 'domcontentloaded', timeout: 45000 });
    F(resp && resp.status() < 400 ? 'PASS' : 'FAIL', '/s/demo responds <400', `http=${resp && resp.status()}`);

    const title = await page.title();
    F(/demo|location/i.test(title) ? 'PASS' : 'FAIL', 'per-tenant <title> (SSR suspect)', `title="${title}"`);

    await page.waitForTimeout(2500);
    const items = page.locator('[data-testid="menu-item"]');
    const itemCount = await items.count();
    F(itemCount > 0 ? 'PASS' : 'FAIL', 'menu products render (menu-item)', `count=${itemCount}`);

    const body = await page.locator('body').innerText();
    F(/\d+\s*ALL/i.test(body) ? 'PASS' : 'FAIL', 'products show prices (ALL)', `priceMatch=${/\d+\s*ALL/i.test(body)}`);

    const tabs = await page.getByRole('tab').count();
    F(tabs >= 1 ? 'PASS' : 'WARN', 'category tabs render', `tabs=${tabs}`);

    // image fallback quality: items must NOT be plain grey/monogram; gradient bg OK
    const fallbackInfo = await items.first().evaluate(el => {
      const node = [...el.querySelectorAll('*')].find(n => /gradient|url\(/.test(getComputedStyle(n).backgroundImage));
      return node ? getComputedStyle(node).backgroundImage.slice(0, 30) : 'none';
    }).catch(() => 'err');
    const realImgs = await page.locator('[data-testid="menu-item"] img').count();
    F(/gradient|url/.test(fallbackInfo) ? 'PASS' : 'FAIL', 'image is crafted fallback or real (not grey box)', `bg="${fallbackInfo}" realImgs=${realImgs}`);
    await h.shot('01-menu');

    // search (placeholder is locale-dependent: Kerko/Search) -> use first input
    const search = page.locator('input').first();
    if (await search.count()) {
      await search.fill('zzzznotaproduct');
      await page.waitForTimeout(900);
      const after = await page.locator('body').innerText();
      const empty = /no products|no results|nuk ka|asnjë/i.test(after) || (await items.count()) === 0;
      F(empty ? 'PASS' : 'FAIL', 'search filters to empty state', `emptyShown=${empty}`);
      await search.fill('');
      await page.waitForTimeout(500);
    } else F('FAIL', 'search input present', 'no input');

    // sort controls
    const sortAZ = await page.getByRole('button', { name: 'A–Z', exact: true }).count();
    F(sortAZ > 0 ? 'PASS' : 'WARN', 'sort controls present (A–Z, price)', `azBtn=${sortAZ}`);

    // language switch sq->en->uk
    const langs = {};
    for (const L of ['EN', 'UA', 'SQ']) langs[L] = await page.getByRole('button', { name: L, exact: true }).count();
    const allLangs = langs.EN && langs.UA && langs.SQ;
    F(allLangs ? 'PASS' : 'FAIL', 'language switch sq/en/uk present', JSON.stringify(langs));
    if (langs.EN) {
      await page.getByRole('button', { name: 'EN', exact: true }).click().catch(() => {});
      await page.waitForTimeout(700);
      const ph = await page.locator('input').first().getAttribute('placeholder');
      F(/search/i.test(ph || '') ? 'PASS' : 'WARN', 'EN switch changes UI strings', `placeholder="${ph}"`);
    }

    // product detail modal (click product name)
    await items.first().click().catch(() => {});
    await page.waitForTimeout(1000);
    const dialog = page.locator('[role="dialog"]');
    const opened = (await dialog.count()) > 0;
    F(opened ? 'PASS' : 'FAIL', 'product detail modal opens', `dialog=${opened}`);
    await h.shot('02-product-modal');
    if (opened) {
      const confirm = page.locator('[data-testid="product-detail-confirm"]');
      if (await confirm.count()) {
        await confirm.first().click().catch(() => {});
        await page.waitForTimeout(800);
        F('PASS', 'add-to-cart from modal', 'clicked product-detail-confirm');
      } else {
        F('WARN', 'add-to-cart in modal', 'no product-detail-confirm; pressing Escape');
        await page.keyboard.press('Escape').catch(() => {});
      }
    }
    // also quick-add a 2nd item
    await page.waitForTimeout(400);
    const quickAdd = page.locator('[data-testid="menu-item-add"]').first();
    if (await quickAdd.count()) { await quickAdd.click().catch(() => {}); await page.waitForTimeout(500); }

    // cart bar / drawer
    await page.waitForTimeout(500);
    const cartBody = await page.locator('body').innerText();
    const cartSignal = /cart|shport|checkout|arkë|total/i.test(cartBody);
    F(cartSignal ? 'PASS' : 'WARN', 'cart bar/drawer reflects items', `signal=${cartSignal}`);
    await h.shot('03-cart');

    // ---- Flow 2: checkout ----
    await page.goto(`${BASE}/s/demo/checkout`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await page.waitForTimeout(1800);
    const phone = page.locator('[data-testid="checkout-phone"]');
    const onCheckout = (await phone.count()) > 0;
    F(onCheckout ? 'PASS' : 'FAIL', 'checkout page renders (phone field)', `url=${page.url()} phone=${onCheckout}`);
    await h.shot('04-checkout');

    if (onCheckout) {
      const nameInput = page.getByPlaceholder(/your name/i).first();
      if (await nameInput.count()) await nameInput.fill('QA Tester');

      // invalid phone validation
      await phone.fill('123');
      await page.locator('[data-testid="order-confirm-button"]').click({ timeout: 3000 }).catch(() => {});
      await page.waitForTimeout(900);
      const invalid = await page.getByText(/valid phone|invalid|\+355/i).count();
      F(invalid > 0 ? 'PASS' : 'WARN', 'invalid phone validated/blocked', `msg=${invalid}`);
      await phone.fill('+355691234567');

      // delivery vs pickup
      const dtabs = await page.getByRole('tab').count();
      F(dtabs >= 1 ? 'PASS' : 'WARN', 'delivery/pickup tabs', `tabs=${dtabs}`);

      const map = await page.locator('canvas, .maplibregl-canvas, [class*="maplibre"]').count();
      F(map > 0 ? 'PASS' : 'WARN', 'delivery map/pin present', `mapEls=${map}`);

      // switch to pickup to bypass address/map requirement
      const pickup = page.getByRole('tab', { name: /pickup|merr/i }).first();
      if (await pickup.count()) { await pickup.click().catch(() => {}); await page.waitForTimeout(600); }
      await h.shot('04b-pickup');

      await page.locator('[data-testid="order-confirm-button"]').click({ timeout: 4000 }).catch(() => {});
      await page.waitForTimeout(4000);
      const url = page.url();
      const placed = /\/order\//.test(url) || (await page.getByText(/order placed|order #|tracking|porosi/i).count()) > 0;
      F(placed ? 'PASS' : 'FAIL', 'order placed (pickup/cash)', `url=${url}`);
      await h.shot('05-order-result');

      if (/\/order\//.test(url)) {
        await page.waitForTimeout(1500);
        const status = await page.getByText(/status|received|preparing|pending|confirmed|pranuar|porosi/i).count();
        F(status > 0 ? 'PASS' : 'WARN', 'order tracking status shows', `statusEls=${status}`);
        await h.shot('06-tracking');
      }
    }

    F(h.pageErrors.length === 0 ? 'PASS' : 'FAIL', 'no uncaught page errors', h.pageErrors.join(' | ') || 'none');
    F(h.netFailures.length === 0 ? 'PASS' : 'WARN', 'failed network reqs', h.netFailures.slice(0, 8).join(' | ') || 'none');
    F(h.consoleErrors.length === 0 ? 'PASS' : 'WARN', 'console errors', [...new Set(h.consoleErrors)].slice(0, 6).join(' | ') || 'none');
  } catch (e) {
    F('FAIL', 'spec crashed', String(e).slice(0, 200));
    await h.shot('crash');
  } finally {
    await h.teardown();
  }
}

(async () => {
  await wakeStaging();
  const browser = await chromium.launch();
  await run(browser, MOBILE);
  await run(browser, DESKTOP);
  await browser.close();
  rec.dump();
})();
