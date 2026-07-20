/**
 * Seed test data for deterministic E2E testing.
 * 
 * These functions call the dev mock endpoints or real seed endpoints
 * to create a known state before each test.
 */

const API_BASE = process.env.API_BASE_URL || 'http://localhost:3000/api';

export interface SeedOrder {
  orderId: string;
  status: string;
}

/**
 * Seed a test order for status tracking.
 */
export async function seedOrder(locationId: string): Promise<SeedOrder> {
  try {
    const res = await fetch(`${API_BASE}/dev/seed/order`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ locationId }),
    });
    if (res.ok) return await res.json() as SeedOrder;
  } catch {
    // Fallback to test defaults
    console.debug('[e2e:seed] seed endpoint unavailable, using test defaults');
  }
  return { orderId: 'o_test_1', status: 'PENDING' };
}

/**
 * Fixed time helper — returns a known timestamp for deterministic tests.
 */
export function fixedTime(): number {
  return new Date('2026-06-04T12:00:00Z').getTime();
}

/**
 * Generate a deterministic idempotency key for testing.
 */
export function idempotencyKey(prefix: string = 'test'): string {
  return `${prefix}_${fixedTime()}`;
}
