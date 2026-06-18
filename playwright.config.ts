import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e/tests',
  fullyParallel: false,
  forbidOnly: true,
  retries: 0,
  workers: 1,
  timeout: 30000,
  expect: { timeout: 10000 },

  use: {
    baseURL: process.env.VITE_BASE_URL || 'http://localhost:3000',
    // Test-only /dev endpoints are gated by DEV_AUTH_SECRET; send it on every
    // request so the harness keeps working while anonymous callers get 404.
    extraHTTPHeaders: process.env.DEV_AUTH_SECRET
      ? { 'x-dev-auth-secret': process.env.DEV_AUTH_SECRET }
      : {},
    trace: 'on-first-retry',
    video: 'on',
    screenshot: 'on',
    headless: true,
    actionTimeout: 10000,
    navigationTimeout: 15000,
  },

  projects: [
    {
      name: 'mobile',
      use: {
        browserName: 'chromium',
        viewport: { width: 390, height: 844 },
        isMobile: true,
        hasTouch: true,
        userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1',
      },
    },
    {
      name: 'tablet',
      use: {
        browserName: 'chromium',
        viewport: { width: 768, height: 1024 },
        isMobile: false,
      },
    },
    {
      name: 'desktop',
      use: {
        browserName: 'chromium',
        viewport: { width: 1280, height: 800 },
        isMobile: false,
      },
    },
  ],

  webServer: process.env.CI || process.env.VITE_BASE_URL ? [] : [
    {
      command: 'pnpm dev:api:1',
      port: 3000,
      timeout: 120000,
      reuseExistingServer: true,
    },
  ],

  reporter: [
    ['list'],
    ['html', { outputFolder: 'e2e/artifacts/html-report', open: 'never' }],
  ],

  outputDir: 'e2e/artifacts/test-results',
});
