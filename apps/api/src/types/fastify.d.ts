// Fastify declaration merging for the composition root's instance decorations
// (server.ts: decorate('db'|'redis'|'wss'|'memory') + registerReplySendError).
// One shared types file so every route/worker sees the SAME precise types —
// previously these lived in server.ts as `any`, which (with @ts-nocheck) made
// the whole boot path type-blind. Per-plugin decorations (verifyAuth, …) stay
// merged next to their plugin (plugins/auth.ts, plugins/turnstile.ts).
import type { Pool } from 'pg';
import type { Redis } from 'ioredis';
import type { WebSocketServer } from 'ws';
import type { MemoryService } from '../lib/memory.js';
import type { ErrorEnvelopeOpts } from '../lib/api-error.js';

declare module 'fastify' {
  interface FastifyInstance {
    /** Operational pg pool (RLS role) — decorated in server.ts main(). */
    db: Pool;
    /** Shared ioredis connection — decorated in server.ts main(). */
    redis: Redis;
    /** Live WS server — null until setupWebSocket() runs in fastify.ready(). */
    wss: WebSocketServer | null;
    /** mem0 persistent agent memory — decorated in server.ts main(). */
    memory: MemoryService;
  }
  interface FastifyReply {
    /**
     * A2 (ADR-0010): emit the structured error envelope for a RETURN-based ad-hoc site (the
     * drop-in for `reply.status(n).send({ error })`). Same envelope as setErrorHandler (shared
     * builder), incl. server correlationId + x-correlation-id echo. `code` must be SCREAMING_SNAKE.
     * Use this for sites that return mid-handler; THROW `new ApiError(...)` where a throw is cleaner.
     */
    sendError(status: number, code: string, message: string, opts?: ErrorEnvelopeOpts): FastifyReply;
  }
}
