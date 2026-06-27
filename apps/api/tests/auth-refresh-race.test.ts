import './_env-stub.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';

// ADR-0003-adjacent (#5): /auth/refresh rotation must be ATOMIC. The old code did
// SELECT (check used) → UPDATE used=true as two statements, so two concurrent requests
// could both read used=false, both pass the JS check, and both mint a fresh family —
// defeating single-use rotation AND bypassing reuse-detection. The fix claims the token
// with a guarded `UPDATE ... WHERE id=$1 AND used=false RETURNING id`; rowCount 0 ⇒ reuse
// ⇒ revoke the family. This test drives the REAL route handler via fastify.inject with a
// fake db that models the guarded-update (only the first claim flips used=false→true).

// Self-contained env (auth.ts calls loadEnv() at module load) — real RSA keypair so
// signAuthToken works; everything else dummy. Set BEFORE the dynamic import below.
const { privateKey, publicKey } = crypto.generateKeyPairSync('rsa', {
  modulusLength: 2048,
  publicKeyEncoding: { type: 'spki', format: 'pem' },
  privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
});
Object.assign(process.env, {
  NODE_ENV: 'test',
  APP_BASE_URL: 'https://x.test',
  DATABASE_URL_OPERATIONAL: 'postgres://u:p@localhost:5432/d',
  DATABASE_URL_SESSION: 'postgres://u:p@localhost:5432/d',
  DATABASE_URL_MIGRATIONS: 'postgres://u:p@localhost:5432/d',
  REDIS_URL: 'redis://localhost:6379',
  JWT_PRIVATE_KEY: privateKey, JWT_PUBLIC_KEY: publicKey, JWT_KID: 'test',
  GOOGLE_CLIENT_ID: 'x', GOOGLE_CLIENT_SECRET: 'x',
  VAPID_PUBLIC_KEY: 'x', VAPID_PRIVATE_KEY: 'x', IP_HASH_SALT: 'x',
});

const FAMILY = 'fam-1';
const LOCATION = '11111111-1111-1111-1111-111111111111';

// A fake db modelling ONE refresh-token row. The guarded UPDATE is the atomicity
// primitive: it flips used only while still false, returning rowCount accordingly.
//   tokenFound=false  → the initial lookup misses (unknown/garbage token).
//   recentFamily=true → a sibling token in this family rotated <5s ago (benign
//                       concurrent loser → soft 409, family preserved).
function makeDb(
  initialUsed = false,
  opts: { tokenFound?: boolean; recentFamily?: boolean } = {},
) {
  const { tokenFound = true, recentFamily = false } = opts;
  const state = { used: initialUsed, deletedFamilies: [] as string[] };
  const db = {
    state,
    query: async (sql: string, params: any[] = []) => {
      if (/^SELECT \* FROM auth_refresh_tokens WHERE token_hash/i.test(sql)) {
        // Yield a real macrotask so two concurrent inject() calls BOTH complete this
        // lookup before either reaches the guarded UPDATE — forcing true interleaving.
        // Without it Node drains the first request's microtask chain to completion and
        // the race is never actually exercised (the loser would trivially see used=true).
        await new Promise((resolve) => setTimeout(resolve, 0));
        if (!tokenFound) return { rowCount: 0, rows: [] };
        return { rowCount: 1, rows: [{ id: 'tok-1', family_id: FAMILY, user_id: 'user-1', used: state.used, expires_at: new Date(Date.now() + 86_400_000) }] };
      }
      if (/UPDATE auth_refresh_tokens SET used = true WHERE id = \$1 AND used = false/i.test(sql)) {
        if (state.used) return { rowCount: 0, rows: [] };
        state.used = true;                       // claim wins
        return { rowCount: 1, rows: [{ id: 'tok-1' }] };
      }
      // Reuse-vs-benign-concurrent disambiguator (auth.ts:276-279): a recent sibling rotation.
      if (/^SELECT 1 FROM auth_refresh_tokens WHERE family_id = \$1 AND created_at/i.test(sql)) {
        return recentFamily ? { rowCount: 1, rows: [{ '?column?': 1 }] } : { rowCount: 0, rows: [] };
      }
      // Re-derive owner authority from live memberships (auth.ts:293-298) — happy path needs ≥1.
      if (/^SELECT location_id\s+FROM memberships/i.test(sql)) {
        return { rowCount: 1, rows: [{ location_id: LOCATION }] };
      }
      if (/DELETE FROM auth_refresh_tokens WHERE family_id/i.test(sql)) {
        state.deletedFamilies.push(params[0]);
        return { rowCount: 1, rows: [] };
      }
      if (/INSERT INTO auth_refresh_tokens/i.test(sql)) return { rowCount: 1, rows: [] };
      return { rowCount: 0, rows: [] };
    },
  };
  return db;
}

