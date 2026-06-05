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

/**
 * Check if the page has network activity (API calls) after a user action.
 * Returns true if at least one fetch/XHR was made.
 */
export async function assertNetworkActivity(
  page: Page,
  action: () => Promise<void>,
  timeout: number = 5000
): Promise<boolean> {
  let activityDetected = false;
  const onRequest = () => { activityDetected = true; };
  page.on('request', onRequest);
  
  await action();
  await page.waitForTimeout(1000);
  
  page.off('request', onRequest);
  return activityDetected;
}

/**
 * Get the current count of API requests made.
 */
export async function getRequestCount(
  page: Page,
  urlPattern: string | RegExp
): Promise<number> {
  const requests = await page.evaluate(() => {
    return (window as any).__playwright_request_count || 0;
  });
  return requests;
}

/**
 * Assert that a specific API call was made with the correct method and body.
 */
export async function assertApiCall(
  page: Page,
  urlPattern: string | RegExp,
  method: string,
  expectedBody?: Record<string, unknown>
): Promise<void> {
  const requests: any[] = [];
  page.on('request', (req) => {
    const url = req.url();
    const matches = typeof urlPattern === 'string'
      ? url.includes(urlPattern)
      : urlPattern.test(url);
    if (matches && req.method() === method) {
      requests.push(req);
    }
  });
}
