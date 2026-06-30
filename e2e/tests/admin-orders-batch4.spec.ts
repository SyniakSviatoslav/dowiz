import { test, expect } from '@playwright/test';

// Admin Orders audit Batch 4 (against deployed staging). Owner directives:
//  - admins land on Orders (no separate Dashboard nav);
//  - Export CSV demoted out of the toolbar (sort "Newest first" is the prominent control);
//  - filtering must NOT yank the scroll (controlled scroll-to-top, not an abrupt clamp).
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

async function authOwner(page: import('@playwright/test').Page, request: import('@playwright/test').APIRequestContext) {
  const res = await request.post(`${BASE}/api/auth/local/login`, { data: { email: 'test@dowiz.com', password: 'test123456' } });
  const token = (await res.json()).access_token as string;
  await page.addInitScript((t) => { try { localStorage.setItem('dos_access_token', t); } catch {} }, token);
}

// Scroll the app shell to the bottom, click `clickFn`, and return scrollTop before/after.
async function measureFilterScroll(page: import('@playwright/test').Page, clickSelector: string) {
  return page.evaluate(async (sel) => {
    const sleep = (ms: number) => new Promise(r => setTimeout(r, ms));
    const main = document.querySelector('.app-shell-main') as HTMLElement | null;
    const scroller: any = main && main.scrollHeight > main.clientHeight ? main : document.scrollingElement;
    scroller.scrollTo(0, scroller.scrollHeight);
    await sleep(300);
    const beforeTop = scroller.scrollTop;
    const scrollable = scroller.scrollHeight - scroller.clientHeight > 400;
    const el = document.querySelector(sel) as HTMLElement | null;
    if (el) el.click();
    await sleep(900); // allow smooth scroll-to-top to settle
    return { beforeTop, afterTop: scroller.scrollTop, scrollable };
  }, clickSelector);
}

test.describe('Admin Orders · Batch 4', () => {
  test('admins land on the Orders page at /admin', async ({ page, request }) => {
    await authOwner(page, request);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    await expect(page.getByRole('heading', { name: /Live Orders|Porosi|Замовлення/i })).toBeVisible({ timeout: 15000 });
  });

  test('toolbar has the Newest-first sort, CSV demoted below the list', async ({ page, request }) => {
    await authOwner(page, request);
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle' });
    // Sort control present (defaults to Newest).
    await expect(page.getByRole('button', { name: /Newest|Sort|Rendit|Найнов/i }).first()).toBeVisible({ timeout: 15000 });
    // The Export CSV control, if present, is NOT inside the sticky header toolbar.
    const headerCsv = page.locator('.sticky').getByRole('button', { name: /Export CSV|CSV/i });
    await expect(headerCsv).toHaveCount(0);
  });

  test('orders: status filter does NOT yank scroll (controlled scroll-to-top)', async ({ page, request }) => {
    await authOwner(page, request);
    await page.goto(`${BASE}/admin/orders`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(1200);
    const r = await measureFilterScroll(page, '[role=group] button:nth-of-type(2)');
    if (r.scrollable) {
      // After my fix the list resets to the top; the OLD bug left it clamped deep down.
      expect(r.afterTop, 'scroll should return to top after filtering').toBeLessThan(80);
    }
  });

  test('menu: category select does NOT yank scroll (controlled scroll-to-top)', async ({ page, request }) => {
    await authOwner(page, request);
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(1500);
    const r = await measureFilterScroll(page, 'button[aria-pressed]:nth-of-type(2)');
    if (r.scrollable) {
      expect(r.afterTop, 'menu should reset to top after picking a category').toBeLessThan(80);
    }
  });
});
