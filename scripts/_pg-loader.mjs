// _pg-loader — resolve the workspace `pg` (8.21.0) from a pnpm store without a
// package.json dep on `pg` in scripts/. The CI-preflight guardrails need the
// exact same driver the app + node-pg-migrate use, so SSL/connection-string
// behaviour (sslmode=require → verify-full, sslmode=no-verify → rejectUnauthorized:false)
// matches production 1:1. Zero new deps; deterministic; no hardcoded creds.
//
// Resolution order:
//   1. bare `import('pg')`            (works if hoisted to a top-level node_modules)
//   2. scan node_modules/.pnpm/pg@*/  (pnpm strict layout — the dowiz case)
import { readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const ROOT = process.cwd();

export async function loadPg() {
  // 1. hoisted
  try {
    const m = await import('pg');
    const Client = m.default?.Client || m.Client;
    if (Client) return { Client };
  } catch { /* fall through to pnpm scan */ }

  // 2. pnpm store scan
  const pnpmDir = join(ROOT, 'node_modules', '.pnpm');
  if (existsSync(pnpmDir)) {
    const pgDirs = readdirSync(pnpmDir).filter((d) => /^pg@\d/.test(d)).sort();
    for (const d of pgDirs) {
      const entry = join(pnpmDir, d, 'node_modules', 'pg', 'lib', 'index.js');
      if (existsSync(entry)) {
        const m = await import(pathToFileURL(entry).href);
        const Client = m.default?.Client || m.Client;
        if (Client) return { Client };
      }
    }
  }

  throw new Error(
    'could not resolve `pg`. Run from the repo root after `pnpm install`, ' +
    'or ensure node_modules/.pnpm/pg@*/node_modules/pg exists.',
  );
}

// Redact the password from a postgres URL for safe logging.
export function redact(url) {
  if (!url) return '(unset)';
  try {
    const u = new URL(url);
    if (u.password) u.password = '***';
    return u.toString();
  } catch {
    // Not a parseable URL — blunt-redact anything after the last ':' before '@'.
    return url.replace(/:\/\/([^:@/]+):[^@]*@/, '://$1:***@');
  }
}

// Classify a connection failure into a store-actionable bucket.
export function classifyPgError(err) {
  const msg = (err?.message || String(err)).toLowerCase();
  const code = err?.code || '';
  // SSL / TLS
  if (
    code === 'ESSLREQUIRED' ||
    /self-signed|self signed|certificate|ssl|tls|verify-full|depth_zero|cert/.test(msg)
  ) return 'SSL';
  // auth (bad password / role missing / db missing)
  if (
    code === '28P01' || code === '28000' || code === '3D000' ||
    /password authentication failed|role .* does not exist|database .* does not exist/.test(msg)
  ) return 'AUTH';
  // host / network
  if (
    ['ENOTFOUND', 'ECONNREFUSED', 'ETIMEDOUT', 'EAI_AGAIN', 'EHOSTUNREACH'].includes(code) ||
    /getaddrinfo|connect etimedout|connection refused|timeout/.test(msg)
  ) return 'HOST';
  return 'OTHER';
}
