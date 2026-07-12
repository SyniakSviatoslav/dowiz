// synthetic-probe tests (node:test, stdlib only) — proves the degraded-detection net actually fires.
//   • healthy payload      → healthy=true, exit 0, NO alert
//   • 503 / sub-check down  → healthy=false, exit non-zero, alert ATTEMPTED
//   • env-unset             → alert skips LOUDLY, never throws, exit still non-zero on down
// Unit tests inject a mock fetch (no network). Integration tests spawn the REAL script against a
// local http server for true exit-code proof. plane-telemetry is neutralized via the kill-switch.
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import {
  classifyHealth, probeOnce, sendTelegramAlert, runProbe,
} from './synthetic-probe.mjs';

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), 'synthetic-probe.mjs');
const jsonResponse = (status, body) => ({ status, ok: status >= 200 && status < 300, json: async () => body });
const noTelemetry = () => ({ emitted: false, reason: 'test' });

// ---------------------------------------------------------------------------
// classifyHealth — the pure decision surface
// ---------------------------------------------------------------------------
test('classifyHealth: 200 + healthy + all sub-checks ok → healthy', () => {
  const v = classifyHealth({ httpStatus: 200, body: { status: 'healthy', checks: { postgres: { status: 'ok' } } } });
  assert.equal(v.healthy, true);
  assert.deepEqual(v.reasons, []);
});

test('classifyHealth: 503 unhealthy with pg down → non-healthy, reasons captured', () => {
  const v = classifyHealth({ httpStatus: 503, body: { status: 'unhealthy', checks: { postgres: { status: 'down' } } } });
  assert.equal(v.healthy, false);
  assert.ok(v.reasons.includes('http_503'));
  assert.ok(v.reasons.includes('status_unhealthy'));
  assert.deepEqual(v.downChecks, ['postgres']);
});

test('classifyHealth: 200 degraded → non-healthy (degraded pages too)', () => {
  const v = classifyHealth({ httpStatus: 200, body: { status: 'degraded', checks: { r2: { status: 'degraded' } } } });
  assert.equal(v.healthy, false);
  assert.ok(v.reasons.includes('status_degraded'));
});

test('classifyHealth: 200 healthy overall but a sub-check down → non-healthy', () => {
  const v = classifyHealth({ httpStatus: 200, body: { status: 'healthy', checks: { telegram: { status: 'down' } } } });
  assert.equal(v.healthy, false);
  assert.deepEqual(v.downChecks, ['telegram']);
});

test('classifyHealth: request error / non-JSON → non-healthy', () => {
  assert.equal(classifyHealth({ error: 'ECONNREFUSED' }).healthy, false);
  assert.equal(classifyHealth({ httpStatus: 200, body: undefined }).healthy, false);
});

// ---------------------------------------------------------------------------
// runProbe — orchestration: healthy → no alert; down → alert attempted
// ---------------------------------------------------------------------------
test('runProbe: healthy payload → healthy=true, NO alert attempted', async () => {
  let alertCalls = 0;
  const fetchImpl = async () => jsonResponse(200, { status: 'healthy', checks: { postgres: { status: 'ok' } } });
  const r = await runProbe({
    url: 'http://x/health', fetchImpl,
    alertFn: async () => { alertCalls++; return { attempted: true }; },
    telemetryFn: noTelemetry,
  });
  assert.equal(r.healthy, true);
  assert.equal(alertCalls, 0, 'no alert on a healthy probe');
});

test('runProbe: 503 + sub-check down → healthy=false, alert ATTEMPTED', async () => {
  let alertCalls = 0;
  const fetchImpl = async () => jsonResponse(503, { status: 'unhealthy', checks: { postgres: { status: 'down' } } });
  const r = await runProbe({
    url: 'http://x/health', fetchImpl,
    alertFn: async () => { alertCalls++; return { attempted: true }; },
    telemetryFn: noTelemetry,
  });
  assert.equal(r.healthy, false);
  assert.equal(alertCalls, 1, 'alert attempted exactly once on a down probe');
  assert.deepEqual(r.downChecks, ['postgres']);
});

