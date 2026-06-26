/**
 * ApiError — the single source of the API error shape (ADR-0010, Area A1).
 *
 * Throw this from any route; the central `setErrorHandler` (server.ts) serializes it
 * into the one structured envelope:
 *   { code, message, fields?, correlationId, retryAfterMs?, status, error }
 *
 * `code` is a STABLE SCREAMING_SNAKE string (the machine code the FE branches on, e.g.
 * MIN_ORDER_NOT_MET, CASH_AMOUNT_TOO_LOW). It is the contract — never invent/normalize/
 * rename/drop an existing code (B1). The numeric HTTP status lives in `status`, NOT `code`.
 *
 * Note on namespaces (ADR-0010 §4b / B15): the SCREAMING_SNAKE contract applies ONLY to
 * this envelope `code`. Business-outcome tokens carried in a 200/422 `reasons[].code`
 * (e.g. lowercase `item_unavailable`) are a SEPARATE namespace, preserved verbatim, and
 * must NOT be routed through this class.
 */
export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
    public readonly fields?: { path: string; code: string }[], // 422 — field PATHS only, never values
    public readonly retryAfterMs?: number, // 429
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

/** A machine code is contract-shaped iff SCREAMING_SNAKE (stable, FE-branchable). */
export function isContractCode(code: unknown): code is string {
  return typeof code === 'string' && /^[A-Z][A-Z0-9_]*$/.test(code);
}

/**
 * A3 (ADR-0010): the 429 envelope for @fastify/rate-limit. The plugin builds its OWN body
 * and never enters `setErrorHandler`, so the envelope is reconstructed here to match. Pure
 * function (testable in isolation). `code:'RATE_LIMIT'` is the SCREAMING_SNAKE contract; the
 * plugin sets `Retry-After` itself. Legacy `error`/`status` kept (B1 code-preserving).
 */
export function rateLimitEnvelope(correlationId: string, ttlMs: number) {
  return {
    code: 'RATE_LIMIT',
    message: `Too many requests. Try again in ${Math.ceil(ttlMs / 1000)}s.`,
    correlationId,
    retryAfterMs: ttlMs,
    status: 429,
    error: 'Too many requests', // legacy string the un-migrated FE still reads
  };
}
