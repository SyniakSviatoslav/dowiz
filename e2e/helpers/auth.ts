import { Page, BrowserContext } from '@playwright/test';

const API_BASE = process.env.API_BASE_URL || 'http://localhost:3000/api';

export interface TestAccount {
  role: 'client' | 'owner' | 'courier';
  token: string;
  slug?: string;
  locationId?: string;
  courierId?: string;
}

/**
 * Inject a JWT token into the page's localStorage for dev auth.
 * The React app reads `dos_access_token` from localStorage.
 */
export async function loginAs(page: Page, account: TestAccount): Promise<void> {
  await page.goto('/');
  await page.evaluate((token) => {
    localStorage.setItem('dos_access_token', token);
    localStorage.setItem('dos_role', 'owner');
  }, account.token);
}

/**
 * Get a dev JWT for testing. In production this would be a real auth flow.
 * For now, uses the dev mock auth endpoint if available.
 */
export async function getDevToken(role: 'client' | 'owner' | 'courier'): Promise<string> {
  try {
    const res = await fetch(`${API_BASE}/auth/dev/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ role }),
    });
    if (res.ok) {
      const data = await res.json() as { token: string };
      return data.token;
    }
  } catch {
    // Dev auth may not be available
  }
  // Fallback: return a hardcoded dev JWT for testing
  // This is a test-only token, not used in production
  return 'dev_test_token';
}

/**
 * Set up auth state for a specific role.
 * Uses Playwright's storageState mechanism — no cookies allowed.
 */
export async function setupAuthState(page: Page, account: TestAccount): Promise<void> {
  await page.evaluate(({ token, role, slug }) => {
    localStorage.setItem('dos_access_token', token);
    if (role) localStorage.setItem('dos_role', role);
    if (slug) localStorage.setItem('dos_slug', slug);
  }, { token: account.token, role: account.role, slug: account.slug });
}
