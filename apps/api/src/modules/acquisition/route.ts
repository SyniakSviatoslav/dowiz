import type { FastifyInstance } from 'fastify';
import type { Pool } from 'pg';
import { z } from 'zod';
import { createSource } from './service.js';
import { mintProvisionToken, provisionShadowSpine, hardDeleteShadow, ProvisionError } from './provisioning.js';
import { provisionOpsAuthorized, PROVISION_OPS_HEADER } from './ops-auth.js';

// P6-1/P6-2 — internal/ops acquisition + provisioning entrypoint. Mounted OUTSIDE /api/dev so the
// global dev-guard does NOT apply (breaker B4); gated solely by its OWN ops-auth secret (decoupled
// from the dev-login owner-JWT minter family). Fail-closed 404 when PROVISION_OPS_SECRET is unset.
// Never public. Rate-limited. Zod .strict() on input. See p6-2-provisioning-council-verdict.md.

interface Opts {
  pool: Pool;
  opsSecret?: string;
}

const placeIdSchema = z.object({ place_id: z.string().trim().min(1).max(512) }).strict();
const mintSchema = z.object({ acquisition_source_id: z.string().uuid() }).strict();
const spineSchema = z
  .object({
    acquisition_source_id: z.string().uuid(),
    token: z.string().min(1).max(256),
    name: z.string().trim().min(1).max(256),
    slug: z
      .string()
      .trim()
      .min(2)
      .max(100)
      .regex(/^[a-z0-9-]+$/, 'slug must be lowercase alphanumeric with hyphens'),
    phone: z.string().trim().max(64).optional(),
  })
  .strict();
const deleteSchema = z.object({ acquisition_source_id: z.string().uuid() }).strict();

export default async function acquisitionRoutes(fastify: FastifyInstance, opts: Opts) {
  const { pool, opsSecret } = opts;

  // Sole gate for this surface: fail-closed 404 (hide existence) unless the ops secret matches.
  fastify.addHook('onRequest', async (request, reply) => {
    if (!provisionOpsAuthorized(request.headers[PROVISION_OPS_HEADER], opsSecret)) {
      return reply.code(404).send({ error: 'NOT_FOUND' });
    }
  });

  fastify.post(
    '/acquisition',
    { config: { rateLimit: { max: 30, timeWindow: '1 minute' } } },
    async (request, reply) => {
      const parsed = placeIdSchema.safeParse(request.body);
      if (!parsed.success) {
        return reply.code(400).send({ error: 'VALIDATION_FAILED', message: 'place_id required' });
      }
      // Idempotent: a repeat place_id returns the existing lifecycle row (never a 2nd).
      const source = await createSource(pool, parsed.data.place_id);
      return reply.code(201).send(source);
    },
  );

  // Mint a single-use provisioning token (plaintext returned ONCE; only the hash is stored).
  fastify.post(
    '/acquisition/provision/mint',
    { config: { rateLimit: { max: 30, timeWindow: '1 minute' } } },
    async (request, reply) => {
      const parsed = mintSchema.safeParse(request.body);
      if (!parsed.success) return reply.code(400).send({ error: 'VALIDATION_FAILED' });
      try {
        const { token, expiresAt } = await mintProvisionToken(pool, parsed.data.acquisition_source_id);
        return reply.code(201).send({ token, expires_at: expiresAt.toISOString() });
      } catch (e) {
        if (e instanceof ProvisionError) return reply.code(409).send({ error: e.code });
        throw e;
      }
    },
  );

  // Write the shadow spine through the provision_shadow RLS policy (one tx, consume-LAST).
  fastify.post(
    '/acquisition/provision/spine',
    { config: { rateLimit: { max: 30, timeWindow: '1 minute' } } },
    async (request, reply) => {
      const parsed = spineSchema.safeParse(request.body);
      if (!parsed.success) return reply.code(400).send({ error: 'VALIDATION_FAILED' });
      try {
        const out = await provisionShadowSpine(pool, {
          acquisitionSourceId: parsed.data.acquisition_source_id,
          token: parsed.data.token,
          name: parsed.data.name,
          slug: parsed.data.slug,
          phone: parsed.data.phone,
        });
        return reply.code(201).send({ org_id: out.orgId, location_id: out.locationId });
      } catch (e) {
        if (e instanceof ProvisionError) return reply.code(409).send({ error: e.code });
        throw e;
      }
    },
  );

  // Day-one hard-delete of a shadow tenant (counsel C2 — born with its erasure path).
  fastify.post(
    '/acquisition/provision/hard-delete',
    { config: { rateLimit: { max: 30, timeWindow: '1 minute' } } },
    async (request, reply) => {
      const parsed = deleteSchema.safeParse(request.body);
      if (!parsed.success) return reply.code(400).send({ error: 'VALIDATION_FAILED' });
      await hardDeleteShadow(pool, parsed.data.acquisition_source_id);
      return reply.code(200).send({ deleted: true });
    },
  );
}
