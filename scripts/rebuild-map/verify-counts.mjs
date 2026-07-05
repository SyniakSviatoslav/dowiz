#!/usr/bin/env node
// scripts/rebuild-map/verify-counts.mjs
//
// Reconcile check (REBUILD-MAP.md §3 Phase-0 item 2): compares each extractor's live count
// against the count recorded in docs/design/rebuild-plan/REBUILD-MAP.md §1's summary table
// (the "consolidated counts" table — lines 19-35 of that file, 2026-07-04 synthesis).
//
// Deltas are FINDINGS, not failures — the inventory docs are point-in-time censuses of a
// tree that keeps moving (voice FE work, i18n edits, etc. landed same-day). This script
// always exits 0; it prints a MATCH/DELTA table and a short note per delta explaining
// whether it's a real drift or a unit-of-measure mismatch (declared inline below).
//
// Usage: node scripts/rebuild-map/verify-counts.mjs

import { extract as extractRoutes } from './extract-routes.mjs';
import { extract as extractFeRoutes } from './extract-fe-routes.mjs';
import { extract as extractComponents } from './extract-components.mjs';
import { extract as extractViteFlags } from './extract-vite-flags.mjs';
import { extract as extractServerFlagsEnvs } from './extract-server-flags-envs.mjs';
import { extract as extractI18nKeys } from './extract-i18n-keys.mjs';
import { extract as extractWsTypes } from './extract-ws-types.mjs';
import { extract as extractQueues } from './extract-queues.mjs';
import { extract as extractErrorCodes } from './extract-error-codes.mjs';
import { extract as extractTables } from './extract-tables.mjs';
import { extract as extractScriptsGates } from './extract-scripts-gates.mjs';

// ---------------------------------------------------------------------------------------
// Expected counts, hardcoded from REBUILD-MAP.md §1 "consolidated counts" table
// (docs/design/rebuild-plan/REBUILD-MAP.md, lines 19-35, 2026-07-04 synthesis).
// Each entry cites the exact table row + a note when the extractor measures a DIFFERENT
// unit than the row's headline number (e.g. "route elements" vs "addressable paths") —
// those are expected/definitional deltas, not extractor bugs.
// ---------------------------------------------------------------------------------------
const EXPECTED = {
  // L19: "HTTP routes | 236 (+2 non-route handlers)"
  routes: { count: 236, source: 'REBUILD-MAP.md:19 "HTTP routes | 236"' },

  // L24: "FE routes / pages | 27 / 35" — the 27 is addressable *paths*; this extractor
  // reproduces inventory/11 §0 C1's raw <Route> ELEMENT count (40), a different unit by
  // design (multiple <Route> elements can share one addressable path via layout nesting).
  'fe-routes': {
    count: 27,
    source:
      'REBUILD-MAP.md:24 "FE routes / pages | 27 / 35" (addressable paths; ' +
      'this extractor counts C1 raw <Route> elements = 40 per inventory/11 §0 — different unit, expected delta)',
  },

  // L25: "Components | 67 (56 ui + 11 web; 8 dead-candidates)"
  components: { count: 67, source: 'REBUILD-MAP.md:25 "Components | 67 (56 ui + 11 web...)"' },

  // L27: "Client flags | 19 VITE_*"
  'vite-flags': { count: 19, source: 'REBUILD-MAP.md:27 "Client flags | 19 VITE_*"' },

  // L23: "Boot/env/flags (server) | 80 EnvSchema + 48 raw reads (~20 shadow) · 35 flags"
  // This namespace folds EnvSchema fields + raw process.env reads into one list (both are
  // "server-flags-envs" per the task brief), so the comparable total is 80+48=128.
  // inventory/14 §1b/§2 (same-day, more careful pass) already found 119 schema fields live —
  // see extract-server-flags-envs.mjs header for the full reconciliation, including a grep
  // binary-file bug this extractor's programmatic parse avoids.
  'server-flags-envs': {
    count: 128,
    source:
      'REBUILD-MAP.md:23 "Boot/env/flags (server) | 80 EnvSchema + 48 raw reads..." (80+48=128; ' +
      'inventory/14 §2 already reconciles the schema half to 119 — see extractor header)',
  },

  // L26: "i18n keys | 1,445 × 3 locales"
  'i18n-keys': { count: 1445, source: 'REBUILD-MAP.md:26 "i18n keys | 1,445 x 3 locales"' },

  // L20: "WS inbound / outbound / rooms | 5 / 24 / 5" — rooms (5) are a separate concept
  // (channel-name grammar, not a message `type`), out of scope for this ws-types namespace.
  'ws-types': {
    count: 29,
    source:
      'REBUILD-MAP.md:20 "WS inbound / outbound / rooms | 5 / 24 / 5" (5+24=29; rooms excluded — ' +
      'not a message-type namespace). This extractor is deliberately un-curated for outbound ' +
      '(§8b "dumb extractors") so a large delta here is expected, not a bug.',
  },

  // L21: "Job queues / crons | 30 live (+3 dead) / 23 UTC crons" -> 33 distinct queue-name constants
  queues: {
    count: 33,
    source:
      'REBUILD-MAP.md:21 "Job queues / crons | 30 live (+3 dead)..." (30+3=33 distinct queue-name constants)',
  },

  // L32: "Error codes | 68 envelope + 8 preflight + 51 ad-hoc sites" -> this namespace = the 68 envelope codes
  'error-codes': {
    count: 68,
    source: 'REBUILD-MAP.md:32 "Error codes | 68 envelope + 8 preflight + 51 ad-hoc sites" (68 envelope codes)',
  },

  // L28: "Tables | 86 live (84 from 157 migrations + 2 out-of-band)"
  tables: {
    count: 86,
    source:
      'REBUILD-MAP.md:28 "Tables | 86 live (84 from 157 migrations + 2 out-of-band)..." ' +
      '(the 2 out-of-band tables are applied by hand outside packages/db/migrations/ and are ' +
      'structurally invisible to a migrations-only extractor)',
  },

  // L31: "Root scripts / script files / guardrails / gates / lint rules | 70 / 93 / 11 / 25 / 26"
  // This namespace folds root scripts + verify:all gates + eslint rules + guardrail files
  // (NOT the 93 "script files" count, a different measure — file count under scripts/).
  'scripts-gates': {
    count: 132,
    source:
      'REBUILD-MAP.md:31 "...70 / 93 / 11 / 25 / 26" (70 scripts + 11 guardrails + 25 gates + ' +
      '26 lint rules = 132; the 93 "script files" figure is a different unit, excluded here)',
  },
};

