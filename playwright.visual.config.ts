import { defineConfig } from '@playwright/test';

/**
 * Critical-Path Visual Regression Net — config (separate from the functional e2e config).
 *
 * Baselines are only trustworthy when rendered deterministically: this config freezes motion, time,
 * and timezone, and uses a PERCEPTUAL threshold (not pixel-perfect, to survive sub-pixel AA). Dynamic
 * zones are masked per-test via the shared MASK helper (locator '[data-dynamic]').
 *
 * IRON RULE: generate/lock baselines ONLY in the pinned Docker image (mcr.microsoft.com/playwright)
 * against a freshly-seeded DB — see docs/operating-model/proposed-visual-ci/APPLY.md. Baselines
 * generated on an arbitrary machine are font/AA/GPU noise and must NOT be committed.
 *
 * Run (in Docker/CI):  pnpm exec playwright test -c playwright.visual.config.ts --update-snapshots
 * Compare:             pnpm exec playwright test -c playwright.visual.config.ts
 */
export default defineConfig({
  testDir: './e2e/visual',
  snapshotPathTemplate: '{testDir}/__screenshots__/{projectName}/{testFilePath}/{arg}{ext}',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  timeout: 45000,
  expect: {
    timeout: 10000,
    // Perceptual tolerance: survive AA jitter, still catch real visual regressions.
    toHaveScreenshot: { maxDiffPixelRatio: 0.01, threshold: 0.2, animations: 'disabled', caret: 'hide' },
  },

  globalSetup: './e2e/visual/global-setup.ts',

  use: {
    baseURL: process.env.VISUAL_BASE_URL || process.env.VITE_BASE_URL || 'http://localhost:3000',
    extraHTTPHeaders: process.env.DEV_AUTH_SECRET ? { 'x-dev-auth-secret': process.env.DEV_AUTH_SECRET } : {},
    headless: true,
    // Determinism: freeze motion + timezone; locale is set per-test via the app's i18n toggle.
    reducedMotion: 'reduce',
    timezoneId: 'Europe/Tirane',
    locale: 'en-US',
    actionTimeout: 10000,
    navigationTimeout: 20000,
  },

  projects: [
    { name: 'mobile-390', use: { browserName: 'chromium', viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } },
    { name: 'tablet-768', use: { browserName: 'chromium', viewport: { width: 768, height: 1024 } } },
    { name: 'desktop-1280', use: { browserName: 'chromium', viewport: { width: 1280, height: 800 } } },
  ],

  reporter: [['list'], ['html', { outputFolder: 'e2e/artifacts/visual-report', open: 'never' }]],
  outputDir: 'e2e/artifacts/visual-results',
});
