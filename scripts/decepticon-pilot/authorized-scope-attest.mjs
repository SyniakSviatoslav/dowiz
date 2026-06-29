#!/usr/bin/env node
// Decepticon AUTHORIZED-SCOPE attestation (load-bearing ETHICS control).
//
// Decepticon (PurpleAILAB, Apache-2.0) is an autonomous OFFENSIVE red-team / C2 attack-chain agent. The
// Ethics Charter + dowiz security policy permit it ONLY as authorized DEFENSIVE red-teaming of dowiz's
// OWN infrastructure. This gate asserts the engagement target list contains ONLY dowiz-owned hosts —
// any non-dowiz target (i.e. attacking someone else) → exit 1. Run BEFORE any engagement.
//
// Run: node scripts/decepticon-pilot/authorized-scope-attest.mjs <targets-file>
//   (targets-file: one host/URL/CIDR per line; comments with # allowed)
import { readFileSync, existsSync } from 'node:fs';

// dowiz-owned scope ONLY. A target matches if its host equals or is a subdomain of one of these.
const ALLOWED_HOSTS = [
  'dowiz.fly.dev', 'dowiz-staging.fly.dev', 'dowiz-staging-db.fly.dev',
  'dowiz-db.fly.dev', 'dowiz.org', 'api.dowiz.org',
];
const ALLOWED_SUFFIXES = ['.dowiz.org', '.dowiz.fly.dev']; // dowiz-owned subdomains
// Loopback/private ranges are allowed (a local sandbox of a dowiz target), public non-dowiz is NOT.
const PRIVATE = /^(127\.|10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.|localhost$|::1$)/i;

const file = process.argv[2];
if (!file || !existsSync(file)) { console.error('usage: authorized-scope-attest.mjs <targets-file>'); process.exit(2); }

const hostOf = (line) => {
  const t = line.trim().replace(/^[a-z]+:\/\//i, '').split('/')[0].split(':')[0];
  return t;
};
const allowed = (host) =>
  PRIVATE.test(host) ||
  ALLOWED_HOSTS.includes(host) ||
  ALLOWED_SUFFIXES.some((s) => host.endsWith(s));

const lines = readFileSync(file, 'utf8').split('\n').map((l) => l.trim()).filter((l) => l && !l.startsWith('#'));
if (lines.length === 0) { console.error('✗ no targets found'); process.exit(2); }

const unauthorized = lines.map(hostOf).filter((h) => h && !allowed(h));
if (unauthorized.length > 0) {
  console.error('❌ UNAUTHORIZED TARGET(S) — Decepticon may run ONLY against dowiz-owned infra:');
  for (const h of [...new Set(unauthorized)]) console.error(`   ${h}`);
  console.error('\nEthics Charter: offensive tooling is permitted only for authorized defensive red-teaming\nof our OWN systems. Remove non-dowiz targets, or do not run.');
  process.exit(1);
}
console.log(`✅ authorized scope: ${lines.length} target(s), all dowiz-owned (or local sandbox).`);
process.exit(0);