test('probeOnce: fetch throws → non-healthy, never throws out', async () => {
  const fetchImpl = async () => { throw new Error('ETIMEDOUT'); };
  const r = await probeOnce({ url: 'http://x/health', fetchImpl });
  assert.equal(r.healthy, false);
  assert.ok(r.reasons[0].startsWith('request_error'));
});

// ---------------------------------------------------------------------------
// sendTelegramAlert — env-unset skips LOUDLY and never throws; set env → attempted
// ---------------------------------------------------------------------------
test('sendTelegramAlert: env unset → skips cleanly (attempted=false, skipped=true), never throws', async () => {
  const res = await sendTelegramAlert(
    { url: 'u', httpStatus: 503, overall: 'unhealthy', latencyMs: 1, downChecks: ['postgres'], reasons: ['http_503'], ts: 't' },
    { env: {}, fetchImpl: async () => { throw new Error('should not be called'); } },
  );
  assert.equal(res.attempted, false);
  assert.equal(res.skipped, true);
  assert.equal(res.reason, 'env_unset');
});

test('sendTelegramAlert: env set → attempts the send (fetch called, token never in args except URL)', async () => {
  let called = null;
  const fetchImpl = async (url, init) => { called = { url, init }; return { ok: true, status: 200 }; };
  const res = await sendTelegramAlert(
    { url: 'u', httpStatus: 503, overall: 'unhealthy', latencyMs: 1, downChecks: ['postgres'], reasons: ['http_503'], ts: 't' },
    { env: { TELEGRAM_BOT_TOKEN: 'BOTTOK', PLANE_REPORT_CHAT_ID: '42' }, fetchImpl },
  );
  assert.equal(res.attempted, true);
  assert.equal(res.sent, true);
  assert.ok(called.url.includes('/botBOTTOK/sendMessage'));
  assert.ok(!called.init.body.includes('BOTTOK'), 'token must live only in the URL, never the body');
});

// ---------------------------------------------------------------------------
// Integration — spawn the REAL script against a local server → true exit codes + env-unset path
// (PLANE_TELEMETRY_DISABLED neutralizes the telemetry side-effect)
// ---------------------------------------------------------------------------
function serveOnce(payload, httpStatus) {
  return new Promise((resolve) => {
    const server = createServer((req, res) => {
      res.writeHead(httpStatus, { 'content-type': 'application/json' });
      res.end(JSON.stringify(payload));
    });
    server.listen(0, '127.0.0.1', () => resolve({ server, port: server.address().port }));
  });
}

// async spawn — spawnSync would BLOCK the parent event loop, starving the in-process http server.
function runScript(url) {
  return new Promise((resolve) => {
    const child = spawn(process.execPath, [SCRIPT, '--once', '--url', url], {
      env: { ...process.env, PLANE_TELEMETRY_DISABLED: 'true', TELEGRAM_BOT_TOKEN: '', PLANE_REPORT_CHAT_ID: '' },
    });
    let stdout = ''; let stderr = '';
    child.stdout.on('data', (d) => { stdout += d; });
    child.stderr.on('data', (d) => { stderr += d; });
    child.on('close', (status) => resolve({ status, stdout, stderr }));
  });
}

test('integration: healthy /health → exit 0', async () => {
  const { server, port } = await serveOnce({ status: 'healthy', checks: { postgres: { status: 'ok' } } }, 200);
  try {
    const r = await runScript(`http://127.0.0.1:${port}/health`);
    assert.equal(r.status, 0, r.stderr);
    assert.match(r.stdout, /healthy=true/);
  } finally { server.close(); }
});

test('integration: 503 /health with pg down + env unset → exit 1 + LOUD skip banner (never throws)', async () => {
  const { server, port } = await serveOnce({ status: 'unhealthy', checks: { postgres: { status: 'down' } } }, 503);
  try {
    const r = await runScript(`http://127.0.0.1:${port}/health`);
    assert.equal(r.status, 1, 'a down probe must exit non-zero even with the alert skipped');
    assert.match(r.stdout, /healthy=false/);
    assert.match(r.stderr, /TELEGRAM ALERT SKIPPED — env unset/);
  } finally { server.close(); }
});
