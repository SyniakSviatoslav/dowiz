// Bebop auth — Better Auth, self-hosted, optional, flag-gated.
//
// Design (RESEARCH §1.7 — zero-cloud determinism): auth is LOCAL and OPTIONAL. Bebop runs fully
// offline with no auth at all (the native single-user case). The Better Auth server only activates
// when BEBOP_SYNC=1 — i.e. the user WANTS multi-device sync, and runs it on THEIR machine/infra.
//
// No Supabase, no Fly, no third party. The adapter is pluggable: in-memory by default (for tests and
// ephemeral sync), or better-sqlite3 when BEBOP_DB points at a file. This is greenfield — Bebop had
// no auth before — so there is nothing to "rip out".

import { betterAuth } from 'better-auth';
import { memoryAdapter } from 'better-auth/adapters/memory';

export interface BebopAuthOptions {
  /** Base URL the sync server is reachable at (used for callbacks/cookies). */
  baseURL?: string;
  /** Secret for session signing. Falls back to a generated-per-process dev secret (NOT for prod). */
  secret?: string;
  /** When true, email+password auth is enabled (recommended for a self-hosted sync node). */
  emailAndPassword?: boolean;
  /** Optional path to a sqlite file; if omitted, an in-memory adapter is used. */
  dbFile?: string;
}

function resolveAdapter(dbFile?: string) {
  if (dbFile) {
    // Lazy require so the native module is only loaded when actually requested (no build in tests).
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const Database = require('better-sqlite3');
    const db = new Database(dbFile);
    // better-auth's sqlite adapter expects a `better-sqlite3` instance directly.
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const { betterSqlite3 } = require('better-auth/adapters/better-sqlite3');
    return betterSqlite3(db);
  }
  return memoryAdapter(
    // The memory adapter does not auto-create its model tables; seed the standard Better Auth
    // models so reads/writes during signup/login have a home. Production uses dbFile (sqlite).
    { user: [], session: [], account: [], verification: [] },
  );
}

export function createBebopAuth(opts: BebopAuthOptions = {}) {
  const secret = opts.secret ?? process.env.BEBOP_AUTH_SECRET ?? `dev-${Math.random().toString(36).slice(2)}`;
  return betterAuth({
    basePath: '/', // sync endpoints at root (e.g. /sign-up/email) — clean for a self-hosted CLI node
    baseURL: opts.baseURL ?? process.env.BEBOP_SYNC_URL,
    secret,
    database: resolveAdapter(opts.dbFile ?? process.env.BEBOP_DB),
    emailAndPassword: {
      enabled: opts.emailAndPassword ?? true,
      minPasswordLength: 12,
      autoSignIn: true,
    },
    session: {
      expiresIn: 60 * 60 * 24 * 7, // 7d
      updateAge: 60 * 60 * 24, // refresh daily
    },
    // Bebop is a CLI node: no social login by default. Add providers by passing them in opts if needed.
  });
}

export type BebopAuth = ReturnType<typeof createBebopAuth>;
