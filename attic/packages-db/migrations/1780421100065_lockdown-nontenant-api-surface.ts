import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`

    -- ── STEP A1: Remove API roles from Data API perimeter ──
    -- These tables contain PII/auth-tokens/infra-state.
    -- App accesses them via operational/session pools with BYPASSRLS roles,
    -- never through Data API (anon/authenticated/service_role per ADR-006).

    REVOKE ALL PRIVILEGES ON TABLE public.users                 FROM anon, authenticated;
    REVOKE ALL PRIVILEGES ON TABLE public.ops_worker_heartbeat   FROM anon, authenticated;
    REVOKE ALL PRIVILEGES ON TABLE public.auth_refresh_tokens    FROM anon, authenticated;

    -- service_role: revoked after confirming app doesn't use it (ADR-006: custom Fastify pooler, not PostgREST)
    REVOKE ALL PRIVILEGES ON TABLE public.users                 FROM service_role;
    REVOKE ALL PRIVILEGES ON TABLE public.ops_worker_heartbeat   FROM service_role;
    REVOKE ALL PRIVILEGES ON TABLE public.auth_refresh_tokens    FROM service_role;

    -- ── STEP A2: Enable + Force RLS (deny-by-default, silences linter 0013) ──
    -- No permissive policies — app roles use BYPASSRLS, API roles already revoked.
    -- users/auth_refresh_tokens: accessed during unauthenticated auth flows
    --   (login/signup/refresh) — no app.user_id set, so policies would break auth.
    -- ops_worker_heartbeat: infrastructure table — worker heartbeat + /health read.

    ALTER TABLE public.users                  ENABLE ROW LEVEL SECURITY;
    ALTER TABLE public.users                  FORCE  ROW LEVEL SECURITY;
    ALTER TABLE public.ops_worker_heartbeat   ENABLE ROW LEVEL SECURITY;
    ALTER TABLE public.ops_worker_heartbeat   FORCE  ROW LEVEL SECURITY;
    ALTER TABLE public.auth_refresh_tokens    ENABLE ROW LEVEL SECURITY;
    ALTER TABLE public.auth_refresh_tokens    FORCE  ROW LEVEL SECURITY;

    -- ── STEP A3: Opt-in exposure for future objects ──
    -- Prevents new tables from auto-granting to Data API roles.
    -- Runs under the same role as migrations (session pool, BYPASSRLS).

    ALTER DEFAULT PRIVILEGES IN SCHEMA public
      REVOKE ALL ON TABLES    FROM anon, authenticated;

    ALTER DEFAULT PRIVILEGES IN SCHEMA public
      REVOKE ALL ON SEQUENCES FROM anon, authenticated;

    ALTER DEFAULT PRIVILEGES IN SCHEMA public
      REVOKE ALL ON FUNCTIONS FROM anon, authenticated;

    -- ── STEP A4: Revoke schema access from API roles ──
    -- In our architecture, public schema is accessed ONLY via
    -- operational/session pool roles (ADR-006). anon/authenticated
    -- never need USAGE on public.

    REVOKE USAGE ON SCHEMA public FROM anon, authenticated;

  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`

    -- Restore API role grants (reverse of A1)
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.users                 TO anon, authenticated;
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.ops_worker_heartbeat   TO anon, authenticated;
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.auth_refresh_tokens    TO anon, authenticated;

    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.users                 TO service_role;
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.ops_worker_heartbeat   TO service_role;
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE public.auth_refresh_tokens    TO service_role;

    -- Undo RLS
    ALTER TABLE public.users                  NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE public.users                  DISABLE ROW LEVEL SECURITY;
    ALTER TABLE public.ops_worker_heartbeat   NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE public.ops_worker_heartbeat   DISABLE ROW LEVEL SECURITY;
    ALTER TABLE public.auth_refresh_tokens    NO FORCE ROW LEVEL SECURITY;
    ALTER TABLE public.auth_refresh_tokens    DISABLE ROW LEVEL SECURITY;

    -- Restore schema access
    GRANT USAGE ON SCHEMA public TO anon, authenticated;

    -- Note: ALTER DEFAULT PRIVILEGES is not reversed here.
    -- Restoring auto-grants would require explicit GRANTs per-role.
    -- This is intentional: down is destructive and should not be used in prod.

  `);
}
