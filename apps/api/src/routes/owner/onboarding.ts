import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'node:crypto';
import { seedMenuFromDraft } from '../../lib/menu-seed.js';

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

export default (async function onboardingRoutes(fastify: any, opts: any) {
  const { db, messageBus, queue } = opts as any;

  // ─── Auth hook for all routes ─────────────────────────────────────
  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  // Note: /onboarding/start has no locationId param (creates a new location), so
  // requireLocationAccess is added per-route on handlers that use :locationId.

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
        // Menu-first onboarding: claim an anonymous parse (POST /menu/import/anonymous)
        // and seed this new location's menu from its stashed draft.
        anonymous_import_id: z.string().uuid().optional(),
      }).strict(),
    },
    config: { rateLimit: { max: 3, timeWindow: '1 minute' } },
  }, async (request: any, reply: any) => {
    const user = request.user as any;
    const body = request.body as any;
    const userId = user.userId;

    const client = await db.connect();
    try {
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

      // 7. Menu-first claim: seed the new location from a stashed anonymous parse.
      // Best-effort and atomic — the account/location must succeed regardless; a
      // failed seed just lands the owner in activation with an empty menu (the
      // pre-menu-first behaviour). The location rows above are already committed,
      // so this runs in its own transaction.
      let seeded: { categories: number; products: number } | null = null;
      if (body.anonymous_import_id) {
        const redis = (fastify as any).redis;
        try {
          const raw = redis ? await redis.get(`import:anon:${body.anonymous_import_id}`) : null;
          if (raw) {
            const { draft } = JSON.parse(raw);
            await client.query('BEGIN');
            const counts = await seedMenuFromDraft(client, locId, draft);
            // Committing a reviewed menu satisfies the human-review publish gate (Z2).
            await client.query(
              `UPDATE locations SET menu_confirmed_at = COALESCE(menu_confirmed_at, now()) WHERE id = $1`,
              [locId],
            );
            // Pre-fill contact from the menu document (fill-if-empty — phone was
            // already set from the form above, so this mainly fills address).
            const rmeta = (draft as any)?._restaurant;
            if (rmeta && (rmeta.address || rmeta.phone)) {
              await client.query(
                `UPDATE locations
                    SET address = CASE WHEN COALESCE(NULLIF(btrim(address), ''), '') = '' THEN COALESCE($2, address) ELSE address END,
                        phone   = CASE WHEN COALESCE(NULLIF(btrim(phone), ''), '')   = '' THEN COALESCE($3, phone)   ELSE phone   END
                  WHERE id = $1`,
                [locId, rmeta.address || null, rmeta.phone || null],
              );
            }
            await client.query('COMMIT');
            await redis.del(`import:anon:${body.anonymous_import_id}`).catch(() => {});
            seeded = { categories: counts.categories, products: counts.products };
          }
        } catch (err: any) {
          await client.query('ROLLBACK').catch(() => {});
          request.log?.warn?.(`[onboarding] menu seed from anonymous import failed: ${err?.message}`);
        }
      }

      return reply.status(201).send({
        locationId: locId,
        slug: body.slug,
        onboardingState: initState,
        currentStep: 1,
        seeded,
      });
    } finally {
      client.release();
    }
  });

  // ─── Get onboarding state ─────────────────────────────────────────
  fastify.get('/onboarding/:locationId/state', {
    preHandler: [fastify.requireLocationAccess],
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
    },
  }, async (request: any, reply: any) => {
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
    preHandler: [fastify.requireLocationAccess],
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: z.object({
        step: z.number().int().min(1).max(STEP_COUNT),
      }).strict(),
    },
  }, async (request: any, reply: any) => {
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
      state.skippedSteps = state.skippedSteps.filter((s: number) => s !== step);

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
    preHandler: [fastify.requireLocationAccess],
    schema: {
      params: z.object({
        locationId: z.string().uuid(),
        stepNum: z.coerce.number().int().min(1).max(STEP_COUNT),
      }),
    },
  }, async (request: any, reply: any) => {
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
    preHandler: [fastify.requireLocationAccess],
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
    },
  }, async (request: any, reply: any) => {
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
  } catch (err: any) {
    console.warn('[onboarding] failed to parse onboarding state, using defaults:', err?.message);
  }
  // Default initial state
  return { v: 1, step: 1, completedSteps: [], skippedSteps: [], data: {} };
}
