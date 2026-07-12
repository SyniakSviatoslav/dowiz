/* eslint-disable @typescript-eslint/no-unused-vars -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import type { APIRequestContext, APIResponse } from '@playwright/test';

/**
 * Retry a request up to `maxRetries` times when the server returns 429 (rate limited).
 * Waits `retryAfterMs` between retries (or reads Retry-After header if present).
 */
export async function retryOn429(
  request: APIRequestContext,
  url: string,
  options?: { data?: unknown; headers?: Record<string, string>; method?: string },
  maxRetries: number = 3,
): Promise<APIResponse> {
  const method = options?.method || (options?.data ? 'POST' : 'GET');
  let lastRes: APIResponse | undefined;

  for (let attempt = 0; attempt < maxRetries; attempt++) {
    let res: APIResponse;
    if (method === 'GET') {
      res = await request.get(url, { headers: options?.headers });
    } else if (method === 'POST') {
      res = await request.post(url, { data: options?.data, headers: options?.headers });
    } else if (method === 'PUT') {
      res = await request.put(url, { data: options?.data, headers: options?.headers });
    } else if (method === 'PATCH') {
      res = await request.patch(url, { data: options?.data, headers: options?.headers });
    } else if (method === 'DELETE') {
      res = await request.delete(url, { headers: options?.headers });
    } else {
      throw new Error(`Unsupported method: ${method}`);
    }

    if (res.status() !== 429) return res;

    lastRes = res;
    const retryAfter = res.headers()['retry-after'];
    const delayMs = retryAfter ? parseInt(retryAfter, 10) * 1000 : 1000;
    console.warn(`[e2e] 429 on ${method} ${url}, retry ${attempt + 1}/${maxRetries} after ${delayMs}ms`);
    await new Promise((r) => setTimeout(r, delayMs));
  }

  throw new Error(`[e2e] 429 persists after ${maxRetries} retries for ${method} ${url}`);
}
