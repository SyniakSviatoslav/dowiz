/**
 * Critical-Path Visual Regression Net — shared harness (the contract every spec builds against).
 *
 * Reuses the existing dev seams: /api/dev/mock-auth (role login) and the new dev-gated
 * /api/dev/seed-visual-state (deterministic fixtures). All dev endpoints 404 without DEV_AUTH_SECRET.
 */
import type { Page, APIRequestContext, Locator } from '@playwright/test';

export type Role = 'owner' | 'courier';

/** Deterministic fixtures seeded once in globalSetup; slugs/ids are stable across runs. */
export interface VisualFixtures {
  open: { slug: string; locationId: string };     // normal open venue: products + modifiers
  closed: { slug: string; locationId: string };   // outside opening hours
  busy: { slug: string; locationId: string };     // busy_mode on
  stoplistProductId: string;                       // a product flagged unavailable (86'd)
  orderId: string;                                 // a seeded order for the status screen
  courierId: string;
}

const SECRET = process.env.DEV_AUTH_SECRET || '';
const devHeaders = { 'content-type': 'application/json', 'x-dev-auth-secret': SECRET };

/** Seed (idempotent) the visual fixtures. Returns stable slugs/ids. Run from globalSetup. */
export async function seedVisualState(request: APIRequestContext): Promise<VisualFixtures> {
  const res = await request.post('/api/dev/seed-visual-state', { headers: devHeaders, data: {} });
  if (!res.ok()) throw new Error(`seed-visual-state failed: ${res.status()} ${await res.text()}`);
  return (await res.json()) as VisualFixtures;
}

/** Log in as a role via the existing mock-auth dev endpoint; returns the bearer token + active location. */
export async function loginAs(
  request: APIRequestContext,
  role: Role,
  opts: { locationSlug?: string } = {},
): Promise<{ token: string; activeLocationId?: string; userId: string }> {
  const res = await request.post('/api/dev/mock-auth', { headers: devHeaders, data: { role, ...opts } });
  if (!res.ok()) throw new Error(`mock-auth(${role}) failed: ${res.status()} ${await res.text()}`);
  const j = await res.json();
  return { token: j.access_token, activeLocationId: j.activeLocationId, userId: j.userId };
}

/** Put a role token into localStorage so the SPA boots authenticated (matches the app's auth storage). */
export async function applyAuth(page: Page, token: string): Promise<void> {
  await page.addInitScript((t) => {
    try { localStorage.setItem('dos_access_token', t as string); } catch { /* ignore */ }
  }, token);
}

/** Switch the SPA UI locale deterministically before a snapshot ('al' or 'en'). */
export async function setLocale(page: Page, locale: 'al' | 'en'): Promise<void> {
  // i18n persists to localStorage key 'dos_locale' (sq = Albanian, en = English).
  const code = locale === 'al' ? 'sq' : 'en';
  await page.addInitScript((c) => { try { localStorage.setItem('dos_locale', c as string); } catch { /* */ } }, code);
}

/** The single dynamic-zone mask. Components that vary run-to-run carry data-dynamic (see masking audit). */
export function MASK(page: Page): Locator[] {
  return [page.locator('[data-dynamic]')];
}

/** Standard pre-snapshot settle: wait for network idle + fonts, then a short paint settle. */
export async function settle(page: Page): Promise<void> {
  await page.waitForLoadState('networkidle').catch(() => {});
  await page.evaluate(() => (document as any).fonts?.ready).catch(() => {});
  await page.waitForTimeout(300);
}

export const BREAKPOINTS = [390, 768, 1280] as const;
export const LOCALES = ['al', 'en'] as const;
