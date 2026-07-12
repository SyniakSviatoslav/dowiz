import { test } from 'node:test';
import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';

const BASE = 'apps/api';
const SRC = `${BASE}/src`;
const MIGRATIONS = 'packages/db/migrations';

// ── R1: Migration — fallback_config JSONB column ────────────────────
await test('R1: Migration adds fallback_config', async (t) => {
  await t.test('R1.1 migration file exists', () => {
    const files = fs.readdirSync(MIGRATIONS);
    const mig = files.find((f) => f.includes('fallback-config'));
    assert.ok(mig, 'fallback-config migration file exists');
    const content = fs.readFileSync(path.join(MIGRATIONS, mig!), 'utf8');
    assert.ok(content.includes('fallback_config'), 'contains fallback_config column');
    assert.ok(content.includes('customer_contact_reveals'), 'contains customer_contact_reveals table');
  });

  await t.test('R1.2 migration has down function', () => {
    const files = fs.readdirSync(MIGRATIONS);
    const mig = files.find((f) => f.includes('fallback-config'));
    const content = fs.readFileSync(path.join(MIGRATIONS, mig!), 'utf8');
    assert.ok(content.includes('export async function down'), 'has down function');
  });
});

// ── R2: Resilience library — withTimeout, retryWithBackoff ──────────
await test('R2: Resilience timeout lib', async (t) => {
  await t.test('R2.1 timeout.ts exports classes and functions', async () => {
    const mod = await import('../src/lib/resilience/timeout.js');
    assert.ok(typeof mod.withTimeout === 'function', 'exports withTimeout');
    assert.ok(typeof mod.withTimeoutFallback === 'function', 'exports withTimeoutFallback');
    assert.ok(typeof mod.retryWithBackoff === 'function', 'exports retryWithBackoff');
    assert.ok(mod.TimeoutError, 'exports TimeoutError class');
  });

  await t.test('R2.2 withTimeout resolves successfully', async () => {
    const mod = await import('../src/lib/resilience/timeout.js');
    const result = await mod.withTimeout(Promise.resolve('ok'), 100);
    assert.strictEqual(result, 'ok');
  });

  await t.test('R2.3 withTimeoutFallback returns fallback on timeout', async () => {
    const mod = await import('../src/lib/resilience/timeout.js');
    const result = await mod.withTimeoutFallback(
      new Promise((r) => setTimeout(r, 500)),
      50,
      'fb',
      'test',
    );
    assert.strictEqual(result, 'fb');
  });

  await t.test('R2.4 retryWithBackoff retries on failure', async () => {
    const mod = await import('../src/lib/resilience/timeout.js');
    let attempts = 0;
    const result = await mod.retryWithBackoff(async () => {
      attempts++;
      if (attempts < 2) throw new Error('fail');
      return 'ok';
    }, { maxAttempts: 3, baseMs: 10 });
    assert.strictEqual(result, 'ok');
    assert.strictEqual(attempts, 2);
  });
});

