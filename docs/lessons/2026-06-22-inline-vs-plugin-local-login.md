---
TRIGGER: apps/api/src/routes/auth/**
CAUSE: >
  HISTORICAL (pre-6fdb6e2a): the live /api/auth/local/login was an INLINE
  handler in server.ts while routes/auth/local.ts registered /auth/local/login
  (no /api prefix) → 404. As of 6fdb6e2a this is CONSOLIDATED: the plugin
  routes/auth/local.ts IS the live handler, registered with the /api prefix
  (server.ts:580-581), and the inline handler was REMOVED (server.ts:877-878).
ACTION: >
  When editing the auth/local login handler in apps/api/src/routes/auth/** →
  the plugin routes/auth/local.ts is now the live handler (verify the
  fastify.register(localAuthRoutes, { prefix: '/api' }) at server.ts:580-581 is
  still present; there is NO inline handler anymore). Note there are TWO token
  paths in local.ts: the flag-gated dev bypass (signDevToken, inert on prod) and
  the real argon2 login — change BOTH when adjusting token TTL/claims. The signed
  token MUST carry activeLocationId (from an active owner membership), not just
  {role,userId,sub} — token-scoped menu/orders endpoints read it.
LINK: apps/api/src/routes/auth/local.ts (live handler; consolidated in 6fdb6e2a, was inline a3efed36)
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
