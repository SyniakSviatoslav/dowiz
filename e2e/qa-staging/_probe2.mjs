import { chromium, makePage, wakeStaging, BASE, DESKTOP } from './_harness.mjs';
(async () => {
  await wakeStaging();
  const browser = await chromium.launch();
  const h = await makePage(browser, DESKTOP);
  const { page } = h;
  await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle', timeout: 45000 }).catch(() => {});
  await page.waitForTimeout(2000);

  // Inspect menu-item visuals
  const items = await page.locator('[data-testid="menu-item"]').all();
  console.log('menu-items:', items.length);
  for (let i = 0; i < items.length; i++) {
    const it = items[i];
    const html = await it.evaluate(el => {
      const imgs = el.querySelectorAll('img').length;
      const bg = [...el.querySelectorAll('*')].map(n => getComputedStyle(n).backgroundImage).filter(b => b && b !== 'none');
      const txt = el.innerText.replace(/\n/g,' ').slice(0,60);
      return { imgs, bgCount: bg.length, bgSample: bg[0]?.slice(0,80), txt };
    });
    console.log(` item[${i}]`, JSON.stringify(html));
  }

  // /v1/rates — what is it?
  console.log('=== /v1/rates direct ===');
  const r = await fetch(`${BASE}/v1/rates`).catch(e => ({ status: 'ERR ' + e }));
  console.log('  status:', r.status);

  // sort buttons present?
  console.log('=== sort/filter controls ===');
  for (const name of ['↑ $','↓ $','A–Z']) console.log('  ', name, await page.getByRole('button', { name, exact: true }).count());

  // allergen filter — search for allergen UI
  const allergenCt = await page.getByText(/allergen|alergjen/i).count();
  console.log('  allergen text count:', allergenCt);

  // language switch test: click EN, see search placeholder change
  await page.getByRole('button', { name: 'EN', exact: true }).click().catch(()=>{});
  await page.waitForTimeout(800);
  const ph = await page.locator('input').first().getAttribute('placeholder');
  console.log('  search placeholder after EN:', ph);

  await h.teardown();
  await browser.close();
})();
