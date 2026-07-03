#!/usr/bin/env node
// guardrail-no-set-cookie — B3 P0-5 · W4 cookie-less (bearer-only) GATE.
//
// dowiz/DeliveryOS is INTENTIONALLY bearer-only: auth travels in the `Authorization: Bearer`
// header, never in a cookie. That posture is what makes the app CSRF-free by construction —
// state-changing owner/courier routes require an explicit header, not an ambient credential the
// browser attaches automatically. This gate makes a SILENT regression (someone wiring in a
// session cookie / @fastify/cookie during an unrelated change) fail loud: it greps the app
// source for response cookie-SETTING and exits non-zero if any is found.
//
// It flags cookie-SETTING only (reply.setCookie / res.cookie / Set-Cookie response header /
// registering the cookie plugin) — NOT the mere string "set-cookie", so log/PII redaction lists
// that name the header to STRIP it (an anti-leak use) are not false positives.
//
// Run:  node scripts/guardrail-no-set-cookie.mjs         (exit 0 clean, exit 1 on any setter)
//       node scripts/guardrail-no-set-cookie.mjs --json
import { readFileSync } from 'node:fs';
import { execSync } from 'node:child_process';

const ROOT = process.cwd();
const JSON_OUT = process.argv.includes('--json');

// Source trees that ship to the client/server runtime. Tests, dist, node_modules excluded.
// NOTE: git pathspec does NOT support brace `{ts,js}` alternation (it silently matches zero
// files → a false green). A single `*` in git pathspec already matches across `/`, so these
// tree-roots recurse; extension filtering is done in JS below.
const SCAN_GLOBS = [
  'apps/api/src/*',
  'apps/web/src/*',
  'packages/*/src/*',
];
const SCAN_EXT = /\.(ts|tsx|js|jsx|mjs|cjs)$/;

// Patterns that SET a cookie on a response (the thing we forbid). Ordered, each with a label.
const SETTER_PATTERNS = [
  { label: 'reply.setCookie(', re: /\breply\.setCookie\s*\(/ },
  { label: 'reply.cookie(', re: /\breply\.cookie\s*\(/ },
  { label: 'res.cookie(', re: /\bres(?:ponse)?\.cookie\s*\(/ },
  { label: 'res.clearCookie(', re: /\b(?:reply|res(?:ponse)?)\.clearCookie\s*\(/ },
  { label: 'setHeader Set-Cookie', re: /\.setHeader\s*\(\s*['"`]set-cookie['"`]/i },
  { label: 'reply.header Set-Cookie', re: /\.header\s*\(\s*['"`]set-cookie['"`]/i },
  { label: "'Set-Cookie': response-header literal", re: /['"`]set-cookie['"`]\s*:/i },
  { label: '@fastify/cookie import/register', re: /['"`]@fastify\/cookie['"`]|fastify-cookie/ },
];

// ── ALLOWLIST ─────────────────────────────────────────────────────────────────────────
// Legitimate NON-auth (anti-leak) references to the "set-cookie" string: log & error-report
// redaction lists that STRIP the header from telemetry. These never SET a cookie. Each entry
// is a file path whose "set-cookie" mentions are redaction keys, hand-verified 2026-07-03.
// A regex-based `'set-cookie':` in these files is a redaction map key, not a response header.
const ALLOWLIST_FILES = new Set([
  'apps/api/src/lib/logger.ts', // pino redact paths: strips set-cookie from request logs
  'apps/api/src/lib/sentry.ts', // Sentry beforeSend: strips set-cookie from error reports
]);

function listFiles() {
  const out = execSync(`git ls-files ${SCAN_GLOBS.map((g) => `'${g}'`).join(' ')}`, {
    cwd: ROOT, encoding: 'utf8',
  });
  return out.split('\n').filter(Boolean)
    .filter((p) => SCAN_EXT.test(p) && !p.includes('/dist/') && !p.includes('node_modules'));
}

const findings = [];
for (const file of listFiles()) {
  if (ALLOWLIST_FILES.has(file)) continue;
  let src;
  try { src = readFileSync(file, 'utf8'); } catch { continue; }
  const lines = src.split('\n');
  lines.forEach((line, i) => {
    // Ignore comment-only lines (a comment mentioning cookies is not a setter).
    const code = line.replace(/\/\/.*$/, '');
    for (const { label, re } of SETTER_PATTERNS) {
      if (re.test(code)) findings.push({ file, line: i + 1, label, text: line.trim().slice(0, 120) });
    }
  });
}

if (JSON_OUT) {
  console.log(JSON.stringify({ ok: findings.length === 0, findings }, null, 2));
} else if (findings.length === 0) {
  console.log('[no-set-cookie] OK — bearer-only posture intact; no response cookie is set in app source.');
} else {
  console.error('[no-set-cookie] FAIL — the app is intentionally bearer-only (no cookies). Found cookie-setting:');
  for (const f of findings) console.error(`  ${f.file}:${f.line}  [${f.label}]  ${f.text}`);
  console.error('\nIf a NON-auth cookie is genuinely required, add an ADR and allowlist the file with a comment.');
}

process.exit(findings.length === 0 ? 0 : 1);
