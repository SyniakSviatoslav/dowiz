/* eslint-disable @typescript-eslint/no-unused-vars, local/no-mock-in-prod -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { request as newRequest, type FullConfig } from '@playwright/test';
import { seedVisualState, type VisualFixtures } from './harness.js';
import { writeFileSync, mkdirSync, readFileSync, existsSync } from 'node:fs';
import { dirname } from 'node:path';

/**
 * Seed the deterministic visual fixtures ONCE before the snapshot suite, and write them to a JSON
 * file the specs read (Playwright globalSetup can't pass values directly to tests). Idempotent.
 */
const FIXTURES_PATH = 'e2e/visual/.fixtures.json';

export default async function globalSetup(config: FullConfig): Promise<void> {
  const baseURL = process.env.VISUAL_BASE_URL || process.env.VITE_BASE_URL || 'http://localhost:3000';
  const ctx = await newRequest.newContext({ baseURL });
  try {
    const fixtures = await seedVisualState(ctx);
    mkdirSync(dirname(FIXTURES_PATH), { recursive: true });
    writeFileSync(FIXTURES_PATH, JSON.stringify(fixtures, null, 2));
    console.log('[visual] seeded fixtures →', FIXTURES_PATH);
  } finally {
    await ctx.dispose();
  }
}

/**
 * Specs import this to read the fixtures seeded above. Returns inert placeholders when the file is
 * absent (e.g. during `--list`/collection before a seeded run) so spec collection never throws — a
 * real run always has the file (globalSetup writes it before any test executes).
 */
export function readFixtures(): VisualFixtures {
  if (!existsSync(FIXTURES_PATH)) {
    const stub = { slug: 'vis-open', locationId: '00000000-0000-0000-0000-000000000000' };
    return { open: stub, closed: { slug: 'vis-closed', locationId: stub.locationId }, busy: { slug: 'vis-busy', locationId: stub.locationId }, stoplistProductId: stub.locationId, orderId: stub.locationId, courierId: stub.locationId };
  }
  return JSON.parse(readFileSync(FIXTURES_PATH, 'utf8')) as VisualFixtures;
}
