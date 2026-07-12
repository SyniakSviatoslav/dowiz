import { chromium, makePage, wakeStaging, gotoSafe, BASE, MOBILE } from './_harness.mjs';
(async () => {
  await wakeStaging();
  const browser = await chromium.launch();
  const h = await makePage(browser, MOBILE);
  const { page } = h;
  page.on('request', r => { if (/\/orders\b/.test(r.url()) && r.method()==='POST') console.log('>>> REQ', r.method(), r.url(), 'body:', (r.postData()||'').slice(0,300)); });
  page.on('response', async r => { if (/\/orders\b/.test(r.url()) && r.request().method()==='POST') { let b=''; try{b=(await r.text()).slice(0,500);}catch{} console.log('<<< RESP', r.status(), b); } });

  await gotoSafe(page, `${BASE}/s/demo`);
  await page.locator('[data-testid="menu-item-add"]').first().click().catch(()=>{});
  await page.waitForTimeout(600);
  await page.getByRole('button', { name:/Shporta|cart/i }).first().click().catch(()=>{});
  await page.waitForTimeout(1000);
  await page.locator('[data-testid="cart-checkout"]').click().catch(()=>{});
  await page.waitForTimeout(1500);

  await page.getByPlaceholder(/your name|emri/i).first().fill('QA Tester').catch(()=>{});
  await page.locator('[data-testid="checkout-phone"]').fill('+355691234567');

  // tabs
  const tabTexts = await page.getByRole('tab').allInnerTexts();
  console.log('TABS:', JSON.stringify(tabTexts));

  // Select PICKUP (Marr)
  const pickup = page.getByRole('tab', { name:/Marr|pickup/i }).first();
  console.log('pickup count', await pickup.count());
  await pickup.click().catch(()=>{});
  await page.waitForTimeout(800);
  await page.screenshot({ path:'/tmp/qa-shots/order-pickup-form.png' });

  // dump all required empty inputs
  const reqEmpty = await page.evaluate(() => {
    return [...document.querySelectorAll('input,textarea')].filter(i=>i.required && !i.value).map(i=>({ph:i.placeholder, tid:i.getAttribute('data-testid'), type:i.type}));
  });
  console.log('REQUIRED-EMPTY inputs:', JSON.stringify(reqEmpty));

  const btn = page.locator('[data-testid="order-confirm-button"]');
  console.log('btn text:', (await btn.first().innerText().catch(()=>'')).replace(/\n/g,' '), 'disabled:', await btn.first().isDisabled().catch(()=>null));
  await btn.scrollIntoViewIfNeeded().catch(()=>{});
  await btn.click({timeout:5000}).catch(e=>console.log('CLICK ERR', String(e).slice(0,100)));
  await page.waitForTimeout(5000);
  console.log('URL:', page.url());
  // any validation message now?
  const after = await page.locator('body').innerText();
  console.log('VISIBLE VALIDATION:', JSON.stringify(after.split('\n').filter(l=>l.trim() && /kerko|required|valid|gabim|minimum|fail|error|kërko|notes|adres/i.test(l)).slice(0,8)));
  await page.screenshot({ path:'/tmp/qa-shots/order-after-click.png' });
  await h.teardown();
  await browser.close();
})();
