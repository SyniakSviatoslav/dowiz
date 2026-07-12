/* eslint-disable @typescript-eslint/no-unused-vars, max-params -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect, type Page, type BrowserContext } from '@playwright/test';
import { checkAxe, checkTouchTargets, checkFormLabels, checkAriaLive, type A11yIssue } from '../helpers/a11y.js';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

interface Issue {
  surface: string; step: string; dimension: 'base' | 'a11y' | 'throttled';
  expected: string; actual: string; evidence: string; severity: '🔴' | '🟠' | '🟡' | '🔵' | '⚪'; hypothesis: string;
}

const issues: Issue[] = [];
const pageErrors: string[] = [];

const NET_PROFILES = ['fast', 'slow-3g'] as const;
const VIEWPORTS = [
  { name: 'mobile', width: 390, height: 844 },
  { name: 'desktop', width: 1280, height: 800 },
];

function emit(surface: string, step: string, status: string, detail: string) {
  console.log(`${surface}|${step}|${status}|${detail}`);
}

async function setupPage(ctx: BrowserContext, profile: typeof NET_PROFILES[number], vp: typeof VIEWPORTS[0]): Promise<Page> {
  const page = await ctx.newPage();
  if (profile === 'slow-3g') {
    await page.route('**/*', async (route) => {
      await new Promise(r => setTimeout(r, 400)); // 400ms delay per request
      await route.continue();
    });
  }
  return page;
}

function record(surface: string, step: string, dimension: Issue['dimension'], severity: Issue['severity'],
  expected: string, actual: string, hypothesis: string) {
  issues.push({ surface, step, dimension, expected, actual, evidence: actual.substring(0, 200), severity, hypothesis });
}

function checkConsoleAndNetwork(surface: string, dimension: Issue['dimension'], page: Page) {
  // Page errors
  for (const pe of pageErrors) {
    record(surface, 'js-error', dimension, '🔴', 'no uncaught exceptions', pe, 'Unhandled JS exception');
  }
}

