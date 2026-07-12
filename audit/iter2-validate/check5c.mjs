import { chromium } from '@playwright/test';
const BASE = 'https://dowiz-staging.fly.dev';
const OWNER = process.env.OWNER_TOKEN;
const OUT = 'audit/iter2-validate';
const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } });
await ctx.addInitScript((t) => localStorage.setItem('dos_access_token', t), OWNER);
const page = await ctx.newPage();
const errs = [];
page.on('console', m => { if (m.type()==='error') errs.push(m.text()); });
page.on('pageerror', e => errs.push('pageerror '+e.message));
await page.goto(BASE + '/admin/menu', { waitUntil: 'networkidle', timeout: 60000 });
await page.waitForTimeout(2500);

// READ-ONLY: open the product EDITOR modal via a pencil (no save), never create data.
let opened='';
let dlg = 0;
// First, try clicking a product edit pencil. Locate edit buttons by aria-label
{
  // grab first product card and its action buttons
  const editCandidates = page.locator('button[aria-label*="edit" i], button[aria-label*="ndrysho" i], button[title*="edit" i]');
  if (await editCandidates.count()) { await editCandidates.first().click().catch(()=>{}); await page.waitForTimeout(1000); }
  dlg = await page.locator('[role="dialog"]').count();
}
// last resort: enumerate buttons w/ svg in first product card and click the pencil (2nd-to-last)
if (!dlg) {
  const cardButtons = page.locator('div').filter({ hasText: /^Cola/ }).locator('button');
  const n = await cardButtons.count();
  if (n >= 2) { await cardButtons.nth(n-2).click().catch(()=>{}); await page.waitForTimeout(1000); }
  dlg = await page.locator('[role="dialog"]').count();
}
opened = dlg ? 'dialog' : 'none';
await page.screenshot({ path: `${OUT}/05b-admin-modal.png` });
let closeInfo='none';
if (dlg) {
  const close = page.locator('[role="dialog"] button').first();
  await close.focus().catch(()=>{});
  closeInfo = await page.evaluate(() => {
    const el = document.activeElement; if(!el) return 'no-active';
    const cs = getComputedStyle(el);
    return `tag=${el.tagName} aria=${el.getAttribute('aria-label')} outline=${cs.outlineWidth}/${cs.outlineStyle} ring=${cs.boxShadow.slice(0,50)}`;
  });
  await page.screenshot({ path: `${OUT}/05c-modal-close-focus.png` });
}
console.log(JSON.stringify({ opened, dlg, closeInfo, errs: errs.slice(0,8) }, null, 2));
await browser.close();
