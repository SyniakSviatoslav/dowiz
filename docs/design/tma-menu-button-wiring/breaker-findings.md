# Breaker Findings — TMA menu-button wiring

**Role:** System Breaker (adversarial). Zero fixes — only where it breaks + which invariant.
**Target:** `docs/design/tma-menu-button-wiring/proposal.md` + `docs/adr/ADR-tma-menu-button-wiring.md`
**Verified against live tree** (telegram-webhook.ts, packages/config, migrations, deployment-plan.md, server.ts).
**Verdict:** No CRITICAL. 1× HIGH, 3× MEDIUM, 3× LOW. The single-file/rare-event framing is largely sound; the breaks are in (a) the host-resolution correctness claim, (b) the failure-recovery narrative, and (c) an under-validated new runtime surface.

---

## [HIGH] B-OPS / config · `APP_BASE_URL` empirically resolves to the WRONG (API) host — §4.3 correctness claim is falsified by the repo's own config

**Claim under attack.** Proposal §4.3 / ADR Decision 4(c): "`APP_BASE_URL` вже `required url` … **вже коректний per-environment** (staging→staging-storefront, prod→prod-storefront), автоматично." R3 rates the wrong-host risk merely **accept-risk**.

**Break (demonstrable, from live config).**
- `docs/deployment-plan.md:51` sets `APP_BASE_URL="https://api.dowiz.org"` — the **API** host, not the storefront.
- `apps/api/src/routes/spa-proxy.ts:155` independently hardcodes a **third** value: `process.env.APP_BASE_URL || 'https://dowiz.fly.dev'` (canonical public storefront per all memory notes: `/s/artepasta`, `/s/demo` live on `dowiz.fly.dev`).
- Audit history: `docs/audit/issues-matrix-2026-06-13.md` S5/P2 — `APP_BASE_URL` was set to `staging.dowiz.app` **on production Fly.io**, caused real image-URL bugs, and the recommended remediation was to **unset it**.

So `buildMiniAppUrl` → `${APP_BASE_URL}/s/${slug}?ch=telegram-tma` on the documented prod config yields `https://api.dowiz.org/s/:slug?ch=telegram-tma` — the API host, not the branded customer storefront `dowiz.fly.dev`. The system has THREE live notions of the base URL (config-required / deploy-plan `api.dowiz.org` / spa-proxy fallback `dowiz.fly.dev`); the design picks `env.APP_BASE_URL` assuming it equals the storefront host, which the repo's own artifacts contradict.

**Invariant violated.** §8 "Кнопка веде на **власний** storefront власника" — on the actual prod env value it opens the API host (unbranded, and possibly a non-SSR/404 surface). §4.3's "source-from-config ⇒ automatically correct host" is empirically false here.

**Why HIGH not accept-risk.** The TMA-VALIDATION manual gate (R3) is therefore **load-bearing**, not optional: the *default/likely* state of `APP_BASE_URL` in this repo is the wrong host, with a documented prior incident of it being misconfigured on prod. This must be a **blocking precondition** to flip, not "accept-risk."

---

## [MEDIUM] B-FAIL / B-CONSIST · The advertised "retry on next `/start`" recovery does NOT exist — the connect token is single-use

**Claim under attack.** §6 "«Eventually»-встановиться на наступному /start"; §7 "наступний /start повторить"; Опція A "(+) ідемпотентність безкоштовна: повторний /start = той самий стан."

**Break (from code).** In `apps/api/src/routes/telegram-webhook.ts`:
- Line 589-594: on success the connect token is consumed — `UPDATE telegram_connect_tokens SET used_at = now() …`.
- Line 558-570: a **second** `/start <same-token>` matches `used_at IS NULL` = false → `tokenRes.rows.length === 0` → sends `start.token_invalid_or_expired` and `return`s — it **never reaches** the `setChatMenuButton` code that lives after line 596.