test.describe('FE-Radar v2 — Full Surface + a11y + Throttled', () => {
  let ctx: BrowserContext;

  test.beforeAll(async ({ browser }) => {
    ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
  });

  for (const vp of VIEWPORTS) {
    for (const profile of NET_PROFILES) {
      test(`[${vp.name}/${profile}] S1: Public Menu /s/demo`, async () => {
        const page = await setupPage(ctx, profile, vp);
        emit('menu', `navigate-${vp.name}-${profile}`, 'OK', '');
        const start = Date.now();
        await page.goto(`${BASE}/s/demo`, { waitUntil: 'networkidle', timeout: 60000 });
        const loadMs = Date.now() - start;

        // Render check
        const body = await page.textContent('body');
        if (body.length > 100) emit('menu', `render-${profile}`, 'OK', `${body.length} chars`);
        else record('menu', `render-${profile}`, 'base', '🔴', 'Body > 100 chars', `${body.length} chars`, 'Page empty');

        // Slow-3g: check loading indicator
        if (profile === 'slow-3g' && loadMs > 2000) {
          const hasSpinner = await page.locator('[class*="spinner"], [class*="skeleton"], [class*="loading"]').count();
          if (hasSpinner === 0) record('menu', 'loading-indicator', 'throttled', '🟠',
            'Loading skeleton/spinner visible during slow fetch', 'No loading indicator found', 'Surface shows blank while waiting for data');
        }

        // a11y checks on mobile baseline
        if (profile === 'fast' && vp.name === 'mobile') {
          const axeIssues = await checkAxe(page);
          for (const ai of axeIssues) {
            const sev = ai.impact === 'critical' ? '🔴' : ai.impact === 'serious' ? '🟠' : '🟡';
            record('menu', `axe-${ai.id}`, 'a11y', sev, `0 violations`, `${ai.nodes}x ${ai.description}`, ai.help);
          }

          const touch = await checkTouchTargets(page);
          const touchSize = touch.filter(t => t.startsWith('size:')).length;
          const touchProx = touch.filter(t => t.startsWith('proximity:')).length;
          if (touchSize > 0) record('menu', 'touch-target-size', 'a11y', '🟠', 'All touch targets ≥44px', `${touchSize} too small`, 'Hard to tap on mobile 390');
          if (touchProx > 0) record('menu', 'touch-target-proximity', 'a11y', '🟡', 'Targets not overlapping', `${touchProx} too close`, 'Adjacent targets may cause mis-taps');

          const formLabels = await checkFormLabels(page);
          if (formLabels.length > 0) record('menu', 'form-labels', 'a11y', '🟠', 'All inputs have labels', `${formLabels.length} missing`, formLabels[0]);
        }

        checkConsoleAndNetwork('menu', 'base', page);
        await page.close();
      });

      test(`[${vp.name}/${profile}] S2: Checkout /s/demo/checkout`, async () => {
        const page = await setupPage(ctx, profile, vp);
        emit('checkout', `navigate-${vp.name}-${profile}`, 'OK', '');
        await page.goto(`${BASE}/s/demo/checkout`, { waitUntil: 'networkidle', timeout: 60000 });

        if (profile === 'fast' && vp.name === 'mobile') {
          const axeIssues = await checkAxe(page);
          for (const ai of axeIssues) {
            const sev = ai.impact === 'critical' ? '🔴' : ai.impact === 'serious' ? '🟠' : '🟡';
            record('checkout', `axe-${ai.id}`, 'a11y', sev, `0 violations`, `${ai.nodes}x ${ai.description}`, ai.help);
          }
          const labels = await checkFormLabels(page);
          if (labels.length > 0) record('checkout', 'form-labels', 'a11y', '🟠', 'All inputs labeled', `${labels.length} missing: ${labels[0]}`, labels[0]);
        }

        await page.close();
      });

      test(`[${vp.name}/${profile}] S3: Order Status /s/demo/order/test-123`, async () => {
        const page = await setupPage(ctx, profile, vp);
        await page.goto(`${BASE}/s/demo/order/test-123`, { waitUntil: 'networkidle', timeout: 60000 });

        if (profile === 'fast' && vp.name === 'mobile') {
          const axeIssues = await checkAxe(page);
          for (const ai of axeIssues) {
            const sev = ai.impact === 'critical' ? '🔴' : ai.impact === 'serious' ? '🟠' : '🟡';
            record('order-status', `axe-${ai.id}`, 'a11y', sev, `0 violations`, `${ai.nodes}x ${ai.description}`, ai.help);
          }
          const hasLive = await checkAriaLive(page);
          if (!hasLive) record('order-status', 'aria-live', 'a11y', '🟡', 'aria-live region for status updates', 'None found', 'Status changes not announced to screen readers');
        }

        await page.close();
      });

      test(`[${vp.name}/${profile}] S4: Admin Login`, async () => {
        const page = await setupPage(ctx, profile, vp);
        await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 60000 });

        if (profile === 'fast' && vp.name === 'mobile') {
          const axeIssues = await checkAxe(page);
          for (const ai of axeIssues) {
            const sev = ai.impact === 'critical' ? '🔴' : ai.impact === 'serious' ? '🟠' : '🟡';
            record('admin-login', `axe-${ai.id}`, 'a11y', sev, `0 violations`, `${ai.nodes}x ${ai.description}`, ai.help);
          }
          const labels = await checkFormLabels(page);
          if (labels.length > 0) record('admin-login', 'form-labels', 'a11y', '🟠', 'All inputs labeled', `${labels.length} missing`, labels[0]);
          const touch = await checkTouchTargets(page);
          const touchSize = touch.filter(t => t.startsWith('size:')).length;
          if (touchSize > 0) record('admin-login', 'touch-targets', 'a11y', '🟠', 'All ≥44px', `${touchSize} too small`, '');
        }

        await page.close();
      });

      test(`[${vp.name}/${profile}] S5: Admin Dashboard`, async () => {
        const page = await setupPage(ctx, profile, vp);
        const auth = await (await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' })).json();
        await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
        await page.evaluate((t) => { localStorage.setItem('dos_access_token', t); }, auth.access_token);
        await page.goto(`${BASE}/admin`, { waitUntil: 'networkidle', timeout: 60000 });
        await page.waitForTimeout(2000);

        if (profile === 'fast' && vp.name === 'mobile') {
          const axeIssues = await checkAxe(page);
          for (const ai of axeIssues) {
            const sev = ai.impact === 'critical' ? '🔴' : ai.impact === 'serious' ? '🟠' : '🟡';
            record('admin-dashboard', `axe-${ai.id}`, 'a11y', sev, `0 violations`, `${ai.nodes}x ${ai.description}`, ai.help);
          }
          const hasLive = await checkAriaLive(page);
          if (!hasLive) record('admin-dashboard', 'aria-live', 'a11y', '🟡', 'aria-live for real-time order updates', 'None found', 'New orders not announced to screen readers');
        }

        await page.close();
      });

      test(`[${vp.name}/${profile}] S6: Admin Settings`, async () => {
        const page = await setupPage(ctx, profile, vp);
        const auth = await (await fetch(`${BASE}/api/dev/mock-auth`, { method: 'POST' })).json();
        await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 30000 });
        await page.evaluate((t) => { localStorage.setItem('dos_access_token', t); }, auth.access_token);
        await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle', timeout: 60000 });
        await page.waitForTimeout(2000);

        if (profile === 'fast' && vp.name === 'mobile') {
          const axeIssues = await checkAxe(page);
          for (const ai of axeIssues) {
            const sev = ai.impact === 'critical' ? '🔴' : ai.impact === 'serious' ? '🟠' : '🟡';
            record('admin-settings', `axe-${ai.id}`, 'a11y', sev, `0 violations`, `${ai.nodes}x ${ai.description}`, ai.help);
          }
        }

        await page.close();
      });

      test(`[${vp.name}/${profile}] S7: Courier Login`, async () => {
        const page = await setupPage(ctx, profile, vp);
        await page.goto(`${BASE}/courier/login`, { waitUntil: 'networkidle', timeout: 60000 });
        const body = await page.textContent('body');

        if (profile === 'fast' && vp.name === 'mobile') {
          const axeIssues = await checkAxe(page);
          for (const ai of axeIssues) {
            const sev = ai.impact === 'critical' ? '🔴' : ai.impact === 'serious' ? '🟠' : '🟡';
            record('courier-login', `axe-${ai.id}`, 'a11y', sev, `0 violations`, `${ai.nodes}x ${ai.description}`, ai.help);
          }
        }
        await page.close();
      });
    }
  }

  // ── REPORT ──
  test.afterAll(() => {
    console.log('\n=== FE RADAR v2 REPORT ===');
    const grouped: Record<string, Issue[]> = {};
    for (const iss of issues) {
      const key = `${iss.surface}/${iss.dimension}`;
      if (!grouped[key]) grouped[key] = [];
      grouped[key].push(iss);
    }
    const sevCounts = { '🔴': 0, '🟠': 0, '🟡': 0, '🔵': 0 };
    for (const iss of issues) { sevCounts[iss.severity]++; }

    console.log(`\nTotal issues: ${issues.length}`);
    console.log(`  🔴 Critical: ${sevCounts['🔴']}`);
    console.log(`  🟠 Serious: ${sevCounts['🟠']}`);
    console.log(`  🟡 Moderate: ${sevCounts['🟡']}`);

    for (const [key, issList] of Object.entries(grouped)) {
      console.log(`\n--- ${key} ---`);
      for (const iss of issList) {
        console.log(`${iss.severity} [${iss.dimension}] ${iss.step}: ${iss.actual.substring(0, 100)}`);
        if (iss.expected) console.log(`   expected: ${iss.expected}`);
        console.log(`   hypothesis: ${iss.hypothesis}`);
      }
    }
  });
});
