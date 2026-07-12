import { test, expect } from '@playwright/test';
import { requireStaging } from '../helpers/staging-guard';

// Soft access gate UI proofs (ADR-soft-access-gate). The CTA is DARK by default
// (ACCESS_GATE_PUBLIC_ENABLED=false) and ships only after owner-onboarding-invite-gating
// (STOP-1). So:
//   • the /privacy proofs run anywhere the build is deployed (the route is ungated);
//   • the form proofs self-skip unless the gate is enabled on the target build.
// Run (post-launch, flag on): VITE_BASE_URL=https://dowiz.fly.dev pnpm exec playwright
//   test soft-access-gate --reporter=list

test.describe('Soft access gate — /privacy (ungated, GDPR consent link target)', () => {
  test('privacy page renders consent basis, 12-month retention, and a working erasure contact', async ({ page }) => {
    await page.goto('/privacy');
    await expect(page.locator('[data-testid="privacy-page"]')).toBeVisible({ timeout: 15000 });
    // basis = consent, retention = 12 months from first contact, reachable erasure contact
    await expect(page.getByText(/consent|pëlqim|згод/i).first()).toBeVisible();
    await expect(page.getByText(/12\s*(months|muaj|місяц)/i).first()).toBeVisible();
    const contact = page.locator('[data-testid="privacy-erasure-contact"]');
    await expect(contact).toBeVisible();
    await expect(contact).toHaveAttribute('href', /^mailto:/);
  });
});

test.describe('Soft access gate — CTA form (runs only when ACCESS_GATE_PUBLIC_ENABLED=true)', () => {
  test('consent not pre-checked; submit disabled until ticked; success on a real consented submit', async ({ page }) => {
    await page.goto('/start');
    const form = page.locator('[data-testid="access-request-form"]');
    if (!(await form.isVisible().catch(() => false))) {
      test.skip(true, 'access gate disabled on this build (ACCESS_GATE_PUBLIC_ENABLED=false) — CTA correctly not rendered');
    }
    // Gate is ON → this submit INSERTs into access_requests; refuse to mutate prod/unknown.
    requireStaging(process.env.VITE_BASE_URL);
    // Counsel R2 #4 — no pre-check dark pattern.
    const consent = page.locator('[data-testid="access-request-consent"]');
    await expect(consent).not.toBeChecked();
    // disabled until ticked
    const submit = page.locator('[data-testid="access-request-submit"]');
    await expect(submit).toBeDisabled();
    // Finding #3 — unique email per run exercises the genuine NEW-insert path; a static
    // address would hit ON CONFLICT (email) DO NOTHING forever and never prove a fresh row.
    await page.locator('[data-testid="access-request-email"]').fill(`e2e+${Date.now()}@dowiz.dev`);
    await consent.check();
    await expect(submit).toBeEnabled();
    await submit.click();
    const success = page.locator('[data-testid="access-request-success"]');
    await expect(success).toBeVisible({ timeout: 10000 });
    // Finding #1 — assert the success element carries the real "being heard" copy, not a
    // hollow placeholder or a stale fragment from a previous render.
    await expect(success).toContainText(/got your email|be in touch|thank/i);
    // Finding #2 — a network error renders access-request-error, NOT success; assert it is
    // absent so the success above cannot be a false-positive from a failed submission.
    await expect(page.locator('[data-testid="access-request-error"]')).not.toBeVisible();
    // anti-enumeration: success copy is "being heard", never scarcity.
    await expect(page.locator('body')).not.toContainText(/waitlist|approved|under review|position #/i);
    // TODO(needs_staging): findings #3 (duplicate email) and #4 (server-side consent=true
    // persistence) are anti-enumeration uniform-200s — black-box-identical to a new submit.
    // Proving "duplicate added no 2nd row" and "no-consent POST inserted ZERO rows" requires
    // staging access_requests table introspection (POST /api/access-requests is 200 either way).
  });

  test('the /privacy link in the consent label resolves to a real page (not 404)', async ({ page }) => {
    await page.goto('/start');
    const link = page.locator('[data-testid="access-request-privacy-link"]');
    if (!(await link.isVisible().catch(() => false))) {
      test.skip(true, 'access gate disabled on this build — CTA not rendered');
    }
    await link.click();
    await expect(page).toHaveURL(/\/privacy/);
    await expect(page.locator('[data-testid="privacy-page"]')).toBeVisible();
  });
});
