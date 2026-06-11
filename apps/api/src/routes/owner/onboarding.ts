// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';

const STEP_COUNT = 8;
const STEP_LABELS: Record<number, string> = {
  1: 'Location Basics',
  2: 'Import Menu',
  3: 'Review & Fix Menu',
  4: 'Branding',
  5: 'Delivery Settings',
  6: 'Publish & Share',
  7: 'Telegram Alerts',
  8: 'Test Order & Go Live',
};
const SKIPPABLE = new Set([4, 5, 7]);
const REQUIRED_WITH_DEFAULTS: Record<number, string> = {
  4: 'Branding skipped — default theme applied',
  5: 'Delivery skipped — pickup-only mode, no delivery radius',
  7: 'Telegram skipped — you\'ll still receive alerts on the dashboard and via push',
};

export default (async function onboardingRoutes(fastify, opts) {
  const { db, messageBus, queue } = opts as any;

  // ─── Auth hook for all routes ─────────────────────────────────────
  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));

  // ─── Start / create location ──────────────────────────────────────
  fastify.post('/onboarding/start', {
    schema: {
      body: z.object({
        name: z.string().min(1).max(200),
        phone: z.string().min(3).max(30),
        slug: z.string().min(2).max(100).regex(/^[a-z0-9-]+$/, 'Slug must be lowercase alphanumeric with hyphens'),
        currency_code: z.string().length(3).default('ALL'),
        default_locale: z.string().min(2).default('sq'),
        supported_locales: z.array(z.string().min(2)).default(['sq', 'en']),
      }).strict(),
    },
    config: { rateLimit: { max: 3, timeWindow: '1 minute' } },
  }, async (request, reply) => {
    const user = request.user as any;
    const body = request.body as any;
    const userId = user.userId;

    const client = await db.connect();
    try {
      // 1. Check slug uniqueness
      const slugCheck = await client.query(`SELECT id FROM locations WHERE slug = $1`, [body.slug]);
      if (slugCheck.rowCount > 0) {
        return reply.status(409).send({ error: 'Slug already taken', code: 'SLUG_TAKEN' });
      }

      // 2. Find or create org for this owner
      const orgRes = await client.query(
        `SELECT id FROM organizations WHERE owner_id = $1 LIMIT 1`,
        [userId],
      );
      let orgId: string;
      if (orgRes.rowCount > 0) {
        orgId = orgRes.rows[0].id;
      } else {
        // Create a personal org for this owner
        const newOrg = await client.query(
          `INSERT INTO organizations (id, name, owner_id) VALUES ($1, $2, $3) RETURNING id`,
          [crypto.randomUUID(), `${body.name} Org`, userId],
        );
        orgId = newOrg.rows[0].id;
      }

      // 3. Create location
      const locId = crypto.randomUUID();
      await client.query(
        `INSERT INTO locations (id, org_id, slug, name, phone, currency_code, default_locale, supported_locales, status, widget_enabled, delivery_fee_flat)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'open', true, 0)`,
        [locId, orgId, body.slug, body.name, body.phone, body.currency_code, body.default_locale, body.supported_locales],
      );

      // 4. Create membership
      await client.query(
        `INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING`,
        [userId, locId],
      );

      // 5. Create menu_versions row with v1
      await client.query(
        `INSERT INTO menu_versions (location_id, version) VALUES ($1, 1) ON CONFLICT DO NOTHING`,
        [locId],
      );

      // 6. Initialize onboarding_state
      const initState = {
        v: 1,
        step: 1,
        completedSteps: [] as number[],
        skippedSteps: [] as number[],
        data: {} as Record<string, unknown>,
      };
      await client.query(
        `UPDATE locations SET onboarding_state = $1::jsonb WHERE id = $2`,
        [JSON.stringify(initState), locId],
      );

      return reply.status(201).send({
        locationId: locId,
        slug: body.slug,
        onboardingState: initState,
        currentStep: 1,
      });
    } finally {
      client.release();
    }
  });

  // ─── Get onboarding state ─────────────────────────────────────────
  fastify.get('/onboarding/:locationId/state', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
    },
  }, async (request, reply) => {
    const { locationId } = request.params as any;
    const user = request.user as any;

    const res = await db.query(
      `SELECT id, slug, name, onboarding_state, onboarding_completed_at
       FROM locations WHERE id = $1`,
      [locationId],
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

    const loc = res.rows[0];
    const state = parseState(loc.onboarding_state);

    return reply.send({
      locationId: loc.id,
      slug: loc.slug,
      name: loc.name,
      onboardingState: state,
      currentStep: state.step,
      completed: !!loc.onboarding_completed_at,
    });
  });

  // ─── Complete a step ──────────────────────────────────────────────
  fastify.post('/onboarding/:locationId/step/complete', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: z.object({
        step: z.number().int().min(1).max(STEP_COUNT),
      }).strict(),
    },
  }, async (request, reply) => {
    const { locationId } = request.params as any;
    const { step } = request.body as any;
    const user = request.user as any;

    const client = await db.connect();
    try {
      const locRes = await client.query(
        `SELECT id, onboarding_state FROM locations WHERE id = $1`,
        [locationId],
      );
      if (locRes.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

      const state = parseState(locRes.rows[0].onboarding_state);

      // Validate step
      if (state.completedSteps.includes(step)) {
        // Idempotent — already completed
      } else if (state.step !== step) {
        return reply.status(400).send({
          error: `Step ${step} is not current. Current step is ${state.step}`,
          currentStep: state.step,
        });
      } else {
        state.completedSteps.push(step);
      }

      // Remove from skipped if was skipped
      state.skippedSteps = state.skippedSteps.filter(s => s !== step);

      // Advance to next incomplete step
      let nextStep = step + 1;
      while (nextStep <= STEP_COUNT && state.completedSteps.includes(nextStep)) {
        nextStep++;
      }

      let completed = false;
      if (nextStep > STEP_COUNT) {
        // All steps done — complete onboarding
        completed = true;
        await client.query(
          `UPDATE locations
           SET onboarding_state = $1::jsonb, onboarding_completed_at = now()
           WHERE id = $2`,
          [JSON.stringify(state), locationId],
        );
      } else {
        state.step = nextStep;
        await client.query(
          `UPDATE locations SET onboarding_state = $1::jsonb WHERE id = $2`,
          [JSON.stringify(state), locationId],
        );
      }

      return reply.send({
        completed,
        currentStep: completed ? null : nextStep,
        onboardingState: state,
      });
    } finally {
      client.release();
    }
  });

  // ─── Skip a step ──────────────────────────────────────────────────
  fastify.post('/onboarding/:locationId/step/:stepNum/skip', {
    schema: {
      params: z.object({
        locationId: z.string().uuid(),
        stepNum: z.coerce.number().int().min(1).max(STEP_COUNT),
      }),
    },
  }, async (request, reply) => {
    const { locationId, stepNum } = request.params as any;

    if (!SKIPPABLE.has(stepNum)) {
      return reply.status(400).send({
        error: `Step ${stepNum} cannot be skipped`,
        label: STEP_LABELS[stepNum],
      });
    }

    const client = await db.connect();
    try {
      const locRes = await client.query(
        `SELECT id, onboarding_state FROM locations WHERE id = $1`,
        [locationId],
      );
      if (locRes.rowCount === 0) return reply.status(404).send({ error: 'Not found' });

      const state = parseState(locRes.rows[0].onboarding_state);

      if (state.completedSteps.includes(stepNum)) {
        return reply.status(400).send({ error: `Step ${stepNum} already completed` });
      }

      state.skippedSteps.push(stepNum);
      state.completedSteps.push(stepNum);

      // Advance to next
      let nextStep = stepNum + 1;
      while (nextStep <= STEP_COUNT && state.completedSteps.includes(nextStep)) {
        nextStep++;
      }

      let completed = false;
      if (nextStep > STEP_COUNT) {
        completed = true;
        await client.query(
          `UPDATE locations SET onboarding_state = $1::jsonb, onboarding_completed_at = now() WHERE id = $2`,
          [JSON.stringify(state), locationId],
        );
      } else {
        state.step = nextStep;
        await client.query(
          `UPDATE locations SET onboarding_state = $1::jsonb WHERE id = $2`,
          [JSON.stringify(state), locationId],
        );
      }

      return reply.send({
        completed,
        currentStep: completed ? null : nextStep,
        onboardingState: state,
        skipNote: REQUIRED_WITH_DEFAULTS[stepNum],
      });
    } finally {
      client.release();
    }
  });

  // ─── Dashboard redirect after onboarding complete ─────────────────
  fastify.get('/onboarding/:locationId/complete', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
    },
  }, async (request, reply) => {
    const { locationId } = request.params as any;
    const res = await db.query(
      `SELECT id, slug, onboarding_completed_at FROM locations WHERE id = $1`,
      [locationId],
    );
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
    if (!res.rows[0].onboarding_completed_at) {
      return reply.status(400).send({ error: 'Onboarding not yet completed' });
    }
    return reply.send({ slug: res.rows[0].slug, dashboardUrl: `/admin/dashboard.html` });
  });
});

function parseState(raw: any): any {
  try {
    const state = typeof raw === 'string' ? JSON.parse(raw) : raw;
    if (state && typeof state === 'object' && state.v) {
      return {
        v: state.v,
        step: state.step || 1,
        completedSteps: Array.isArray(state.completedSteps) ? state.completedSteps : [],
        skippedSteps: Array.isArray(state.skippedSteps) ? state.skippedSteps : [],
        data: state.data || {},
      };
    }
  } catch {
    // fall through — return default initial state
    console.debug('[onboarding] failed to parse onboarding state, using defaults');
  }
  // Default initial state
  return { v: 1, step: 1, completedSteps: [], skippedSteps: [], data: {} };
}
