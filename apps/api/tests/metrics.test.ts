import test from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import { recordHttp, renderMetrics, resetMetrics, registerMetrics } from '../src/lib/metrics.js';

test('metrics registry', async (t) => {
  await t.test('counts requests by method/route/status and buckets latency', async () => {
    resetMetrics();
    recordHttp('GET', '/api/orders/:id', 200, 0.03);
    recordHttp('GET', '/api/orders/:id', 200, 0.2);
    recordHttp('GET', '/api/orders/:id', 500, 4);
    const out = await renderMetrics();
    assert.match(out, /http_requests_total\{method="GET",route="\/api\/orders\/:id",status="200"\} 2/);
    assert.match(out, /http_requests_total\{method="GET",route="\/api\/orders\/:id",status="500"\} 1/);
    // 0.03 lands in le="0.05"; only it is ≤0.05. All three are ≤ +Inf.
    assert.match(out, /http_request_duration_seconds_bucket\{method="GET",route="\/api\/orders\/:id",le="0.05"\} 1/);
    assert.match(out, /http_request_duration_seconds_bucket\{method="GET",route="\/api\/orders\/:id",le="\+Inf"\} 3/);
    assert.match(out, /http_request_duration_seconds_count\{method="GET",route="\/api\/orders\/:id"\} 3/);
  });

  await t.test('gauges are read per scrape; a throwing gauge never breaks the scrape', async () => {
    resetMetrics();
    const out = await renderMetrics({
      pg_pool_total: () => 7,
      broken_gauge: () => { throw new Error('boom'); },
    });
    assert.match(out, /pg_pool_total 7/);
    assert.ok(!out.includes('broken_gauge'), 'throwing gauge must be omitted');
    assert.match(out, /process_resident_memory_bytes \d+/);
  });

  await t.test('label values are escaped', async () => {
    resetMetrics();
    recordHttp('GET', '/weird"route\\x', 200, 0.01);
    const out = await renderMetrics();
    assert.match(out, /route="\/weird\\"route\\\\x"/);
  });
});

test('GET /metrics gating', async (t) => {
  const prev = process.env.METRICS_TOKEN;

  await t.test('404 when METRICS_TOKEN unset (dark by default)', async () => {
    delete process.env.METRICS_TOKEN;
    const app = Fastify();
    registerMetrics(app);
    const res = await app.inject({ method: 'GET', url: '/metrics' });
    assert.equal(res.statusCode, 404);
    await app.close();
  });

  await t.test('401 on wrong token, 200 + text format on correct bearer', async () => {
    process.env.METRICS_TOKEN = 'scrape-secret';
    resetMetrics();
    const app = Fastify();
    registerMetrics(app, { ws_connections: () => 3 });
    const bad = await app.inject({ method: 'GET', url: '/metrics', headers: { authorization: 'Bearer nope' } });
    assert.equal(bad.statusCode, 401);
    const ok = await app.inject({ method: 'GET', url: '/metrics', headers: { authorization: 'Bearer scrape-secret' } });
    assert.equal(ok.statusCode, 200);
    assert.match(ok.headers['content-type'] as string, /text\/plain/);
    assert.match(ok.body, /ws_connections 3/);
    await app.close();
  });

  await t.test('requests to other routes are recorded; /metrics itself is not', async () => {
    process.env.METRICS_TOKEN = 'scrape-secret';
    resetMetrics();
    const app = Fastify();
    registerMetrics(app);
    app.get('/ping', async () => ({ ok: true }));
    await app.inject({ method: 'GET', url: '/ping' });
    const res = await app.inject({ method: 'GET', url: '/metrics', headers: { authorization: 'Bearer scrape-secret' } });
    assert.match(res.body, /http_requests_total\{method="GET",route="\/ping",status="200"\} 1/);
    assert.ok(!res.body.includes('route="/metrics"'), '/metrics must not self-record');
    await app.close();
  });

  if (prev === undefined) delete process.env.METRICS_TOKEN; else process.env.METRICS_TOKEN = prev;
});
