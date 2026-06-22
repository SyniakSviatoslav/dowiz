import fp from 'fastify-plugin';
import type { FastifyRequest, FastifyReply } from 'fastify';
import { verifyAuthToken } from '@deliveryos/platform';
import { AuthToken } from '@deliveryos/shared-types';
import { loadEnv } from '@deliveryos/config';
import { devLoginAllowed } from './dev-guard.js';

const env = loadEnv();

export interface CourierSessionRow {
  courier_id: string;
  revoked_at: Date | string | null;
  expires_at: Date | string | null;
  has_location: boolean;
}

/**
 * Decide whether a courier's access token is still live. A signed JWT alone is
 * not enough: the token must map to a courier_sessions row that is (a) present,
 * (b) not revoked — logout / password-change / refresh-rotation all set
 * revoked_at — (c) not past its own expiry, and (d) the courier must STILL hold
 * membership in the token's activeLocationId. Pure so it is unit-testable.
 */
export function courierSessionValid(row: CourierSessionRow | null | undefined, nowMs: number): boolean {
  if (!row) return false;
  if (row.revoked_at) return false;
  if (row.expires_at && new Date(row.expires_at).getTime() < nowMs) return false;
  if (!row.has_location) return false;
  return true;
}

declare module 'fastify' {
  interface FastifyRequest {
    user: AuthToken | null;
  }
  interface FastifyInstance {
    verifyAuth: any;
    softVerifyAuth: any;
    requireRole: (roles: AuthToken['role'][]) => any;
    requireLocationAccess: any;
  }
}

export const verifyAuth = async (request: FastifyRequest, reply: FastifyReply) => {
  const authHeader = request.headers.authorization;
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return reply.status(401).send({ error: 'Missing or invalid token' });
  }

  const token = authHeader.substring(7);
  try {
    request.user = await verifyAuthToken(token);
  } catch (err) {
    request.log.error(err);
    return reply.status(401).send({ error: 'Token expired or invalid' });
  }

  // Couriers: bind the access token to live server-side state (session +
  // membership), so revocation and removal take effect immediately rather than
  // waiting out the JWT's 24h–14d expiry.
  const user = request.user;
  if (user && user.role === 'courier') {
    if (!user.jti) {
      // Real courier logins always carry a session jti; a courier token without
      // one can only originate from the dev-login-gated mock endpoint (ADR-0003).
      if (!devLoginAllowed(env)) {
        return reply.status(401).send({ error: 'Token expired or invalid' });
      }
      return;
    }
    const pool = request.server.db;
    if (!pool) throw new Error('Database pool not attached to fastify');
    try {
      const res = await pool.query(
        `SELECT s.courier_id, s.revoked_at, s.expires_at,
                EXISTS(
                  SELECT 1 FROM courier_locations cl
                  WHERE cl.courier_id = s.courier_id AND cl.location_id = $2
                ) AS has_location
         FROM courier_sessions s
         WHERE s.id = $1 AND s.courier_id = $3`,
        [user.jti, user.activeLocationId, user.sub]
      );
      if (!courierSessionValid(res.rows[0], Date.now())) {
        return reply.status(401).send({ error: 'Session revoked or access removed' });
      }
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send({ error: 'Internal server error' });
    }
  }
};

/** Sets request.user if a valid Bearer token is present; silently skips if missing/invalid. */
export const softVerifyAuth = async (request: FastifyRequest, _reply: FastifyReply) => {
  const authHeader = request.headers.authorization;
  if (!authHeader?.startsWith('Bearer ')) return;
  try {
    request.user = await verifyAuthToken(authHeader.substring(7));
  } catch {
    // Invalid token — treat as anonymous
  }
};

export const requireRole = (roles: AuthToken['role'][]) => {
  return async (request: FastifyRequest, reply: FastifyReply) => {
    if (!request.user) {
      return reply.status(401).send({ error: 'Unauthorized' });
    }
    if (!roles.includes(request.user.role)) {
      return reply.status(403).send({ error: 'Forbidden role' });
    }
  };
};

// Check if user has active membership in the target location
export const requireLocationAccess = async (request: FastifyRequest, reply: FastifyReply) => {
  const { locationId } = request.params as { locationId?: string };
  if (!locationId) return reply.status(400).send({ error: 'Missing location_id parameter' });
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(locationId)) {
    return reply.status(400).send({ error: 'Invalid location_id format' });
  }

  const user = request.user;
  if (!user) return reply.status(401).send({ error: 'Unauthorized' });

  if (user.role === 'customer') {
    if (user.locationId !== locationId) {
      return reply.status(404).send({ error: 'Not found' }); // Don't leak existence
    }
    return;
  }

  // For courier, activeLocationId in JWT MUST match the requested location
  if (user.role === 'courier') {
    if (user.activeLocationId !== locationId) {
      return reply.status(404).send({ error: 'Not found' }); // Cross-tenant courier is 404
    }
    return;
  }

  const pool = request.server.db;
  if (!pool) throw new Error('Database pool not attached to fastify');

  try {
    const res = await pool.query(
      `SELECT 1 FROM memberships WHERE location_id = $1 AND user_id = $2 AND role = 'owner'`,
      [locationId, user.userId]
    );
    if (res.rowCount === 0) {
      return reply.status(404).send({ error: 'Not found' });
    }
  } catch (err) {
    request.log.error(err);
    return reply.status(500).send({ error: 'Internal server error' });
  }
};

export default fp(async (fastify) => {
  fastify.decorateRequest('user', null);
  fastify.decorate('verifyAuth', verifyAuth);
  fastify.decorate('softVerifyAuth', softVerifyAuth);
  fastify.decorate('requireRole', requireRole);
  fastify.decorate('requireLocationAccess', requireLocationAccess);
});