// ── R3: Customer fallback UX — shared modules ──────────────────────
await test('R3: Customer fallback phone banner', async (t) => {
  await t.test('R3.1 fallback-phone.ts exists and exports', async () => {
    const content = fs.readFileSync(`${SRC}/client/shared/fallback-phone.ts`, 'utf8');
    assert.ok(content, 'fallback-phone.ts exists');
    assert.ok(content.includes('showFallbackBanner'), 'exports showFallbackBanner');
    assert.ok(content.includes('fetchFallbackConfig'), 'exports fetchFallbackConfig');
    assert.ok(content.includes('showDegradedBanner'), 'exports showDegradedBanner');
  });

  await t.test('R3.2 error-boundary.ts exists and exports', async () => {
    const content = fs.readFileSync(`${SRC}/client/shared/error-boundary.ts`, 'utf8');
    assert.ok(content, 'error-boundary.ts exists');
    assert.ok(content.includes('installCustomerErrorBoundary'), 'exports installCustomerErrorBoundary');
  });

  await t.test('R3.3 place-order.ts dispatches fallback event on network error', () => {
    const content = fs.readFileSync(`${SRC}/client/checkout/place-order.ts`, 'utf8');
    assert.ok(content.includes('fallback:needed'), 'dispatches fallback:needed on network error');
    assert.ok(content.includes('post_failed'), 'reason includes post_failed');
  });

  await t.test('R3.4 ws.ts dispatches fallback event when max reconnects exceeded', () => {
    const content = fs.readFileSync(`${SRC}/client/status/ws.ts`, 'utf8');
    assert.ok(content.includes('fallback:needed'), 'dispatches fallback:needed');
    assert.ok(content.includes('ws_offline'), 'reason includes ws_offline');
  });

  await t.test('R3.5 pin.ts dispatches fallback event on geolocation failure', () => {
    const content = fs.readFileSync(`${SRC}/client/checkout/pin.ts`, 'utf8');
    assert.ok(content.includes('fallback:needed'), 'dispatches fallback:needed');
    assert.ok(content.includes('geocode_failed'), 'reason includes geocode_failed');
  });

  await t.test('R3.6 checkout app.ts installs error boundary and listens for fallback', () => {
    const content = fs.readFileSync(`${SRC}/client/checkout/app.ts`, 'utf8');
    assert.ok(content.includes('installCustomerErrorBoundary'), 'installs error boundary');
    assert.ok(content.includes('fallback:needed'), 'listens for fallback:needed');
  });

  await t.test('R3.7 status app.ts installs error boundary and listens for fallback', () => {
    const content = fs.readFileSync(`${SRC}/client/status/app.ts`, 'utf8');
    assert.ok(content.includes('installCustomerErrorBoundary'), 'installs error boundary');
    assert.ok(content.includes('fallback:needed'), 'listens for fallback:needed');
  });

  await t.test('R3.8 cart app.ts installs error boundary and listens for fallback', () => {
    const content = fs.readFileSync(`${SRC}/client/cart/app.ts`, 'utf8');
    assert.ok(content.includes('installCustomerErrorBoundary'), 'installs error boundary');
    assert.ok(content.includes('fallback:needed'), 'listens for fallback:needed');
  });
});

// ── R4: Owner fallback settings route ──────────────────────────────
await test('R4: Owner fallback settings route', async (t) => {
  await t.test('R4.1 route file exists with GET and PUT', () => {
    const content = fs.readFileSync(`${SRC}/routes/owner/fallback.ts`, 'utf8');
    assert.ok(content.includes('/:locationId/settings/fallback'), 'has settings/fallback endpoint');
    assert.ok(content.includes('fastify.get('), 'has GET');
    assert.ok(content.includes('fastify.put('), 'has PUT');
    assert.ok(content.includes('.strict()'), 'uses Zod .strict()');
  });

  await t.test('R4.2 agent auth hook present', () => {
    const content = fs.readFileSync(`${SRC}/routes/owner/fallback.ts`, 'utf8');
    assert.ok(content.includes('jwtVerify'), 'has auth');
    assert.ok(content.includes('owner'), 'checks owner role');
  });

  await t.test('R4.3 degradation endpoint exists', () => {
    const content = fs.readFileSync(`${SRC}/routes/owner/fallback.ts`, 'utf8');
    assert.ok(content.includes('/:locationId/degradation'), 'has degradation endpoint');
    assert.ok(content.includes('deadChannels'), 'reports dead channels');
  });
});

// ── R5: Owner reveal-customer-contact route ────────────────────────
await test('R5: Reveal customer contact route', async (t) => {
  await t.test('R5.1 route file exists with POST endpoint', () => {
    const content = fs.readFileSync(`${SRC}/routes/owner/reveal-contact.ts`, 'utf8');
    assert.ok(content, 'reveal-contact.ts exists');
    assert.ok(content.includes('reveal-customer-contact'), 'has reveal-customer-contact endpoint');
    assert.ok(content.includes('fastify.post('), 'uses POST');
    assert.ok(content.includes('.strict()'), 'uses Zod .strict()');
  });

  await t.test('R5.2 rate limited', () => {
    const content = fs.readFileSync(`${SRC}/routes/owner/reveal-contact.ts`, 'utf8');
    assert.ok(content.includes('rateLimit'), 'has rate limit config');
    assert.ok(content.includes('10'), 'max 10 per minute');
  });

  await t.test('R5.3 inserts audit record', () => {
    const content = fs.readFileSync(`${SRC}/routes/owner/reveal-contact.ts`, 'utf8');
    assert.ok(content.includes('customer_contact_reveals'), 'inserts into audit table');
  });

  await t.test('R5.4 emits PII-free MessageBus event', () => {
    const content = fs.readFileSync(`${SRC}/routes/owner/reveal-contact.ts`, 'utf8');
    assert.ok(content.includes('customer.contact_revealed'), 'emits contact_revealed event');
  });
});

