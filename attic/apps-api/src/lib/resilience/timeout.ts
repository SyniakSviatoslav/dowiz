// @ts-nocheck
export class TimeoutError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'TimeoutError';
  }
}

export async function withTimeout<T>(
  promise: Promise<T>,
  ms: number,
  fallback?: T,
): Promise<T> {
  const result = await Promise.race([
    promise,
    new Promise<never>((_, reject) => {
      const id = setTimeout(() => reject(new TimeoutError(`Timed out after ${ms}ms`)), ms);
      if (typeof id === 'object' && typeof id.unref === 'function') id.unref();
    }),
  ]);
  return result;
}

export async function withTimeoutFallback<T>(
  promise: Promise<T>,
  ms: number,
  fallback: T,
  label?: string,
): Promise<T> {
  try {
    return await withTimeout(promise, ms);
  } catch (err: any) {
    if (err instanceof TimeoutError) {
      console.warn(`[Timeout] ${label || 'call'} timed out after ${ms}ms, using fallback`);
    }
    return fallback;
  }
}

export interface RetryOpts {
  maxAttempts?: number;
  baseMs?: number;
  maxMs?: number;
  jitter?: number;
}

export async function retryWithBackoff<T>(
  fn: (attempt: number) => Promise<T>,
  opts: RetryOpts = {},
): Promise<T> {
  const { maxAttempts = 3, baseMs = 1000, maxMs = 30000, jitter = 0.2 } = opts;
  let lastErr: any;

  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    try {
      return await fn(attempt);
    } catch (err: any) {
      lastErr = err;
      if (err instanceof TimeoutError) throw err;
      if (attempt === maxAttempts - 1) break;

      let delay = baseMs * Math.pow(2, attempt);
      delay = Math.min(delay, maxMs);
      const j = 1 - jitter + (Math.random() * jitter * 2);
      await new Promise((r) => setTimeout(r, Math.floor(delay * j)));
    }
  }
  throw lastErr;
}
