import pino from 'pino';
import { AsyncLocalStorage } from 'node:async_hooks';
import { PiiRedactor } from './pii-redactor.js';

export const correlationStore = new AsyncLocalStorage<string>();

const piiRedactor = new PiiRedactor();

const SENSITIVE_HEADERS = new Set([
  'cookie', 'authorization', 'set-cookie', 'x-api-key',
]);

const SENSITIVE_KEYS = new Set([
  'email', 'phone', 'phone_encrypted', 'email_encrypted',
  'password', 'secret', 'token', 'private_key', 'api_key',
  'full_name', 'full_name_encrypted', 'address',
  'delivery_address', 'client_ip', 'client_ip_hash',
  'customer_phone', 'customer_address', 'customer_name',
  'subject_phone', 'courier_phone', 'owner_phone',
]);

function deepRedact(obj: unknown, depth = 0): unknown {
  if (depth > 10) return obj;
  if (typeof obj === 'string') {
    const { text, redactions } = piiRedactor.redact(obj);
    if (redactions.length > 0) return text;
    return obj;
  }
  if (obj === null || obj === undefined) return obj;
  if (Array.isArray(obj)) return obj.map(v => deepRedact(v, depth + 1));
  if (typeof obj === 'object') {
    const result: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(obj as Record<string, unknown>)) {
      if (SENSITIVE_KEYS.has(k)) {
        result[k] = '[REDACTED]';
      } else {
        result[k] = deepRedact(v, depth + 1);
      }
    }
    return result;
  }
  return obj;
}

export function createPinoLogger(name?: string): pino.Logger {
  return pino({
    name,
    level: process.env.LOG_LEVEL || 'info',
    mixin() {
      const correlationId = (correlationStore as any).get();
      return correlationId ? { correlationId } : {};
    },
    serializers: {
      err: pino.stdSerializers.err,
      req: (req) => ({
        method: req.method,
        url: req.url,
        headers: deepRedact(
          Object.fromEntries(
            Object.entries(req.headers || {}).filter(
              ([k]) => !SENSITIVE_HEADERS.has(k.toLowerCase())
            )
          )
        ),
      }),
      res: (res) => ({ statusCode: res.statusCode }),
    },
    redact: {
      paths: [
        'req.headers.cookie', 'req.headers.authorization',
        'req.headers["set-cookie"]', 'req.headers["x-api-key"]',
      ],
      censor: '[REDACTED]',
    },
  });
}

export function getFastifyLoggerConfig(): Record<string, unknown> {
  return {
    level: process.env.LOG_LEVEL || 'info',
    transport: undefined,
    serializers: {
      req: (req: any) => ({
        method: req.method,
        url: req.url,
        hostname: req.hostname,
        remoteAddress: req.ip,
      }),
      res: (res: any) => ({ statusCode: res.statusCode }),
      err: pino.stdSerializers.err,
    },
    redact: {
      paths: [
        'req.headers.cookie', 'req.headers.authorization',
        'req.headers["set-cookie"]', 'req.headers["x-api-key"]',
      ],
      censor: '[REDACTED]',
    },
  };
}

const logger = createPinoLogger('deliveryos');

export function runWithCorrelationId<T>(correlationId: string, fn: () => T): T {
  return correlationStore.run(correlationId, fn);
}

export function generateCorrelationId(): string {
  const base = Date.now().toString(36) + Math.random().toString(36).substring(2, 8);
  return base;
}

export default logger;
