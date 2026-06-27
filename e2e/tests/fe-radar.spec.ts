import { test, expect, type Page, type BrowserContext } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Never default to the PROD host: this spec hits the /dev/mock-auth token minter, so a
// prod default would exercise an auth backdoor against production. Default to staging and
// hard-guard in beforeAll.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

// ── Collectors ──
interface NetworkRecord { url: string; method: string; status: number; duration: number; type: string; }
interface ConsoleRecord { level: string; text: string; source: string; }
interface Issue { surface: string; step: string; expected: string; actual: string; evidence: string; severity: '🔴' | '🟠' | '🟡' | '🔵' | '⚪'; hypothesis: string; }

const networkLog: NetworkRecord[] = [];
const consoleLog: ConsoleRecord[] = [];
const issues: Issue[] = [];
const pageErrors: string[] = [];

function setupCollectors(page: Page) {
  networkLog.length = 0; consoleLog.length = 0; pageErrors.length = 0;
  page.on('requestfailed', (req) => {
    networkLog.push({ url: req.url().substring(0, 100), method: req.method(), status: 0, duration: 0, type: req.resourceType() });
  });
  page.on('response', (res) => {
    if (res.status() >= 400) {
      networkLog.push({ url: res.url().substring(0, 100), method: res.request().method(), status: res.status(), duration: 0, type: res.request().resourceType() });
    }
  });
  page.on('pageerror', (err) => pageErrors.push(err.message));
  page.on('console', (msg) => {
    if (msg.type() === 'error' || msg.type() === 'warning') {
      consoleLog.push({ level: msg.type(), text: msg.text().substring(0, 200), source: msg.location().url?.substring(0, 80) || '' });
    }
  });
}

function emit(surface: string, step: string, status: string, detail: string) {
  console.log(`${surface}|${step}|${status}|${detail}`);
}

function checkIssues(surface: string, page: Page) {
  for (const ne of networkLog) {
    if (ne.status === 0) issues.push({ surface, step: 'network-fail', expected: '2xx/3xx', actual: `Failed: ${ne.url}`, evidence: 'requestfailed', severity: '🔴', hypothesis: 'Network error — DNS/CORS/server down' });
    else if (ne.status >= 500) issues.push({ surface, step: 'network-5xx', expected: '2xx/3xx', actual: `${ne.status} ${ne.url}`, evidence: 'response', severity: '🔴', hypothesis: 'Server error on expected endpoint' });
    else if (ne.status === 404) issues.push({ surface, step: 'network-404', expected: 'asset/exists', actual: `${ne.status} ${ne.url}`, evidence: 'response', severity: '🟠', hypothesis: 'Missing asset or wrong path' });
    else if (ne.status === 401 || ne.status === 403) issues.push({ surface, step: 'network-auth', expected: 'authorized', actual: `${ne.status} ${ne.url}`, evidence: 'response', severity: '🟠', hypothesis: 'Auth token missing/expired for expected endpoint' });
  }
  for (const pe of pageErrors) issues.push({ surface, step: 'js-error', expected: 'no uncaught exceptions', actual: pe, evidence: 'pageerror', severity: '🔴', hypothesis: 'Unhandled JS exception — component/data issue' });
  for (const ce of consoleLog) {
    if (ce.level === 'error') issues.push({ surface, step: 'console-error', expected: 'no console.error', actual: ce.text, evidence: 'console', severity: '🟠', hypothesis: 'API/component error logged' });
  }
}

