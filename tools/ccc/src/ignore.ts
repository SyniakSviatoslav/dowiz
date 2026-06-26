/**
 * ccc ignore rules (ADR-0012 C1, B10 — the security core).
 *
 * `isIgnored(relPath)` is consulted by the walker BEFORE a file's bytes are ever read, so a
 * secret on disk (`.env`, a key, a `.gitignore`d file) is never opened — not even to filter it.
 * Two layers:
 *   1. HARD secret deny-list — always wins, independent of `.gitignore`. A missing/edited
 *      `.gitignore` can never expose `.env*`/keys to the indexer.
 *   2. Best-effort `.gitignore` glob match (defence in depth) + standard build/vendor skips.
 */
import { readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

// Layer 1 — real secrets, denied unconditionally (never indexed, never read).
const SECRET_DENY: RegExp[] = [
  /(^|\/)\.env($|\.)/, // .env, .env.local, .env.production … (but see SECRET_ALLOW for .env.example)
  /\.pem$/,
  /\.key$/,
  /\.p12$/,
  /\.pfx$/,
  /\.keystore$/,
  /(^|\/)id_(rsa|ed25519|ecdsa|dsa)/,
  /(^|\/)\.fly[^/]*token/i, // .fly-staging-token etc
  /(^|\/)credentials($|\.|\/)/i,
  /\.(crt|cer)$/,
  /secrets?\.(json|ya?ml|txt)$/i,
];
// A documentation example file is NOT a secret.
const SECRET_ALLOW: RegExp[] = [/(^|\/)\.env\.example$/];

// Standard directories never worth indexing (also keeps the index small + dist-free).
const DIR_SKIP = new Set([
  'node_modules',
  '.git',
  'dist',
  'build',
  'coverage',
  '.ccc',
  '.repowise',
  'playwright-report',
  'test-results',
  'artifacts',
  '.venv-paddle',
]);

/** True iff a real secret — the hard, gitignore-independent layer. */
export function isSecretPath(relPath: string): boolean {
  if (SECRET_ALLOW.some((re) => re.test(relPath))) return false;
  return SECRET_DENY.some((re) => re.test(relPath));
}

// Minimal `.gitignore` glob → RegExp. Covers the patterns this repo uses (dir/, *.ext, name,
// path/sub, leading-slash anchor). Negation (!) is handled by the caller's ordered evaluation.
function gitignoreToRegExp(pattern: string): { re: RegExp; negate: boolean } | null {
  let p = pattern.trim();
  if (!p || p.startsWith('#')) return null;
  const negate = p.startsWith('!');
  if (negate) p = p.slice(1);
  const anchored = p.startsWith('/');
  if (anchored) p = p.slice(1);
  const dirOnly = p.endsWith('/');
  if (dirOnly) p = p.slice(0, -1);
  // Escape regex specials, then translate glob * and ? (no extglob — good enough for .gitignore).
  const body = p
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*/g, '[^/]*')
    .replace(/\?/g, '[^/]');
  // Anchored patterns match from root; unanchored match any path segment boundary.
  const prefix = anchored ? '^' : '(^|/)';
  const suffix = dirOnly ? '(/|$)' : '($|/)';
  return { re: new RegExp(prefix + body + suffix), negate };
}

export interface IgnoreRules {
  /** Consulted BEFORE reading bytes. `relPath` is POSIX-relative to the index root. */
  isIgnored(relPath: string): boolean;
}

export function loadIgnore(root: string): IgnoreRules {
  const rules: { re: RegExp; negate: boolean }[] = [];
  const giPath = join(root, '.gitignore');
  if (existsSync(giPath)) {
    // Reading .gitignore itself is fine (it is not a secret); its CONTENTS are patterns.
    for (const line of readFileSync(giPath, 'utf8').split('\n')) {
      const r = gitignoreToRegExp(line);
      if (r) rules.push(r);
    }
  }
  return {
    isIgnored(relPath: string): boolean {
      // 1. Hard secret deny — always wins.
      if (isSecretPath(relPath)) return true;
      // 2. Standard dir skips (any segment).
      if (relPath.split('/').some((seg) => DIR_SKIP.has(seg))) return true;
      // 3. .gitignore (ordered; a later negation can re-include).
      let ignored = false;
      for (const { re, negate } of rules) {
        if (re.test(relPath)) ignored = !negate;
      }
      return ignored;
    },
  };
}