async function buildApp(db: any) {
  const Fastify = (await import('fastify')).default;
  const { default: authRoutes } = await import('../src/routes/auth.js');
  const { registerReplySendError } = await import('../src/lib/reply-send-error.js');
  const app = Fastify();
  // Mirror server.ts: native Zod safeParse validator (the routes use Zod schemas).
  app.setValidatorCompiler(({ schema }: any) => (data: any) => {
    const r = schema.safeParse(data);
    return r.success ? { value: r.data } : { error: r.error };
  });
  // auth.ts:251/285/300 emit error envelopes via reply.sendError — register it like server.ts,
  // else those 401/409 paths throw 500 (the A2-sweep regression). Required, not cosmetic.
  registerReplySendError(app as any);
  // /auth/logout (auth.ts:326) registers verifyAuth as a preHandler at PLUGIN LOAD; without the
  // decorator authRoutes fails to register and EVERY route (incl. /auth/refresh) is unwired →
  // FST_ERR_HOOK_INVALID_HANDLER. The refresh tests never hit logout, so a no-op is inert here.
  (app as any).decorate('verifyAuth', async () => {});
  (app as any).decorate('db', db);
  (app as any).decorate('redis', { setex: async () => {}, get: async () => null });
  await app.register(authRoutes);
  await app.ready();
  return app;
}

test('/auth/refresh atomic rotation (#5)', async (t) => {
  await t.test('a fresh token rotates once → 200 with new tokens', async () => {
    const db = makeDb(false);
    const app = await buildApp(db);
    const res = await app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } });
    assert.equal(res.statusCode, 200);
    const body = res.json();
    // Shape-assert, not truthy: access_token is an RS256 JWT (3 base64url segments);
    // refresh_token is 32 random bytes hex (auth.ts:308). A truthy check would pass on "x".
    assert.match(body.access_token, /^[\w-]+\.[\w-]+\.[\w-]+$/, 'access_token is a JWT');
    assert.match(body.refresh_token, /^[0-9a-f]{64}$/, 'refresh_token is 32-byte hex');
    assert.equal(db.state.used, true, 'token marked used');
    assert.equal(db.state.deletedFamilies.length, 0, 'no family revoked on a clean rotation');
    await app.close();
  });

  await t.test('replay of an already-used token → 401 + family revoked', async () => {
    const db = makeDb(true);   // token already used
    const app = await buildApp(db);
    const res = await app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } });
    assert.equal(res.statusCode, 401);
    assert.deepEqual(db.state.deletedFamilies, [FAMILY], 'reuse must revoke the whole family');
    await app.close();
  });

  await t.test('benign concurrent loser (recent family rotation) → 409, family NOT revoked', async () => {
    // Token already consumed by a sibling that rotated <5s ago: the loser must get a soft 409 so
    // the client retries with the freshly-stored token — revoking would log every session out
    // (auth.ts:269-282, the "expires too soon" bug).
    const db = makeDb(true, { recentFamily: true });
    const app = await buildApp(db);
    const res = await app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } });
    assert.equal(res.statusCode, 409);
    assert.equal(res.json().error, 'concurrent_refresh');
    assert.equal(db.state.deletedFamilies.length, 0, 'a benign concurrent refresh must NOT revoke the family');
    await app.close();
  });

  await t.test('unknown refresh token → 401, no family touched', async () => {
    const db = makeDb(false, { tokenFound: false });
    const app = await buildApp(db);
    const res = await app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'garbage' } });
    assert.equal(res.statusCode, 401);
    assert.equal(db.state.deletedFamilies.length, 0, 'a lookup miss must not revoke anything');
    await app.close();
  });

  await t.test('TWO concurrent refreshes of the same token → exactly one 200, one 401 (race closed)', async () => {
    const db = makeDb(false);
    const app = await buildApp(db);
    const [a, b] = await Promise.all([
      app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } }),
      app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } }),
    ]);
    const codes = [a.statusCode, b.statusCode].sort();
    assert.deepEqual(codes, [200, 401], 'exactly one request may win the rotation; the loser is rejected');
    assert.deepEqual(db.state.deletedFamilies, [FAMILY], 'the losing (reuse) request revokes the family');
    await app.close();
  });
});
