# B3 Phase-0 — Protect-Path Proposals (STAGED, not applied)

Status: **PROPOSAL — awaiting operator/DB-owner apply.** Date: 2026-07-03.
Author: AppSec (Phase-0 guardrail-lock lane).

> These two items land in **protect-paths** (`packages/db/**`, `tools/eslint-plugin-local/**`),
> so this lane does **not** apply them — it stages the exact diffs for the gated owner. Both are
> **dark / no-op on today's prod** by construction. They complement the four already-shipped,
> red→green-proven guardrails (argon2 lock, refresh W3 lock, `scripts/guardrail-no-set-cookie.mjs`,
> JWT alg/kid pin) — see `docs/regressions/REGRESSION-LEDGER.md`.

Source dispositions: `docs/design/b3-auth-hardening/resolution.md` §1 (P0-4), §6/W1/W4.

---

## 1. P0-4 — Two-mode BYPASSRLS boot-guard (`packages/db/src/index.ts`)

### Rationale
Today `createOperationalPool()`'s `on('connect')` guard rejects **only** literal `current_user =
'postgres'`. Under the deferred NOBYPASSRLS ramp the login role stays BYPASSRLS while a new
**enforcement** role `dowiz_app_rls` carries `rolbypassrls=false`. A `current_user`-based
`NOT rolbypassrls` check is therefore *dead during the ramp* (M1): it would either FATAL forever
or never observe the enforcement role. The two-mode guard asserts the **enforcement role's**
attribute via the `pg_roles` catalog, **gated on the role existing** — so it is a pure **no-op on
today's prod** (where `dowiz_app_rls` does not exist and the runtime logs in as
`deliveryos_api_user`), and only acquires teeth once the (deferred, operator-created) role lands.

This changes **no runtime auth behavior today**. It is an assert that goes FATAL only if, after the
enforcement role exists, it is accidentally granted BYPASSRLS (the exact silent regression B3 guards).

### Flag
New env var (add to `packages/config/src/index.ts` env schema, alongside `OPERATIONAL_POOL_SIZE`):

```ts
  // B3 P0-4: name of the RLS enforcement role whose rolbypassrls=false is asserted at pool
  // connect, gated on the role existing (no-op until the deferred role is created). Empty
  // string disables the catalog assertion entirely (today's prod default).
  RLS_ENFORCEMENT_ROLE: z.string().default('dowiz_app_rls'),
```

Default `'dowiz_app_rls'` is safe because the guard is **existence-gated** — a non-existent role
short-circuits to a no-op. Set to `''` to hard-disable.

### Exact diff — `packages/db/src/index.ts`
Replace the operational-pool `on('connect')` handler (currently lines ~32–39):

```ts
  // FX-9: statement_timeout for operational queries — kill slow queries fast
  // DB Role Guardrail: Prevent operational pool from connecting as superuser (which bypasses RLS)
  pool.on('connect', async (client) => {
    await client.query("SET statement_timeout = '10s'");
    const res = await client.query('SELECT current_user');
    if (res.rows[0].current_user === 'postgres') {
      client.release(true); // Destroy the connection
      throw new Error("SECURITY FAULT: Operational pool connected as 'postgres' superuser. This bypasses RLS. Use a dedicated restricted role.");
    }
  });
```

with:

```ts
  // FX-9: statement_timeout for operational queries — kill slow queries fast
  // DB Role Guardrail (B3 P0-4, two-mode): the operational pool must never run RLS-bypassing.
  pool.on('connect', async (client) => {
    await client.query("SET statement_timeout = '10s'");

    // Mode 1 (always): the literal superuser is a hard fault — it bypasses RLS unconditionally.
    const who = await client.query('SELECT current_user');
    if (who.rows[0].current_user === 'postgres') {
      client.release(true); // Destroy the connection
      throw new Error("SECURITY FAULT: Operational pool connected as 'postgres' superuser. This bypasses RLS. Use a dedicated restricted role.");
    }

    // Mode 2 (existence-gated, dark until the enforcement role exists): once the deferred
    // NOBYPASSRLS ramp role is created, assert it is genuinely NOBYPASSRLS. This is a NO-OP on
    // today's prod (the role does not exist → zero rows → skip). It only FATALs if a future
    // change accidentally re-grants BYPASSRLS to the enforcement role — the silent regression B3
    // exists to catch. Catalog-based (pg_roles), NOT current_user-based (M1: the login role
    // stays BYPASSRLS during the ramp, so a current_user check would be dead here).
    const enforcementRole = env.RLS_ENFORCEMENT_ROLE?.trim();
    if (enforcementRole) {
      const roleAttr = await client.query(
        'SELECT rolbypassrls FROM pg_roles WHERE rolname = $1',
        [enforcementRole],
      );
      if (roleAttr.rowCount && roleAttr.rows[0].rolbypassrls === true) {
        client.release(true);
        throw new Error(
          `SECURITY FAULT: RLS enforcement role '${enforcementRole}' has BYPASSRLS — it must be NOBYPASSRLS. ` +
          `A NOBYPASSRLS ramp role that bypasses RLS silently defeats the whole enforcement layer (B3 P0-4).`,
        );
      }
      // rowCount === 0 → role not yet created → dark no-op (today's prod). Intentional.
    }
  });
