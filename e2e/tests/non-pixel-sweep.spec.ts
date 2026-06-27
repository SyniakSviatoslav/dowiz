/**
 * Non-Pixel Sweep — the discovery engine behind the four senses.
 *
 * Walks every primary surface across all THREE roles (client / owner / courier),
 * mobile-first, and at each surface records the a11y-tree violations (Sense 1)
 * and the console/runtime stream (Sense 2) into a findings ledger
 * (e2e/artifacts/non-pixel-sweep.json) + a live video per role journey
 * (video:'on' in playwright.config). This is DIAGNOSTIC (collect-mode, never
 * weakens a gate) — it surfaces the full cross-role findings set the committed
 * per-flow gates then enforce.
 *
 * Run (mobile-first, against staging):
 *   DEV_AUTH_SECRET=stg-e2e-secret VITE_BASE_URL=https://dowiz-staging.fly.dev \
 *   pnpm exec playwright test e2e/tests/non-pixel-sweep.spec.ts --project=mobile --reporter=list
 */
import { test, expect } from '@playwright/test';
import { writeFileSync, mkdirSync } from 'node:fs';
import { attachConsoleGuard, stripCrossOriginAuth } from '../fixtures/console-guard';
import { checkAxe, type A11yIssue } from '../helpers/a11y';
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const API = `${BASE}/api`;
const SLUG = process.env.DEMO_SLUG || 'demo';

interface SurfaceFinding {
  role: string;
  surface: string;
  url: string;
  axe: A11yIssue[];
  console: string[];
  reachedDom: boolean;
}
const ledger: SurfaceFinding[] = [];

async function mockAuth(role: 'owner' | 'courier'): Promise<string | null> {
  const body = role === 'courier' ? { role: 'courier', synthetic: true } : {};
  try {
    const res = await fetch(`${API}/dev/mock-auth`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(process.env.DEV_AUTH_SECRET ? { 'x-dev-auth-secret': process.env.DEV_AUTH_SECRET } : {}),
      },
      body: JSON.stringify(body),
    });
    if (!res.ok) return null;
    const data = (await res.json()) as { access_token?: string };
    return data.access_token ?? null;
  } catch {
    return null;
  }
}

/** Visit a surface, optionally run an interaction, record axe + console. */
async function probe(
  page: import('@playwright/test').Page,
  role: string,
  surface: string,
  path: string,
  interact?: (p: import('@playwright/test').Page) => Promise<void>,
) {
  const guard = attachConsoleGuard(page);
  const url = `${BASE}${path}`;
  let reachedDom = false;
  try {
    await page.goto(url, { waitUntil: 'networkidle', timeout: 25000 });
    reachedDom = await page
      .locator('main, [role="main"], h1, form, [data-testid]')
      .first()
      .isVisible({ timeout: 8000 })
      .catch(() => false);
    if (interact)
      await interact(page).catch((e) =>
        // a failed interaction is itself a finding — record it (don't swallow) so the
        // afterAll ledger surfaces it instead of staying silently green.
        guard.errors.push(`[interaction] ${e instanceof Error ? e.message : String(e)}`),
      );
    await page.waitForTimeout(500);
  } catch {
    /* navigation issue is itself a finding (reachedDom=false) */
  }
  let axe: A11yIssue[] = [];
  try {
    axe = await checkAxe(page);
  } catch {
    /* axe needs a context page; skip on hard failure */
  }
  ledger.push({ role, surface, url, axe, console: [...guard.errors], reachedDom });
  const crit = axe.filter((a) => a.impact === 'critical' || a.impact === 'serious').length;
  // eslint-disable-next-line no-console
  console.log(
    `  [${role}] ${surface}: dom=${reachedDom} axe(crit/ser)=${crit}/${axe.length} console=${guard.errors.length}`,
  );
}

