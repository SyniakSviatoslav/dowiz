import test from 'node:test';
import assert from 'node:assert';
import { redactUrlSecrets, getFastifyLoggerConfig } from '../src/lib/logger.js';

// Guardrail (P1) for docs/design/ws-token-in-url/escalation.md — the WS authenticates
// via `?token=<JWT>` and the Pino `req` serializer logs `req.url`; without redaction the
// full bearer token reaches app/Fly/aggregator logs on every WS upgrade.
// RED→GREEN: against the pre-fix serializer (url: req.url) every assertion below fails.

const JWT = 'eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJjOXNlY3JldCJ9.SIGNATUREabc123';

test('redactUrlSecrets strips token on the main WS upgrade URL', () => {
  const out = redactUrlSecrets(`/ws?token=${JWT}`);
  assert.ok(!out.includes(JWT), `token leaked: ${out}`);
  assert.strictEqual(out, '/ws?token=[REDACTED]');
});

test('redactUrlSecrets strips token on the order-status widget URL', () => {
  const out = redactUrlSecrets(`/ws/orders/abc-123?token=${JWT}`);
  assert.ok(!out.includes(JWT));
  assert.strictEqual(out, '/ws/orders/abc-123?token=[REDACTED]');
});

test('redactUrlSecrets redacts secret params but keeps benign ones', () => {
  const out = redactUrlSecrets(`/api/x?limit=10&token=${JWT}&sort=asc`);
  assert.ok(!out.includes(JWT));
  assert.ok(out.includes('limit=10'));
  assert.ok(out.includes('sort=asc'));
  assert.ok(out.includes('token=[REDACTED]'));
});

test('redactUrlSecrets also covers refresh_token / api_key / access_token', () => {
  for (const k of ['access_token', 'refresh_token', 'api_key', 'secret', 'jwt', 'auth']) {
    const out = redactUrlSecrets(`/p?${k}=${JWT}`);
    assert.ok(!out.includes(JWT), `${k} leaked`);
    assert.strictEqual(out, `/p?${k}=[REDACTED]`);
  }
});

test('redactUrlSecrets leaves URLs without a query string untouched', () => {
  assert.strictEqual(redactUrlSecrets('/ws'), '/ws');
  assert.strictEqual(redactUrlSecrets('/api/owner/orders'), '/api/owner/orders');
});

test('the live Pino req serializer applies the redaction', () => {
  const cfg: any = getFastifyLoggerConfig();
  const serialized = cfg.serializers.req({ method: 'GET', url: `/ws?token=${JWT}`, hostname: 'h', ip: '127.0.0.1' });
  assert.ok(!String(serialized.url).includes(JWT), `serializer leaked token: ${serialized.url}`);
  assert.strictEqual(serialized.url, '/ws?token=[REDACTED]');
});