So if `setChatMenuButton` transiently fails (429/timeout/5xx), the "next /start will repeat it" claim is false for the same deep-link. Recovery requires the owner to go back to `/admin`, mint a **new** connect token, and re-scan — i.e. what the proposal calls "reconnect," conflated with "next /start." Worse: the owner got `start.connected` (success signal) and **no error**, so they have zero reason to reconnect → the button is **silently, permanently absent** for that owner until a manual full re-connect.

**Invariant violated.** "best-effort … eventually-consistent via next /start" (§6/§7) — the eventual-consistency mechanism as described is unreachable on the consumed token.

---

## [MEDIUM] B-FAIL / product · New runtime surface (Telegram in-app WebView) is unvalidated for anything beyond page-load; "money-path не торкається" gives false assurance

**Claim under attack.** Non-goals + §8: "Mini App = **той самий** /s/:slug у WebView; оплата лишається звичайним web-checkout. Money-path не торкається." DoD/TMA-VALIDATION only asserts "button opens the right `/s/:slug` SSR."

**Break.** The change introduces a **new hostile runtime** (Telegram in-app WebView), but validates only that the page loads. Telegram WebView is a known-problematic environment for exactly the storefront's non-render flows:
- External payment redirects (crypto/Plisio per ADR-0017, card) — navigating away from the initial WebView origin to a payment provider is restricted; a plain web page must call the Telegram SDK `openLink`/`openInvoice` to escape, which this storefront **deliberately does not do** (explicit non-goal: "storefront не читає Telegram-контекст / НЕ Telegram SDK").
- Google OAuth popup/redirect login (`GOOGLE_OAUTH_ENABLED`) — popup/redirect auth is fragile in in-app webviews.

Result: the proposal's own stated value "self-order/demo" (§Опції, R1) silently dies at checkout/login **only inside this surface** (works fine in a normal browser → hard to catch). The code path is unchanged, but "money-path untouched" is misleading: the *code* is untouched, the *environment it now runs in* is new and unverified. The DoD has **zero** coverage of a completed order or login inside the WebView.

**Invariant risk.** Failure-first / "кожен зовнішній виклик має fallback" is applied only to `setChatMenuButton`, not to the storefront flows the button now exposes in a new client. Verification gap, not just an open product question.

---

## [MEDIUM] B-SCALE / B-FAIL · Operational-pool client pinned across an unbounded (no-timeout) Telegram fetch — feeds a documented incident class

**Claim under attack.** §2 "+0 нових DB-конектів"; R2 accept-risk "не новий клас hang."

**Break (from code + numbers).**
- `callTelegramApi` (telegram-webhook.ts:727-739) uses raw `fetch` with **no AbortController / no timeout** (undici default = no request timeout).
- `handleMessage` checks out `client = await db.connect()` at line 493 and releases it **only** in the `finally` at line 722. The new `setChatMenuButton` runs inside that window (after line 596), plus the added `SELECT slug` — so the pooled operational connection is **held across two sequential no-timeout network calls** (`sendMessage` then `setChatMenuButton`) on `/start`.
- Pool size: operational pool max = `OPERATIONAL_POOL_SIZE` default **20** (packages/config/src/index.ts:88; `createOperationalPool`). `server.ts:258` explicitly documents "**the operational-pool starvation incident class**."

Back-of-envelope: if `api.telegram.org` blackholes (TCP hang), each in-flight `/start` pins 1 of 20 operational connections for the full TCP timeout (tens of seconds → minutes with no request timeout). `/start` is rare, so exhaustion probability is low — but the change **doubles** the external-IO window on the held client and lands squarely on a class the repo has already been bitten by. The proposal's `Promise.race(~5s)` hardening is "recommended, not required," so the default-shipped code pins the connection unboundedly.

**Invariant violated.** "рантайм мінімальний / нуль каскаду" — a hung best-effort side-effect holds a scarce operational connection and blocks the webhook 200 ack; R2 frames only "hang," under-weighting the **pinned-pool-connection** aspect.

---

## [LOW] B-SEC footgun · Omitting `chat_id` makes `setChatMenuButton` bot-GLOBAL (default button for every user of the single shared bot)

