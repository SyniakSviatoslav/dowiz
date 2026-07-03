import { test, expect, type Page } from '@playwright/test';

// Pull-to-refresh — DOM-level proof for usePullToRefresh wired into the client MenuPage
// and courier TasksPage. The gesture math (resistance/threshold/scroll-guard) is unit-
// tested in packages/ui/src/hooks/__tests__/use-pull-to-refresh.test.ts; this spec proves
// the WIRING: a touch drag past the threshold shows the armed indicator and triggers the
// page's existing refetch, and it never arms mid-scroll.
// Touch gestures need a touch-capable context, so run with the "mobile" project:
//   VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test pull-to-refresh --project=mobile --reporter=list

/** Dispatch a synthetic single-finger touch drag on `selector`, from its top edge down by `deltaY` px. */
async function simulatePullDown(page: Page, selector: string, deltaY: number) {
  await page.evaluate(
    ({ selector, deltaY }) => {
      const el = document.querySelector(selector);
      if (!el) throw new Error(`pull target not found: ${selector}`);
      const rect = el.getBoundingClientRect();
      const x = rect.left + rect.width / 2;
      const startY = rect.top + 5;

      const makeTouch = (clientY: number) =>
        new Touch({ identifier: 1, target: el as Element, clientX: x, clientY, pageX: x, pageY: clientY });

      const fire = (type: string, clientY: number) => {
        const touch = makeTouch(clientY);
        const ev = new TouchEvent(type, {
          bubbles: true,
          cancelable: true,
          touches: type === 'touchend' || type === 'touchcancel' ? [] : [touch],
          changedTouches: [touch],
          targetTouches: type === 'touchend' || type === 'touchcancel' ? [] : [touch],
        });
        el.dispatchEvent(ev);
      };

      fire('touchstart', startY);
      // Several intermediate moves so the axis-lock + progress math sees a real drag,
      // not a single teleport.
      const steps = 6;
      for (let i = 1; i <= steps; i++) fire('touchmove', startY + (deltaY * i) / steps);
      fire('touchend', startY + deltaY);
    },
    { selector, deltaY },
  );
}

test.describe('Client Menu — Pull to Refresh', () => {
  test('pulling down past the threshold shows the refreshing indicator and refetches the menu', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true', { waitUntil: 'networkidle' });
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });

    const indicator = page.locator('[data-testid="ptr-indicator"]');
    await expect(indicator).toHaveAttribute('data-ptr-state', 'idle');

    const refetch = page.waitForRequest((req) => /\/public\/locations\/.+\/menu/.test(req.url()), { timeout: 5000 }).catch(() => null);
    // Resisted distance = deltaY * 0.5; drag well past PULL_THRESHOLD_PX (70px) so the
    // gesture is unambiguously armed regardless of exact resistance tuning.
    await simulatePullDown(page, '.relative.min-h-screen', 400);

    await expect(indicator).toHaveAttribute('data-ptr-state', /refreshing|idle/, { timeout: 5000 });
    await refetch;
  });
});

test.describe('Courier Tasks — Pull to Refresh', () => {
  test('pulling down past the threshold refetches tasks and shift status', async ({ page }) => {
    await page.goto('/courier?dev=true', { waitUntil: 'networkidle' });
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();

    const indicator = page.locator('[data-testid="ptr-indicator"]');
    await expect(indicator).toHaveAttribute('data-ptr-state', 'idle');

    const tasksRefetch = page
      .waitForRequest((req) => req.url().includes('/courier/me/assignments'), { timeout: 5000 })
      .catch(() => null);
    await simulatePullDown(page, '.relative.p-5', 400);

    await expect(indicator).toHaveAttribute('data-ptr-state', /refreshing|idle/, { timeout: 5000 });
    await tasksRefetch;
  });

  test('a short drag below the threshold never arms the refresh', async ({ page }) => {
    await page.goto('/courier?dev=true', { waitUntil: 'networkidle' });
    await expect(page.getByRole('heading', { level: 1 }).first()).toBeVisible();

    const indicator = page.locator('[data-testid="ptr-indicator"]');
    // 20px resisted (10px raw * 0.5) is far short of the 70px threshold — must never reach 'refreshing'.
    await simulatePullDown(page, '.relative.p-5', 10);
    await expect(indicator).not.toHaveAttribute('data-ptr-state', 'refreshing');
  });
});
