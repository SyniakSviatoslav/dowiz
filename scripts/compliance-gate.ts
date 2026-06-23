#!/usr/bin/env tsx
/**
 * compliance-gate.ts — enforces compliance/privacy-invariants.md before merge (v1).
 *
 * Run: `pnpm compliance:gate` (locally) or in CI. Exits 1 on any violation.
 *
 * This is the DRAFT backstop the compliance spec calls for. It checks the
 * mechanically-checkable invariants with conservative, low-false-positive rules:
 *   A. Every PII-bearing table in a migration is documented in compliance/data-map.md.
 *   B. Every external-service env var (new integration) is listed in compliance/subprocessors.md.
 *   C. No RAW customer PII (name/phone/address) on logs / message bus / queues.
 *   D. Known high-risk processing has a DPIA in compliance/dpia/.
 *
 * Escape hatch: append `// compliance-gate:allow <reason>` to a flagged line to accept it
 * (forces a human to write down why). Tune the allowlists/patterns below as the system grows.
 * Full schema-diff/PR-diff enforcement is a later iteration; this catches the common slips.
 */
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { execSync } from 'node:child_process';

const ROOT = process.cwd();
const violations: string[] = [];
const note = (m: string) => violations.push(m);

function read(p: string): string {
  try { return readFileSync(join(ROOT, p), 'utf8'); } catch { return ''; }
}
function walk(dir: string, out: string[] = []): string[] {
  if (!existsSync(join(ROOT, dir))) return out;
  for (const e of readdirSync(join(ROOT, dir), { withFileTypes: true })) {
    const rel = `${dir}/${e.name}`;
    if (e.isDirectory()) {
      if (/node_modules|dist|\.git|build|coverage/.test(e.name)) continue;
      walk(rel, out);
    } else if (/\.(ts|tsx)$/.test(e.name) && !/\.test\.ts$|\.spec\.ts$/.test(e.name)) {
      out.push(rel);
    }
  }
  return out;
}

const dataMap = read('compliance/data-map.md');
const subprocessors = read('compliance/subprocessors.md');
const dpiaDir = 'compliance/dpia';
const dpiaText = existsSync(join(ROOT, dpiaDir))
  ? readdirSync(join(ROOT, dpiaDir)).map((f) => read(`${dpiaDir}/${f}`)).join('\n')
  : '';

// ── A. PII tables in migrations must be documented in data-map.md ──────────────
// Conservative: column names that are unambiguously personal data.
const PII_COL = /\b(phone|email|delivery_address|delivery_lat|delivery_lng|full_name|display_name|subject_phone|ip_hash|phone_hash|user_agent_hash|consent_at|telegram_user_id|push_subscription|vapid_endpoint)\b/;
const migDir = 'packages/db/migrations';
if (existsSync(join(ROOT, migDir))) {
  for (const f of readdirSync(join(ROOT, migDir)).filter((f) => f.endsWith('.ts'))) {
    const sql = read(`${migDir}/${f}`);
    // tables touched by a CREATE TABLE / ALTER TABLE that also mentions a PII column
    const tableRe = /(?:CREATE TABLE(?: IF NOT EXISTS)?|ALTER TABLE)\s+(?:public\.)?([a-z_][a-z0-9_]*)/gi;
    let m: RegExpExecArray | null;
    const piiTables = new Set<string>();
    while ((m = tableRe.exec(sql))) {
      const table = m[1];
      // Scope to THIS statement only (up to the next `;`) so a multi-table migration
      // doesn't attribute a neighbouring table's PII column to this one.
      const semi = sql.indexOf(';', m.index);
      const stmt = sql.slice(m.index, semi === -1 ? m.index + 1200 : semi);
      if (PII_COL.test(stmt)) piiTables.add(table);
    }
    for (const t of piiTables) {
      if (!new RegExp(`\\b${t}\\b`).test(dataMap)) {
        note(`A: migration ${f} defines PII on table "${t}" not documented in compliance/data-map.md`);
      }
    }
  }
}

