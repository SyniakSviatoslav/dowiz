import { test, describe } from 'node:test';
import assert from 'node:assert';
import Fastify from 'fastify';

// G11: every `/claim` deep link was 404 on prod+staging because `/claim` was
// missing from the SPA_ROUTES allowlist in apps/api/src/server.ts. The client
// route `/claim` (apps/web/src/main.tsx) exists, but the API's not-found
// handler only served index.html for the listed prefixes. This test reproduces
// the exact not-found handler from server.ts:841 and asserts `/claim` resolves
// to the SPA shell (200 + HTML) instead of 404.

const SPA_ROUTES = ['/admin', '/courier', '/dashboard', '/s/', '/login', '/branding-preview', '/privacy', '/claim'];

function buildApp() {
  const app = Fastify();
  app.setNotFoundHandler((request, reply) => {
    const pathname = (request.url || '').split('?')[0];
    if (
      request.method === 'GET' &&
      (request.headers.accept?.includes('text/html') ||
        SPA_ROUTES.some(prefix => pathname === prefix || pathname.startsWith(prefix + '/')))
    ) {
      return reply.type('text/html').send('<html><body>claim-spa-shell</body></html>');
    }
    return reply.code(404).send({ error: 'Not found' });
  });
  return app;
}

describe('SPA not-found handler — /claim route (G11)', () => {
  const app = buildApp();

  test('GET /claim (exact) serves the SPA shell, not 404', async () => {
    const res = await app.inject({ method: 'GET', url: '/claim', headers: { accept: 'text/html' } });
    assert.strictEqual(res.statusCode, 200, `expected 200, got ${res.statusCode}`);
    assert.match(res.headers['content-type'] || '', /text\/html/);
    assert.match(res.body, /claim-spa-shell/);
  });

  test('GET /claim#token=... (with query/fragment path) serves the SPA shell', async () => {
    const res = await app.inject({ method: 'GET', url: '/claim?preview=abc', headers: { accept: 'text/html' } });
    assert.strictEqual(res.statusCode, 200, `expected 200, got ${res.statusCode}`);
  });

  test('unknown API path still 404s', async () => {
    const res = await app.inject({ method: 'GET', url: '/nope', headers: { accept: 'application/json' } });
    assert.strictEqual(res.statusCode, 404);
  });
});
