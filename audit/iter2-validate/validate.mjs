import { chromium } from '@playwright/test';

const BASE = 'https://dowiz-staging.fly.dev';
const OWNER = process.env.OWNER_TOKEN;
const COURIER = process.env.COURIER_TOKEN;
const OUT = 'audit/iter2-validate';
const M = { width: 390, height: 844 };
const D = { width: 1280, height: 900 };

const results = {};
const consoleErrors = [];

async function newCtx(viewport, token) {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport });
  const page = await ctx.newPage();
  page.on('console', (msg) => {
    if (msg.type() === 'error') consoleErrors.push(`[${page.url()}] ${msg.text()}`);
  });
  page.on('pageerror', (e) => consoleErrors.push(`[pageerror ${page.url()}] ${e.message}`));
  if (token) {
    await ctx.addInitScript((t) => {
      localStorage.setItem('dos_access_token', t);
    }, token);
  }
  return { browser, ctx, page };
}

async function go(page, path) {
  await page.goto(BASE + path, { waitUntil: 'networkidle', timeout: 60000 }).catch(() => {});
  await page.waitForTimeout(2500);
}

async function shot(page, name) {
  await page.screenshot({ path: `${OUT}/${name}.png`, fullPage: false });
}

// CHECK 1: STOREFRONT cart + product modal (mobile)
async function check1() {
  const { browser, page } = await newCtx(M, null);
  try {
    await go(page, '/s/demo');
    await shot(page, '01-storefront');
    const addBtns = page.locator('[data-testid="menu-item-add"]');
    const addCount = await addBtns.count();
    let note = `add-btns=${addCount}`;
    if (addCount > 0) {
      await addBtns.first().click().catch((e) => (note += ` addClickErr=${e.message}`));
      await page.waitForTimeout(1200);
      // cart FAB
      const fab = page.getByText('Shporta', { exact: false }).first();
      const fabVisible = await fab.isVisible().catch(() => false);
      note += ` fabVisible=${fabVisible}`;
      if (fabVisible) {
        await fab.click().catch((e) => (note += ` fabClickErr=${e.message}`));
        await page.waitForTimeout(1500);
      }
      await shot(page, '01b-cart-drawer');
      // check overflow: scrollWidth vs clientWidth
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      note += ` horizOverflowPx=${overflow}`;
      // detect checkout CTA text
      const cta = await page.evaluate(() => {
        const txt = document.body.innerText;
        return /checkout|pagesa|porosit|vazhdo|total/i.test(txt);
      });
      note += ` ctaOrTotalText=${cta}`;
    }
    // close drawer & open product modal
    await page.keyboard.press('Escape').catch(() => {});
    await page.waitForTimeout(500);
    // click a product card (try common selectors)
    const card = page.locator('[data-testid="menu-item"], [data-testid="product-card"]').first();
    const cardExists = await card.count();
    note += ` productCards=${cardExists}`;
    if (cardExists > 0) {
      await card.first().click().catch(() => {});
      await page.waitForTimeout(1500);
      await shot(page, '01c-product-modal');
      const dialog = await page.locator('[role="dialog"]').count();
      note += ` dialogAfterCardClick=${dialog}`;
    }
    results.check1 = note;
  } catch (e) {
    results.check1 = 'ERR ' + e.message;
  } finally {
    await browser.close();
  }
}

// CHECK 2: ADMIN orders status badges (desktop + mobile)
async function check2() {
  for (const [vp, label] of [[D, 'desktop'], [M, 'mobile']]) {
    const { browser, page } = await newCtx(vp, OWNER);
    try {
      await go(page, '/admin/orders');
      await shot(page, `02-admin-orders-${label}`);
      const info = await page.evaluate(() => {
        const body = document.body.innerText;
        return {
          hasOrdersWord: /porosi|order/i.test(body),
          textLen: body.length,
          title: document.title,
        };
      });
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      results['check2_' + label] = `title="${info.title}" hasOrders=${info.hasOrdersWord} textLen=${info.textLen} horizOverflowPx=${overflow}`;
    } catch (e) {
      results['check2_' + label] = 'ERR ' + e.message;
    } finally {
      await browser.close();
    }
  }
}

// CHECK 3: COURIER task list (mobile)
async function check3() {
  const { browser, page } = await newCtx(M, COURIER);
  try {
    await go(page, '/courier');
    await shot(page, '03-courier');
    const info = await page.evaluate(() => ({
      title: document.title,
      textSnippet: document.body.innerText.slice(0, 300),
      bodyLen: document.body.innerText.length,
    }));
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    results.check3 = `title="${info.title}" bodyLen=${info.bodyLen} horizOverflowPx=${overflow} snippet="${info.textSnippet.replace(/\n/g, ' ').slice(0,160)}"`;
  } catch (e) {
    results.check3 = 'ERR ' + e.message;
  } finally {
    await browser.close();
  }
}

// CHECK 5: admin menu modal
async function check5() {
  const { browser, page } = await newCtx(D, OWNER);
  try {
    await go(page, '/admin/menu');
    await shot(page, '05-admin-menu');
    // try to find an "add category" / "add product" button
    const candidates = ['Shto kategori', 'Shto produkt', 'Add category', 'Add product', 'Shto', 'Add'];
    let clicked = '';
    for (const t of candidates) {
      const btn = page.getByRole('button', { name: new RegExp(t, 'i') }).first();
      if (await btn.count() > 0 && await btn.isVisible().catch(() => false)) {
        await btn.click().catch(() => {});
        await page.waitForTimeout(1200);
        const dlg = await page.locator('[role="dialog"]').count();
        if (dlg > 0) { clicked = t; break; }
      }
    }
    if (clicked) {
      await shot(page, '05b-admin-modal');
      const closeBtn = await page.locator('[role="dialog"] button[aria-label], [role="dialog"] button').count();
      results.check5 = `modalTriggeredBy="${clicked}" dialogPresent=true closeBtnsInDialog=${closeBtn}`;
    } else {
      results.check5 = 'SKIPPED: no add-modal button found on /admin/menu';
    }
  } catch (e) {
    results.check5 = 'ERR ' + e.message;
  } finally {
    await browser.close();
  }
}

(async () => {
  await check1();
  await check2();
  await check3();
  await check5();
  console.log('=== RESULTS ===');
  console.log(JSON.stringify(results, null, 2));
  console.log('=== CONSOLE ERRORS (' + consoleErrors.length + ') ===');
  console.log(consoleErrors.slice(0, 40).join('\n'));
})();
