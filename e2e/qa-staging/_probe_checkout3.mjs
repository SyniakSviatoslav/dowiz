import { chromium, makePage, wakeStaging, gotoSafe, BASE, MOBILE } from './_harness.mjs';
(async () => {
  await wakeStaging();
  const browser = await chromium.launch();
  const h = await makePage(browser, MOBILE);
  const { page } = h;
  page.on('response', async r => {
    if (/\/orders\b/.test(r.url()) && r.request().method() === 'POST') {
      let b=''; try{b=(await r.text()).slice(0,600);}catch{}
      console.log('>>> ORDERS POST', r.status(), b);
    }
  });

  await gotoSafe(page, `${BASE}/s/demo`);
  await page.locator('[data-testid="menu-item"]').first().click().catch(()=>{});
  await page.waitForTimeout(700);
  const confirm = page.locator('[data-testid="product-detail-confirm"]');
  if (await confirm.count()) { await confirm.first().click().catch(()=>{}); await page.waitForTimeout(700); }

  // open cart bar (bottom), then cart-checkout testid
  const bar = page.getByRole('button', { name: /Shporta|cart/i }).first();
  console.log('cart bar count:', await bar.count());
  await bar.click().catch(e=>console.log('bar click', String(e).slice(0,60)));
  await page.waitForTimeout(1200);
  console.log('cart-checkout testid count:', await page.locator('[data-testid="cart-checkout"]').count());
  await page.locator('[data-testid="cart-checkout"]').click().catch(e=>console.log('cc click', String(e).slice(0,60)));
  await page.waitForTimeout(1800);
  console.log('URL after cart-checkout:', page.url());
  console.log('phone field:', await page.locator('[data-testid="checkout-phone"]').count());

  // Now HARD RELOAD checkout to test deep-link/refresh resilience
  await page.reload({ waitUntil:'domcontentloaded' }).catch(()=>{});
  await page.waitForTimeout(2500);
  console.log('--- after RELOAD ---');
  console.log('URL:', page.url(), 'phone field:', await page.locator('[data-testid="checkout-phone"]').count());
  const rt = await page.locator('body').innerText().catch(()=>'');
  console.log('reload body first120:', rt.slice(0,120).replace(/\n/g,' '));

  // Proceed with order on whichever has phone field
  if (await page.locator('[data-testid="checkout-phone"]').count() === 0) {
    // re-navigate via SPA
    await gotoSafe(page, `${BASE}/s/demo`);
    await page.locator('[data-testid="menu-item-add"]').first().click().catch(()=>{});
    await page.waitForTimeout(600);
    await page.getByRole('button', { name: /Shporta|cart/i }).first().click().catch(()=>{});
    await page.waitForTimeout(1000);
    await page.locator('[data-testid="cart-checkout"]').click().catch(()=>{});
    await page.waitForTimeout(1500);
  }
  if (await page.locator('[data-testid="checkout-phone"]').count()) {
    await page.getByPlaceholder(/your name/i).first().fill('QA Tester').catch(()=>{});
    await page.locator('[data-testid="checkout-phone"]').fill('+355691234567');
    const pickup = page.getByRole('tab', { name:/pickup|merr/i }).first();
    if (await pickup.count()) { await pickup.click(); await page.waitForTimeout(600); }
    const btn = page.locator('[data-testid="order-confirm-button"]');
    console.log('place disabled:', await btn.first().isDisabled().catch(()=>null));
    await btn.click({timeout:5000}).catch(e=>console.log('place err', String(e).slice(0,80)));
    await page.waitForTimeout(6000);
    console.log('URL after PLACE:', page.url());
    const after = await page.locator('body').innerText();
    console.log('errors:', JSON.stringify(after.split('\n').filter(l=>/fail|error|required|valid|minimum|gabim|notes/i.test(l)).slice(0,6)));
    await page.screenshot({ path:'/tmp/qa-shots/probe3-after-place.png' });
    console.log('tracking?', /\/order\//.test(page.url()), 'status text:', /order placed|received|preparing|pending|porosi/i.test(after));
  }
  await h.teardown();
  await browser.close();
})();
