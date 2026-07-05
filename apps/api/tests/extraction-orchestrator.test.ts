import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { createSource, getById } from '../src/modules/acquisition/service.js';
import { orchestrateExtraction, type MenuParser } from '../src/modules/acquisition/extraction-orchestrator.js';
import type { LocateResult } from '../src/modules/acquisition/menu-source.js';

// P6 extraction orchestrator — drives SOURCED→ENRICHED (or to a terminal verdict) over the real state
// machine, with locate + parser INJECTED (no network / no LLM). Proves the wiring + the H4 verdict routing.
const url = process.env.PROV_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;

let pool: Pool;
before(() => { if (url) pool = new Pool({ connectionString: url }); });
after(async () => { if (pool) await pool.end(); });

const htmlLocate = async (): Promise<LocateResult> => ({ kind: 'html', bytes: Buffer.from('<html>menu</html>') });
const noneLocate = async (): Promise<LocateResult> => ({ kind: 'none', note: 'robots.txt disallows' });

function parserReturning(over: Partial<{ valid: number; issues: any[]; low: number }>): MenuParser {
  return {
    parse: async () => ({
      draft: {
        categories: [{ externalKey: 'c1', name: 'Pizza' }],
        products: [{ externalKey: 'p1', categoryKey: 'c1', name: 'Margherita', price: 850 }],
      },
      issues: over.issues ?? [],
      summary: { valid: over.valid ?? 1, low_confidence_count: over.low ?? 0 },
    }),
  };
}

async function freshSource(): Promise<string> {
  const s = await createSource(pool, 'ChIJ_orch_' + crypto.randomBytes(6).toString('hex'));
  return s.id; // SOURCED
}

maybe('clean extraction → ENRICHED with a menu_draft', async () => {
  const id = await freshSource();
  const r = await orchestrateExtraction(pool, id, 'https://x.test/menu', parserReturning({ valid: 1 }), { locate: htmlLocate });
  assert.equal(r.state, 'ENRICHED');
  const src = await getById(pool, id);
  assert.equal(src?.state, 'ENRICHED');
  assert.ok(src?.menu_draft, 'menu_draft populated');
  assert.equal((src!.menu_draft as any).categories[0].products[0].name, 'Margherita');
});

maybe('locate finds nothing → MENU_NOT_FOUND (with reason)', async () => {
  const id = await freshSource();
  const r = await orchestrateExtraction(pool, id, 'https://x.test', parserReturning({ valid: 1 }), { locate: noneLocate });
  assert.equal(r.state, 'MENU_NOT_FOUND');
  const src = await getById(pool, id);
  assert.equal(src?.state, 'MENU_NOT_FOUND');
  assert.ok((src?.failure_reason ?? '').length > 0, 'reason required on the exit state');
});

maybe('0 valid items → MANUAL_REVIEW (H4 no-fabrication)', async () => {
  const id = await freshSource();
  const r = await orchestrateExtraction(pool, id, 'https://x.test', parserReturning({ valid: 0 }), { locate: htmlLocate });
  assert.equal(r.state, 'MANUAL_REVIEW');
  const src = await getById(pool, id);
  assert.equal(src?.state, 'MANUAL_REVIEW');
});

maybe('an error-severity issue → LOW_QUALITY', async () => {
  const id = await freshSource();
  const r = await orchestrateExtraction(pool, id, 'https://x.test', parserReturning({ valid: 2, issues: [{ severity: 'error', code: 'PARSE_ERROR', message: 'bad' }] }), { locate: htmlLocate });
  assert.equal(r.state, 'LOW_QUALITY');
});
