import pw from '/root/dowiz/node_modules/.pnpm/playwright@1.60.0/node_modules/playwright/index.js';
const { chromium } = pw;
const BASE = 'https://dowiz-staging.fly.dev';
const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
const page = await ctx.newPage();
const pageErrors = [], consoleErrors = [];
page.on('pageerror', e => pageErrors.push(String(e.message || e)));
page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text()); });
await page.goto(BASE + '/s/demo', { waitUntil: 'networkidle', timeout: 30000 });
await page.waitForTimeout(2500);

// Click a product row that shows a price (ALL). Find an element whose text contains "ALL" and a product name.
const handle = await page.evaluateHandle(() => {
  const els = [...document.querySelectorAll('button, [role="button"], li, div')];
  // a product card likely contains a price token "ALL" and an "Add"/"+" affordance, and is reasonably small
  for (const el of els) {
    const t = el.innerText || '';
    if (/ALL/.test(t) && t.length < 200 && t.length > 5 && el.querySelectorAll('*').length < 40) {
      const r = el.getBoundingClientRect();
      if (r.width > 100 && r.height > 30 && r.height < 400) return el;
    }
  }
  return null;
});
let clicked = false;
try { if (handle) { await handle.asElement().click({ timeout: 4000 }); clicked = true; } } catch(e){}
await page.waitForTimeout(1500);
const modal = await page.evaluate(() => {
  const dlg = document.querySelector('[role="dialog"],[class*="modal" i],[class*="sheet" i],[class*="drawer" i],[aria-modal="true"]');
  return { found: !!dlg, role: dlg?.getAttribute('role'), text: dlg ? dlg.innerText.replace(/\n/g,' ').slice(0,180) : '' };
});
await page.screenshot({ path: '/root/dowiz/audit/emil-validate/modal-check2.png' });
console.log('clicked=', clicked, 'modal=', JSON.stringify(modal));
console.log('pageErrors=', JSON.stringify(pageErrors));
console.log('consoleErrors(non404)=', JSON.stringify(consoleErrors.filter(e=>!/404/.test(e))));
await browser.close();
