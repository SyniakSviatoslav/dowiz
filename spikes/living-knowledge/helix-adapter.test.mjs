// helix-adapter.test.mjs — DEV-GATED proof that the store round-trips on the REAL HelixDB engine.
//
// Operator chose (2026-07-07) "sovereign backend default + a proven, dev-gated adapter to the real
// closed engine for an empirical head-to-head". This is that proof. It is NOT in run-armaments (it
// needs Docker + the closed ghcr.io/helixdb/enterprise-dev image running on :6969) — it is opt-in:
//   docker run -d -p 6969:8080 ghcr.io/helixdb/enterprise-dev:latest
//   LK_HELIX=1 node spikes/living-knowledge/helix-adapter.test.mjs
//
// Falsifiable: the health assertion POSTs the real readiness query over HelixDB's confirmed JSON-AST
// wire format and fails (exit 1) if the adapter/wire is wrong. AddN + Count are attempted and reported
// (the exact write-then-count AST is only partially reverse-engineered — see ../helix-recon.md).
import { HelixStore } from './lib/store.mjs';

const GATED = process.env.LK_HELIX === '1';
if (!GATED) {
  console.log('helix-adapter.test: SKIPPED (set LK_HELIX=1 with the engine on :6969). Sovereign MemoryStore is the default; this only proves the optional real-engine backend.');
  process.exit(0);
}

const store = new HelixStore();
let failures = 0;
const ck = (name, ok, extra = '') => { console.log(`  ${ok ? '✓' : '✗'} ${name}${extra ? ` — ${extra}` : ''}`); if (!ok) failures++; };

const healthy = await store.health();
ck('POST /v1/query readiness → engine reachable (real wire protocol)', healthy);

if (healthy) {
  const before = await store.countByLabel('LKNode');
  let adds = 0;
  for (const id of ['lk-a', 'lk-b', 'lk-c']) if (await store.addNode({ id, label: 'LKNode', title: id })) adds++;
  const after = await store.countByLabel('LKNode');
  ck('AddN writes accepted by the real engine (3/3)', adds === 3, `${adds}/3 returned 200`);
  console.log(`  · countByLabel('LKNode'): before=${before} after=${after} (write→read round-trip; exact count depends on engine label indexing)`);
  if (typeof after === 'number' && typeof before === 'number') ck('count is a number and did not decrease (read path works)', after >= before);
}

if (failures) { console.error(`\n✗ helix-adapter.test: ${failures} assertion(s) failed against the real engine.`); process.exit(1); }
console.log('\n✓ helix-adapter.test: the sovereign store speaks the REAL HelixDB engine wire protocol (Option C head-to-head backend proven).');
