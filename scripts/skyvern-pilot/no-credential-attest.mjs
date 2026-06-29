#!/usr/bin/env node
// G4 control (load-bearing-adjacent) — Skyvern sidecar NO-CREDENTIAL attestation.
// The Skyvern pilot is an OUT-OF-BAND, out-of-tree AGPL sidecar reached over HTTP. It must NEVER hold
// a dowiz database / RLS-bearing / tenant secret (Breaker H4; ADR-tooling-integration-eval G4(c)).
// This asserts a given sidecar env file (arg, or $SKYVERN_ENV_FILE) carries none of them.
//
// Run: node scripts/skyvern-pilot/no-credential-attest.mjs <path-to-sidecar.env>
import { readFileSync, existsSync } from 'node:fs';

const file = process.argv[2] || process.env.SKYVERN_ENV_FILE;
if (!file) { console.error('usage: no-credential-attest.mjs <sidecar.env>'); process.exit(2); }
if (!existsSync(file)) { console.error(`✗ sidecar env file not found: ${file}`); process.exit(2); }

// Any of these in the sidecar env = a dowiz secret leaked into the AGPL out-of-tree process.
const FORBIDDEN = [
  /\bDATABASE_URL/i, /\bDATABASE_URL_MIGRATIONS/i, /\bDATABASE_URL_OPERATIONAL/i,
  /\bJWT_(DEV_)?(PRIVATE|PUBLIC)_KEY/i, /\bBACKUP_ENCRYPTION_KEY/i, /\bCOURIER_PII_ENCRYPTION_KEY/i,
  /\bDEV_AUTH_SECRET/i, /\bVAPID_PRIVATE_KEY/i, /\bREDIS_URL/i, /\bR2_SECRET_ACCESS_KEY/i,
  /\bTELEGRAM_BOT_(TOKEN|SECRET)/i, /\bSUPABASE/i, /\bSERVICE_ROLE/i, /\bRLS\b/i,
];
const hits = [];
for (const raw of readFileSync(file, 'utf8').split('\n')) {
  const line = raw.trim();
  if (!line || line.startsWith('#')) continue;
  const key = line.split('=')[0].trim();
  for (const re of FORBIDDEN) if (re.test(key)) hits.push(key);
}
if (hits.length) {
  console.error(`✗ Skyvern no-credential attestation FAILED — dowiz secret(s) present in the sidecar env:`);
  for (const h of [...new Set(hits)]) console.error('  - ' + h);
  console.error('\nThe AGPL out-of-tree sidecar must hold ONLY its own config + the local-LLM endpoint. Remove these.');
  process.exit(1);
}
console.log('✓ Skyvern sidecar attestation: no dowiz database / RLS / tenant secret present.');
