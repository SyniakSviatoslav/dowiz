import pw from '/root/dowiz/node_modules/.pnpm/playwright@1.60.0/node_modules/playwright/index.js';
const { chromium } = pw;
import { writeFileSync } from 'node:fs';

const BASE = 'https://dowiz-staging.fly.dev';
const OWNER = process.env.OWNER_TOKEN;
const COURIER = process.env.COURIER_TOKEN;

const pages = [
  { name: 'storefront-s-demo', path: '/s/demo', token: null },
  { name: 'admin-orders', path: '/admin/orders', token: OWNER },
  { name: 'admin-menu', path: '/admin/menu', token: OWNER },
  { name: 'admin-analytics', path: '/admin/analytics', token: OWNER },
  { name: 'admin-settings', path: '/admin/settings', token: OWNER },
  { name: 'admin-branding', path: '/admin/branding', token: OWNER },
  { name: 'admin-promotions', path: '/admin/promotions', token: OWNER },
  { name: 'admin-crm', path: '/admin/crm', token: OWNER },
  { name: 'admin-couriers', path: '/admin/couriers', token: OWNER },
  { name: 'admin-supplies', path: '/admin/supplies', token: OWNER },
  { name: 'admin-activation', path: '/admin/activation', token: OWNER },
  { name: 'courier-home', path: '/courier', token: COURIER },
  { name: 'courier-shift', path: '/courier/shift', token: COURIER },
  { name: 'courier-earnings', path: '/courier/earnings', token: COURIER },
  { name: 'courier-history', path: '/courier/history', token: COURIER },
  { name: 'courier-login', path: '/courier/login', token: null },
];

const results = [];

const browser = await chromium.launch();
for (const p of pages) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
  const page = await ctx.newPage();
  const pageErrors = [];
  const consoleErrors = [];
  page.on('pageerror', e => pageErrors.push(String(e.message || e)));
  page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text()); });
  if (p.token) {
    await page.addInitScript(tok => {
      localStorage.setItem('dos_access_token', tok);
    }, p.token);
  }
  let navOk = true, navErr = '';
  try {
    await page.goto(BASE + p.path, { waitUntil: 'networkidle', timeout: 30000 });
  } catch (e) { navOk = false; navErr = String(e.message || e); }
  await page.waitForTimeout(2500);

  // detect blank / error boundary / stuck
  const probe = await page.evaluate(() => {
    const bodyText = (document.body && document.body.innerText || '').trim();
    const root = document.querySelector('#root') || document.body;
    const rootText = (root && root.innerText || '').trim();
    const hasH = !!document.querySelector('h1,h2,h3');
    const errBoundary = /something went wrong|error boundary|unexpected error|application error|reload the page/i.test(bodyText);
    const skeletons = document.querySelectorAll('[class*="skeleton"],[class*="Skeleton"],[aria-busy="true"]').length;
    return { bodyLen: bodyText.length, rootLen: rootText.length, hasH, errBoundary, skeletons, snippet: bodyText.slice(0, 200) };
  });

  await page.screenshot({ path: `/root/dowiz/audit/emil-validate/${p.name}.png`, fullPage: false });
  results.push({ ...p, navOk, navErr, pageErrors, consoleErrors, probe });
  await ctx.close();
}

// storefront product modal
{
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
  const page = await ctx.newPage();
  const pageErrors = [];
  const consoleErrors = [];
  page.on('pageerror', e => pageErrors.push(String(e.message || e)));
  page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text()); });
  let navOk = true, navErr = '', modalOpened = false;
  try {
    await page.goto(BASE + '/s/demo', { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2500);
    // click first product card-like element
    const card = page.locator('button, [role="button"], article, [class*="product"], [class*="Product"], [class*="card"], [class*="Card"]').first();
    await card.click({ timeout: 8000 }).catch(()=>{});
    await page.waitForTimeout(1500);
    modalOpened = await page.evaluate(() => !!document.querySelector('[role="dialog"], [class*="modal"], [class*="Modal"], [class*="sheet"], [class*="Sheet"]'));
  } catch (e) { navOk = false; navErr = String(e.message || e); }
  await page.screenshot({ path: `/root/dowiz/audit/emil-validate/storefront-product-modal.png` });
  results.push({ name: 'storefront-product-modal', path: '/s/demo (modal)', token: null, navOk, navErr, pageErrors, consoleErrors, probe: { modalOpened } });
  await ctx.close();
}

await browser.close();
writeFileSync('/root/dowiz/audit/emil-validate/results.json', JSON.stringify(results, null, 2));

for (const r of results) {
  const pe = r.pageErrors.length;
  const ce = r.consoleErrors.length;
  console.log(`\n=== ${r.name} (${r.path}) navOk=${r.navOk} ===`);
  if (r.navErr) console.log('  navErr:', r.navErr);
  if (r.probe) console.log('  probe:', JSON.stringify(r.probe));
  if (pe) console.log('  PAGE ERRORS:', JSON.stringify(r.pageErrors));
  if (ce) console.log('  CONSOLE ERRORS:', JSON.stringify(r.consoleErrors.slice(0, 12)));
}
console.log('\nDONE');
