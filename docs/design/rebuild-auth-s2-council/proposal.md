# S2-AUTH Port — Council Packet · PROPOSAL

> **STATUS: DRAFT — NOT APPROVED.** This is the *description* input to the live Triadic
> Council (system-architect + system-breaker + counsel + human). No S2 auth code is ported
> to Rust until this packet is council-APPROVED and every quirk-register row (§10) and
> open-question (see `open-questions.md`) is resolved one by one. Docs only.

- **Lane:** R3 (complete-rebuild program) · **Surface:** S2 auth 🔴 (REBUILD-MAP §3 Phase B)
- **Date:** 2026-07-04 · **Source commit:** `fix/audit-remediation@ae9f5360` (working tree)
- **Contract SSOT:** `docs/design/rebuild-plan/openapi-contracts/openapi-s2-auth.yaml` (20 ops,
  authored-from-live-source, x-quirk annotated) + `CONVENTIONS.md`
- **Census SSOT:** `inventory/14-crosscutting-proofnet.md §4` (AUTH-01..12, gaps 1–5)
- **Governing ADRs:** ADR-0003 (dev-login fail-closed), ADR-0004 (owner-token revocation)
- **Parity oracle:** the 174-spec Playwright net — no behavior change is real unless a spec
  proves it (Mandatory Proof Rule). `traceability-s1-s2.csv` maps operationId → E2E.

---

## 1. Port objective and the load-bearing seam

