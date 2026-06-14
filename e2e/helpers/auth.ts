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
    console.debug('[e2e:auth] dev auth endpoint unavailable, using fallback token');
  }
  // Fallback: return a hardcoded dev JWT for testing
  // This is a test-only token, not used in production
  return 'dev_test_token';
}


