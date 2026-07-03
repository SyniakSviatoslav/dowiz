import { test, expect } from '@playwright/test';

// PWA install feature proof (Mandatory Proof Rule).
// Read-only: fetches the manifest, checks SW registration, and drives the
// first-visit InstallPrompt UI (iOS hint + Android/Chromium banner). No mutation,
// so no requireStaging guard is needed.
//
// Against a deployed build set VITE_BASE_URL; otherwise defaults to a local
// `vite preview` of apps/web (see the run command in the PR notes).
const BASE = process.env.VITE_BASE_URL || 'http://localhost:4173';

// A real iOS Safari UA — iOS never fires `beforeinstallprompt`, so this drives
// the manual "Add to Home Screen" path.
const IOS_UA =
  'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1';

// A minimal, API-light SPA route — the InstallPrompt is app-wide so any route works.
const NAV = '/privacy';

test.describe('PWA — installable + first-visit install prompt', () => {
  // (a) Manifest linked + fetches 200 + parses standalone + 192/512 maskable icons.
  test('manifest is linked, fetches 200, parses standalone with 192+512 maskable icons', async ({
    request,
  }) => {
    const shell = await request.get(`${BASE}${NAV}`);
    expect(shell.status()).toBe(200);
    expect(await shell.text()).toContain('rel="manifest"');

    const res = await request.get(`${BASE}/manifest.json`);
    expect(res.status()).toBe(200);
    const m = await res.json();
    expect(m.display).toBe('standalone');
    expect(m.start_url).toBe('/');
    expect(m.scope).toBe('/');
    expect(m.theme_color).toBe('#ea4f16');
    expect(m.background_color).toBe('#121212');

    const sizes = (m.icons || []).map((i: { sizes: string }) => i.sizes);
    expect(sizes).toContain('192x192');
    expect(sizes).toContain('512x512');
    const maskable = (m.icons || []).some((i: { purpose?: string }) =>
      String(i.purpose || '').includes('maskable'),
    );
    expect(maskable).toBe(true);

    // The referenced icon file actually resolves (no dangling /dist path).
    const icon192 = m.icons.find((i: { sizes: string }) => i.sizes === '192x192');
    const iconRes = await request.get(`${BASE}${icon192.src}`);
    expect(iconRes.status()).toBe(200);
    expect(iconRes.headers()['content-type']).toContain('image/png');
  });

  // (b) Service worker registers.
  test('service worker script serves and registers', async ({ page }) => {
    const sw = await page.request.get(`${BASE}/sw.js`);
    expect(sw.status()).toBe(200);

    await page.goto(`${BASE}${NAV}`);
    const registered = await page.evaluate(async () => {
      if (!('serviceWorker' in navigator)) return false;
      const existing = await navigator.serviceWorker.getRegistration();
      if (existing) return true;
      return await Promise.race([
        navigator.serviceWorker.ready.then(() => true),
        new Promise<boolean>((resolve) =>
          setTimeout(
            () => navigator.serviceWorker.getRegistration().then((r) => resolve(!!r)),
            5000,
          ),
        ),
      ]);
    });
    expect(registered).toBe(true);
  });

  // (c) iOS Safari hint renders + (d) it is dismissible and stays hidden on reload.
  test('iOS Safari shows the Add-to-Home-Screen hint; dismiss persists across reload', async ({
    browser,
  }) => {
    const context = await browser.newContext({ userAgent: IOS_UA });
    // Force English so the assertion is locale-deterministic (default is sq).
    await context.addInitScript(() => {
      try {
        localStorage.setItem('dos_locale', 'en');
      } catch {
        /* storage blocked — ignore */
      }
    });
    const page = await context.newPage();
    await page.goto(`${BASE}${NAV}`);

    const prompt = page.getByTestId('pwa-install-prompt');
    await expect(prompt).toBeVisible();
    await expect(prompt).toContainText('Install dowiz');
    await expect(prompt).toContainText('Add to Home Screen');

    // Dismiss → gone now, and STILL gone after a reload (persisted in localStorage).
    await page.getByTestId('pwa-install-dismiss').click();
    await expect(prompt).toHaveCount(0);

    await page.reload();
    await page.waitForTimeout(2500); // outlive the iOS hint delay (1500ms)
    await expect(page.getByTestId('pwa-install-prompt')).toHaveCount(0);

    await context.close();
  });

  // Android/Chromium native path: capture beforeinstallprompt → banner with Install.
  // Headless Chromium never fires the event organically, so we dispatch it.
  test('Chromium beforeinstallprompt shows the Install banner; accepting hides it', async ({
    browser,
  }) => {
    const context = await browser.newContext(); // default (non-iOS) desktop UA
    await context.addInitScript(() => {
      try {
        localStorage.setItem('dos_locale', 'en');
      } catch {
        /* storage blocked — ignore */
      }
    });
    const page = await context.newPage();
    await page.goto(`${BASE}${NAV}`);

    await page.evaluate(() => {
      const e = new Event('beforeinstallprompt') as Event & {
        platforms?: string[];
        prompt?: () => Promise<void>;
        userChoice?: Promise<{ outcome: string; platform: string }>;
      };
      e.platforms = ['web'];
      e.prompt = async () => {};
      e.userChoice = Promise.resolve({ outcome: 'accepted', platform: 'web' });
      window.dispatchEvent(e);
    });

    const prompt = page.getByTestId('pwa-install-prompt');
    await expect(prompt).toBeVisible();
    await expect(prompt).toContainText('Install dowiz');
    const cta = page.getByTestId('pwa-install-cta');
    await expect(cta).toBeVisible();

    await cta.click();
    await expect(prompt).toHaveCount(0);

    await context.close();
  });
});
