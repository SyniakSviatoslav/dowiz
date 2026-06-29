import type { FastifyPluginAsync } from 'fastify';
import { requirePlatformAdmin } from '../../lib/platform-admin.js';
import backupAdminRoutes from './backups.js';
import fallbackAdminRoutes from './fallback.js';
import notificationAuditRoutes from './notification-audit.js';

/**
 * ADR-admin-platform-authz (B4) — the admin plane parent (RESOLVE F1 / RA2-1).
 *
 * This is the ONLY thing mounted at `prefix:/api/admin`. It registers `verifyAuth` THEN
 * `requirePlatformAdmin` as `onRequest` hooks (order load-bearing: `request.user` must be populated
 * before the gate dereferences `userId`), then the three children with NO prefix (they inherit
 * `/api/admin` + both hooks). The children carry NO per-file `verifyAuth`/`requireRole(['owner'])` —
 * this parent (and the root-instance gate in server.ts) fully replaces them.
 *
 * Defense-in-depth: the STRUCTURAL authority for siblings/future routes is the root-instance
 * `onRequest` gate (registerAdminPlaneGate in server.ts); this parent is the organizational primary
 * for the 3 known routes. On a <1 req/s plane the redundant point-read is negligible (root denies
 * short-circuit, so an owner pays exactly one read).
 */
const adminPlane: FastifyPluginAsync = async (fastify, opts) => {
  fastify.addHook('onRequest', fastify.verifyAuth);      // (1) MUST be first — populates request.user
  fastify.addHook('onRequest', requirePlatformAdmin);    // (2) the gate: 403 non-admin / 503 fail-closed
  // Children inherit THIS plugin's /api/admin prefix from the encapsulation — they must NOT re-apply
  // it. `opts` carries the `prefix:'/api/admin'` Fastify passed us; forwarding it verbatim would
  // double-prefix the children to /api/admin/api/admin/* (→ the real /api/admin/* 404s). Strip it.
  const { prefix: _ignored, ...childOpts } = opts as Record<string, unknown>;
  await fastify.register(backupAdminRoutes, childOpts);
  await fastify.register(fallbackAdminRoutes, childOpts);
  await fastify.register(notificationAuditRoutes, childOpts);
};

export default adminPlane;
