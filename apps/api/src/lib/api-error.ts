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

  /**
   * Fastify's default error handler (and any thrower that bypasses our setErrorHandler)
   * reads `statusCode`; mirror `status` onto it so a THROWN ApiError carries the right HTTP
   * status everywhere. @fastify/rate-limit throws the `errorResponseBuilder` return value
   * (index.js:333) — without this it would land as a 500.
   */
  get statusCode(): number {
    return this.status;
  }
}

/** A machine code is contract-shaped iff SCREAMING_SNAKE (stable, FE-branchable). */
export function isContractCode(code: unknown): code is string {
  return typeof code === 'string' && /^[A-Z][A-Z0-9_]*$/.test(code);
}

export interface ErrorEnvelopeOpts {
  fields?: { path: string; code: string }[]; // 422 — field PATHS only, never values (B4)
  retryAfterMs?: number; // 429
}

/**
 * The ONE error-envelope shape (ADR-0010). Built here so `setErrorHandler` (thrown errors) and
 * `reply.sendError` (A2 sweep — return-based ad-hoc sites) emit byte-identical bodies: a single
 * source for the contract, no drift. `code` is the SCREAMING_SNAKE machine code; `error` is the
 * retained legacy string the un-migrated FE still reads (B1 code-preserving).
 */
export function buildErrorEnvelope(
  status: number,
  code: string,
  message: string,
  correlationId: string,
  opts?: ErrorEnvelopeOpts,
) {
  return {
    code,
    message,
    fields: opts?.fields,
    correlationId,
    retryAfterMs: opts?.retryAfterMs,
    status, // numeric status (legacy lived in `code` pre-A1)
    error: message, // legacy string the un-migrated FE still reads
  };
}

/**
 * A3 (ADR-0010): the rate-limit error for @fastify/rate-limit. The plugin THROWS the
 * `errorResponseBuilder` return value (index.js:333) — so it must be a throwable ApiError,
 * NOT a plain body. Returning a plain object made `setErrorHandler` read `.statusCode` as
 * undefined → 500. Throwing an ApiError routes the 429 through the ONE envelope source
 * (setErrorHandler), which renders `{code:'RATE_LIMIT', message, retryAfterMs, correlationId,
 * status:429, error}`. The plugin sets `Retry-After`/`x-ratelimit-*` headers before the throw.
 * `statusCode` here is the plugin's context status (429, or 403 on ban).
 */
export function rateLimitError(statusCode: number, ttlMs: number): ApiError {
  return new ApiError(
    statusCode,
    'RATE_LIMIT',
    `Too many requests. Try again in ${Math.ceil(ttlMs / 1000)}s.`,
    undefined,
    ttlMs,
  );
}
