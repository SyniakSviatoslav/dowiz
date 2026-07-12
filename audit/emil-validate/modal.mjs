import pw from '/root/dowiz/node_modules/.pnpm/playwright@1.60.0/node_modules/playwright/index.js';
const { chromium } = pw;
const BASE = 'https://dowiz-staging.fly.dev';
const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
const page = await ctx.newPage();
const pageErrors = [], consoleErrors = [], failed404 = [];
page.on('pageerror', e => pageErrors.push(String(e.message || e)));
page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text()); });
page.on('response', r => { if (r.status() === 404) failed404.push(r.url()); });

await page.goto(BASE + '/s/demo', { waitUntil: 'networkidle', timeout: 30000 });
await page.waitForTimeout(2500);

// Find a product name from the body and click it
const beforeUrl = page.url();
// product cards on storefront — click the first menu item heading/text
const candidates = ['California', 'Pizzas', 'Premium'];
let clicked = false;
for (const t of ['text=California Roll', 'text=California', 'h3', 'article', '[data-testid*="product"]']) {
  const loc = page.locator(t).first();
  if (await loc.count() > 0) {
    try { await loc.click({ timeout: 4000 }); clicked = true; break; } catch {}
  }
}
await page.waitForTimeout(1500);
const modal = await page.evaluate(() => {
  const dlg = document.querySelector('[role="dialog"],[class*="modal"],[class*="Modal"],[class*="sheet"],[class*="Sheet"],[class*="drawer"],[class*="Drawer"]');
  return { found: !!dlg, text: dlg ? dlg.innerText.slice(0,150) : '' };
});
await page.screenshot({ path: '/root/dowiz/audit/emil-validate/modal-check.png' });
console.log('clicked=', clicked, 'urlChanged=', page.url()!==beforeUrl, 'modal=', JSON.stringify(modal));
console.log('pageErrors=', JSON.stringify(pageErrors));
console.log('consoleErrors=', JSON.stringify(consoleErrors));
console.log('404s=', JSON.stringify(failed404.slice(0,15)));
await browser.close();
