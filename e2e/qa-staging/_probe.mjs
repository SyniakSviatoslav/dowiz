import { chromium, makePage, wakeStaging, BASE, DESKTOP } from './_harness.mjs';
(async () => {
  await wakeStaging();
  const browser = await chromium.launch();
  const h = await makePage(browser, DESKTOP);
  const { page } = h;
  await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle', timeout: 45000 }).catch(() => {});
  await page.waitForTimeout(2000);

  console.log('=== TITLE ===', await page.title());
  console.log('=== INPUTS ===');
  const inputs = await page.locator('input, textarea').all();
  for (const i of inputs) console.log(' input', JSON.stringify({ ph: await i.getAttribute('placeholder'), type: await i.getAttribute('type'), tid: await i.getAttribute('data-testid'), aria: await i.getAttribute('aria-label') }));

  console.log('=== TESTIDS ===');
  const tids = await page.locator('[data-testid]').all();
  const seen = new Set();
  for (const t of tids) { const v = await t.getAttribute('data-testid'); if (!seen.has(v)) { seen.add(v); console.log(' ', v); } }

  console.log('=== first 40 buttons text ===');
  const btns = await page.getByRole('button').all();
  for (let i = 0; i < Math.min(btns.length, 40); i++) {
    const tx = (await btns[i].innerText().catch(() => '')).replace(/\n/g, ' ').slice(0, 40);
    if (tx.trim()) console.log(`  btn[${i}] "${tx}"`);
  }
  console.log('=== IMG/PICTURE/CANVAS counts ===', { img: await page.locator('img').count(), picture: await page.locator('picture').count(), canvas: await page.locator('canvas').count(), svg: await page.locator('svg').count(), bgDivs: await page.locator('[style*="background-image"]').count() });

  // Click a product card region to find what opens the modal
  console.log('=== Try clicking product by name "Burger"/first product ===');
  const menuJson = await (await fetch(`${BASE}/public/locations/demo/menu`)).json().catch(() => ({}));
  const firstProd = (menuJson.categories?.[0]?.products?.[0]?.name) || '';
  console.log('first product name from API:', firstProd);
  if (firstProd) {
    const el = page.getByText(firstProd, { exact: false }).first();
    console.log('  product text found count:', await page.getByText(firstProd, { exact: false }).count());
    await el.click().catch(e => console.log('  click err', String(e).slice(0,80)));
    await page.waitForTimeout(1200);
    console.log('  dialogs after click:', await page.locator('[role="dialog"]').count());
    await h.shot('probe-after-product-click');
  }
  await h.teardown();
  await browser.close();
})();
