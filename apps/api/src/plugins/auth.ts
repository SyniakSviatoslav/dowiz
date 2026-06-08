// @ts-nocheck
import fp from 'fastify-plugin';
import type { FastifyRequest, FastifyReply } from 'fastify';
import { verifyAuthToken } from '@deliveryos/platform';
import { AuthToken } from '@deliveryos/shared-types';

declare module 'fastify' {
  interface FastifyRequest {
    user: AuthToken | null;
  }
  interface FastifyInstance {
    verifyAuth: any;
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
  fastify.decorate('requireRole', requireRole);
  fastify.decorate('requireLocationAccess', requireLocationAccess);
});