test.describe('FE-Radar — Full Surface Scan', () => {
  let ctx: BrowserContext;

  test.beforeAll(async ({ browser }) => {
    // This spec mints tokens via /dev/mock-auth — refuse to run against prod/unknown targets.
    requireStaging(BASE);
    ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
  });

  // ── SURFACE 1: PUBLIC MENU ──
  test('S1: Public Menu (client /s/:slug)', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    emit('menu', 'navigate', 'OK', 'Navigating to /s/demo');
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(3000);
    const body = await page.textContent('body');
    const bodyLen = body.length;
    const hasContent = body.includes('Menu') || body.includes('menu') || body.includes('Category') || body.includes('Lek') || body.includes('ALL');
    if (bodyLen > 100 && hasContent) {
      emit('menu', 'render', 'OK', `Menu rendered, ${bodyLen} chars`);
    } else {
      emit('menu', 'render', 'DIVERGENCE', `No menu text found (${bodyLen} chars)`);
      issues.push({ surface: 'menu', step: 'render', expected: 'Menu content rendered', actual: 'No menu text found', evidence: body.substring(0, 200), severity: '🔴', hypothesis: 'SSR shell rendered but menu fetch failed or hydration error' });
    }
    checkIssues('menu', page);
    await page.close();
  });

  // ── SURFACE 2: CHECKOUT ──
  test('S2: Checkout /s/:slug/checkout', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    emit('checkout', 'navigate', 'OK', 'Navigating to /s/demo/checkout');
    // Add an item to the cart via localStorage so checkout renders the FORM (not the
    // empty-cart EmptyState). The cart key is `dos_cart_<locationId>` (CartProvider) — a UUID
    // resolved at runtime, NOT the slug. Read the key the provider persists on mount, then
    // inject a valid CartItem into THAT key (the old `dowiz_cart_demo` key never landed).
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000); // let CartProvider mount and persist its dos_cart_<id> key
    const cartKey = await page.evaluate(() =>
      Object.keys(localStorage).find((k) => k.startsWith('dos_cart_')) || null,
    );
    expect(cartKey, 'CartProvider must persist a dos_cart_<locationId> key once the menu resolves').not.toBeNull();
    await page.evaluate((key) => {
      localStorage.setItem(
        key as string,
        JSON.stringify({ version: 1, items: [{ id: 'test_item_1', productId: 'test', name: 'Test Item', quantity: 1, price: 500 }], pricedVersion: null }),
      );
    }, cartKey);
    await page.goto(`${BASE}/s/demo/checkout`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);
    // A non-empty cart MUST render the form (phone field + total); an empty cart shows the
    // EmptyState with no phone field. Assert the real form element, not loose body text.
    await expect(page.locator('[data-testid=checkout-phone]'), 'checkout form (phone field) must render with a non-empty cart').toBeVisible();
    await expect(page.locator('[data-testid=checkout-total]'), 'checkout total must render with a non-empty cart').toBeVisible();
    emit('checkout', 'render', 'OK', 'Checkout form rendered');
    checkIssues('checkout', page);
    await page.close();
  });

  // ── SURFACE 3: ORDER STATUS ──
  test('S3: Order Status /s/:slug/order/:id', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    emit('order-status', 'navigate', 'OK', 'Navigating to /s/demo/order/test-123');
    await page.goto(`${BASE}/s/demo/order/test-123`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);
    // A non-existent order id MUST resolve to the not-found / link-expired EmptyState, which
    // always renders the "back to menu" CTA. Assert that recognisable element — emitting 'OK'
    // unconditionally would pass on a spinner, blank shell, or 500.
    await expect(page.locator('[data-testid=order-back-to-menu]'), 'unknown order id must render the not-found EmptyState (back-to-menu CTA)').toBeVisible();
    emit('order-status', 'render', 'OK', 'Not-found state rendered');
    checkIssues('order-status', page);
    await page.close();
  });

  // ── SURFACE 4: ADMIN LOGIN ──
  test('S4: Admin Login', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    emit('admin-login', 'navigate', 'OK', 'Navigating to /login');
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(1000);
    const body = await page.textContent('body');
    emit('admin-login', 'render', body.includes('Login') || body.includes('login') || body.includes('Email') ? 'OK' : 'ISSUE', `Body: ${body.substring(0, 100)}`);
    checkIssues('admin-login', page);
    await page.close();
  });

  // ── SURFACE 5: ADMIN DASHBOARD (authenticated) ──
  test('S5: Admin Dashboard', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    // Login first via API
    const loginRes = await (await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' })).json();
    const token = loginRes.access_token;
    expectJwt(token, 'mock-auth access_token'); // '' / 'null' / an error string must not pass
    // TODO(needs_staging): negative control — this token is scoped to ONE tenant. Add an assertion
    // that navigating to a *second* real tenant's /admin resource is rejected (no cross-tenant read).
    // Requires a real 2nd-tenant id from a live staging seed; do not fake with a nil/zero id.
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate((t) => { localStorage.setItem('dos_access_token', t); }, token);
    emit('admin-dashboard', 'navigate', 'OK', 'Navigating to /admin with stored token');
    await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(3000);
    // The dashboard always mounts the realtime connection indicator; assert it instead of a
    // body-text OR-chain (an error page / login redirect shell would satisfy loose words).
    await expect(page.locator('[data-testid=ws-status-dot]'), 'authenticated dashboard must render the realtime status indicator').toBeVisible();
    emit('admin-dashboard', 'render', 'OK', 'Dashboard rendered');
    checkIssues('admin-dashboard', page);
    await page.close();
  });

  // ── SURFACE 6: ADMIN MENU MANAGER ──
  test('S6: Admin Menu Manager', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    const loginRes = await (await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' })).json();
    expectJwt(loginRes.access_token, 'mock-auth access_token');
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate((t) => { localStorage.setItem('dos_access_token', t); }, loginRes.access_token);
    emit('admin-menu', 'navigate', 'OK', 'Navigating to /admin/menu');
    await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(3000);
    // body.length > 200 passes on an error page / login shell / skeleton. Assert a real control.
    await expect(page.locator('[data-testid=kitchen-busy-toggle]'), 'menu manager must render its kitchen-busy control').toBeVisible();
    emit('admin-menu', 'render', 'OK', 'Menu manager rendered');
    checkIssues('admin-menu', page);
    await page.close();
  });

  // ── SURFACE 7: ADMIN SETTINGS ──
  test('S7: Admin Settings', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    const loginRes = await (await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' })).json();
    expectJwt(loginRes.access_token, 'mock-auth access_token');
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate((t) => { localStorage.setItem('dos_access_token', t); }, loginRes.access_token);
    emit('admin-settings', 'navigate', 'OK');
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(3000);
    await expect(page.locator('[data-testid=notif-categories]'), 'settings page must render its notification-category controls').toBeVisible();
    emit('admin-settings', 'render', 'OK', 'Settings rendered');
    checkIssues('admin-settings', page);
    await page.close();
  });

  // ── SURFACE 8: ADMIN BRANDING ──
  test('S8: Admin Branding', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    const loginRes = await (await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' })).json();
    expectJwt(loginRes.access_token, 'mock-auth access_token');
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate((t) => { localStorage.setItem('dos_access_token', t); }, loginRes.access_token);
    emit('admin-branding', 'navigate', 'OK');
    await page.goto(`${BASE}/admin/branding`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(3000);
    await expect(page.locator('[data-testid=branding-page]'), 'branding page container must render').toBeVisible();
    emit('admin-branding', 'render', 'OK', 'Branding rendered');
    checkIssues('admin-branding', page);
    await page.close();
  });

  // ── SURFACE 9: ADMIN COURIERS ──
  test('S9: Admin Couriers', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    const loginRes = await (await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' })).json();
    expectJwt(loginRes.access_token, 'mock-auth access_token');
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate((t) => { localStorage.setItem('dos_access_token', t); }, loginRes.access_token);
    emit('admin-couriers', 'navigate', 'OK');
    await page.goto(`${BASE}/admin/couriers`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(3000);
    // CouriersPage has no testid; assert its content-specific heading (not body.length).
    await expect(page.getByRole('heading', { name: /Couriers/i }), 'couriers page heading must render').toBeVisible();
    emit('admin-couriers', 'render', 'OK', 'Couriers rendered');
    checkIssues('admin-couriers', page);
    await page.close();
  });

  // ── SURFACE 10: ADMIN ANALYTICS ──
  test('S10: Admin Analytics', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    const loginRes = await (await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' })).json();
    expectJwt(loginRes.access_token, 'mock-auth access_token');
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.evaluate((t) => { localStorage.setItem('dos_access_token', t); }, loginRes.access_token);
    emit('admin-analytics', 'navigate', 'OK');
    await page.goto(`${BASE}/admin/analytics`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(3000);
    await expect(page.locator('[data-testid=kpi-card]').first(), 'analytics page must render at least one KPI card').toBeVisible();
    emit('admin-analytics', 'render', 'OK', 'Analytics rendered');
    checkIssues('admin-analytics', page);
    await page.close();
  });

  // ── SURFACE 11: COURIER LOGIN ──
  test('S11: Courier Login', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    emit('courier-login', 'navigate', 'OK');
    await page.goto(`${BASE}/courier/login`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);
    const body = await page.textContent('body');
    emit('courier-login', 'render', body.length > 100 ? 'OK' : 'ISSUE', `Body length: ${body.length}`);
    checkIssues('courier-login', page);
    await page.close();
  });

  // ── SURFACE 12: CSS/ASSETS CHECK ──
  test('S12: Assets & PWA', async () => {
    const page = await ctx.newPage();
    setupCollectors(page);
    emit('pwa', 'manifest', 'OK', 'Checking /manifest.json');
    const man = await page.goto(`${BASE}/manifest.json`);
    if (man?.status() === 200) emit('pwa', 'manifest', 'OK', '200');
    else { emit('pwa', 'manifest', 'ISSUE', `${man?.status()}`); issues.push({ surface: 'pwa', step: 'manifest', expected: '200', actual: `${man?.status()}`, evidence: 'network', severity: '🟠', hypothesis: 'PWA manifest missing or wrong path' }); }

    emit('pwa', 'sw', 'OK', 'Checking /sw.js');
    const sw = await page.goto(`${BASE}/sw.js`);
    if (sw?.status() === 200) emit('pwa', 'sw', 'OK', '200');
    else { emit('pwa', 'sw', 'ISSUE', `${sw?.status()}`); issues.push({ surface: 'pwa', step: 'sw', expected: '200', actual: `${sw?.status()}`, evidence: 'network', severity: '🟠', hypothesis: 'Service worker file missing' }); }

    // Check for 404 JS/CSS chunks (stale deploys)
    emit('pwa', 'chunks', 'OK', 'Checking for 404 chunks on menu page');
    await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);
    const js404s = networkLog.filter(n => n.type === 'script' && n.status === 404);
    if (js404s.length > 0) issues.push({ surface: 'pwa', step: 'stale-chunks', expected: 'all JS/CSS 200', actual: `${js404s.length} 404 on scripts`, evidence: JSON.stringify(js404s), severity: '🔴', hypothesis: 'Stale deployment — old chunk hashes in index.html' });
    await page.close();
  });

  // ── REPORT ──
  test.afterAll(() => {
    console.log('\n=== ISSUES FOUND ===\n');
    const grouped: Record<string, Issue[]> = {};
    for (const iss of issues) {
      if (!grouped[iss.surface]) grouped[iss.surface] = [];
      grouped[iss.surface].push(iss);
    }
    for (const [surface, issList] of Object.entries(grouped)) {
      console.log(`\n--- ${surface} ---`);
      for (const iss of issList) {
        console.log(`${iss.severity} ${iss.step}: ${iss.actual.substring(0, 120)}`);
        console.log(`   expected: ${iss.expected}`);
        console.log(`   hypothesis: ${iss.hypothesis}`);
      }
    }
    console.log(`\nTotal: ${issues.length} issues across ${Object.keys(grouped).length} surfaces`);
  });
});