Port the S2 auth surface to Rust/axum such that **during Phase-B overlap both stacks verify the
same RS256 tokens against the same keys and the same `kid`** (inventory 14 §4: "the strangler's
load-bearing seam"). The E2E net must stay green through cutover with an **empty `openapi-diff`**.

Non-negotiable seam facts (verified in live source, carry verbatim):

1. **RS256 only, ever.** Sign `jwt.ts:55-56`; verify pins `algorithms:['RS256']` AND throws again on
   any `protectedHeader.alg !== 'RS256'` (`jwt.ts:105-111`) — alg-confusion and `alg=none` both
   rejected. The security sweep (2026-07-02) rated this "airtight — red tools bounce off." **Port
   the double-check verbatim; do not collapse it to a single guard.**
2. **`kid`-selected key before verify.** `decodeProtectedHeader` picks the trusted key by header
   `kid` *before* the signature check (standard JWKS pattern), then the signature still gates
   acceptance (`jwt.ts:87-102`). Prod-kid → prod public key; dev-kid → dev public key **only when
   `NODE_ENV != production` AND a dev keypair is present** (`jwt.ts:91,96`).
3. **Claims are a strict discriminated union** on `role` (`shared-types/src/legacy.ts:163-174`), one
   variant each for owner/courier/customer, `.strict()` (no extra claims), base `sub/iat/exp/kid`.
   The Rust deserializer must be equally strict — an unknown claim or a missing required claim
   **must fail closed** (see §3, and the drift finding in §10 row Q-DEAD).
4. **Zero cookies.** The API sets no cookie anywhere (`grep -rn cookie apps/api/src` → only redaction
   lists). Session state is client-side `localStorage` (`dos_access_token`/`dos_refresh_token`). See §9.

---

## 2. Operations in scope (and what is deliberately out)

The S2 YAML documents **20 operations**. In scope for this port:

| AUTH-xx | Operation(s) | Audience minted |
|---|---|---|
| AUTH-01 | `POST /api/auth/local/login` (argon2id) | owner |
| AUTH-02 | `GET /api/auth/google` · `/google/callback` · `POST /api/auth/exchange` | owner |
| AUTH-03 | `POST /api/auth/telegram/start` · `GET /api/auth/telegram/poll` | owner |
| AUTH-09 | `POST /api/auth/refresh` · `POST /api/auth/logout` (ADR-0004) | owner |
| AUTH-05 | `GET /api/courier/auth/invites/:id` · `POST …/redeem` | courier |
| AUTH-06 | `POST /api/courier/auth/login` · `/refresh` · `/logout` | courier |
| AUTH-GAP-2 | `POST /api/auth/courier/activate` (**DEAD** — retire/port decision) | courier (broken) |
| AUTH-07 | `POST /api/claim/accept` · `/request` · `/decline` (web side) | (transfers, no mint) |
| AUTH-10 | `POST /api/customer/track/exchange` | customer |
| AUTH-08 | `POST /dev/mock-auth` + `/api/dev/mock-auth` alias (dev-kid) | owner/courier (dev) |

**Explicitly OUT of S2 (do not port here):**
- **AUTH-04 customer OTP** — order-adjacent, ships with **S5** (orders/money). Port dark, same
  kill switch (`OTP_ENABLED` default false; send path is a `console.log` scaffold).
- **AUTH-11 platform-admin** — not a JWT flow; a structural `platform_admins` allowlist re-read
  every request. Ports as an axum layer on the `/api/admin` router (S10 platform-admin). Named here
  because the S2 extractor must not accidentally shadow it.
- **AUTH-12 WS auth** — ships with **S6** (realtime WS). The customer-JWT-mint↔WS-authz linkage
  (REBUILD-MAP §7 open item 1) is folded into the S6/WS-authz council packet, not this one.

---

## 3. The axum claims-extractor: type-state design

Replace the Fastify `verifyAuth` decorator + `requireRole` + `requireLocationAccess` tower with a
**type-state extractor family** so that "this handler requires an active owner for location L" is a
*type*, checked by the compiler, not a runtime convention a handler can forget (this is the Rust
answer to the sweep's "identity-split × RLS-reliance" structural root — see `threat-model.md`).

Proposed shape (design intent, not code):

- `Claims<Unverified>` — raw bearer, only `kid`-selected + RS256-verified + strict-parsed. Produced
  by the extractor; carries the discriminated `Role` enum (`Owner|Courier|Customer`).
- `Claims<Owner>` / `Claims<Courier>` / `Claims<Customer>` — role-narrowed by `FromRequestParts`
  extractors; a handler that binds `Claims<Owner>` is *unconstructible* from a courier token. This
  kills the class where a handler reads `claims.userId` on a non-owner token (see AUTH-09 logout
  quirk, §10 row Q-LOGOUT).
- `OwnerAt<Loc>` — an owner extractor that has additionally performed the **ADR-0004 P-d per-route
  `status='active'` membership re-read** for the target location. Owner-*write* routes bind this;
  the re-read is confined to write surfaces (not the global hot path — ADR-0004 keeps the hot path
  pure signature+exp). This is where the sweep's finding #6 (revocation not enforced at the hook)
  is structurally closed instead of relying on each handler to remember.
- `CourierSession` — a courier extractor that has performed the live `courier_sessions` bind check
  (`jti` → session row, `status='active'`), which today lives inside `plugins/auth.ts:63-83`. Carry
  the per-request bind verbatim (the sweep confirmed courier revocation is immediate and sound).

**Global Bearer-presence pre-gate parity:** the pre-route gate (`server.ts:399-427`) that returns
bare `401 {error:'Unauthorized'}` for `AUTH_PREFIXES` minus `NO_AUTH_PATHS` minus the OTP regex
becomes an axum middleware layer that runs *before* the extractor, preserving the divergent shape
(`GlobalBearerGate401`, §10 row Q-BEARERGATE). Middleware order is load-bearing — see §11.

---

## 4. RS256 verify + kid selection (parity with `jwt.ts`)

Port `verifyAuthToken` (`jwt.ts:82-115`) as a single verifier module (Rust: `jsonwebtoken` or
`jose`-equivalent). Requirements, each a red→green test vector at council DoD:

- **Two-key registry keyed by `kid`.** Prod `(JWT_KID, JWT_PUBLIC_KEY)`; dev `(JWT_DEV_KID,
  JWT_DEV_PUBLIC_KEY)`. Unknown `kid` → reject ("Invalid Key ID").
- **Dev-kid acceptance gated by build + runtime.** Rust: `#[cfg(feature="dev-routes")]` compiles the
  dev branch out of release, AND a runtime guard equivalent to `NODE_ENV != production && dev keypair
  present` (defence in depth — ADR-0003 keeps *both*). A prod binary must be structurally incapable
  of accepting a dev-kid token.
- **Alg pinned twice** (see §1.1).
- **Strict claim parse after signature.** `AuthToken.parse` equivalent — deny unknown/missing claims.
- **PEM `\n`-unescaping** parity (`jwt.ts:16`) — keys arrive with literal `\n`.

Keys are shared across stacks during overlap. **No key material appears in this packet.** During the
audit of live source no secret values were read; env *names* only. (Note for the human: the
2026-07-02 sweep flags `PLISIO_SECRET_KEY` and `PROVISION_OPS_SECRET` as secrets read via raw
`process.env` outside the schema/compliance gate — S2 does not touch them, but they are named in
inventory 14 §2 for the config-struct lane. No S2 secret was found in git during this drafting.)

---

## 5. Token mint parity — the audiences and the TTL matrix

Five mint audiences, each a distinct claims shape. **Carry the exact TTLs verbatim** (parity oracle
sees identical `exp`); the TTL *inconsistencies* are AUTH-GAP-3, a council unify-vs-carry decision
(see `open-questions.md` Q3), **not** a silent port fix.

| Audience | Mint site (live) | Access TTL | Refresh / session | Claims (strict) |
|---|---|---|---|---|
| Owner (argon2) | `auth/local.ts:146` | **24h** (ADR-0004 P-a) | opaque 64-hex, sha256-hashed, `auth_refresh_tokens`, **7d** | `{role:owner, userId, sub=userId, activeLocationId?}` |
| Owner (dev bypass) | `auth/local.ts:69` | **7d** dev-kid (ADR-0004 R2-3 — kept 7d, no refresh) | none | same, dev-signed |
| Owner (OAuth) | `auth.ts:149` | 24h | 7d family + one-time opaque code (Redis 60s) | `{role:owner, userId}` (+sub via signer) |
| Owner (Telegram) | `auth.ts:224` | 24h | 7d family | `{role:owner, userId}` |
| Owner (refresh re-mint) | `auth.ts:307` | 24h | new 7d in same family | `refreshedOwnerClaims` |
| Courier (redeem) | `courier/auth.ts:136` | **14d** (AUTH-GAP-3) | `courier_sessions` **30d** | `{role:courier, activeLocationId!, jti=sessionId}` |
| Courier (login/refresh) | `courier/auth.ts` | **24h** | 30d session | `{role:courier, activeLocationId!, jti}` |
| Customer (track) | `jwt.ts:117-132` | **7d** | grant reusable to expiry (14d) | `{role:customer, orderId, locationId, sub=customerId}` — **NO phone claim (P0-PII)** |
| Dev mock-auth | `dev/mock-auth.ts` | **1d** dev-kid | none | owner/courier under dev kid |

Notes for parity:
- Owner mint sites vary in whether `sub` is set explicitly (`local.ts:142` sets `sub`; OAuth/Telegram
  rely on the signer filling `sub = userId`, `jwt.ts:49`). Net result is identical (`sub === userId`
  on every owner token). The Rust signer must reproduce "sub defaults to userId" to keep byte parity.
- Customer token carries `sub = customerId` and **must never carry phone** (`jwt.ts:122-125`,
  P0-PII). This is a 🔴 privacy invariant — red→green test that a minted customer token has no phone
  claim.
- `refresh_token` is **conditionally omitted** on the argon2 path when the `auth_refresh_tokens`
  INSERT fails (`local.ts:149-165`) and **always** absent on the dev path (`local.ts:70`). Carry
  verbatim (§10 row Q-REFRESH-OMIT).

---

## 6. Revocation semantics — ADR-0004 parity (verified present in live source)

ADR-0004 is **implemented** in the current tree (verified): access TTL is 24h everywhere except the
dev path, logout exists, refresh re-derives role, first-login families are 7d (F-N applied —
`auth.ts:155,229` both `interval '7 days'`, no residual 30d). Port each behavior with its own test
vector (inventory 14 AUTH-09: "needs a dedicated Rust test-vector set").

**Owner refresh rotation (`/api/auth/refresh`, `auth.ts:235-318`) — the subtle one:**
1. **Atomic single-use claim** — `UPDATE … SET used=true WHERE id=$1 AND used=false RETURNING id`;
   exactly one winner (`auth.ts:265-268`). Rust: same guarded UPDATE, `rowCount==1` semantics.
2. **Lost claim + a family token created `< 5 seconds` ago** = benign concurrent refresh → soft
   **409 `{error:'concurrent_refresh'}`** (lowercase, non-envelope), family **NOT** revoked
   (`auth.ts:276-282`). ⚠ The SQL window is `interval '5 seconds'` (`auth.ts:277`) but the code
   *comment* says "last 10s" (`auth.ts:274`) — an internal doc inconsistency; **the SQL (5s) is
   authority**, the YAML documents 5s. Port 5s; fix the stale comment in the Rust port (cosmetic).
3. **Lost claim + no recent rotation** = genuine replay → `DELETE` whole family, **401 UNAUTHORIZED**
   "Token reuse detected. Family revoked." (`auth.ts:283-286`).
4. **ADR-0004 P-c live role re-derive** — role re-read from `memberships WHERE role='owner' AND
   status='active'` **every refresh**; none → **401 OWNER_REVOKED**, token already consumed
   (`auth.ts:293-301`). Never re-mint owner for a demoted user.
5. **`activeLocationId` preserved** iff still an active owner membership, else deterministic
   `ORDER BY created_at, location_id` first-pick (`auth.ts:302-306`) — never the tiebreaker-less
   `LIMIT 1` that silently swaps a multi-location owner's tenant.

**Logout (`/api/auth/logout`, `auth.ts:325-333`, ADR-0004 P-b):** `verifyAuth`-gated; user-wide
`DELETE FROM auth_refresh_tokens WHERE user_id = $1`; **204**. Access token stays valid ≤24h
(accepted risk). Reads `claims.userId` → present only on owner tokens (§10 row Q-LOGOUT).

**Courier refresh (`courier/auth.ts:354-476`):** lookup by `sessionId` prefix, argon2-verify the
secret half, `FOR UPDATE NOWAIT`; reuse of a revoked session → revoke **family** + audit + 401
`REFRESH_REUSED` (committed). ⚠ **Carried gap:** no ADR-0004-style per-location membership re-check —
only `couriers.status != 'active'` is re-checked. This asymmetry vs owners is AUTH-GAP-adjacent and a
council row (`open-questions.md` Q6).

**P-d per-route status re-check (owner-write surfaces):** the sweep (finding #6) shows this is *not*
enforced at the global hook and `spa-proxy.ts:66` trusts the baked `activeLocationId`. In Rust the
`OwnerAt<Loc>` extractor (§3) makes the re-read structural on write routes. The spa-proxy authn path
dissolves when Astro takes the shell, but its *authz decisions must be re-homed* to the extractor.

---

## 7. Session TTL & storage table (carry verbatim; unify only via council)

| Token/row | TTL | Store | Revocation trigger |
|---|---|---|---|
| Owner access | 24h (dev 7d, mock 1d) | client localStorage | expiry only (≤24h accepted risk) |
| Owner refresh family | 7d | `auth_refresh_tokens` (sha256 hash) | single-use rotate; reuse → family DELETE; logout → user-wide DELETE |
| Courier access JWT | 24h (redeem 14d — GAP-3) | client localStorage | `jti`→session live bind per request |
| Courier session | 30d | `courier_sessions` | `revoked_at`; reuse → family revoke |
| Customer JWT | 7d | client localStorage | expiry only |
| Customer track grant | 14d | `customer_track_grants` (sha256) | expiry only; **reusable** (`use_count` = observability) |
| OAuth handoff code | 60s | Redis (→ Pg/in-proc per A19) | one-shot GET+DEL |
| Telegram login token | 5min | `telegram_login_tokens` | single-use atomic flip authenticated→consumed |
| OAuth state/nonce/PKCE | 600s | Redis (→ Pg/in-proc per A19) | single-use GET+DEL |

---

## 8. dev-login PERMANENTLY EXCLUDED as a prod path (ADR-0003)

Dev auth is the **staging E2E backbone** (~80 specs depend on `/dev/mock-auth` and the local-login
dev bypass) and must be ported — but **cryptographically incapable of authenticating on prod**. Port
**all four ADR-0003 layers** (do not drop any):

1. **Flag + secret** — `ALLOW_DEV_LOGIN === 'true'` AND `x-dev-auth-secret` timing-safe ==
   `DEV_AUTH_SECRET` (`plugins/dev-guard.ts:29-31`).
2. **Path-404 existence-hiding** — global pre-route gate 404s dev paths when the gate is closed
   (`server.ts:405-427`), bare `{error:'Not found'}` (never 401 — no existence leak).
3. **Dev-kid segregation** — dev tokens signed under `JWT_DEV_KID` + dev keypair; the prod verifier
   holds no dev public key and (on prod) `acceptDevKid` short-circuits false (`jwt.ts:73-115`). A
   dev-kid-with-prod-key token is *unrepresentable* (kid+key passed as a pair, `jwt.ts:48-60`).
   `signDevToken` **throws** if the dev keypair is absent — a mint site can never silently fall back
   to prod-key signing (`jwt.ts:76-78`).
4. **Boot fail-fast** — prod boot FATAL-exits if any dev var is set (`config/index.ts:230-244`).

**Rust:** `#[cfg(feature="dev-routes")]` compiles the dev handlers **out of the release binary**
entirely, AND the runtime gate + dev-kid verifier gate remain (belt and suspenders). The two mint
duplicates (`/dev/mock-auth` + inline `server.ts:549` `/api/dev/mock-auth`) **collapse to ONE handler
registered at both paths** (§10 row Q-DUP); the alias contract entry exists only so `openapi-diff`
proves both paths still answer.

**Residual (ADR-0003 R-6, carry into threat model):** the *already-minted* leaked `kid:1` prod-token
from the historic incident is killable **only by operator key rotation** — this design prevents
recurrence but cannot reject a pre-existing leaked prod-kid token. Key rotation stays an operator
action, tracked in `threat-model.md` AR-6.

---

## 9. Transport: header-only, zero cookies (AUTH-GAP-5 deferred)

- Bearer `Authorization: Bearer <jwt>`; **no `Set-Cookie` anywhere.** Session in client
  `localStorage` (`dos_access_token`/`dos_refresh_token`), silent-refresh trigger
  `apiClient.ts:26,155`, cross-tab Web-Locks single-flight (`dos-token-refresh`, `apiClient.ts:52-66`
  — pairs with AUTH-09 to avoid racing tabs tripping reuse-detection).
- URL-carriage is **opaque codes in the fragment only**, never the JWT: OAuth handoff `#code=<uuid>`
  (`auth.ts:166`), claim token `#token=` (`ClaimPage.tsx:144`), track `?t=` opaque grant. Fragments
  are chosen anti-Referer-leak. The Rust port keeps fragment carriage (see §10 rows Q-FRAG-*).
- **AUTH-GAP-5** (tokens XSS-exfiltrable from localStorage; httpOnly-cookie redesign) is an explicit
  council decision at *this* port — **not** a port default. Recommendation & options in
  `open-questions.md` Q5 and the accepted-risk row in `threat-model.md`.

---

## 10. Quirk register — carry-vs-fix for every `x-quirk`

**Default = CARRY-VERBATIM** (the parity oracle and today's FE branch on the exact shape).
**FIX-IN-PORT only for 🔴 security-correctness, with council + an explicit E2E delta.** Every
divergent-shape row is *also* a keep-verbatim-vs-migrate-with-FE-lockstep decision that CONVENTIONS.md
defers to post-Astro; this table records the recommendation, the council ratifies.

| ID | Quirk (source) | Recommendation |
|---|---|---|
| Q-REFRESH-OMIT | `refresh_token` omitted when INSERT fails / dev path (`local.ts:70,149-165`) | **CARRY** — client tolerates no-refresh |
| Q-DOC-1H | doc-comment "1h" vs code signs "24h" (`local.ts:26` vs `:146`) | **FIX (cosmetic)** — correct comment in port; behavior 24h unchanged |
| Q-ROLE-DEGRADE | role silently degrades to `customer` on resolver error (`local.ts:136-139`) | **CARRY** — fail-safe (grants *less*); add test vector |
| Q-GOOGLE-IDTOK | Google `id_token` signature NOT verified — trusted via direct-TLS fetch (`auth.ts:105-106`) | **CARRY** (documented rationale). Council eye: consider verifying against Google JWKS as a FIX-with-delta (Q-OQ) |
| Q-OAUTH-ERRSHAPE | all callback failures → `VALIDATION_FAILED` w/ differing messages | **CARRY** |
| Q-FRAG-CODE | OAuth handoff via `#code=` fragment | **CARRY** (anti-Referer-leak, load-bearing) |
| Q-TG-BOTENV | `botUsername` from raw `process.env.TELEGRAM_BOT_USERNAME` (schema-drift var) | **CARRY behavior**; declare the var in the Rust config struct (kills drift) |
| Q-TG-POLLSHAPE | telegram-poll non-envelope `{status:'unknown'|'expired'|'consumed'}` on 404/410 | **CARRY** — FE poll branches on `status`, not envelope `code` |
| Q-CONCURRENT | refresh soft-409 `{error:'concurrent_refresh'}` lowercase non-envelope | **CARRY** — FE single-flight depends on it |
| Q-5S-COMMENT | refresh window SQL 5s vs comment "10s" (`auth.ts:274,277`) | **FIX (cosmetic)** — port 5s (SQL authority), fix comment |
| Q-LOGOUT | logout reads `claims.userId` → non-owner bearer 401 (`auth.ts:329`) | **CARRY** — made structural by `Claims<Owner>` extractor (§3) |
| Q-DEAD | `/auth/courier/activate` dead: 7d TTL, courier-refresh-in-owner-table, role-confusion — **AND its minted courier token violates strict `CourierClaims` (no `activeLocationId`, stray `userId`) so it is unverifiable** (auth.ts:374 vs legacy.ts:165) | **FIX = RETIRE** (proof-of-deadness: 0 FE callers, 0 E2E). Delete-vs-port is a council row; recommend RETIRE. See Q2 |
| Q-REDEEM-PW | redeem `ON CONFLICT(email_hash) UPDATE password_hash` — re-redeem overwrites password | **CARRY**, council eye (invite-code = sufficient authority to reset pw?) |
| Q-COURIER-ZOD | courier-auth manual-Zod `400 {error:'Validation failed', details:<ZodError tree>}` | **CARRY** (3rd shape family; FE branches). Migrate-to-envelope = FE-lockstep council row |
| Q-COURIER-EMAIL | courier login `email` not format-checked; doubles as phone, one sha256 vs email_hash OR phone_hash | **CARRY** — behavior, not defect |
| Q-COURIER-NORECHECK | courier refresh has no per-location membership re-check (only status) | **Council row** — carry (parity) vs FIX-with-delta to mirror ADR-0004 P-c. See Q6 |
| Q-COURIER-LOGOUT | courier logout ALWAYS 200 `{success:true}`, ignores bad input | **CARRY** — logout must never error |
| Q-CLAIM-BARE | claim surface bare `{error:CODE}` on all responses (ClaimBareError) | **CARRY** — ClaimPage FE branches on `error`-as-code |
| Q-CLAIM-DECLINE | decline maps *any* ClaimError → 401 regardless of kind | **CARRY** |
| Q-FRAG-TOKEN | claim token via `#token=` fragment, stashed `dos_claim_token` | **CARRY** |
| Q-TRACK-SHAPE | track `410 {error:'TRACK_LINK_EXPIRED', message}` (error=CODE, inverse of envelope) | **CARRY** — FE branches |
| Q-TRACK-REUSE | track grant reusable to expiry (`use_count` = observability, not single-use) | **CARRY** — by design |
| Q-TRACK-POOL | track-exchange runs on BYPASSRLS operational pool with explicit WHERE | **CARRY**; B3 flip changes the assumption — see `threat-model.md` §B3 |
| Q-DEV-404 | dev-guard closed → bare `{error:'Not found'}` (existence-hiding) | **CARRY** (deliberate) |
| Q-DEV-409 | mock-auth synthetic-missing `{error:<prose>, code:'SYNTHETIC_COURIER_MISSING'}` (fields swapped) | **CARRY** |
| Q-DEV-RANDUID | mock-auth mints random `userId` for non-synthetic courier (no DB row) | **CARRY** (dev-only) |
| Q-DUP | duplicate mock-auth body (`mock-auth.ts:14` vs `server.ts:549`) | **FIX (structural, behavior-identical)** — one handler, two routes |
| Q-BEARERGATE | global pre-route bare `401 {error:'Unauthorized'}` (server.ts:417-426) | **CARRY** — as an axum layer before the extractor |
| Q-VAL-400 | validation is 400 not 422 (ADR-0010 code-preserving) | **CARRY** — ~10 E2E assert 400 |
| Q-ENVELOPE-LEGACY | envelope retains legacy `status`+`error` aliases | **CARRY** — FE reads `message||error`; drop only post-Astro re-audit |

FIX-IN-PORT rows are only Q-DOC-1H / Q-5S-COMMENT (cosmetic comments), Q-DUP (structural, identical
behavior), and **Q-DEAD (RETIRE, security-correctness)**. Everything else CARRIES; the shape-migration
rows are deferred to a post-Astro FE-lockstep pass.

---

## 11. Middleware tower ordering (port as an ordered stack — load-bearing)

Preserve the exact order (inventory 14 §4 "port as an ordered tower stack"):

1. authn decorators available (`server.ts:395`)
2. dev-path 404 gate (`server.ts:405-416`) — before anything can leak dev-route existence
3. `AUTH_PREFIXES` Bearer-presence pre-check minus `NO_AUTH_PATHS`/OTP regex (`server.ts:417-426`)
4. per-route extractor `Claims<Role>` (+ courier live session bind)
5. role narrowing (`requireRole` → the type-state narrows this away)
6. `requireLocationAccess` — owner branch does a live `status='active'` re-read and returns **404
   not 403** to avoid an existence leak (`plugins/auth.ts:117-159`) — carry the 404 verbatim
7. admin-plane gate — MUST run AFTER the extractor (S10; named for ordering only)

The 4 direct `verifyAuthToken` call sites in `spa-proxy.ts:62,100,113,127` are a parallel authn path
that dissolves when Astro takes the shell — but the **authz decisions must be re-homed** to the
extractor, not dropped.

---

## 12. Cutover DoD (per REBUILD-MAP §3, this surface)

E2E auth slice green (as-is specs, `traceability-s1-s2.csv` E2E column) · `openapi-diff` empty ·
AUTH invariant-cluster tests red→green (incl. ADR-0004 refresh vectors, dev-kid prod-rejection,
customer-token-has-no-phone, reuse→family-revoke) · map-coverage zero-diff for the auth namespace ·
**council sign-off + rollback plan** (flag flip back to Node behind the proxy). No 🔴 auth row builds
before this packet is APPROVED.
