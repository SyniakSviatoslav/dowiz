import { chromium, makePage, wakeStaging, gotoSafe, BASE, MOBILE } from './_harness.mjs';
(async () => {
  await wakeStaging();
  const browser = await chromium.launch();
  const h = await makePage(browser, MOBILE);
  const { page } = h;
  page.on('response', async r => {
    if (/\/orders\b/.test(r.url()) && r.request().method() === 'POST') {
      let b=''; try{b=(await r.text()).slice(0,500);}catch{}
      console.log('>>> ORDERS POST', r.status(), b);
    }
  });

  await gotoSafe(page, `${BASE}/s/demo`);
  // add via modal (proper flow) to ensure cart options set
  await page.locator('[data-testid="menu-item"]').first().click().catch(()=>{});
  await page.waitForTimeout(800);
  const confirm = page.locator('[data-testid="product-detail-confirm"]');
  if (await confirm.count()) { await confirm.first().click().catch(()=>{}); await page.waitForTimeout(800); }

  const cartLS = await page.evaluate(() => {
    const out = {};
    for (let i=0;i<localStorage.length;i++){const k=localStorage.key(i); if(/cart/i.test(k)) out[k]=localStorage.getItem(k);}
    return out;
  });
  console.log('CART LS:', JSON.stringify(cartLS).slice(0,300));

  // Navigate to checkout WITHIN spa (click checkout button) instead of hard goto
  await page.waitForTimeout(500);
  const bodyBtns = await page.getByRole('button').allInnerTexts();
  console.log('STOREFRONT BUTTONS:', JSON.stringify(bodyBtns.filter(b=>b.trim()).slice(0,20)));

  // find checkout/cart button
  const co = page.getByRole('button', { name: /checkout|cart|shport|arkë|view cart|porosit/i }).first();
  console.log('checkout-ish btn:', await co.count());
  if (await co.count()) { await co.click().catch(()=>{}); await page.waitForTimeout(1500); }
  console.log('URL after cart click:', page.url());
  // if a drawer opened, find a proceed button
  const proceed = page.getByRole('button', { name: /checkout|proceed|continue|vazhdo|porosit/i }).first();
  if (await proceed.count()) { await proceed.click().catch(()=>{}); await page.waitForTimeout(1500); }
  console.log('URL after proceed:', page.url());

  const phone = page.locator('[data-testid="checkout-phone"]');
  console.log('phone field on checkout:', await phone.count());
  const txt = await page.locator('body').innerText().catch(()=>'');
  console.log('CHECKOUT BODY (first 200):', txt.slice(0,200).replace(/\n/g,' '));
  await page.screenshot({ path:'/tmp/qa-shots/probe2-checkout.png' });

  if (await phone.count()) {
    await page.getByPlaceholder(/your name/i).first().fill('QA Tester').catch(()=>{});
    await phone.fill('+355691234567');
    const tabs = await page.getByRole('tab').allInnerTexts();
    console.log('CHECKOUT TABS:', JSON.stringify(tabs));
    const pickup = page.getByRole('tab', { name: /pickup|merr/i }).first();
    if (await pickup.count()) { await pickup.click(); await page.waitForTimeout(600); }
    const btn = page.locator('[data-testid="order-confirm-button"]');
    console.log('place btn disabled:', await btn.first().isDisabled().catch(()=>null));
    await btn.click({timeout:5000}).catch(e=>console.log('place click err', String(e).slice(0,80)));
    await page.waitForTimeout(5000);
    console.log('URL after place:', page.url());
    const after = await page.locator('body').innerText();
    console.log('AFTER PLACE (errors):', JSON.stringify(after.split('\n').filter(l=>/fail|error|required|valid|minimum|gabim/i.test(l)).slice(0,6)));
    await page.screenshot({ path:'/tmp/qa-shots/probe2-after-place.png' });
  }
  await h.teardown();
  await browser.close();
})();
