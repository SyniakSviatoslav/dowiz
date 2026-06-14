import { Page, APIRequestContext } from '@playwright/test';

const API_BASE = process.env.API_BASE_URL || 'http://localhost:3000/api';

/**
 * Wait for a network response matching the given URL pattern and method.
 */
export async function waitForApiResponse(
  page: Page,
  urlPattern: string | RegExp,
  method: string = 'GET',
  timeout: number = 10000
): Promise<any> {
  const response = await page.waitForResponse(
    (resp) => {
      const urlMatch = typeof urlPattern === 'string'
        ? resp.url().includes(urlPattern)
        : urlPattern.test(resp.url());
      return urlMatch && resp.request().method() === method;
    },
    { timeout }
  );
  return response.json();
}

/**
 * Intercept and mock an API endpoint with a custom handler.
 */
export async function interceptApi(
  page: Page,
  urlPattern: string | RegExp,
  handler: (route: any) => Promise<void>
): Promise<void> {
  await page.route(urlPattern, handler);
}

/**
 * Simulate a network error for a specific API endpoint.
 */
export async function simulateNetworkError(
  page: Page,
  urlPattern: string | RegExp
): Promise<void> {
  await page.route(urlPattern, (route) => route.abort('timedout'));
}

/**
 * Simulate a server error (5xx) for a specific API endpoint.
 */
export async function simulateServerError(
  page: Page,
  urlPattern: string | RegExp,
  status: number = 500
): Promise<void> {
  await page.route(urlPattern, (route) =>
    route.fulfill({
      status,
      contentType: 'application/json',
      body: JSON.stringify({ error: 'Internal Server Error' }),
    })
  );
}

/**
 * Simulate a specific HTTP error for an API endpoint.
 */
export async function simulateHttpError(
  page: Page,
  urlPattern: string | RegExp,
  status: number,
  body: Record<string, unknown> = {}
): Promise<void> {
  await page.route(urlPattern, (route) =>
    route.fulfill({
      status,
      contentType: 'application/json',
      body: JSON.stringify({ error: 'Error', ...body }),
    })
  );
}


