import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import { acceptClaim, declineAndErase, ClaimError } from '../../modules/acquisition/claim.js';
import type { Pool } from 'pg';

// P6 CLAIM PHASE — the public claim surface (council K3 / H-decline).
//  • POST /api/claim/accept  — verifyAuth ONLY (the claimer has no membership/owner role yet; role
//    re-derives from membership after claim, ADR-0004). The TOKEN is the sole transfer authority.
//  • POST /api/claim/decline — NO auth, token-only: the restaurant can erase the unconsented preview
//    in one action without creating an account (mandatory, equally-prominent to claim — counsel CC2).
const tokenSchema = z.object({ token: z.string().min(16).max(256) });

export default async function claimRoutes(fastify: FastifyInstance, opts: { pool: Pool }) {
  const { pool } = opts;

  fastify.post(
    '/claim/accept',
    { preValidation: [(fastify as any).verifyAuth], config: { rateLimit: { max: 10, timeWindow: '1 minute' } } },
    async (request: any, reply: any) => {
      const parsed = tokenSchema.safeParse(request.body);
      if (!parsed.success) return reply.code(400).send({ error: 'VALIDATION_FAILED' });
      const userId = request.user?.sub;
      if (!userId) return reply.code(401).send({ error: 'UNAUTHENTICATED' });
      try {
        const out = await acceptClaim(pool, parsed.data.token, userId);
        // role re-derives from the new membership on the next request (ADR-0004); hint a re-auth.
        return reply.code(200).send({ org_id: out.orgId, location_id: out.locationId, reauth: true });
      } catch (e) {
        if (e instanceof ClaimError) {
          const code = (e as ClaimError).code;
          const status = code === 'ALREADY_CLAIMED' ? 409 : code === 'INVALID_OR_EXPIRED_TOKEN' ? 401 : 422;
          return reply.code(status).send({ error: code });
        }
        throw e;
      }
    },
  );

  fastify.post(
    '/claim/decline',
    { config: { rateLimit: { max: 10, timeWindow: '1 minute' } } },
    async (request: any, reply: any) => {
      const parsed = tokenSchema.safeParse(request.body);
      if (!parsed.success) return reply.code(400).send({ error: 'VALIDATION_FAILED' });
      try {
        await declineAndErase(pool, parsed.data.token);
        return reply.code(200).send({ erased: true });
      } catch (e) {
        if (e instanceof ClaimError) return reply.code(401).send({ error: (e as ClaimError).code });
        throw e;
      }
    },
  );
}
