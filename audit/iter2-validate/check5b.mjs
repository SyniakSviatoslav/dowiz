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
await page.goto(BASE + '/admin/menu', { waitUntil: 'networkidle', timeout: 60000 });
await page.waitForTimeout(2500);
// Click a product edit pencil to open the product editor modal
let opened = '';
const pencil = page.locator('button:has(svg)').filter({ hasNot: page.locator('text=/Importo/') });
// Try clicking edit icon near first product card: use the pencil button. Fall back: round + add-category button
// First try the round "+" add-category button
const plus = page.getByRole('button').last();
// More targeted: find button with aria or the + . We'll click first product's edit (pencil) by approximate locator.
const editBtns = await page.locator('button').all();
// Try the "+" add category: it's the round red button. Use getByRole name '+' unlikely. Click the pencil instead.
try {
  // click first edit pencil: pencils are buttons inside product cards
  const firstPencil = page.locator('[class*="card"], div').locator('button').nth(0);
  // Simplest robust: click first product card title to maybe open editor
  await page.getByText('Cola', { exact: true }).first().click().catch(()=>{});
  await page.waitForTimeout(1000);
  let dlg = await page.locator('[role="dialog"]').count();
  if (!dlg) {
    // try the round + add-category button (sibling of "Kategori e re...")
    const addCat = page.locator('input[placeholder*="Kategori e re" i]').locator('xpath=following-sibling::button[1]');
    if (await addCat.count()) { await addCat.first().click().catch(()=>{}); await page.waitForTimeout(800); }
    dlg = await page.locator('[role="dialog"]').count();
  }
  if (!dlg) {
    // click an edit pencil (svg button) within first card region
    const card = page.locator('text=Cola').first();
    const editIcon = page.locator('button').filter({ has: page.locator('svg') });
    // click the pencil that sits in first card: heuristics — nth around 6
  }
  if (dlg) opened = 'dialog';
} catch(e){ opened = 'err '+e.message; }

// final attempt: directly click any pencil-looking button by position in first product card
if (opened !== 'dialog') {
  const box = await page.getByText('Cola', { exact: true }).first().boundingBox();
  if (box) {
    // pencil is bottom-right of card; click near x+230,y+75
    await page.mouse.click(box.x + 218, box.y + 78).catch(()=>{});
    await page.waitForTimeout(1000);
  }
}
const dlgCount = await page.locator('[role="dialog"]').count();
await page.screenshot({ path: `${OUT}/05b-admin-modal.png` });
// check close button focus-visible: focus it
let closeInfo = 'none';
if (dlgCount) {
  const close = page.locator('[role="dialog"] button[aria-label*="close" i], [role="dialog"] button[aria-label*="mbyll" i], [role="dialog"] button').first();
  if (await close.count()) {
    await close.focus().catch(()=>{});
    closeInfo = await page.evaluate(() => {
      const el = document.activeElement;
      if (!el) return 'no-active';
      const cs = getComputedStyle(el);
      return `tag=${el.tagName} outline=${cs.outlineWidth}/${cs.outlineStyle} boxShadow=${cs.boxShadow.slice(0,40)}`;
    });
    await page.screenshot({ path: `${OUT}/05c-modal-close-focus.png` });
  }
}
console.log(JSON.stringify({ opened, dlgCount, closeInfo, errs: errs.slice(0,10) }, null, 2));
await browser.close();
