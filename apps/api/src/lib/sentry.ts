import * as Sentry from '@sentry/node';
import { PiiRedactor } from './pii-redactor.js';

const SENSITIVE_FIELDS = new Set([
  'password', 'secret', 'token', 'authorization', 'cookie',
  'set-cookie', 'x-api-key', 'api_key', 'private_key',
]);

const PII_KEYS = new Set([
  'email', 'phone', 'phone_encrypted', 'email_encrypted',
  'address', 'delivery_address', 'client_ip', 'client_ip_hash',
  'full_name', 'full_name_encrypted', 'name', 'customer_phone',
  'customer_address', 'customer_name', 'courier_phone',
  'owner_phone', 'subject_phone',
]);

const piiRedactor = new PiiRedactor();

function redactValue(value: unknown, depth = 0): unknown {
  if (depth > 10) return value;
  if (typeof value === 'string') {
    const { text, redactions } = piiRedactor.redact(value);
    if (redactions.length > 0) return text;
    return value;
  }
  if (value === null || value === undefined) return value;
  if (Array.isArray(value)) return value.map(v => redactValue(v, depth + 1));
  if (typeof value === 'object') {
    const result: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      if (SENSITIVE_FIELDS.has(k) || PII_KEYS.has(k)) {
        result[k] = '[REDACTED]';
      } else {
        result[k] = redactValue(v, depth + 1);
      }
    }
    return result;
  }
  return value;
}

function redactSentryData(data: Record<string, unknown>): Record<string, unknown> {
  const result: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(data)) {
    if (SENSITIVE_FIELDS.has(k) || PII_KEYS.has(k)) {
      result[k] = '[REDACTED]';
    } else {
      result[k] = v;
    }
  }
  return result;
}

export function initSentry(dsn: string, gitSha?: string): void {
  if (!dsn) return;
  Sentry.init({
    dsn,
    environment: process.env.NODE_ENV || 'development',
    release: gitSha || 'dev',
    tracesSampleRate: 0.1,
    sampleRate: 1.0,
    normalizeDepth: 6,
    beforeSend(event) {
      if (event.exception?.values) {
        for (const exc of event.exception.values) {
          if (exc.value) exc.value = piiRedactor.redact(exc.value).text;
        }
      }
      if (event.request) {
        if (event.request.cookies) (event.request as any).cookies = '[REDACTED]';
        if (event.request.headers) (event.request as any).headers = redactSentryData(event.request.headers as Record<string, unknown>);
      }
      if (event.user && event.user.id) {
        event.user = { id: event.user.id };
      } else {
        event.user = undefined;
      }
      if (event.contexts) {
        (event as any).contexts = redactSentryData(event.contexts as Record<string, unknown>);
      }
      if (event.extra) {
        (event as any).extra = redactSentryData(event.extra as Record<string, unknown>);
      }
      if (event.tags) {
        const allowlist = new Set(['role', 'location_id', 'order_id', 'worker', 'db', 'error_code']);
        event.tags = Object.fromEntries(
          Object.entries(event.tags).filter(([k]) => allowlist.has(k))
        );
      }
      return event;
    },
    beforeBreadcrumb(breadcrumb) {
      if (breadcrumb.data) {
        breadcrumb.data = redactSentryData(breadcrumb.data as Record<string, unknown>);
      }
      if (breadcrumb.message) {
        breadcrumb.message = piiRedactor.redact(breadcrumb.message).text;
      }
      return breadcrumb;
    },
    integrations: [Sentry.onUncaughtExceptionIntegration(), Sentry.onUnhandledRejectionIntegration()],
  });
}

export function getSentry(): typeof Sentry | null {
  try {
    return Sentry;
  } catch (err: any) {
    console.warn('[sentry] getSentry failed:', err?.message);
    return null;
  }
}