```

(`env` is already in module scope at the top of `packages/db/src/index.ts`: `const env = loadEnv();`.)

### Proof plan (for the gated applier)
1. **No-op on prod today:** with `dowiz_app_rls` absent, the catalog query returns 0 rows → connect
   succeeds unchanged. (Rehearse in a unit test with a fake client returning `{rowCount:0}` →
   `doesNotThrow`; matches the `boot-guard-prod.test.ts` pure-function pattern.)
2. **Red arm:** fake client returns `{rowCount:1, rows:[{rolbypassrls:true}]}` → guard throws
   `/BYPASSRLS/`. Green arm: `{rolbypassrls:false}` → no throw.
3. **Superuser arm unchanged:** `current_user='postgres'` still FATALs (Mode 1 preserved verbatim).

### `verify:rls` companion assertion (optional, same lane)
Add to the RLS verify script: assert `RLS_ENFORCEMENT_ROLE` (when it exists) has
`rolbypassrls=false` AND the membership grant `dowiz_app → dowiz_app_rls` exists — so a missing
grant (a hard-deny under enforcement, R-5) is caught pre-flip, not at ramp time.

---

## 2. Lint-time enforcement of the cookie-less posture (`tools/eslint-plugin-local`)

### Rationale
`scripts/guardrail-no-set-cookie.mjs` (shipped this lane, red→green proven) catches cookie-setting
at CI/pre-commit time by grepping source. A lint rule catches it **in-editor / on-save**, one step
earlier, with AST precision (no regex false-positive risk on redaction lists). Both layers are
cheap; the ESLint rule is defense-in-depth, not a replacement.

`tools/eslint-plugin-local` is a protect-path, so the rule is **staged here**, not applied.

### Exact diff — new rule in `tools/eslint-plugin-local/src/index.js`
Add to the `rules: { … }` object (peer of `no-hardcoded-string`):

```js
    'no-response-cookie': {
      meta: {
        type: 'problem',
        docs: { description: 'dowiz is bearer-only (no cookies) — forbid setting response cookies (B3 W4)' },
        schema: [],
      },
      create(context) {
        const filename = context.getFilename();
        // Allowlist: the guardrail script itself + log/PII redaction modules that NAME the
        // set-cookie header to STRIP it (anti-leak), never to set it.
        if (/guardrail-no-set-cookie|\/lib\/logger\.|\/lib\/sentry\./.test(filename)) return {};
        const COOKIE_SETTERS = new Set(['setCookie', 'cookie', 'clearCookie']);
        return {
          // reply.setCookie(...) / res.cookie(...) / reply.clearCookie(...)
          CallExpression(node) {
            const c = node.callee;
            if (c.type === 'MemberExpression' && c.property && COOKIE_SETTERS.has(c.property.name)) {
              const obj = c.object && (c.object.name || (c.object.property && c.object.property.name));
              if (/^(reply|res|response)$/.test(obj || '')) {
                context.report({ node, message: `response cookie set via .${c.property.name}() — dowiz is bearer-only (no cookies). Use Authorization: Bearer, or add an ADR + allowlist.` });
              }
            }
            // reply.header('Set-Cookie', …) / res.setHeader('Set-Cookie', …)
            if (c.type === 'MemberExpression' && /^(header|setHeader)$/.test(c.property && c.property.name)) {
              const arg0 = node.arguments[0];
              if (arg0 && arg0.type === 'Literal' && typeof arg0.value === 'string' && /^set-cookie$/i.test(arg0.value)) {
                context.report({ node, message: 'Set-Cookie response header — dowiz is bearer-only (no cookies).' });
              }
            }
          },
          // A literal { 'Set-Cookie': … } key in a response-headers object.
          Property(node) {
            const k = node.key;
            const kv = k && (k.value !== undefined ? k.value : k.name);
            if (typeof kv === 'string' && /^set-cookie$/i.test(kv)) {
              context.report({ node, message: "'Set-Cookie' response-header literal — dowiz is bearer-only (no cookies)." });
            }
          },
        };
      },
    },
```

Then enable it in the repo ESLint config (`eslint.config.*` / `.eslintrc*`) under the
`local/` plugin namespace: `'local/no-response-cookie': 'error'`.

### Proof plan (for the gated applier)
- **Green:** run ESLint over the current tree → 0 `no-response-cookie` reports (posture already clean;
  logger/sentry allowlisted).
- **Red:** add `reply.setCookie('s','x')` to any handler → lint errors; remove → clean. Mirrors the
  `scripts/guardrail-no-set-cookie.mjs` red→green already proven this lane.

---

## Summary for the gated owner
| Item | File (protect-path) | Today's effect | Teeth when |
|------|--------------------|----------------|-----------|
| P0-4 two-mode boot-guard | `packages/db/src/index.ts` + `RLS_ENFORCEMENT_ROLE` flag in `packages/config/src/index.ts` | **no-op** (role absent) | enforcement role exists + accidentally BYPASSRLS → FATAL |
| Cookie-less ESLint rule | `tools/eslint-plugin-local/src/index.js` + ESLint config | **no-op** (tree already cookie-less) | any `reply.setCookie`/Set-Cookie header added → lint error |

Neither changes runtime auth behavior. Both are asserts/locks, consistent with the Phase-0 mandate.
