import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  isPlatformAdmin,
  requirePlatformAdmin,
  isAdminRoutedPath,
  auditCtx,
  auditStart,
  auditFinish,
  auditCompleted,
} from '../src/lib/platform-admin.js';

// Mock pg Pool: returns a configurable rowCount for the point-read, or throws.
function mockPool(opts: { rows?: number; throws?: boolean } = {}) {
  const queries: { sql: string; params?: any[] }[] = [];
  const pool: any = {
    async query(sql: string, params?: any[]) {
      queries.push({ sql, params });
      if (opts.throws) throw new Error('db boom');
      return { rowCount: opts.rows ?? 0 };
    },
  };
  return { pool, queries };
}

// Mock reply: captures status + whether a response was sent.
function mockReply() {
  const state: { code?: number; body?: any; sent: boolean } = { sent: false };
  const reply: any = {
    status(c: number) { state.code = c; return reply; },
    code(c: number) { state.code = c; return reply; },
    send(b: any) { state.body = b; state.sent = true; return reply; },
    get sent() { return state.sent; },
  };
  return { reply, state };
}

function mockRequest(user: any, pool: any) {
  return { user, server: { db: pool }, log: { error() {} } } as any;
}

const UID = '11111111-1111-1111-1111-111111111111';

test('isPlatformAdmin: row present → true, absent → false; query is the active point-read', async () => {
  const a = mockPool({ rows: 1 });
  assert.equal(await isPlatformAdmin(a.pool, UID), true);
  assert.match(a.queries[0].sql, /platform_admins/);
  assert.match(a.queries[0].sql, /revoked_at IS NULL/);
  assert.equal(a.queries[0].params?.[0], UID);

  const b = mockPool({ rows: 0 });
  assert.equal(await isPlatformAdmin(b.pool, UID), false);
});

test('requirePlatformAdmin: allowlisted (revoked_at IS NULL) → passes (no reply sent)', async () => {
  const { pool } = mockPool({ rows: 1 });
  const { reply, state } = mockReply();
  await requirePlatformAdmin(mockRequest({ userId: UID, role: 'owner' }, pool), reply);
  assert.equal(state.sent, false, 'gate passes → does not send a response');
});

test('requirePlatformAdmin: non-allowlisted owner → 403', async () => {
  const { pool } = mockPool({ rows: 0 });
  const { reply, state } = mockReply();
  await requirePlatformAdmin(mockRequest({ userId: UID, role: 'owner' }, pool), reply);
  assert.equal(state.code, 403);
  assert.equal(state.sent, true);
});

test('requirePlatformAdmin: revoked admin (revoked_at set → 0 rows) → 403 at request-entry', async () => {
  // After --revoke, the active partial predicate returns 0 rows → denied next request.
  const { pool } = mockPool({ rows: 0 });
  const { reply, state } = mockReply();
  await requirePlatformAdmin(mockRequest({ userId: UID, role: 'owner' }, pool), reply);
  assert.equal(state.code, 403);
});

test('requirePlatformAdmin: re-check throws (DB blip) → 503 fail CLOSED, never fail-open', async () => {
  const { pool } = mockPool({ throws: true });
  const { reply, state } = mockReply();
  await requirePlatformAdmin(mockRequest({ userId: UID, role: 'owner' }, pool), reply);
  assert.equal(state.code, 503, 'a DB error denies (503), it does NOT admit');
  assert.equal(state.sent, true);
});

test('requirePlatformAdmin: no userId (courier/customer token, or unauth) → 401, no db read', async () => {
  const { pool, queries } = mockPool({ rows: 1 });
  const { reply, state } = mockReply();
  await requirePlatformAdmin(mockRequest({ role: 'courier', sub: 'c1' }, pool), reply);
  assert.equal(state.code, 401);
  assert.equal(queries.length, 0, 'no userId → never touches the DB');
});

test('auditCtx: actor = user.userId, ip/ua hashed (no raw PII), target carried', () => {
  const ctx = auditCtx({ user: { userId: UID }, ip: '1.2.3.4', headers: { 'user-agent': 'curl/8' } }, 'backups.verify', 'bk-1');
  assert.equal(ctx.actorId, UID);
  assert.equal(ctx.action, 'backups.verify');
  assert.equal(ctx.target, 'bk-1');
  assert.match(ctx.ipHash!, /^[0-9a-f]{64}$/, 'ip is sha256-hashed, not raw');
  assert.notEqual(ctx.ipHash, '1.2.3.4');
  assert.match(ctx.uaHash!, /^[0-9a-f]{64}$/);
});

test('auditStart writes a write-ahead started row and returns its id', async () => {
  const { pool, queries } = mockPool({ rows: 1 });
  // mock RETURNING id
  pool.query = async (sql: string, params?: any[]) => { queries.push({ sql, params }); return { rows: [{ id: 42 }], rowCount: 1 }; };
  const id = await auditStart(pool, auditCtx({ user: { userId: UID }, ip: '', headers: {} }, 'backups.verify', 'bk-1'));
  assert.equal(id, '42');
  assert.match(queries[0].sql, /INSERT INTO platform_admin_audit_log/);
  assert.match(queries[0].sql, /'started'/);
});

test('auditFinish updates the row status', async () => {
  const { pool, queries } = mockPool({ rows: 1 });
  await auditFinish(pool, '42', 'completed');
  assert.match(queries[0].sql, /UPDATE platform_admin_audit_log SET status/);
  assert.deepEqual(queries[0].params, ['42', 'completed']);
});

test('auditCompleted is best-effort — a write failure does NOT throw (a read must not fail on audit)', async () => {
  const { pool } = mockPool({ throws: true });
  let threw = false;
  try { await auditCompleted(pool, auditCtx({ user: { userId: UID }, ip: '', headers: {} }, 'backups.list'), { error() {} }); }
  catch { threw = true; }
  assert.equal(threw, false, 'read-path audit swallows its own failure');
});

test('isAdminRoutedPath: gates the matched PATTERN, excludes the lookalike, ignores 404s', () => {
  const mk = (url: string | undefined) => ({ routeOptions: { url } }) as any;
  assert.equal(isAdminRoutedPath(mk('/api/admin/backups')), true);
  assert.equal(isAdminRoutedPath(mk('/api/admin/notification-audit')), true);
  assert.equal(isAdminRoutedPath(mk('/api/admin')), true);
  assert.equal(isAdminRoutedPath(mk('/api/administrators')), false, 'lookalike must NOT be gated');
  assert.equal(isAdminRoutedPath(mk('/api/orders')), false);
  assert.equal(isAdminRoutedPath(mk(undefined)), false, '404 (no matched route) → not gated, no handler reached');
});