**Break.** The bot is a **single global** token — `process.env.TELEGRAM_BOT_TOKEN` (telegram-webhook.ts:728); all owners talk to the same bot. Telegram's `setChatMenuButton` treats `chat_id` as **optional**: omit it and it sets the **default** menu button for **all** users of the bot. A one-line omission of `chat_id` in `buildSetChatMenuButtonRequest` → owner A's storefront becomes the default menu button for every other owner (cross-tenant clobber). The design correctly passes `chat_id` (ON-happy DoD body), but the blast radius of a single-token-argument regression is **bot-global**, so the per-chat invariant deserves an explicit negative assertion, not only presence-in-happy-body.

**Invariant at stake.** "нуль cross-tenant" (§8) — held only by an easy-to-drop argument on a shared-bot global-state API.

---

## [LOW] B-OPS · Systemic failure is invisible — no owner error + no metric/health signal

**Break.** By design (§9) a failed button is swallowed to a `warn` log and "degraded state externally invisible — acceptable." Combined with the HIGH wrong-host finding: if `APP_BASE_URL` resolves to a host that 400s every `setChatMenuButton` (or Telegram changes the API), the feature is **100% broken but green** — every newly-connected owner silently lacks a button, the owner sees `start.connected` (success), and the only signal is unwatched `console.warn` lines. There is no counter/health probe distinguishing "0 buttons set because no connects" from "0 buttons set because every call 400s."

**Invariant at stake.** B-OPS "видимість падіння <1 хв / health розрізняє degraded vs down" — this path is intentionally invisible, which is defensible for a rare additive feature but compounds the wrong-host risk into a silent-total-failure mode.

---

## [LOW] doc-drift · R4 is stale — `TMA_ENABLED` is already in the EnvSchema

**Break.** R4 (and ADR Decision 2) assert `TMA_ENABLED` is "**НЕ присутній** у packages/config EnvSchema (grep = 0 збігів)" and must be added by the build-lane. It is **already present**: `packages/config/src/index.ts:50` `TMA_ENABLED: z.enum(['true','false']).default('false')` (with a full comment block at lines 44-50). No functional impact, but the risk table is inaccurate — the proposal was authored against an older tree, which erodes confidence that other "grep = 0 / current-state" assertions in the doc were re-verified against HEAD.

---

## Regression anchors (verified-correct — keep intact; NOT findings)

- **RLS SELECT-slug claim holds.** `locations` has `FORCE ROW LEVEL SECURITY` (migrations/1780310071220_core-identity.ts:85) **and** `public_select ON locations FOR SELECT USING (true)` with no `TO role` (migrations/1780338909301_public-locations-rls.ts:7). So `SELECT slug FROM locations WHERE id=$1` returns a row on the operational role without any `set_config('app.current_tenant', …)`. **Coupling to watch:** the `/start` branch (unlike `order.confirm` at telegram-webhook.ts:281) does **not** set the tenant GUC before this read — it relies entirely on `public_select`. If a future RLS-hardening migration replaces `public_select` with a tenant-scoped policy, this SELECT silently returns 0 rows → guard skips → button silently never set. The proposal already flags this in §8 + a DoD RLS check; keep that guardrail.
- **Inner try/catch is genuinely load-bearing.** `callTelegramApi` throws on non-2xx (line 735-737) and the outer catch (line 713) sends `botT(locale,'msg.error')`. So Decision 4(a) is correct: without an inner try/catch a `setChatMenuButton` 429 surfaces `msg.error` after `start.connected`. Ensure the ON-degraded DoD test (429 → no `msg.error`, webhook 200) actually ships as the guardrail — it is the only thing enforcing this prose rule.
- **OFF-parity holds byte-for-byte** — gate is a call-site `process.env.TMA_ENABLED === 'true'` read; when unset the branch is skipped (no SELECT, no outbound). (Minor: reads raw `process.env` rather than the validated `env` object, so the schema default at :50 is decorative for the runtime gate — same OFF result, noted for consistency only.)
