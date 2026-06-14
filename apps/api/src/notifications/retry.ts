// @ts-nocheck
export class RetryPolicy {
  maxAttempts: number;
  baseMs: number;
  maxMs: number;
  jitter: number;

  constructor({ maxAttempts = 5, baseMs = 1000, maxMs = 60000, jitter = 0.2 } = {}) {
    this.maxAttempts = maxAttempts;
    this.baseMs = baseMs;
    this.maxMs = maxMs;
    this.jitter = jitter;
  }

  getDelay(attempt: number): number {
    if (attempt >= this.maxAttempts) return -1;
    let delay = this.baseMs * Math.pow(2, attempt);
    delay = Math.min(delay, this.maxMs);
    const j = 1 - this.jitter + (Math.random() * this.jitter * 2);
    return Math.floor(delay * j);
  }
}
