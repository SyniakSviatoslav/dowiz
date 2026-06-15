// @ts-nocheck
import type { FastifyInstance } from 'fastify';

export interface SecurityHeadersOpts {
  nonce?: string;
  frameAncestors?: string;
  isSsr?: boolean;
}

export function setSecurityHeaders(reply: any, opts: SecurityHeadersOpts = {}): void {
  const { nonce, frameAncestors = "'self'", isSsr = false } = opts;


  let r2ImgSrc = '';
  const r2PublicUrl = process.env.R2_PUBLIC_URL;
  if (r2PublicUrl) {
    try {
      const u = new URL(r2PublicUrl);
      r2ImgSrc = ' ' + u.origin;
    } catch (_) {}
  }
  const cspParts = [
    `default-src 'self'`,
    `img-src 'self' data: https:` + r2ImgSrc,
    `style-src 'self'${nonce ? ` 'nonce-${nonce}'` : ''} https://fonts.googleapis.com`,
    `font-src 'self' https://fonts.gstatic.com`,
    `script-src 'self'${nonce ? ` 'nonce-${nonce}'` : ''} https://cdn.tailwindcss.com`,
    `worker-src 'self' blob:`,
    `connect-src 'self' https://tiles.openfreemap.org https://router.project-osrm.org`,
    `frame-ancestors ${frameAncestors}`,
    `base-uri 'self'`,
    `form-action 'self'`,
  ];

  reply.header('Content-Security-Policy', cspParts.join('; '));

  if (!isSsr) {
    reply.header('X-Content-Type-Options', 'nosniff');
    reply.header('X-Frame-Options', 'SAMEORIGIN');
    reply.header('Referrer-Policy', 'strict-origin-when-cross-origin');
    reply.header('Permissions-Policy', 'camera=(), geolocation=(self), microphone=(), payment=(self)');
  }

  if (process.env.NODE_ENV === 'production') {
    reply.header('Strict-Transport-Security', 'max-age=31536000; includeSubDomains');
  }
}

export function setPublicCorsHeaders(reply: any): void {
  reply.header('Access-Control-Allow-Origin', '*');
  reply.header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  reply.header('Access-Control-Allow-Headers', 'Content-Type, Authorization');
  reply.header('Access-Control-Max-Age', '86400');
}

export function setStrictCorsHeaders(reply: any): void {
  reply.header('Access-Control-Allow-Origin', '');
  reply.header('Access-Control-Allow-Methods', 'GET');
}

export default async function securityHeadersPlugin(fastify: FastifyInstance): Promise<void> {
  fastify.addHook('onRequest', async (request, reply) => {
    if (request.url.startsWith('/api/admin') ||
        request.url.startsWith('/api/owner') ||
        request.url.startsWith('/api/customer') ||
        request.url.startsWith('/api/courier') ||
        request.url.startsWith('/api/orders') ||
        request.url.startsWith('/api/telemetry') ||
        request.url.startsWith('/api/push')) {
      setSecurityHeaders(reply, { isSsr: false });
    }
    if (request.url.startsWith('/auth/')) {
      setSecurityHeaders(reply, { isSsr: false });
    }
    if (request.url.startsWith('/couriers/')) {
      setSecurityHeaders(reply, { isSsr: false });
    }
  });

  fastify.addHook('onSend', async (request, reply, payload) => {
    try {
      if (reply.getHeader('Content-Security-Policy')) return payload;
      const ct = String(reply.getHeader('content-type') || '');
      if (ct.includes('text/html') || ct.includes('application/json')) {
        setSecurityHeaders(reply, { isSsr: ct.includes('text/html') && !request.url.startsWith('/api') });
      }
    } catch (err) {
      // Never break the response due to security header injection
      console.debug('[security-headers] header injection skipped', err);
    }
    return payload;
  });

  fastify.addHook('onSend', async (request, reply, payload) => {
    if (typeof payload === 'string' && reply.statusCode === 404) {
      return `{"error":"Not found","status":404}`;
    }
    return payload;
  });
}
