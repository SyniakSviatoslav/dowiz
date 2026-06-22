---
TRIGGER: apps/api/src/routes/auth/**
CAUSE: >
  The active /api/auth/local/login handler is registered INLINE in
  apps/api/src/server.ts ("inline below for reliability"). The
  routes/auth/local.ts plugin registers /auth/local/login (no /api prefix)
  which 404s — so editing the plugin alone has ZERO runtime effect.
ACTION: >
  When editing the auth/local login handler in apps/api/src/routes/auth/** →
  cause: the live route is served inline in server.ts, not this plugin → do:
  grep server.ts for the inline /api/auth/local/login handler FIRST and fix
  THAT, or confirm the plugin is actually registered with the /api prefix
  before assuming your edit takes effect. Also: the signed token MUST carry
  activeLocationId (resolved from an active owner membership), not just
  {role,userId,sub} — token-scoped menu/orders endpoints read it.
LINK: apps/api/src/server.ts:868 (commit a3efed36; later consolidated in 6fdb6e2a)
SCOPE: apps/api/src/routes/auth/** AND the inline login handler in server.ts ONLY. Not other routes.
STATUS: active
---

# Inline vs plugin: the live local-login handler is in server.ts

Source: memory `local-login-active-handler.md`, `auth-import-fixes-2026-06-22.md`.

`POST /api/auth/local/login` (email/password dev login, e.g. `test@dowiz.com`)
was served by an inline handler in `apps/api/src/server.ts` while the
`routes/auth/local.ts` plugin registered `/auth/local/login` (no `/api`
prefix) → 404, effectively dead. A 2026-06-22 fix consolidated the flag-gated
dev bypass and the real argon2 path INTO `local.ts`, registered it with the
`/api` prefix, and removed the inline handler. Until you confirm that
consolidation is live in the tree you are editing, assume the inline handler
in `server.ts` is the one that actually runs.

Twin bug from the same area: the inline handler signed the token with only
`{role,userId,sub}`, so token-scoped endpoints saw no location and returned
empty. The signed token must include `activeLocationId`.