const EXTRACTORS = [
  ['routes', extractRoutes],
  ['fe-routes', extractFeRoutes],
  ['components', extractComponents],
  ['vite-flags', extractViteFlags],
  ['server-flags-envs', extractServerFlagsEnvs],
  ['i18n-keys', extractI18nKeys],
  ['ws-types', extractWsTypes],
  ['queues', extractQueues],
  ['error-codes', extractErrorCodes],
  ['tables', extractTables],
  ['scripts-gates', extractScriptsGates],
];

async function main() {
  console.log('Reconcile check: extractor counts vs REBUILD-MAP.md §1 table\n');
  console.log(
    ['namespace'.padEnd(20), 'live'.padStart(6), 'expected'.padStart(9), 'verdict'.padStart(8)].join(' | '),
  );
  console.log('-'.repeat(60));

  const rows = [];
  for (const [ns, fn] of EXTRACTORS) {
    const records = await fn();
    const live = records.length;
    const exp = EXPECTED[ns];
    const verdict = live === exp.count ? 'MATCH' : 'DELTA';
    rows.push({ ns, live, expected: exp.count, verdict, source: exp.source });
    console.log(
      [ns.padEnd(20), String(live).padStart(6), String(exp.count).padStart(9), verdict.padStart(8)].join(' | '),
    );
  }

  console.log('\nDELTA detail (finding, not failure — see source note):');
  for (const r of rows) {
    if (r.verdict === 'DELTA') {
      console.log(`  [DELTA] ${r.ns}: live=${r.live} expected=${r.expected} (Δ${r.live - r.expected})`);
      console.log(`          source: ${r.source}`);
    }
  }

  const matchCount = rows.filter((r) => r.verdict === 'MATCH').length;
  console.log(`\n${matchCount}/${rows.length} namespaces MATCH; ${rows.length - matchCount} DELTA (see above).`);
  // Always exit 0 — see file header: deltas are findings, this is a reconcile report, not a gate.
}

main();
