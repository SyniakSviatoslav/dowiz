import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { isVoiceEnabled } from '../../lib/voice-flag.js';

// GET /api/public/voice-config — the runtime kill-switch the storefront polls before activating voice.
// It lives under /api/ ON PURPOSE: the storefront service worker (apps/api/public/sw.js) serves every
// non-/api/ GET cache-first, so a /api/ path is the ONE place a kill signal is never pinned in the SW
// cache (breaker finding R2-A). The client fetches with cache:'no-store' and treats reject / !ok /
// enabled!==true / absent all as OFF (fail-closed). No DB, no auth, no PII — a single global boolean.
export default (async function publicVoiceConfigRoutes(fastify: any, _opts: any) {
  fastify.get('/api/public/voice-config', async (_request: any, reply: any) => {
    // no-store on the response too, so no shared/edge cache can delay an emergency VOICE_KILL.
    reply.header('Cache-Control', 'no-store');
    return reply.send({ enabled: isVoiceEnabled() });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
