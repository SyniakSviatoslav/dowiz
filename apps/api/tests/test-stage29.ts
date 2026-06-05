import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import { signAuthToken } from '@deliveryos/platform';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 3003}`;

test('Stage 29: Onboarding Wizard', async (t) => {
  const pool = createSessionPool();
  const orgId = crypto.randomUUID();
  const locId = crypto.randomUUID();
  const locIdB = crypto.randomUUID();
  const userId = crypto.randomUUID();

  let ownerToken: string;
  let ownerTokenB: string;

  // ─── Setup ────────────────────────────────────────────────────────
  await t.test('setup test data', async () => {
    await pool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId, `owner-p29-${Date.now()}@test.com`]);
    await pool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'P29 Org', $2) ON CONFLICT DO NOTHING`,
      [orgId, userId]);
    await pool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, currency_code, default_locale, supported_locales, widget_enabled, delivery_fee_flat)
      VALUES ($1, $2, $3, 'P29 Loc', '123', 'open', 'ALL', 'sq', '["sq","en"]'::jsonb, true, 0) ON CONFLICT DO NOTHING`,
      [locId, orgId, `p29-loc-${Date.now()}`]);
    await pool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, currency_code, default_locale, supported_locales, widget_enabled, delivery_fee_flat)
      VALUES ($1, $2, $3, 'P29 Loc B', '456', 'open', 'ALL', 'sq', '["sq","en"]'::jsonb, true, 0) ON CONFLICT DO NOTHING`,
      [locIdB, orgId, `p29-loc-b-${Date.now()}`]);
    await pool.query(`INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING`,
      [userId, locId]);
    await pool.query(`INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING`,
      [userId, locIdB]);

    ownerToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
    ownerTokenB = await signAuthToken({ role: 'owner', userId, activeLocationId: locIdB }, '15m');
  });

  // ═══════════════════════════════════════════════════════════════
  // R1: START CREATES RESOURCES
  // ═══════════════════════════════════════════════════════════════
  await t.test('R1.1: POST /onboarding/start creates location, org, membership, menu_versions, onboarding_state', async () => {
    const slug = `p29-start-${Date.now()}`;
    const res = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Test Onboarding Loc', phone: '+355690000001', slug }),
    });
    assert.strictEqual(res.status, 201);
    const data = await res.json();
    assert.ok(data.locationId);
    assert.strictEqual(data.slug, slug);
    assert.strictEqual(data.currentStep, 1);
    assert.deepStrictEqual(data.onboardingState.completedSteps, []);
    assert.deepStrictEqual(data.onboardingState.skippedSteps, []);

    const locIdNew = data.locationId;

    // Verify location row
    const locRes = await pool.query(
      `SELECT id, slug, name, phone, org_id, status, onboarding_state FROM locations WHERE id = $1`,
      [locIdNew],
    );
    assert.strictEqual(locRes.rowCount, 1);
    assert.strictEqual(locRes.rows[0].slug, slug);
    assert.strictEqual(locRes.rows[0].name, 'Test Onboarding Loc');
    assert.strictEqual(locRes.rows[0].phone, '+355690000001');
    assert.strictEqual(locRes.rows[0].status, 'open');

    // Verify org exists
    const orgRes = await pool.query(`SELECT id FROM organizations WHERE id = $1`, [locRes.rows[0].org_id]);
    assert.ok(orgRes.rowCount > 0);

    // Verify membership
    const memRes = await pool.query(
      `SELECT user_id, location_id, role FROM memberships WHERE location_id = $1 AND user_id = $2`,
      [locIdNew, userId],
    );
    assert.strictEqual(memRes.rowCount, 1);
    assert.strictEqual(memRes.rows[0].role, 'owner');

    // Verify menu_versions row
    const mvRes = await pool.query(
      `SELECT location_id, version FROM menu_versions WHERE location_id = $1`,
      [locIdNew],
    );
    assert.strictEqual(mvRes.rowCount, 1);
    assert.strictEqual(mvRes.rows[0].version, 1);

    // Verify onboarding_state initialized
    const rawState = locRes.rows[0].onboarding_state;
    assert.ok(rawState);
    const parsed = typeof rawState === 'string' ? JSON.parse(rawState) : rawState;
    assert.strictEqual(parsed.v, 1);
    assert.strictEqual(parsed.step, 1);
    assert.deepStrictEqual(parsed.completedSteps, []);
    assert.deepStrictEqual(parsed.skippedSteps, []);

    // Cleanup
    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locIdNew]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locIdNew]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locIdNew]);
  });

  await t.test('R1.2: POST /onboarding/start reuses existing org for same owner', async () => {
    const slug = `p29-reuse-${Date.now()}`;
    const existingOrgCount = (await pool.query(`SELECT COUNT(*)::int AS cnt FROM organizations WHERE owner_id = $1`, [userId])).rows[0].cnt;

    const res = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Reuse Org', phone: '+355690000099', slug }),
    });
    assert.strictEqual(res.status, 201);
    const data = await res.json();
    const locIdNew = data.locationId;

    // Org count should not have increased
    const afterCount = (await pool.query(`SELECT COUNT(*)::int AS cnt FROM organizations WHERE owner_id = $1`, [userId])).rows[0].cnt;
    assert.strictEqual(afterCount, existingOrgCount, 'Should reuse existing org');

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locIdNew]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locIdNew]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locIdNew]);
  });

  await t.test('R1.3: POST /onboarding/start rejects duplicate slug with 409', async () => {
    const slug = `p29-dupe-${Date.now()}`;
    // First creation
    const r1 = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'First', phone: '+355690000011', slug }),
    });
    assert.strictEqual(r1.status, 201);
    const d1 = await r1.json();

    // Second with same slug
    const r2 = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Second', phone: '+355690000012', slug }),
    });
    assert.strictEqual(r2.status, 409);
    const d2 = await r2.json();
    assert.strictEqual(d2.code, 'SLUG_TAKEN');

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [d1.locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [d1.locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [d1.locationId]);
  });

  await t.test('R1.4: POST /onboarding/start validates slug format', async () => {
    const res = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Bad Slug', phone: '+355690000013', slug: 'BAD SLUG WITH SPACES' }),
    });
    assert.strictEqual(res.status, 400);
  });

  // ═══════════════════════════════════════════════════════════════
  // R2: STEP COMPLETION
  // ═══════════════════════════════════════════════════════════════
  await t.test('R2.1: POST /onboarding/:locationId/step/complete marks step done and advances', async () => {
    const slug = `p29-comp-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Comp Test', phone: '+355690000002', slug }),
    });
    const { locationId } = await createRes.json();

    const res = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ step: 1 }),
    });
    assert.strictEqual(res.status, 200);
    const d = await res.json();
    assert.strictEqual(d.completed, false);
    assert.strictEqual(d.currentStep, 2);
    assert.ok(d.onboardingState.completedSteps.includes(1));
    assert.strictEqual(d.onboardingState.step, 2);
    assert.notStrictEqual(d.onboardingState.skippedSteps.includes(1), true);

    // Verify via GET state
    const stateRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/state`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(stateRes.status, 200);
    const stateData = await stateRes.json();
    assert.strictEqual(stateData.currentStep, 2);
    assert.ok(stateData.onboardingState.completedSteps.includes(1));

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  await t.test('R2.2: Completing all 8 steps sets completed=true and onboarding_completed_at', async () => {
    const slug = `p29-all-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'All Steps', phone: '+355690000014', slug }),
    });
    const { locationId } = await createRes.json();

    for (const step of [1, 2, 3, 4, 5, 6, 7, 8]) {
      const res = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ step }),
      });
      assert.strictEqual(res.status, 200);
      const d = await res.json();
      if (step < 8) {
        assert.strictEqual(d.completed, false);
        assert.strictEqual(d.currentStep, step + 1);
      } else {
        assert.strictEqual(d.completed, true);
        assert.strictEqual(d.currentStep, null);
      }
    }

    // Verify onboarding_completed_at is set
    const locRes = await pool.query(
      `SELECT onboarding_completed_at FROM locations WHERE id = $1`,
      [locationId],
    );
    assert.ok(locRes.rows[0].onboarding_completed_at !== null);

    // GET state now returns completed: true
    const stateRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/state`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const stateData = await stateRes.json();
    assert.strictEqual(stateData.completed, true);

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  // ═══════════════════════════════════════════════════════════════
  // R3: SKIP STEP
  // ═══════════════════════════════════════════════════════════════
  await t.test('R3.1: Skip step 4 returns Branding default theme note', async () => {
    const slug = `p29-sk4-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Skip4', phone: '+355690000015', slug }),
    });
    const { locationId } = await createRes.json();

    for (const step of [1, 2, 3]) {
      await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ step }),
      });
    }

    const skipRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/4/skip`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
    });
    assert.strictEqual(skipRes.status, 200);
    const d = await skipRes.json();
    assert.strictEqual(d.skipNote, 'Branding skipped — default theme applied');
    assert.strictEqual(d.currentStep, 5);
    assert.ok(d.onboardingState.skippedSteps.includes(4));
    assert.ok(d.onboardingState.completedSteps.includes(4));

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  await t.test('R3.2: Skip step 5 returns pickup-only note', async () => {
    const slug = `p29-sk5-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Skip5', phone: '+355690000016', slug }),
    });
    const { locationId } = await createRes.json();

    for (const step of [1, 2, 3, 4]) {
      await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ step }),
      });
    }

    const skipRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/5/skip`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
    });
    assert.strictEqual(skipRes.status, 200);
    const d = await skipRes.json();
    assert.strictEqual(d.skipNote, 'Delivery skipped — pickup-only mode, no delivery radius');

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  await t.test('R3.3: Skip step 7 returns dashboard+push note', async () => {
    const slug = `p29-sk7-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Skip7', phone: '+355690000017', slug }),
    });
    const { locationId } = await createRes.json();

    for (const step of [1, 2, 3, 4, 5, 6]) {
      await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ step }),
      });
    }

    const skipRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/7/skip`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
    });
    assert.strictEqual(skipRes.status, 200);
    const d = await skipRes.json();
    assert.strictEqual(d.skipNote, "Telegram skipped — you'll still receive alerts on the dashboard and via push");
    assert.strictEqual(d.currentStep, 8);
    assert.strictEqual(d.completed, false);

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  // ═══════════════════════════════════════════════════════════════
  // R4: GET STATE
  // ═══════════════════════════════════════════════════════════════
  await t.test('R4.1: GET /onboarding/:locationId/state returns correct state', async () => {
    const slug = `p29-state-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'State Test', phone: '+355690000018', slug }),
    });
    const { locationId } = await createRes.json();

    const stateRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/state`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(stateRes.status, 200);
    const state = await stateRes.json();
    assert.strictEqual(state.locationId, locationId);
    assert.strictEqual(state.slug, slug);
    assert.strictEqual(state.name, 'State Test');
    assert.strictEqual(state.currentStep, 1);
    assert.strictEqual(state.completed, false);
    assert.deepStrictEqual(state.onboardingState.completedSteps, []);
    assert.deepStrictEqual(state.onboardingState.skippedSteps, []);

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  await t.test('R4.2: GET /onboarding/:locationId/state for non-existent location returns 404', async () => {
    const fakeId = crypto.randomUUID();
    const res = await fetch(`${BASE}/api/owner/onboarding/${fakeId}/state`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    assert.strictEqual(res.status, 404);
  });

  // ═══════════════════════════════════════════════════════════════
  // R5: IDEMPOTENT STEP COMPLETION
  // ═══════════════════════════════════════════════════════════════
  await t.test('R5.1: Completing the same step twice returns 200 (idempotent)', async () => {
    const slug = `p29-idem-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Idem Test', phone: '+355690000019', slug }),
    });
    const { locationId } = await createRes.json();

    // First completion
    const r1 = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ step: 1 }),
    });
    assert.strictEqual(r1.status, 200);

    // Same step again — idempotent
    const r2 = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ step: 1 }),
    });
    assert.strictEqual(r2.status, 200);
    const d = await r2.json();
    assert.strictEqual(d.currentStep, 2);
    // completedSteps should have 1 exactly once
    const ones = d.onboardingState.completedSteps.filter((s: number) => s === 1);
    assert.strictEqual(ones.length, 1);

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  // ═══════════════════════════════════════════════════════════════
  // R6: NON-SKIPPABLE STEPS
  // ═══════════════════════════════════════════════════════════════
  await t.test('R6.1: Non-skippable steps (1,2,3,6,8) return 400 on skip', async () => {
    const slug = `p29-noskip-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'NoSkip', phone: '+355690000020', slug }),
    });
    const { locationId } = await createRes.json();

    for (const step of [1, 2, 3, 6, 8]) {
      const skipRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/${step}/skip`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      });
      assert.strictEqual(skipRes.status, 400, `Step ${step} should not be skippable`);
      const d = await skipRes.json();
      assert.ok(d.error.includes('cannot be skipped'), `Step ${step} error: ${d.error}`);
    }

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  // ═══════════════════════════════════════════════════════════════
  // R7: STEP ORDERING
  // ═══════════════════════════════════════════════════════════════
  await t.test('R7.1: Cannot complete step 3 when step 1 is current', async () => {
    const slug = `p29-order-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Order Test', phone: '+355690000021', slug }),
    });
    const { locationId } = await createRes.json();

    const res = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ step: 3 }),
    });
    assert.strictEqual(res.status, 400);
    const d = await res.json();
    assert.strictEqual(d.error, 'Step 3 is not current. Current step is 1');
    assert.strictEqual(d.currentStep, 1);

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  await t.test('R7.2: Steps must be completed in order 1→2→3→4→5→6→7→8', async () => {
    const slug = `p29-seq-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Seq Test', phone: '+355690000022', slug }),
    });
    const { locationId } = await createRes.json();

    for (const step of [1, 2, 3, 4, 5, 6, 7, 8]) {
      const res = await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ step }),
      });
      assert.strictEqual(res.status, 200, `Step ${step} should succeed when current`);
      const d = await res.json();
      if (step < 8) {
        assert.strictEqual(d.currentStep, step + 1);
      }
    }

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  // ═══════════════════════════════════════════════════════════════
  // R8: ONBOARDING COMPLETE
  // ═══════════════════════════════════════════════════════════════
  await t.test('R8.1: GET /onboarding/:locationId/complete returns dashboard URL after completion', async () => {
    const slug = `p29-done-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'Done Test', phone: '+355690000023', slug }),
    });
    const { locationId } = await createRes.json();

    // Complete all steps
    for (const step of [1, 2, 3, 4, 5, 6, 7, 8]) {
      await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
        body: JSON.stringify({ step }),
      });
    }

    const completeRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/complete`, {
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
    });
    assert.strictEqual(completeRes.status, 200);
    const d = await completeRes.json();
    assert.strictEqual(d.slug, slug);
    assert.strictEqual(d.dashboardUrl, '/admin/dashboard.html');

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  await t.test('R8.2: GET /onboarding/:locationId/complete fails if onboarding not completed', async () => {
    const slug = `p29-nodone-${Date.now()}`;
    const createRes = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'NoDone', phone: '+355690000024', slug }),
    });
    const { locationId } = await createRes.json();

    // Only complete step 1
    await fetch(`${BASE}/api/owner/onboarding/${locationId}/step/complete`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      body: JSON.stringify({ step: 1 }),
    });

    const completeRes = await fetch(`${BASE}/api/owner/onboarding/${locationId}/complete`, {
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
    });
    assert.strictEqual(completeRes.status, 400);
    const d = await completeRes.json();
    assert.strictEqual(d.error, 'Onboarding not yet completed');

    await pool.query(`DELETE FROM menu_versions WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM memberships WHERE location_id = $1`, [locationId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locationId]);
  });

  // ═══════════════════════════════════════════════════════════════
  // R9: LOCATION MEMBERSHIP CHECK
  // ═══════════════════════════════════════════════════════════════
  await t.test('R9.1: Owner from location A cannot access onboarding for location B', async () => {
    // ownerToken has activeLocationId = locId (location A)
    // locIdB is a different location (location B)
    const res = await fetch(`${BASE}/api/owner/onboarding/${locIdB}/state`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    // Auth hook checks activeLocationId against params.locationId
    // ownerToken.activeLocationId = locId ≠ locIdB → 404
    assert.strictEqual(res.status, 404);
  });

  await t.test('R9.2: Owner with correct activeLocationId can access their onboarding', async () => {
    // ownerTokenB has activeLocationId = locIdB
    const res = await fetch(`${BASE}/api/owner/onboarding/${locIdB}/state`, {
      headers: { Authorization: `Bearer ${ownerTokenB}` },
    });
    assert.strictEqual(res.status, 200);
  });

  await t.test('R9.3: Non-owner role cannot access onboarding endpoints', async () => {
    const customerToken = await signAuthToken({ role: 'customer', userId, activeLocationId: locId }, '15m');
    const res = await fetch(`${BASE}/api/owner/onboarding/${locId}/state`, {
      headers: { Authorization: `Bearer ${customerToken}` },
    });
    assert.strictEqual(res.status, 403);
  });

  await t.test('R9.4: Unauthenticated request to onboarding returns 401', async () => {
    const res = await fetch(`${BASE}/api/owner/onboarding/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name: 'No Auth', phone: '+355690000025', slug: 'no-auth-test' }),
    });
    assert.strictEqual(res.status, 401);
  });

  // ═══════════════════════════════════════════════════════════════
  // CLEANUP
  // ═══════════════════════════════════════════════════════════════
  await t.test('cleanup test data', async () => {
    await pool.query(`DELETE FROM menu_versions WHERE location_id IN ($1, $2)`, [locId, locIdB]);
    await pool.query(`DELETE FROM memberships WHERE location_id IN ($1, $2)`, [locId, locIdB]);
    await pool.query(`DELETE FROM locations WHERE id IN ($1, $2)`, [locId, locIdB]);
    await pool.query(`DELETE FROM organizations WHERE id = $1`, [orgId]);
    await pool.query(`DELETE FROM users WHERE id = $1`, [userId]);
  });

  await pool.end();
});