// ── R6: Public fallback-config endpoint ────────────────────────────
await test('R6: Public fallback-config endpoint', async (t) => {
  await t.test('R6.1 route file exists', () => {
    const content = fs.readFileSync(`${SRC}/routes/public/fallback-config.ts`, 'utf8');
    assert.ok(content, 'fallback-config.ts exists');
    assert.ok(content.includes('/api/public/locations/:slug/fallback-config'), 'has public endpoint');
  });
});

// ── R7: Admin fallback route ───────────────────────────────────────
await test('R7: Admin fallback route', async (t) => {
  await t.test('R7.1 admin route file exists', () => {
    const content = fs.readFileSync(`${SRC}/routes/admin/fallback.ts`, 'utf8');
    assert.ok(content, 'admin/fallback.ts exists');
    assert.ok(content.includes('/fallback/health'), 'has health endpoint');
    assert.ok(content.includes('/fallback/r2-check'), 'has coverage check');
  });
});

// ── R8: Owner settings HTML page ───────────────────────────────────
await test('R8: Owner fallback settings UI', async (t) => {
  await t.test('R8.1 settings-fallback.html exists', () => {
    const content = fs.readFileSync(`${BASE}/src/public/admin/settings-fallback.html`, 'utf8');
    assert.ok(content, 'settings-fallback.html exists');
    assert.ok(content.includes('fallbackPhone'), 'has phone input');
    assert.ok(content.includes('showPhoneOnError'), 'has show on error toggle');
    assert.ok(content.includes('showPhoneOnOffline'), 'has show on offline toggle');
    assert.ok(content.includes('saveSettings'), 'has save function');
  });

  await t.test('R8.2 includes dashboard.js', () => {
    const content = fs.readFileSync(`${BASE}/src/public/admin/settings-fallback.html`, 'utf8');
    assert.ok(content.includes('dashboard.js'), 'includes dashboard.js');
  });
});

// ── R9: Dashboard dead-channel detection ───────────────────────────
await test('R9: Dashboard dead-channel banner', async (t) => {
  await t.test('R9.1 dashboard.js has checkDeadChannels', () => {
    const content = fs.readFileSync(`${BASE}/src/public/admin/dashboard.js`, 'utf8');
    assert.ok(content.includes('checkDeadChannels'), 'has checkDeadChannels function');
    assert.ok(content.includes('reEnableChannel'), 'has reEnableChannel function');
    assert.ok(content.includes('deadChannelBanner'), 'references deadChannelBanner element');
  });

  await t.test('R9.2 dashboard.html has deadChannelBanner', () => {
    const content = fs.readFileSync(`${BASE}/src/public/admin/dashboard.html`, 'utf8');
    assert.ok(content.includes('deadChannelBanner'), 'has dead channel banner div');
    assert.ok(content.includes('checkDeadChannels('), 'calls checkDeadChannels on init');
  });
});

// ── R10: Health endpoint — fallback check ──────────────────────────
await test('R10: Health endpoint extended with fallback', async (t) => {
  await t.test('R10.1 health.ts references fallback', () => {
    const content = fs.readFileSync(`${SRC}/routes/health.ts`, 'utf8');
    assert.ok(content.includes('fallback'), 'health.ts checks fallback');
    assert.ok(content.includes('fallback_config'), 'queries fallback_config column');
    assert.ok(content.includes('coveragePct'), 'reports coverage percentage');
  });
});

// ── R11: Server registration ───────────────────────────────────────
await test('R11: Route registration in server.ts', async (t) => {
  await t.test('R11.1 owner fallback routes imported and registered', () => {
    const content = fs.readFileSync(`${SRC}/server.ts`, 'utf8');
    assert.ok(content.includes('ownerFallbackRoutes'), 'imports ownerFallbackRoutes');
    assert.ok(content.includes('ownerRevealContactRoutes'), 'imports ownerRevealContactRoutes');
    assert.ok(content.includes('publicFallbackConfigRoutes'), 'imports publicFallbackConfigRoutes');
    assert.ok(content.includes('/api/owner/locations'), 'registers with owner prefix');
    assert.ok(content.includes('/api/admin'), 'registers admin fallback route');
  });
});

// ── R12: Migration column presence verification ────────────────────
await test('R12: Data model verification', async (t) => {
  await t.test('R12.1 fallback_config check constraint exists', () => {
    const migFile = fs.readdirSync(MIGRATIONS).find((f) => f.includes('fallback-config'));
    const content = fs.readFileSync(path.join(MIGRATIONS, migFile!), 'utf8');
    assert.ok(content.includes('jsonb_typeof(fallback_config)'), 'has jsonb type check');
  });
});
