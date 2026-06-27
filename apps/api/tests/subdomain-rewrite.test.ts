import { test } from 'node:test';
import assert from 'node:assert/strict';
import { resolveSubdomainRewrite } from '../src/lib/subdomain-rewrite.js';

// Guardrail for the server.ts subdomain-routing resolver. Each case pins one
// arm of the reserved-path boolean so a future edit that broadens/narrows tenant
// rewriting fails RED.

test('tenant subdomain root → /s/:slug', () => {
  assert.equal(resolveSubdomainRewrite('margherita.dowiz.org', '/'), '/s/margherita');
});

test('tenant subdomain preserves the query string', () => {
  assert.equal(resolveSubdomainRewrite('margherita.dowiz.org', '/menu?table=4'), '/s/margherita?table=4');
});

test('host port is stripped before matching', () => {
  assert.equal(resolveSubdomainRewrite('margherita.dowiz.org:8080', '/'), '/s/margherita');
});

test('reserved subdomains (www/api/app) are not rewritten', () => {
  for (const sub of ['www', 'api', 'app']) {
    assert.equal(resolveSubdomainRewrite(`${sub}.dowiz.org`, '/'), null, `${sub} should pass through`);
  }
});

test('app/API paths pass through unchanged', () => {
  for (const p of ['/api/orders', '/public/locations/x/menu', '/s/already', '/admin', '/courier/x', '/dashboard']) {
    assert.equal(resolveSubdomainRewrite('margherita.dowiz.org', p), null, `${p} should pass through`);
  }
});

test('static asset paths (file extension) pass through', () => {
  assert.equal(resolveSubdomainRewrite('margherita.dowiz.org', '/logo.png'), null);
  assert.equal(resolveSubdomainRewrite('margherita.dowiz.org', '/app.js?v=2'), null);
});

test('non-dowiz.org hosts are never rewritten', () => {
  assert.equal(resolveSubdomainRewrite('dowiz.fly.dev', '/'), null);
  assert.equal(resolveSubdomainRewrite('example.com', '/menu'), null);
});

test('apex / two-label host (dowiz.org) is not a tenant subdomain', () => {
  assert.equal(resolveSubdomainRewrite('dowiz.org', '/'), null);
});
