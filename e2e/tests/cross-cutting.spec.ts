import { test, expect } from '@playwright/test';

test.describe('Cross-Cutting', () => {

  test('error boundary renders the real fallback on a render crash', async ({ page }) => {
    await page.goto('/s/test-slug?dev=true');
    await page.waitForSelector('[data-testid="menu-item"]', { timeout: 15000 });
    // Trigger a REAL React render error — the previous version hand-injected a
    // "Something went wrong" <div> and then asserted on its own injected text, which
    // never exercises the ErrorBoundary (a guaranteed false-green). The boundary can
    // only be reached by a render-time throw inside React.
    // TODO(needs_staging/app-hook): app must expose a dev-gated window.__dosCrash() that
    // throws inside a mounted component, and add data-testid="error-boundary" to the
    // DefaultFallback in packages/ui/src/components/ErrorBoundary.tsx. Until then this
    // test is an honest pending-red (a finding), never a fake-green.
    await page.evaluate(() => { (window as { __dosCrash?: () => void }).__dosCrash?.(); });
    await expect(page.locator('[data-testid="error-boundary"]')).toBeVisible({ timeout: 5000 });
  });

  test('theme cycling through all 6 presets does not crash', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 15000 });
    // Apply each preset's REAL brand values (mirrors packages/ui/src/theme PRESETS) so the
    // loop variable is actually used and each iteration paints a DISTINCT theme — the old
    // version set the same two crimson vars every pass, so 5 of 6 presets were never tested.
    // (Closest honest exercise without a real theme-picker control / __setTheme dev hook.)
    const presets: Record<string, Record<string, string>> = {
      'Crimson Classic': { '--brand-primary': '#C1121F', '--brand-bg': '#FFFFFF', '--brand-surface': '#F8F9FA', '--brand-text': '#1A1A1A', '--brand-border': '#E5E7EB' },
      'Ocean Fresh': { '--brand-primary': '#0D9488', '--brand-bg': '#FFFFFF', '--brand-surface': '#F8FFFE', '--brand-text': '#134E4A', '--brand-border': '#CCFBF1' },
      'Midnight Urban': { '--brand-primary': '#F97316', '--brand-bg': '#0C0C0C', '--brand-surface': '#1A1A1A', '--brand-text': '#FAFAFA', '--brand-border': '#262626' },
      'Sage Garden': { '--brand-primary': '#4D7C0F', '--brand-bg': '#FAFAF5', '--brand-surface': '#F5F5F0', '--brand-text': '#1A2E05', '--brand-border': '#D9F99D' },
      'Royal Gold': { '--brand-primary': '#B45309', '--brand-bg': '#0A0A0A', '--brand-surface': '#1A1A1A', '--brand-text': '#FEF3C7', '--brand-border': '#292524' },
      'Coral Breeze': { '--brand-primary': '#DB2777', '--brand-bg': '#FFFBFB', '--brand-surface': '#FFF5F5', '--brand-text': '#1A1A2E', '--brand-border': '#FBCFE8' },
    };
    for (const [name, vars] of Object.entries(presets)) {
      await page.evaluate((v) => {
        for (const [k, val] of Object.entries(v)) document.documentElement.style.setProperty(k, val);
      }, vars);
      await expect(
        page.locator('[data-testid="menu-item"]').first(),
        `menu-item must stay rendered under preset "${name}"`,
      ).toBeVisible({ timeout: 5000 });
    }
    expect(errors, `JS errors after theme cycling: ${errors.join('; ')}`).toEqual([]);
  });

  test('slow network does not crash the app — paints shell, then loads items', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/api/**', async route => {
      await new Promise(r => setTimeout(r, 2000));
      await route.continue();
    });
    await page.goto('/s/test-slug?dev=true');
    // Shell must paint during the slow window (category nav renders skeleton-blocks while loading) …
    await expect(page.locator('[data-testid="category-nav"]')).toBeVisible({ timeout: 15000 });
    // … and once the delayed API resolves, real menu items must appear — a stuck spinner,
    // a 500 screen, or a redirect would all fail this (the old body.length/regex passed them).
    await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 25000 });
    expect(errors, `JS errors with slow network: ${errors.join('; ')}`).toEqual([]);
  });

  test('rapid navigation: storefront loads, owner route is auth-gated, courier renders', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));

    // 1) Public storefront actually renders its menu (not just a >100-char HTML shell).
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('[data-testid="category-nav"]')).toBeVisible({ timeout: 15000 });

    // 2) Protected owner route WITHOUT credentials — and WITHOUT the ?dev=true mock-auth
    //    bypass — must be rejected by the real auth guard (AdminLayout → navigate('/login')).
    //    Negative control: a 200 shell or a silent stay must FAIL.
    await page.goto('/admin');
    await expect(page).toHaveURL(/\/login(\?|$)/, { timeout: 15000 });
    await expect(page.locator('#login-email')).toBeVisible();

    // 3) Courier route renders without crashing. TODO(app-gap, escalate): CourierRoutes has
    //    NO auth guard (unlike AdminRoutes) — an unauthenticated visit renders the Tasks shell
    //    rather than redirecting to login. Here we only assert it does not crash; a real-session
    //    PII-leak check belongs on staging (see needs_staging).
    await page.goto('/courier');
    await expect(page.getByText('Courier', { exact: true })).toBeVisible({ timeout: 15000 });

    expect(errors, `JS errors after rapid nav: ${errors.join('; ')}`).toEqual([]);
  });

  test('corrupted localStorage does not crash the app on fresh load', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?dev=true');
    await expect(page.locator('[data-testid="category-nav"]')).toBeVisible({ timeout: 15000 });
    await page.evaluate(() => {
      localStorage.setItem('dos_cart_test-slug', 'invalid_json{{{');
    });
    await page.reload();
    // After a corrupted-cart reload the menu must still render — not a blank/error screen.
    await expect(page.locator('[data-testid="category-nav"]')).toBeVisible({ timeout: 15000 });
    await expect(page.locator('[data-testid="menu-item"]').first()).toBeVisible({ timeout: 15000 });
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('JSON')
    );
    expect(criticalErrors, `JS errors after corrupted localStorage: ${criticalErrors.join('; ')}`).toEqual([]);
  });

  test('map component handles maplibre failure gracefully — shows delivery shell', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.route('**/maplibre-gl**', route => route.abort());
    await page.goto('/courier/delivery/test-id?dev=true');
    const criticalErrors = errors.filter(e =>
      !e.includes('favicon') && !e.includes('404') && !e.includes('manifest') && !e.includes('maplibregl')
    );
    // Graceful degradation: a maplibre load failure must not blank the page via the error
    // boundary — the delivery sheet (drop-off address from the dev mock) stays visible.
    await expect(page.getByText('Rruga e Elbasanit 12')).toBeVisible({ timeout: 15000 });
    expect(criticalErrors, `JS errors: ${criticalErrors.join('; ')}`).toEqual([]);
    // No session cookie was set for this anonymous visit.
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
    // TODO(needs_staging): the leak dimension can only be proven against a REAL courier
    // session + a REAL delivery id: visit /courier/delivery/<real-id> WITHOUT ?dev (no mock
    // auth) and assert the customer phone/address are NOT rendered to the unauthenticated
    // visitor. The ?dev mock returns synthetic PII, so it cannot prove isolation here.
  });

  test('embed mode adds embed-mode class and no fixed positioning', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', err => errors.push(err.message));
    await page.goto('/s/test-slug?embed=true');
    await expect(page.locator('body')).toBeAttached({ timeout: 15000 });
    expect(errors, `JS errors in embed mode: ${errors.join('; ')}`).toEqual([]);
    const hasEmbedClass = await page.evaluate(() => {
      return document.documentElement.classList.contains('embed-mode') ||
             document.body.classList.contains('embed-mode') ||
             document.getElementById('root')?.classList.contains('embed-mode');
    });
    expect(hasEmbedClass).toBe(true);
    const hasFixed = await page.evaluate(() => {
      const all = document.querySelectorAll('*');
      for (const el of all) {
        const pos = getComputedStyle(el).position;
        if (pos === 'fixed') return true;
      }
      return false;
    });
    expect(hasFixed).toBe(false);
    const cookies = await page.context().cookies();
    expect(cookies).toEqual([]);
  });

});
