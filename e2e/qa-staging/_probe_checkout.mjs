import { chromium, makePage, wakeStaging, gotoSafe, BASE, MOBILE } from './_harness.mjs';
(async () => {
  await wakeStaging();
  const browser = await chromium.launch();
  const h = await makePage(browser, MOBILE);
  const { page } = h;
  // capture order POST
  page.on('response', async r => {
    if (/\/orders\b/.test(r.url()) && r.request().method() === 'POST') {
      let body = '';
      try { body = (await r.text()).slice(0, 400); } catch {}
      console.log('>>> ORDERS POST', r.status(), body);
    }
  });

  // add an item first via storefront
  await gotoSafe(page, `${BASE}/s/demo`);
  await page.locator('[data-testid="menu-item-add"]').first().click().catch(()=>{});
  await page.waitForTimeout(800);

  await gotoSafe(page, `${BASE}/s/demo/checkout`);

  await page.getByPlaceholder(/your name/i).first().fill('QA Tester').catch(()=>{});
  await page.locator('[data-testid="checkout-phone"]').fill('+355691234567').catch(()=>{});

  // list tabs
  const tabs = await page.getByRole('tab').allInnerTexts();
  console.log('TABS:', JSON.stringify(tabs));

  // click pickup
  const pickup = page.getByRole('tab', { name: /pickup|merr/i }).first();
  console.log('pickup tab count:', await pickup.count());
  if (await pickup.count()) { await pickup.click(); await page.waitForTimeout(800); }

  // any required fields still empty? dump visible labels with *
  await page.screenshot({ path: '/tmp/qa-shots/probe-checkout-pickup.png' });

  // click place order, capture errors shown
  const btn = page.locator('[data-testid="order-confirm-button"]');
  console.log('place btn count:', await btn.count(), 'disabled:', await btn.first().isDisabled().catch(()=>null));
  await btn.click({ timeout: 4000 }).catch(e=>console.log('click err', String(e).slice(0,100)));
  await page.waitForTimeout(4000);
  console.log('URL after place:', page.url());
  const err = await page.locator('body').innerText();
  const errLines = err.split('\n').filter(l=>/fail|error|required|valid|minimum|gabim|kërko/i.test(l)).slice(0,8);
  console.log('ERROR LINES:', JSON.stringify(errLines));
  await page.screenshot({ path: '/tmp/qa-shots/probe-checkout-after.png' });
  await h.teardown();
  await browser.close();
})();
