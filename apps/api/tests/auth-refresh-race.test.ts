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

// A fake db modelling ONE refresh-token row. The guarded UPDATE is the atomicity
// primitive: it flips used only while still false, returning rowCount accordingly.
function makeDb(initialUsed = false) {
  const state = { used: initialUsed, deletedFamilies: [] as string[], rotations: 0 };
  const db = {
    state,
    query: async (sql: string, params: any[] = []) => {
      if (/^SELECT \* FROM auth_refresh_tokens WHERE token_hash/i.test(sql)) {
        return { rowCount: 1, rows: [{ id: 'tok-1', family_id: FAMILY, user_id: 'user-1', used: state.used, expires_at: new Date(Date.now() + 86_400_000) }] };
      }
      if (/UPDATE auth_refresh_tokens SET used = true WHERE id = \$1 AND used = false/i.test(sql)) {
        if (state.used) return { rowCount: 0, rows: [] };
        state.used = true;                       // claim wins
        return { rowCount: 1, rows: [{ id: 'tok-1' }] };
      }
      // Benign-concurrent detector: "was a token in this family created in the last N seconds?"
      // Models a sibling request's freshly-INSERTed rotation token.
      if (/SELECT 1 FROM auth_refresh_tokens WHERE family_id/i.test(sql)) {
        return state.rotations > 0 ? { rowCount: 1, rows: [{ '?column?': 1 }] } : { rowCount: 0, rows: [] };
      }
      // P-c (ADR-0004): live owner-membership re-derive on every refresh.
      if (/SELECT location_id FROM memberships/i.test(sql)) {
        return { rowCount: 1, rows: [{ location_id: 'loc-1' }] };
      }
      if (/DELETE FROM auth_refresh_tokens WHERE family_id/i.test(sql)) {
        state.deletedFamilies.push(params[0]);
        return { rowCount: 1, rows: [] };
      }
      if (/INSERT INTO auth_refresh_tokens/i.test(sql)) {
        state.rotations += 1;                    // new rotation token in the family
        return { rowCount: 1, rows: [] };
      }
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
  // Same decorator server.ts registers (ADR-0010 A2) — without it every
  // reply.sendError route 500s in a bare test app.
  registerReplySendError(app as any);
  // Mirror server.ts: native Zod safeParse validator (the routes use Zod schemas).
  app.setValidatorCompiler(({ schema }: any) => (data: any) => {
    const r = schema.safeParse(data);
    return r.success ? { value: r.data } : { error: r.error };
  });
  (app as any).decorate('db', db);
  (app as any).decorate('redis', { setex: async () => {}, get: async () => null });
  // server.ts decorates verifyAuth (used by /auth/logout's preHandler); the
  // refresh tests never hit it — a pass-through keeps route registration valid.
  (app as any).decorate('verifyAuth', async () => {});
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
    assert.ok(body.access_token && body.refresh_token, 'new access+refresh tokens issued');
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

  await t.test('TWO concurrent refreshes of the same token → exactly one 200 (race closed)', async () => {
    const db = makeDb(false);
    const app = await buildApp(db);
    const [a, b] = await Promise.all([
      app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } }),
      app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } }),
    ]);
    const codes = [a.statusCode, b.statusCode].sort();
    // The single-use invariant: exactly ONE winner ever mints tokens. The loser is
    // rejected either as a benign concurrent sibling (soft 409, no revoke — the
    // winner's rotation INSERT already landed) or, if it raced ahead of that INSERT,
    // via the reuse path (401 + family revoked). Never two 200s.
    assert.equal(codes[0], 200, 'exactly one request wins the rotation');
    assert.ok(codes[1] === 401 || codes[1] === 409, `loser must be rejected (got ${codes[1]})`);
    if (codes[1] === 401) {
      assert.deepEqual(db.state.deletedFamilies, [FAMILY], 'a 401 loser means the reuse path revoked the family');
    } else {
      assert.deepEqual(db.state.deletedFamilies, [], 'a benign 409 loser must NOT revoke the family');
    }
    await app.close();
  });

  await t.test('sibling refresh right after a rotation → soft 409 concurrent_refresh, family kept', async () => {
    const db = makeDb(false);
    const app = await buildApp(db);
    const first = await app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } });
    assert.equal(first.statusCode, 200, 'first refresh rotates normally');
    // Same (now-used) token again, with a fresh rotation in the family (two tabs /
    // StrictMode / parallel 401-retries): benign concurrent loser, NOT a breach.
    const second = await app.inject({ method: 'POST', url: '/auth/refresh', payload: { refresh_token: 'r1' } });
    assert.equal(second.statusCode, 409, 'benign concurrent sibling gets a soft 409');
    assert.equal(second.json().error, 'concurrent_refresh');
    assert.deepEqual(db.state.deletedFamilies, [], 'family must NOT be revoked on a benign concurrent refresh');
    await app.close();
  });
});