// ── B. External-service env vars must be in subprocessors.md ───────────────────
// A new integration shows up as a new external-service env var in packages/config.
const config = read('packages/config/src/index.ts');
const SERVICE_ENV = [
  'RESEND_API_KEY', 'TELEGRAM_BOT_TOKEN', 'SENTRY_DSN', 'GROQ_API_KEY', 'OPENAI_API_KEY',
  'OPENROUTER_API_KEY', 'OPENCODE_ZEN_API_KEY',
  'REDIS_URL', 'ROUTING_BASE_URL', 'R2_ENDPOINT', 'VAPID_PUBLIC_KEY', 'DATABASE_URL_OPERATIONAL',
];
for (const env of SERVICE_ENV) {
  if (new RegExp(`\\b${env}\\b`).test(config) && !new RegExp(`\\b${env.replace(/_(API_KEY|DSN|URL|ENDPOINT|TOKEN|BASE_URL|PUBLIC_KEY|OPERATIONAL)$/, '')}`, 'i').test(subprocessors) && !subprocessors.includes(env.split('_')[0])) {
    note(`B: external-service env "${env}" used in config but its service is not in compliance/subprocessors.md`);
  }
}

// ── C. No RAW customer PII on logs / message bus / queues ──────────────────────
// Raw tokens only — masked/hashed/encrypted variants are allowed.
const SINK = /(console\.(log|error|warn|info|debug)|\.log\.(info|warn|error|debug)|messageBus\.publish|\.boss\.send|queue\.boss\.send|pgNotify)\s*\(/;
const RAW_PII = /(customerName|customerPhone|deliveryAddress|customer_name|customer_phone|delivery_address|subject_phone)\b/;
const SAFE_SUFFInX = /(Masked|masked|_hash|Hash|_encrypted|Encrypted)/;
for (const file of [...walk('apps/api/src'), ...walk('packages/platform/src'), ...walk('apps/web/src')]) {
  const lines = read(file).split('\n');
  for (let i = 0; i < lines.length; i++) {
    if (!SINK.test(lines[i])) continue;
    // gather the call's window (until rough paren balance or 10 lines)
    const window = lines.slice(i, i + 10).join('\n').split(/\)\s*;|\)\s*\.catch/)[0];
    if (window.includes('compliance-gate:allow')) continue;
    const mm = window.match(RAW_PII);
    if (mm) {
      // allow if the very token occurrence is a masked/hashed form
      const idx = window.indexOf(mm[0]);
      const around = window.slice(Math.max(0, idx - 2), idx + mm[0].length + 12);
      if (SAFE_SUFFInX.test(around)) continue;
      note(`C: raw PII "${mm[0]}" on a log/bus/queue sink — ${file}:${i + 1}`);
    }
  }
}

// ── D. Known high-risk processing must have a DPIA ─────────────────────────────
// courier_positions = systematic worker location monitoring (the strongest trigger).
if (existsSync(join(ROOT, migDir))) {
  const anyMig = readdirSync(join(ROOT, migDir)).map((f) => read(`${migDir}/${f}`)).join('\n');
  if (/courier_positions/.test(anyMig) && !/courier_positions|GPS|gps/.test(dpiaText)) {
    note('D: courier_positions (high-risk location tracking) has no DPIA in compliance/dpia/');
  }
}

// ── Report ─────────────────────────────────────────────────────────────────────
if (violations.length === 0) {
  console.log('✓ compliance-gate: all invariants hold (data-map / subprocessors / no-raw-PII / DPIA).');
  process.exit(0);
}
console.error(`✗ compliance-gate: ${violations.length} violation(s) — see compliance/privacy-invariants.md\n`);
for (const v of violations) console.error('  • ' + v);
console.error('\nFix the artifact (document the column/subprocessor/DPIA) or, if intentional & safe,');
console.error('append `// compliance-gate:allow <reason>` to the flagged code line.');
process.exit(1);
