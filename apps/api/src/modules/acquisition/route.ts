import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import { createSource, type Queryable } from './service.js';

// P6-1 — internal/ops acquisition entrypoint. Mounted at prefix `/api/dev` so it rides the
// global dev-guard (server.ts onRequest: isDevPath → ALLOW_DEV_LOGIN + x-dev-auth-secret,
// fail-closed on prod). On M0 the dev-auth secret IS the internal-auth; a dedicated ops-auth
// can replace it later. Never public. Rate-limited. Zod .strict() on input.

const bodySchema = z.object({ place_id: z.string().trim().min(1).max(512) }).strict();

interface Opts {
  db: Queryable;
}

export default async function acquisitionRoutes(fastify: FastifyInstance, opts: Opts) {
  fastify.post(
    '/acquisition',
    { config: { rateLimit: { max: 30, timeWindow: '1 minute' } } },
    async (request, reply) => {
      const parsed = bodySchema.safeParse(request.body);
      if (!parsed.success) {
        return reply.code(400).send({ error: 'VALIDATION_FAILED', message: 'place_id required' });
      }
      // Idempotent: a repeat place_id returns the existing lifecycle row (never a 2nd).
      const source = await createSource(opts.db, parsed.data.place_id);
      return reply.code(201).send(source);
    },
  );
}
