// @ts-nocheck
import type { FastifyInstance, FastifyRequest, FastifyReply } from 'fastify';
import fp from 'fastify-plugin';

interface TurnstilePluginOpts {
  secret: string;
  /** Routes where Turnstile is enforced unconditionally */
  enforcedRoutes?: string[];
}

async function turnstilePlugin(fastify: FastifyInstance, opts: TurnstilePluginOpts) {
  const { secret, enforcedRoutes = [] } = opts;

  fastify.decorate('verifyTurnstile', async (request: FastifyRequest, reply: FastifyReply) => {
    const token = (request.body as any)?.turnstile_token;
    if (!token) {
      return reply.status(403).send({ error: 'challenge_required', message: 'CAPTCHA verification required' });
    }

    try {
      const res = await fetch('https://challenges.cloudflare.com/turnstile/v0/siteverify', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          secret,
          response: token,
          remoteip: request.ip,
        }),
      });
      const data = await res.json();

      if (!data.success) {
        request.log.warn({ turnstileError: data['error-codes'] }, 'Turnstile verification failed');
        return reply.status(403).send({ error: 'challenge_failed', message: 'Verification failed. Please try again.' });
      }

      (request as any).turnstilePassed = true;
    } catch (err) {
      request.log.error(err, 'Turnstile siteverify request failed');
      return reply.status(503).send({ error: 'challenge_unavailable', message: 'Verification service unavailable. Please try again.' });
    }
  });
}

declare module 'fastify' {
  interface FastifyInstance {
    verifyTurnstile: (request: FastifyRequest, reply: FastifyReply) => Promise<void>;
  }
}

export default fp(turnstilePlugin, { name: 'turnstile-plugin' });
