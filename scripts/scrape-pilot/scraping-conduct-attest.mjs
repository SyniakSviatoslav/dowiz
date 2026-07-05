#!/usr/bin/env node
// Scraping-conduct attestation (ethics/legal control for the STORM/Scrapling pilot).
//
// "Scrape the market" is legitimate ONLY as respectful, public-source research: robots.txt honored,
// rate-limited, public sources only, NO PII harvesting, no ToS-violating/auth-walled targets. This gate
// asserts a scrape-config JSON declares those controls and that no target is auth-walled/private/PII-dense.
// Fail-closed. Run BEFORE any scrape run.
//
// Run: node scripts/scrape-pilot/scraping-conduct-attest.mjs <config.json>
//   config.json: { respect_robots_txt, rate_limited, min_delay_ms, public_sources_only,
//                  no_pii_harvest, targets: ["https://..."] }
import { readFileSync, existsSync } from 'node:fs';

const file = process.argv[2];
if (!file || !existsSync(file)) { console.error('usage: scraping-conduct-attest.mjs <config.json>'); process.exit(2); }

let cfg;
try { cfg = JSON.parse(readFileSync(file, 'utf8')); } catch { console.error('✗ config is not valid JSON'); process.exit(2); }

const errors = [];
// Required conduct flags — must be explicitly true.
for (const flag of ['respect_robots_txt', 'rate_limited', 'public_sources_only', 'no_pii_harvest']) {
  if (cfg[flag] !== true) errors.push(`conduct flag '${flag}' must be explicitly true`);
}
if (!(Number(cfg.min_delay_ms) >= 500)) errors.push('min_delay_ms must be >= 500 (respect target servers)');

// Targets must be public pages — reject auth-walled / private / PII-dense surfaces.
const FORBIDDEN_TARGET = /(\/login|\/signin|\/account|\/admin|\/api\/|\/dm\/|\/messages|\/inbox|mailto:|\/profile\/|instagram\.com\/(?!explore)|facebook\.com\/(?!.*\/posts)|linkedin\.com\/in\/)/i;
const targets = Array.isArray(cfg.targets) ? cfg.targets : [];
if (targets.length === 0) errors.push('no targets declared');
for (const t of targets) {
  if (FORBIDDEN_TARGET.test(String(t))) errors.push(`auth-walled/private/PII target not allowed: ${t}`);
}

if (errors.length > 0) {
  console.error('❌ scraping-conduct attestation FAILED:');
  for (const e of errors) console.error(`   - ${e}`);
  console.error('\nScraping is permitted only as respectful public-source research (robots/rate-limit/\npublic-only/no-PII). Fix the config or do not run.');
  process.exit(1);
}
console.log(`✅ scraping conduct attested: ${targets.length} public target(s), robots+rate-limit honored, no PII harvest.`);
process.exit(0);
