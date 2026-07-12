import type { FastifyInstance } from 'fastify';
import { signDevToken } from '@deliveryos/platform';
import crypto from 'node:crypto';
import { dashboardChannel } from '../../lib/registry.js';
import { encryptPII } from '../../lib/pii-cipher.js';
import argon2 from 'argon2';
import {
  SYNTHETIC_COURIER_EMAIL_HASH,
  SYNTHETIC_COURIER_DISPLAY_NAME,
} from '../../lib/synthetic-courier.js';

export default async function mockAuthRoutes(fastify: FastifyInstance) {
  console.log('[API] Registering mockAuthRoutes: /dev/mock-auth');
  fastify.post('/dev/mock-auth', async (request: any, reply: any) => {
    const body = request.body as Record<string, unknown> || {};
    const role = body.role === 'courier' ? 'courier' : 'owner';

    if (role === 'courier') {
      // SYNTHETIC-ONLY RE-DERIVED MINT (constraint #1 / resolution NEW-M1, L1):
      // when — and only when — the caller explicitly asks for synthetic:true, mint a token for
      // the ONE seeded synthetic courier. The id is RE-DERIVED server-side by SELECTing on the
      // sentinel email_hash — it reads NO caller-supplied courierId, so the "impersonate any
      // courier" capability is reduced to "impersonate the one synthetic fixture" by
      // construction (not by a guard — the echo-back/equality-check variant is deliberately
      // NOT used; it would re-create the dev-login-backdoor shape). Any other input → the
      // existing random-uuid behaviour below, unchanged.
      if (body.synthetic === true) {
        const cRes = await (fastify as any).db.query(
          `SELECT c.id, cl.location_id
             FROM couriers c
             JOIN courier_locations cl ON cl.courier_id = c.id
            WHERE c.email_hash = $1
            ORDER BY cl.added_at ASC
            LIMIT 1`,
          [SYNTHETIC_COURIER_EMAIL_HASH],
        );
        if (cRes.rowCount === 0) {
          return reply.status(409).send({
            error: 'synthetic courier not seeded — run /dev/seed-visual-state first',
            code: 'SYNTHETIC_COURIER_MISSING',
          });
        }
        const syntheticId = cRes.rows[0].id as string;
        const syntheticLocationId = cRes.rows[0].location_id as string;
        const accessToken = await signDevToken({
          role: 'courier',
          sub: syntheticId,
          activeLocationId: syntheticLocationId,
        } as any, '1d');
        return reply.send({
          access_token: accessToken,
          userId: syntheticId,
          activeLocationId: syntheticLocationId,
          synthetic: true,
        });
      }

      const locationId = (body.locationId as string) || '1f609add-062a-4bb5-89bf-d695f963ede6';
      const courierId = crypto.randomUUID();

      const accessToken = await signDevToken({
        role: 'courier',
        sub: courierId,
        activeLocationId: locationId,
      } as any, '1d');

      return reply.send({ access_token: accessToken, userId: courierId, activeLocationId: locationId });
    }

    // Owner role (default)
    const email = 'dev@deliveryos.com';
    const googleSub = 'mock-google-12345';
    const ownerName = 'Dev Owner';

    let userId: string;
    try {
      const res = await (fastify as any).db.query(
        `INSERT INTO users (email, google_sub, display_name) 
         VALUES ($1, $2, $3)
         ON CONFLICT (google_sub) DO UPDATE SET email = EXCLUDED.email, display_name = EXCLUDED.display_name
         RETURNING id`,
        [email, googleSub, ownerName]
      );
      userId = res.rows[0].id;
    } catch (e) {
      const updateRes = await (fastify as any).db.query(
        `UPDATE users SET google_sub = $2, display_name = COALESCE(users.display_name, $3) WHERE email = $1 RETURNING id`,
        [email, googleSub, ownerName]
      );
      if (updateRes.rowCount === 0) {
        throw new Error('Failed to upsert dev user');
      }
      userId = updateRes.rows[0].id;
    }

    let activeLocationId: string | undefined;

    if (body.locationSlug) {
      const locRes = await (fastify as any).db.query(
        `SELECT id FROM locations WHERE slug = $1 AND status = 'active' LIMIT 1`,
        [body.locationSlug]
      );
      if (locRes.rowCount > 0) activeLocationId = locRes.rows[0].id;
    }

    if (!activeLocationId) {
      const memberRes = await (fastify as any).db.query(
        `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' AND status = 'active' LIMIT 1`, // P-d (ADR-0004)
        [userId]
      );
      if (memberRes.rowCount > 0) activeLocationId = memberRes.rows[0].location_id;
    }

    const tokenPayload: Record<string, unknown> = { role: 'owner', userId };
    if (activeLocationId) tokenPayload.activeLocationId = activeLocationId;

    const accessToken = await signDevToken(tokenPayload as any, '1d');
    return reply.send({ access_token: accessToken, userId, activeLocationId });
  });

  // Test helper: create courier assignment for an order
  fastify.post('/dev/create-assignment', async (request: any, reply: any) => {
    const { orderId, courierId, locationId } = request.body as Record<string, string>;
    if (!orderId || !courierId || !locationId) {
      return reply.status(400).send({ error: 'orderId, courierId, locationId required' });
    }

    const ownerRes = await (fastify as any).db.query(
      `SELECT id FROM users WHERE email = 'dev@deliveryos.com' LIMIT 1`
    );
    const ownerId = ownerRes.rowCount > 0 ? ownerRes.rows[0].id : '00000000-0000-0000-0000-000000000000';

    const client = await (fastify as any).db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);
      await client.query(`SELECT set_config('app.user_id', $1, true)`, [ownerId]);

      const shiftRes = await client.query(
        `INSERT INTO courier_shifts (courier_id, location_id, status, started_at)
         VALUES ($1, $2, 'available', now())
         ON CONFLICT DO NOTHING
         RETURNING id`,
        [courierId, locationId]
      );
      const shiftId = shiftRes.rows[0]?.id || null;

      const asgnRes = await client.query(
        `INSERT INTO courier_assignments (order_id, courier_id, location_id, shift_id, status)
         VALUES ($1, $2, $3, $4, 'assigned')
         ON CONFLICT (order_id) DO UPDATE SET courier_id = EXCLUDED.courier_id, status = 'assigned'
         RETURNING id`,
        [orderId, courierId, locationId, shiftId]
      );

      await client.query('COMMIT');

      // Publish WS events for test verification
      const messageBus = (fastify as any).messageBus;
      if (messageBus) {
        await messageBus.publish(dashboardChannel(locationId), {
          type: 'assignment.created',
          orderId,
          courierId,
        });
        await messageBus.publish(`courier:${courierId}`, {
          type: 'task_assigned',
          payload: { id: orderId, orderId, status: 'assigned', courierId },
        });
      }

      return reply.send({ assignmentId: asgnRes.rows[0].id, shiftId });
    } catch (err) {
      await client.query('ROLLBACK');
      throw err;
    } finally {
      client.release();
    }
  });

  // Test helper: seed an active telegram notification target for a location so the
  // category preference-centre (and /settings) has something to render against.
  fastify.post('/dev/seed-telegram-target', async (request: any, reply: any) => {
    const body = (request.body as Record<string, unknown>) || {};
    const locationId = body.locationId as string;
    const userId = (body.userId as string) || null;
    const address = (body.address as string) || `test-chat-${crypto.randomUUID().slice(0, 8)}`;
    if (!locationId) return reply.status(400).send({ error: 'locationId required' });

    const res = await (fastify as any).db.query(
      `INSERT INTO owner_notification_targets (location_id, channel, address, status, user_id)
       VALUES ($1, 'telegram', $2, 'active', $3)
       ON CONFLICT (location_id, channel, address) DO UPDATE SET status = 'active'
       RETURNING id`,
      [locationId, address, userId]
    );
    return reply.send({ targetId: res.rows[0].id, address });
  });

  // Test helper: report + repair the local-login fixture so test@dowiz.com owns the demo
  // location (the seed's membership link can be missing on a DB that was provisioned before
  // the demo location existed). Idempotent.
  fastify.post('/dev/repair-test-owner', async (request: any, reply: any) => {
    const body = (request.body as Record<string, unknown>) || {};
    const email = (body.email as string) || 'test@dowiz.com';
    const slug = (body.slug as string) || 'demo';
    const db = (fastify as any).db;

    const u = await db.query(`SELECT id FROM users WHERE email = $1`, [email]);
    if (u.rowCount === 0) return reply.status(404).send({ error: `user ${email} not found` });
    const userId = u.rows[0].id;
    // Resolve by explicit locationId if given, else by slug (NO status filter — the demo
    // location's status is not 'active' yet it is the live storefront).
    let locationId = (body.locationId as string) || null;
    let locName: string | null = null;
    if (locationId) {
      const l = await db.query(`SELECT name FROM locations WHERE id = $1`, [locationId]);
      locName = l.rowCount > 0 ? l.rows[0].name : null;
    } else {
      const loc = await db.query(`SELECT id, name FROM locations WHERE slug = $1 LIMIT 1`, [slug]);
      if (loc.rowCount === 0) return reply.status(404).send({ error: `location '${slug}' not found` });
      locationId = loc.rows[0].id; locName = loc.rows[0].name;
    }

    const before = await db.query(
      `SELECT location_id, role, status FROM memberships WHERE user_id = $1`, [userId]);
    const ownedOrgs = await db.query(`SELECT id FROM organizations WHERE owner_id = $1`, [userId]);

    await db.query(
      `INSERT INTO memberships (user_id, location_id, role, status)
       VALUES ($1, $2, 'owner', 'active')
       ON CONFLICT (user_id, location_id, role) DO UPDATE SET status = 'active'`,
      [userId, locationId]);

    const after = await db.query(
      `SELECT location_id, role, status FROM memberships WHERE user_id = $1`, [userId]);
    return reply.send({
      email, userId, slug, locationId, locationName: locName,
      ownedOrgs: ownedOrgs.rowCount, membershipsBefore: before.rows, membershipsAfter: after.rows,
    });
  });

  // ── Visual-regression fixtures ──────────────────────────────────────────────
  // Seeds the DETERMINISTIC state the visual net's harness contract expects
  // (e2e/visual/harness.ts → VisualFixtures): one OPEN venue (categories +
  // products, one product carrying a modifier_group + modifiers, one product
  // 86'd via is_available=false), one CLOSED venue, one BUSY venue, plus a
  // seeded order on the open venue and a (stable) courier id.
  //
  // GATING — none added here. The /dev + /api/dev family is gated globally by the
  // server.ts onRequest hook (isDevRequestAuthorized): every dev path 404s unless
  // BOTH ALLOW_DEV_LOGIN='true' AND a matching x-dev-auth-secret header are present
  // (ADR-0003, fails closed on prod). Registering under /dev (+ the /api/dev alias
  // the harness calls) automatically inherits that gate — no new prod surface.
  //
  // RLS — uses the same plain (fastify as any).db.query path as /dev/repair-test-owner
  // above: the operational pool role is the established write path for these dev
  // seeders, so we follow that proven convention rather than re-deriving tenant ctx.
  //
  // OPEN vs CLOSED vs BUSY is driven through the REAL read paths so the storefront
  // actually renders each state (see apps/api/src/routes/public/menu.ts):
  //   • read_public_menu() only returns a venue when status IN ('active','open')
  //     OR published_at IS NOT NULL — so ALL three set published_at (else 404, not
  //     "closed"). Product visibility = is_available=true (the 86'd product vanishes).
  //   • GET /:slug/info computes status: 'closed' when hours_json reads closed now /
  //     delivery_paused; 'busy' when (open AND) kitchen_busy_until is a future ts;
  //     else 'open'. MenuPage.tsx derives venueStatus from that field.
  // Determinism: CLOSED uses an always-closed hours_json (every day isOpen:false) so
  // the verdict is time-of-run independent; BUSY pins kitchen_busy_until to a fixed
  // far-future ts. busy_mode=true is also set on BUSY to satisfy the contract literally
  // (it's the column read by orders.ts for confirm-timeout doubling).
  const VIS_COURIER_ID = '00000000-0000-4000-8000-0000000000c1'; // stable, returned to the harness
  const seedVisualHandler = async (_request: any, reply: any) => {
    const db = (fastify as any).db;

    // hours_json shapes consumed by /info's open/closed computation.
    const DAYS = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'];
    const OPEN_ALL_DAY = JSON.stringify(
      Object.fromEntries(DAYS.map((d) => [d, { isOpen: true, open: '00:00', close: '23:59' }])),
    );
    const CLOSED_ALL_DAY = JSON.stringify(
      Object.fromEntries(DAYS.map((d) => [d, { isOpen: false }])),
    );

    // 1. Owner user + organization (org_id is NOT NULL on locations). Both upsert by a
    //    stable natural key so their ids never churn across runs.
    const ownerRes = await db.query(
      `INSERT INTO users (email, google_sub, display_name)
         VALUES ('vis-owner@dowiz.com', 'vis-owner-sub', 'Visual Net Owner')
       ON CONFLICT (google_sub) DO UPDATE SET email = EXCLUDED.email
       RETURNING id`,
    );
    const ownerId = ownerRes.rows[0].id;

    // organizations has no unique business key → look up by owner+name, insert once.
    let orgId: string;
    const orgSel = await db.query(
      `SELECT id FROM organizations WHERE owner_id = $1 AND name = 'Visual Net Org' LIMIT 1`,
      [ownerId],
    );
    if (orgSel.rowCount > 0) {
      orgId = orgSel.rows[0].id;
    } else {
      const orgIns = await db.query(
        `INSERT INTO organizations (name, owner_id) VALUES ('Visual Net Org', $1) RETURNING id`,
        [ownerId],
      );
      orgId = orgIns.rows[0].id;
    }

    // 2. The three venues — UPSERT on slug (UNIQUE) so re-running never duplicates.
    //    phone is NOT NULL; currency/locales default sensibly but we pin them for snapshot
    //    stability. timezone='UTC' makes the schedule engine deterministic.
    async function upsertLocation(
      slug: string,
      name: string,
      opts: { status: string; busyMode: boolean; hours: string; kitchenBusyUntil: string | null },
    ): Promise<string> {
      const res = await db.query(
        `INSERT INTO locations
           (org_id, slug, name, phone, status, busy_mode, published_at,
            delivery_paused, hours_json, kitchen_busy_until, timezone,
            default_locale, supported_locales, currency_code, currency_minor_unit,
            lat, lng)
         VALUES
           ($1, $2, $3, '+355690000000', $4, $5, now(),
            false, $6::jsonb, $7::timestamptz, 'UTC',
            'sq', ARRAY['sq','en'], 'ALL', 0,
            41.3275, 19.8187)
         ON CONFLICT (slug) DO UPDATE SET
           org_id = EXCLUDED.org_id,
           name = EXCLUDED.name,
           status = EXCLUDED.status,
           busy_mode = EXCLUDED.busy_mode,
           published_at = COALESCE(locations.published_at, EXCLUDED.published_at),
           delivery_paused = EXCLUDED.delivery_paused,
           hours_json = EXCLUDED.hours_json,
           kitchen_busy_until = EXCLUDED.kitchen_busy_until,
           timezone = EXCLUDED.timezone
         RETURNING id`,
        [orgId, slug, name, opts.status, opts.busyMode, opts.hours, opts.kitchenBusyUntil],
      );
      return res.rows[0].id;
    }

    const openId = await upsertLocation('vis-open', 'Visual Open Venue', {
      status: 'active', busyMode: false, hours: OPEN_ALL_DAY, kitchenBusyUntil: null,
    });
    const closedId = await upsertLocation('vis-closed', 'Visual Closed Venue', {
      // published (so the menu renders, not 404) but hours read closed → /info ⇒ 'closed'.
      status: 'active', busyMode: false, hours: CLOSED_ALL_DAY, kitchenBusyUntil: null,
    });
    const busyId = await upsertLocation('vis-busy', 'Visual Busy Venue', {
      // open hours + busy_mode + a fixed far-future kitchen_busy_until → /info ⇒ 'busy'.
      status: 'active', busyMode: true, hours: OPEN_ALL_DAY, kitchenBusyUntil: '2999-01-01T00:00:00Z',
    });

    // 3. menu_versions row (PK location_id) for each venue — read_public_menu defaults to 1
    //    when absent, but seeding it keeps the X-Menu-Version header stable.
    for (const id of [openId, closedId, busyId]) {
      await db.query(
        `INSERT INTO menu_versions (location_id, version) VALUES ($1, 1)
         ON CONFLICT (location_id) DO NOTHING`,
        [id],
      );
    }

    // 4. Owner membership on the OPEN venue (so /admin snapshots can authenticate to it).
    await db.query(
      `INSERT INTO memberships (user_id, location_id, role, status)
         VALUES ($1, $2, 'owner', 'active')
       ON CONFLICT (user_id, location_id, role) DO UPDATE SET status = 'active'`,
      [ownerId, openId],
    );

    // 5. Menu on the OPEN venue. Children have no natural unique key, so wipe-and-reseed
    //    scoped to this location → re-runs stay clean and never accumulate. (modifiers /
    //    product_modifier_groups cascade from their parents; products clear first.)
    await db.query(`DELETE FROM modifiers WHERE location_id = $1`, [openId]);
    await db.query(`DELETE FROM modifier_groups WHERE location_id = $1`, [openId]);
    await db.query(`DELETE FROM products WHERE location_id = $1`, [openId]);
    await db.query(`DELETE FROM categories WHERE location_id = $1`, [openId]);

    const catRes = await db.query(
      `INSERT INTO categories (location_id, name, sort_order)
         VALUES ($1, 'Pizza', 0) RETURNING id`,
      [openId],
    );
    const categoryId = catRes.rows[0].id;

    // Product A — carries a modifier group (radio: pick a size) with two modifiers.
    const prodARes = await db.query(
      `INSERT INTO products (location_id, category_id, name, description, price, is_available, sort_order)
         VALUES ($1, $2, 'Margherita', 'Tomato, mozzarella, basil', 850, true, 0) RETURNING id`,
      [openId, categoryId],
    );
    const productWithModifiersId = prodARes.rows[0].id;

    // Product B — flagged unavailable (86'd). This is the stoplistProductId; read_public_menu
    // filters is_available=true so it is absent from the public menu (the stoplist behaviour).
    const prodBRes = await db.query(
      `INSERT INTO products (location_id, category_id, name, description, price, is_available, sort_order)
         VALUES ($1, $2, 'Quattro Formaggi (86''d)', 'Currently unavailable', 950, false, 1) RETURNING id`,
      [openId, categoryId],
    );
    const stoplistProductId = prodBRes.rows[0].id;

    // Modifier group + modifiers, linked to product A.
    const mgRes = await db.query(
      `INSERT INTO modifier_groups (location_id, name, min_select, max_select, required, display_type)
         VALUES ($1, 'Size', 1, 1, true, 'radio') RETURNING id`,
      [openId],
    );
    const groupId = mgRes.rows[0].id;
    await db.query(
      `INSERT INTO modifiers (group_id, location_id, name, price_delta, available, sort_order)
         VALUES ($1, $2, 'Small', 0, true, 0), ($1, $2, 'Large', 300, true, 1)`,
      [groupId, openId],
    );
    await db.query(
      // product_modifier_groups.location_id is NOT NULL (RLS retrofit, migration 1780338982019).
      `INSERT INTO product_modifier_groups (product_id, group_id, location_id, sort_order)
         VALUES ($1, $2, $3, 0) ON CONFLICT (product_id, group_id) DO NOTHING`,
      [productWithModifiersId, groupId, openId],
    );

    // 6. One order on the OPEN venue for the status screen. orders has no natural unique
    //    key, so a sentinel pickup_code makes it idempotent: reuse if present, else create
    //    (with its customer + one line item). Pricing in integer minor units (cash, pending).
    let orderId: string;
    const existingOrder = await db.query(
      `SELECT id FROM orders WHERE location_id = $1 AND pickup_code = 'VIS-ORDER' LIMIT 1`,
      [openId],
    );
    if (existingOrder.rowCount > 0) {
      orderId = existingOrder.rows[0].id;
    } else {
      const custRes = await db.query(
        `INSERT INTO customers (location_id, phone, name)
           VALUES ($1, '+355690000001', 'Visual Net Customer')
         ON CONFLICT (location_id, phone) DO UPDATE SET name = EXCLUDED.name
         RETURNING id`,
        [openId],
      );
      const customerId = custRes.rows[0].id;
      const orderRes = await db.query(
        // request_hash is NOT NULL with its default dropped (migration 1780338982024) → must be explicit.
        `INSERT INTO orders
           (location_id, customer_id, type, status, delivery_address,
            subtotal, total, payment_method, payment_outcome, pickup_code, request_hash)
         VALUES ($1, $2, 'delivery', 'CONFIRMED', 'Rruga e Durresit 1, Tirana',
            850, 850, 'cash', 'pending', 'VIS-ORDER', 'visual-seed')
         RETURNING id`,
        [openId, customerId],
      );
      orderId = orderRes.rows[0].id;
      await db.query(
        `INSERT INTO order_items (order_id, product_id, name_snapshot, price_snapshot, quantity)
           VALUES ($1, $2, 'Margherita', 850, 1)`,
        [orderId, productWithModifiersId],
      );
    }

    // 7. SYNTHETIC COURIER + SHIFT + ASSIGNMENT (Item 3, council-hardened) so the live courier
    //    active-delivery view (/courier/delivery/:assignmentId) renders against a REAL encrypted
    //    courier + shift + assignment — impersonatable ONLY as this one identity via
    //    /dev/mock-auth (role:'courier', synthetic:true; the id is re-derived from the sentinel
    //    hash, never caller input).
    //
    //    email_hash is the NAMESPACED NON-EMAIL sentinel (SYNTHETIC_COURIER_EMAIL_HASH =
    //    sha256('synthetic:visual-net-courier:v1')) — no z.string().email() input can produce it,
    //    so ON CONFLICT (email_hash) DO UPDATE provably touches ONLY this row (never resurrects a
    //    real courier). PII is synthetic constants only, encrypted with the app's real crypto
    //    (encryptPII / argon2), matching courier/auth.ts's canonical email_hash/encrypt/insert flow.
    //
    //    Idempotency (constraint #2): couriers UPSERT on email_hash; courier_locations ON CONFLICT;
    //    courier_shifts (no natural key) insert-fresh-then-delete-stale scoped to (synthetic
    //    courier, open venue) — ordered so the assignment is re-pointed off the old shift before
    //    that shift is deleted (FK-safe); courier_assignments ON CONFLICT (order_id) (UNIQUE) on
    //    the seed's own order. Re-running never duplicates and never 500s on a unique/FK constraint.
    const SYNTH_FULL_NAME = SYNTHETIC_COURIER_DISPLAY_NAME; // "Visual Net Courier" (Counsel A4)
    const synthEmailEncrypted = encryptPII('synthetic+visual-net-courier@invalid'); // never parsed; PII at rest only
    const synthFullNameEncrypted = encryptPII(SYNTH_FULL_NAME);
    // A fixed argon2 hash of a constant throwaway password — never used to log in (mint is
    // re-derived, not password-authed) but password_hash is NOT NULL on couriers.
    const synthPasswordHash = await argon2.hash('synthetic-visual-net-courier-pw', {
      type: argon2.argon2id, memoryCost: 65536, timeCost: 3, parallelism: 4,
    });

    // couriers has NO RLS → plain pool query. UPSERT on the sentinel email_hash.
    const synthCourierRes = await db.query(
      `INSERT INTO couriers (email_encrypted, email_hash, full_name_encrypted, password_hash, status)
         VALUES ($1, $2, $3, $4, 'active')
       ON CONFLICT (email_hash) DO UPDATE SET full_name_encrypted = EXCLUDED.full_name_encrypted, status = 'active'
       RETURNING id`,
      [synthEmailEncrypted, SYNTHETIC_COURIER_EMAIL_HASH, synthFullNameEncrypted, synthPasswordHash],
    );
    const syntheticCourierId = synthCourierRes.rows[0].id as string;

    // courier_locations / courier_shifts / courier_assignments are FORCE RLS → tenant-scoped txn,
    // exactly like /dev/create-assignment above (set app.current_tenant + app.user_id on the OPEN
    // venue). app.user_id = the seeded owner so audit/insert policies that read it are satisfied.
    let syntheticAssignmentId: string;
    const synthClient = await db.connect();
    try {
      await synthClient.query('BEGIN');
      await synthClient.query(`SELECT set_config('app.current_tenant', $1, true)`, [openId]);
      await synthClient.query(`SELECT set_config('app.user_id', $1, true)`, [ownerId]);

      // Membership (role courier) on the OPEN venue.
      await synthClient.query(
        `INSERT INTO courier_locations (courier_id, location_id, role, added_by_owner_id)
           VALUES ($1, $2, 'courier', $3)
         ON CONFLICT (courier_id, location_id) DO NOTHING`,
        [syntheticCourierId, openId, ownerId],
      );

      // Shift (status 'available' — the proven pre-pickup state, M3). No natural key, so for
      // idempotency we INSERT the fresh shift FIRST, re-point the assignment onto it, and only
      // THEN delete any stale shift(s) from a prior run. Ordering matters: courier_assignments
      // .shift_id REFERENCES courier_shifts(id) with NO ON DELETE cascade (NO ACTION/restrict),
      // so deleting the old shift while a prior-run assignment still references it would raise an
      // FK violation. Re-pointing the assignment first frees the old shift for deletion.
      const synthShiftRes = await synthClient.query(
        `INSERT INTO courier_shifts (courier_id, location_id, status, started_at, last_heartbeat_at)
           VALUES ($1, $2, 'available', now(), now())
         RETURNING id`,
        [syntheticCourierId, openId],
      );
      const synthShiftId = synthShiftRes.rows[0].id as string;

      // Assignment (status 'assigned') for the seed's own order. UNIQUE (order_id) → ON CONFLICT
      // re-points to the synthetic courier + NEW shift on re-run (the seed owns this order,
      // L3-bounded). This also detaches the prior-run assignment from the old shift below.
      const synthAsgnRes = await synthClient.query(
        `INSERT INTO courier_assignments (order_id, courier_id, location_id, shift_id, status)
           VALUES ($1, $2, $3, $4, 'assigned')
         ON CONFLICT (order_id) DO UPDATE SET
           courier_id = EXCLUDED.courier_id,
           shift_id = EXCLUDED.shift_id,
           status = 'assigned'
         RETURNING id`,
        [orderId, syntheticCourierId, openId, synthShiftId],
      );
      syntheticAssignmentId = synthAsgnRes.rows[0].id as string;

      // Now delete any STALE shift(s) for this synthetic courier+venue (i.e. a prior run's shift,
      // no longer referenced by the just-re-pointed assignment). Scoped to the synthetic courier
      // only and excluding the shift we just created → re-runs converge to exactly one shift, and
      // the FK is never violated because nothing references the old shift any more.
      await synthClient.query(
        `DELETE FROM courier_shifts WHERE courier_id = $1 AND location_id = $2 AND id <> $3`,
        [syntheticCourierId, openId, synthShiftId],
      );

      await synthClient.query('COMMIT');
    } catch (err) {
      await synthClient.query('ROLLBACK').catch(() => {});
      throw err;
    } finally {
      synthClient.release();
    }

    // 8. Return the contract shape verbatim (VisualFixtures) + the synthetic-courier handles so
    //    the capture harness can mint synthetic:true and hit /courier/delivery/:assignmentId.
    return reply.send({
      open: { slug: 'vis-open', locationId: openId },
      closed: { slug: 'vis-closed', locationId: closedId },
      busy: { slug: 'vis-busy', locationId: busyId },
      stoplistProductId,
      orderId,
      courierId: VIS_COURIER_ID,
      syntheticCourierId,
      syntheticAssignmentId,
      syntheticCourier: true,
    });
  };

  // Register on both the /dev and /api/dev paths the harness may call. Both are
  // covered by isDevPath() → the same global dev gate applies to each.
  fastify.post('/dev/seed-visual-state', seedVisualHandler);
  fastify.post('/api/dev/seed-visual-state', seedVisualHandler);
}
