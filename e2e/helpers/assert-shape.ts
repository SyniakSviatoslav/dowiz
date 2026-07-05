import { expect } from '@playwright/test';

// Shape assertions for Test Integrity (no `.toBeTruthy()` on tokens/ids — '' / 'null' /
// an error string would pass). Assert the actual SHAPE instead.
const JWT = /^[\w-]+\.[\w-]+\.[\w-]+$/;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Assert `token` is a 3-segment JWT. */
export function expectJwt(token: unknown, label = 'token'): void {
  expect(String(token ?? ''), `${label} must be a 3-segment JWT`).toMatch(JWT);
}

/** Assert `id` is a UUID. */
export function expectUuid(id: unknown, label = 'id'): void {
  expect(String(id ?? ''), `${label} must be a UUID`).toMatch(UUID);
}
