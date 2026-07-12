/* eslint-disable local/no-empty-catch -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect, type Page } from '@playwright/test';

// Real-UI proof of the Nomadic/Moebius redesign on deployed staging. The skin is flag-off by
// default, so each test opts in per-session via localStorage('dos_paper_skin','on'). Verifies
// the journey scenes render, the honourable-mention credit links the right muse, the storefront
// adopts the paper world, and login titles are READABLE (locks the contrast fix — the #6 class).
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test nomadic-skin --project=desktop --reporter=list
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const AWWWARDS = 'awwwards.com/sites/nomadic-tribe';

async function enableSkin(page: Page) {
  await page.goto(`${BASE}/`, { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => { try { localStorage.setItem('dos_paper_skin', 'on'); } catch { /* private mode */ } });
}

// WCAG contrast of an element's text vs its first opaque solid ancestor bg (skip if over an image).
const CONTRAST = `(sel)=>{const L=(r,g,b)=>{const f=c=>{c/=255;return c<=0.03928?c/12.92:Math.pow((c+0.055)/1.055,2.4)};return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b)};const P=s=>{const m=(s||'').match(/rgba?\\(([^)]+)\\)/);if(!m)return null;const p=m[1].split(',').map(parseFloat);return{r:p[0],g:p[1],b:p[2],a:p[3]===undefined?1:p[3]}};const el=document.querySelector(sel);if(!el)return null;let n=el,bg=null;while(n){const s=getComputedStyle(n);if((s.backgroundImage||'none')!=='none')return{skip:true};const c=P(s.backgroundColor);if(c&&c.a>0.5){bg=c;break}n=n.parentElement}bg=bg||{r:255,g:255,b:255};const fg=P(getComputedStyle(el).color);if(!fg)return null;const r=(Math.max(L(fg.r,fg.g,fg.b),L(bg.r,bg.g,bg.b))+0.05)/(Math.min(L(fg.r,fg.g,fg.b),L(bg.r,bg.g,bg.b))+0.05);return{ratio:Math.round(r*100)/100}}`;

test('courier login — Nomadic journey hero + honourable-mention credit links makemepulse', async ({ page }) => {
  await enableSkin(page);
  await page.goto(`${BASE}/courier/login`, { waitUntil: 'networkidle' });
  await expect(page.locator('[data-skin="paper"] svg').first(), 'a Moebius scene renders').toBeVisible({ timeout: 25000 });
  await expect(page.getByText(/Design inspired by/i), 'honourable mention is present').toBeVisible();
  const link = page.getByRole('link', { name: /Nomadic Tribe by makemepulse/i });
  await expect(link).toBeVisible();
  expect(await link.getAttribute('href'), 'credit links the real muse').toContain(AWWWARDS);
});

test('owner login — oasis hero + credit + a READABLE Fraunces title (contrast locked)', async ({ page }) => {
  await enableSkin(page);
  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
  await expect(page.locator('[data-skin="paper"] svg').first()).toBeVisible({ timeout: 25000 });
  await expect(page.getByText(/Design inspired by/i)).toBeVisible();
  const title = page.getByRole('heading', { name: 'DeliveryOS' });
  await expect(title).toBeVisible();
  const c = await page.evaluate(`(${CONTRAST})('h1')`) as { ratio?: number; skip?: boolean } | null;
  if (c && !c.skip && c.ratio !== undefined) {
    expect(c.ratio, 'the hero title clears WCAG AA-large (was 1.09:1 light-on-light before the fix)').toBeGreaterThanOrEqual(3);
  }
});

test('storefront — adopts the paper world under the skin (flag-gated)', async ({ page }) => {
  await enableSkin(page);
  await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle' });
  await expect(page.locator('[data-skin="paper"]').first(), 'client shell carries the skin').toBeVisible({ timeout: 25000 });
  const bg = await page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  expect(bg, 'body paints a resolved opaque surface').toMatch(/^rgb/);
  expect(bg).not.toBe('rgba(0, 0, 0, 0)');
});
