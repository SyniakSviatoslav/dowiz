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
    baseURL: process.env.VITE_BASE_URL || 'http://localhost:5173',
    trace: 'on-first-retry',
    video: 'on',
    screenshot: 'on',
    headless: false,
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

  webServer: [],

  reporter: [
    ['list'],
    ['html', { outputFolder: 'e2e/artifacts/html-report', open: 'never' }],
  ],

  outputDir: 'e2e/artifacts/test-results',
});
