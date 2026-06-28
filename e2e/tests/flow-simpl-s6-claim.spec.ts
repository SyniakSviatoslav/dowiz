import { test, expect } from '@playwright/test';

// flow-simplification §6 — owner claim page, happy path against a REAL provisioned shadow.
// The invite is minted (out of band) with invited_contact = the mock-auth dev owner's email
// (dev@deliveryos.com) so claim_transfer's contact-hash check passes; E2E_CLAIM_TOKEN carries the raw token.
// Asserts: the token is SCRUBBED from the URL (fragment transport), and an authenticated owner can claim →
// ownership transfers → "review & publish" (3-act: claim ≠ publish).
const TOKEN = process.env.E2E_CLAIM_TOKEN || '';

test('§6 · authenticated owner claims a shadow via the fragment-token page (claim ≠ publish)', async ({ page, request }) => {
  test.skip(!TOKEN, 'no E2E_CLAIM_TOKEN provisioned');

  // mock-auth dev owner (email dev@deliveryos.com — matches the invite contact) → an owner JWT.
  const owner = await (await request.post('/api/dev/mock-auth', { data: {} })).json();
  await page.addInitScript((tok) => { window.localStorage.setItem('dos_access_token', tok); }, owner.access_token);

  await page.goto(`/claim#token=${TOKEN}`);

  // Token-safe transport: the page scrubs the token from the visible URL immediately.
  await expect(page.getByTestId('claim-heading')).toBeVisible({ timeout: 20_000 });
  expect(page.url(), 'token must be scrubbed from the URL (no query/fragment leak)').not.toContain('token=');

  // Claim → ownership transfers; the success screen routes to review & publish (NOT auto-published).
  await page.getByTestId('claim-accept').click();
  await expect(page.getByTestId('claim-go-admin')).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText(/review & publish/i)).toBeVisible();
});