test.describe('Non-Pixel Sweep (3 roles, mobile-first)', () => {
  // The sweep uses the /dev/mock-auth backdoor — refuse to run against prod/unknown targets.
  test.beforeAll(() => {
    requireStaging(BASE);
  });
  test.beforeEach(async ({ page }) => {
    await stripCrossOriginAuth(page); // don't leak the dev-auth header cross-origin
  });

  test('client journey', async ({ page }) => {
    await probe(page, 'client', 'storefront', `/s/${SLUG}`, async (p) => {
      const item = p.locator('[data-testid="category-nav"] ~ * button, article button, [role="button"]').first();
      if (await item.count()) await item.click({ timeout: 4000 }).catch((e) => { void e; /* tolerated: best-effort tap to surface post-interaction a11y/console findings; click miss must not fail the sweep */ });
    });
    await probe(page, 'client', 'checkout', `/s/${SLUG}/checkout`);
  });

  test('owner journey', async ({ page, context }) => {
    const token = await mockAuth('owner');
    // Finding 2: a null/failed mock-auth token would silently probe public/redirect surfaces
    // instead of the authed owner UI. Fail loud so the journey can't pass unauthenticated.
    expectJwt(token, 'owner mock-auth token');
    await context.addInitScript((t) => localStorage.setItem('dos_access_token', t), token as string);
    // TODO(needs_staging): cross-tenant isolation negative control — with a REAL second
    // tenant's owner token, assert this token cannot read tenant-B's /api/owner/orders
    // (expect 403/404, never tenant-B rows). Requires a seeded 2nd tenant; do not fake with nil-UUID.
    for (const [surface, path] of [
      ['dashboard', '/admin'],
      ['menu', '/admin/menu'],
      ['orders', '/admin/orders'],
      ['analytics', '/admin/analytics'],
      ['settings', '/admin/settings'],
      // round 2 — previously unexamined owner surfaces
      ['activation', '/admin/activation'],
      ['supplies', '/admin/supplies'],
      ['promotions', '/admin/promotions'],
      ['couriers', '/admin/couriers'],
      ['crm', '/admin/crm'],
      ['branding', '/admin/branding'],
    ] as const) {
      await probe(page, 'owner', surface, path);
    }
  });

  test('courier journey', async ({ page, context }) => {
    const token = await mockAuth('courier');
    expectJwt(token, 'courier mock-auth token'); // Finding 2: no silent unauthenticated probing
    await context.addInitScript((t) => localStorage.setItem('dos_access_token', t), token as string);
    for (const [surface, path] of [
      ['tasks', '/courier'],
      ['shift', '/courier/shift'],
      ['earnings', '/courier/earnings'],
      ['history', '/courier/history'], // round 2
    ] as const) {
      await probe(page, 'courier', surface, path);
    }
  });

  test('public journey', async ({ page }) => {
    // round 2 — public/unauthenticated surfaces (no token)
    for (const [surface, path] of [
      ['landing', '/'],
      ['start', '/start'],
      ['login', '/login'],
      ['privacy', '/privacy'],
      ['not-found', '/this-route-does-not-exist'],
    ] as const) {
      await probe(page, 'public', surface, path);
    }
  });

  test.afterAll(() => {
    mkdirSync('e2e/artifacts', { recursive: true });
    const summary = ledger.map((f) => ({
      role: f.role,
      surface: f.surface,
      reachedDom: f.reachedDom,
      axeCritSerious: f.axe.filter((a) => a.impact === 'critical' || a.impact === 'serious').length,
      axeTotal: f.axe.length,
      consoleErrors: f.console.length,
      topAxe: f.axe.slice(0, 5).map((a) => `${a.id}(${a.impact}×${a.nodes})`),
      topConsole: f.console.slice(0, 4),
    }));
    writeFileSync('e2e/artifacts/non-pixel-sweep.json', JSON.stringify({ base: BASE, ledger: summary }, null, 2));
    // eslint-disable-next-line no-console
    console.log('\n=== NON-PIXEL SWEEP LEDGER ===\n' + JSON.stringify(summary, null, 2));
    expect(ledger.length).toBeGreaterThan(0);
    // Finding 1: ledger.length alone stayed green when a surface 404'd or crashed. Assert the
    // actual sense outputs: every probed surface must render real DOM, and none may carry a
    // critical/serious a11y violation (a red here is a PRODUCT finding to escalate, not to weaken).
    const noDom = ledger.filter((f) => !f.reachedDom).map((f) => `${f.role}/${f.surface}`);
    expect(noDom, 'surfaces that failed to render real DOM (404/crash)').toEqual([]);
    const axeOffenders = ledger
      .filter((f) => f.axe.some((a) => a.impact === 'critical' || a.impact === 'serious'))
      .map((f) => `${f.role}/${f.surface}`);
    expect(axeOffenders, 'surfaces with critical/serious a11y violations').toEqual([]);
  });
});
