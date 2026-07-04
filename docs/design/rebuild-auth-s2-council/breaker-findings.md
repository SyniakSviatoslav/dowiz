# S2-AUTH Port — Council Packet · BREAKER FINDINGS

> **Seat:** system-breaker (S2-auth Triadic Council). **Charge:** prove the Rust/axum port breaks.
> No fixes, no design (architect's job). Every finding is demonstrable — a concrete token/repro or a
> back-of-envelope number — with `file:line` evidence against the live Node ground truth
> (`fix/audit-remediation@ae9f5360`) and a mapped open-question / quirk-row.
>
> **Verdict up front: the port breaks.** Not at the crypto (RS256 double-pin is airtight and ports
> cleanly) but at the **strict-claim seam** and the **byte-compat seam (Q10)** — the two places the
> proposal itself names as load-bearing. 1 CRITICAL, 6 HIGH, 3 MED, 1 LOW. Two of these (F1, F3) are
> also **governance holes**: quirks with no register row, so the packet's own "carry-verbatim unless
> council-FIX-with-E2E-delta" rule *cannot be applied to them* — they will change silently in either
> direction. Read F1 first (Q10 cutover) and F3 (undocumented authz scope drift).

Legend: **[BREAK]** the exact failure · **[EVID]** file:line · **[REPRO]** exploit/repro · **[MAPS]** OQ/quirk.

---

## CRITICAL

### C1 · B-CONSIST / Q10 · `kid` is a REQUIRED BODY claim — an idiomatic Rust mint drops it → Node rejects Rust tokens (both directions), rollback loses every session

- **[BREAK]** The strict claims union requires `kid` **inside the JWT body**, not just the JOSE
  header. `AuthBase = { sub, iat, exp, kid: z.string() }` and every variant is `.strict()`
  (`packages/shared-types/src/legacy.ts:162-173`). The Node signer writes `kid` into the **body**:
  `signWith` builds `jwtPayload = { ...payload, sub, kid }` and *also* sets it in the protected
  header (`packages/platform/src/auth/jwt.ts:50-56`). So every live Node token carries `kid` twice
  (header + body), and `verifyAuthToken` → `AuthToken.parse(payload)` (`jwt.ts:114`) **rejects any
  token whose body omits `kid`**.
- **[REPRO]** The idiomatic Rust mint (`jsonwebtoken`, which the proposal §4 names) puts `kid` in
  `Header.kid` **only** — the body is the `Claims` struct. A porter reasoning "kid is a JOSE header
  field, the body copy is redundant" (it *is* functionally redundant — Node selects the key by the
  **header** kid at `jwt.ts:87,94`, and validates the body kid only as `z.string()`, never comparing
  them) drops it from the struct. Result:
  - **Rust→Node:** a Rust-minted access token or refresh-remint has no body `kid` → Node's still-live
    verifier (during Phase-B overlap, and every request that rolls back to Node) throws on
    `AuthToken.parse` → **401 mid-session**. Rollback (route back to Node, the entire Q10(a) safety
    net) leaves **every token minted during the Rust window unverifiable** → forced mass re-login,
    the exact opposite of "rollback leaves every live session valid."
  - **Node→Rust:** if the Rust `Claims` struct mirrors `.strict()` with `#[serde(deny_unknown_fields)]`
    and omits a `kid` field, then Node-minted tokens (which *do* carry body `kid`) are rejected by
    Rust as an **unknown field `kid`**. Both stacks reject each other's tokens.
- **[EVID]** `legacy.ts:162` (`kid` required in `AuthBase`); `jwt.ts:50-54` (kid written to body);
  `jwt.ts:114` (strict parse gates acceptance); `jwt.ts:87,94` (key selected by *header* kid, so the
  body kid is a pure parity-tax claim → maximally likely to be dropped).
- **[MAPS]** Q10 (🔴 live-session migration — "Rust-minted verifies on Node, Node-minted rotates on
  Rust"); the packet's DoD §12 says "shared-token/hash parity test" but does **not** enumerate a body-
  `kid` round-trip vector. This is the single sharpest threat to the cutover seam.

---

## HIGH

### H1 · B-SEC / T-3 · Strict-parse parity silently lost: serde `deny_unknown_fields` is IGNORED on an internally-tagged enum

- **[BREAK]** The claims union is a Zod `discriminatedUnion('role', …)` with `.strict()` per variant
  (`legacy.ts:163-173`) — extra claims are **rejected**. The natural Rust model of a `role`-tagged
  union is `#[serde(tag = "role")]` (internally-tagged enum). serde **documents that
  `deny_unknown_fields` cannot be used with internally-tagged (or adjacently-tagged) enums — it is
  silently ignored**, not an error. So a Rust port that models the union idiomatically will **accept
  extra claims that Node `.strict()` rejects**, and a happy-path round-trip test (valid claims only)
  passes green over the hole.
- **[REPRO]** Craft/emit a token `{role:'owner', userId, activeLocationId, sub, iat, exp, kid,
  x:'…'}`. Node: `AuthToken.parse` → `unrecognized_keys` → **reject** (`jwt.ts:114`). Rust internally-
  tagged enum: extra field buffered and dropped → **accept**. The strictness is a **live control** —
  it is *why* the dead activate token is unverifiable (see H2) and *why* claim-smuggling is blocked
  (`threat-model.md` T-3 names this verbatim). Note it does **not** resurrect the activate token
  (missing-*required* `activeLocationId` still fails regardless of unknown-field handling) — the
  regression is one-directional: **extra-claim tolerance**, a defense-in-depth strictness weakened
  with no red test to catch it.
- **[EVID]** `legacy.ts:163-173` (`.strict()` union); serde limitation is a documented crate
  behavior, not a Node artifact — the parity test must be authored to catch it.
- **[MAPS]** T-3; Q9 (claims-schema freeze). The DoD §12 must add an **extra-claim-rejection** vector
  per variant, or this ships silently weaker than Node.

### H2 · B-SEC / Q2 / Q-DEAD · The dead `/auth/courier/activate` token is unverifiable BY CONSTRUCTION — confirmed; carrying it (not retiring) is strictly more dangerous

- **[BREAK]** `activate` mints `{ role:'courier', userId }` at `auth.ts:374`. `signWith` then produces
  a body of `{ role:'courier', userId, sub:userId, kid, iat, exp }` — it has an **extra `userId`**
  and **no `activeLocationId`**. Strict `CourierClaims` **requires** `activeLocationId` and forbids
  `userId` (`legacy.ts:165`). So the token fails `AuthToken.parse` on **two** counts — the flow mints
  a credential its own verifier rejects. The refresh half is written to the **owner** table
  `auth_refresh_tokens` (`auth.ts:378-382`), so `/api/auth/refresh` would rotate it as `role:'owner'`
  (`auth.ts:307`) — a latent role-confusion path.
- **[REPRO]** grep-confirmed 0 FE callers / 0 E2E (matches the packet's proof-of-deadness). The break
  is the *carry* option: porting by inertia re-homes a live-reachable role-confusion mint into Rust
  with no consumer. If H1's strictness is *also* weakened, the extra `userId` stops being rejected —
  narrowing the two-count failure to one — so H1 and H2 compound.
- **[EVID]** `auth.ts:374` (mint), `auth.ts:378-382` (courier refresh in owner table),
  `legacy.ts:165` (CourierClaims requires activeLocationId, forbids userId).
- **[MAPS]** Q2 (🔴 RETIRE vs port), AR-2, Q-DEAD. **Confirms the R3 recommendation (RETIRE).**
  Port-blocking — do not settle any other row until this is a deliberate delete with the register row.

### H3 · B-SEC / B-CONSIST · Customer-token order-scope DRIFT — one token, two order-read paths, two different scopes; **NOT in the quirk register or threat model**

- **[BREAK]** The customer JWT `{role:'customer', orderId, locationId, sub=customerId}` is authorized
  by **inconsistent predicates across two live order paths**:
  - `GET /orders/:id` (`apps/api/src/routes/orders.ts:750-754`) binds to the **token's `orderId`
    claim**: `if (user.orderId !== id) 404`. → **per-order** scope (the intended tracking-link model).
  - `GET|POST /api/customer/orders/:orderId/{status,cancel,rating}`
    (`apps/api/src/routes/customer/orders.ts:50,237,284`) binds to **`customer_id = sub`**:
    `WHERE o.id = $1 AND o.customer_id = $2`, and **never reads the token's `orderId` claim**. →
    **customer-wide** scope.
- **[REPRO]** Customer (phone P @ location L → stable `customer_id X`, upserted `ON CONFLICT
  (location_id, phone)`) places order **A**, opens its tracking link `?t=grantA`, exchanges →
  `JWT_A {sub:X, orderId:A, locationId:L}`. Same customer later places order **B**. With `JWT_A`:
  - `POST /api/customer/orders/B/cancel` → handler matches `o.id=B AND customer_id=X` → **cancels B**
    (`customer/orders.ts:284-330`). The token was minted for order **A** only; its `orderId=A` claim
    is never checked.
  - `GET /orders/B` on the *other* path → `user.orderId(A) !== B` → **404** (`orders.ts:752`).
  Same token, same order B: **one path 403/404s, the other executes a destructive cancel.** A
  per-order tracking link (14d, reusable, carried in the `?t=` **query** — referer/history/log-
  exfiltrable, unlike the fragment-carried codes) therefore grants **read/cancel/rate over the whole
  customer's order history at L**, not the one shared order.
- **[GOVERNANCE HOLE]** This drift appears in **neither** the quirk register (`proposal.md §10`) **nor**
  the accepted-risk rows (`threat-model.md §3`). `Claims<Customer>` (proposal §3) is *role*-narrowed
  only — it says nothing about binding `orderId` to the route. When the port re-homes both paths to a
  single `Claims<Customer>` extractor it will **unify to one scope**, silently changing the other on a
  🔴 surface with **no council decision, no E2E delta, no register row** — a direct violation of the
  packet's own rule ("quirks never silently fixed"). Threat-model asset A4 ("Order status, address,
  contact reveal") *understates* the true blast radius (customer-wide orders).
- **[EVID]** `orders.ts:750-754` vs `customer/orders.ts:50,237,284`; token shape `legacy.ts:166-173`;
  grant is per-order (`lib/order-persistence.ts:145-148`, cited in the YAML).
- **[MAPS]** No existing row — **this is a missing quirk row and a missing AR row.** Council must add
  it and decide carry (customer-wide) vs FIX (bind orderId) *with* an E2E delta before any code.

### H4 · B-ANTIPATTERN / Q11 / Q-DUP · "Collapse the two mock-auth handlers" — the premise that they are behavior-identical is FALSE, and the named gate (openapi-diff) is blind to the divergence

- **[BREAK]** Q11/§8 assert the two mock-auth bodies are "behavior-identical" and that "openapi-diff
  proves both paths still answer." They are **not** identical — four concrete divergences between
  `/dev/mock-auth` (`routes/dev/mock-auth.ts`) and `/api/dev/mock-auth` (`server.ts:549+`):
  1. **`fresh:true` throwaway-owner mode** exists **only** in `server.ts:595-604` (mints a location-
     less owner for the onboarding-wizard E2E). Absent in `mock-auth.ts`.
  2. **`locationSlug` resolution** is honored **only** in `mock-auth.ts:98-104`. `server.ts` ignores it.
  3. **Owner auto-membership INSERT on `demo`** happens **only** in `server.ts:638-647`. `mock-auth.ts`
     does not create a membership.
  4. **Courier default location**: hardcoded UUID `1f609add-…` (`mock-auth.ts:58`) vs a live
     `SELECT id FROM locations WHERE slug='demo'` (`server.ts:586-587`).
- **[REPRO]** Collapse to one handler → you must pick one body. Pick `mock-auth.ts` semantics →
  every spec calling `/api/dev/mock-auth` with `fresh:true` (onboarding wizard) breaks. Pick
  `server.ts` semantics → every spec relying on `locationSlug` targeting breaks. **openapi-diff cannot
  catch it**: `fresh` is not in the YAML request body at all (the schema lists only
  `role/locationSlug/locationId/synthetic`, lines 889-894 / 954-959), and the auto-membership /
  default-location side effects are not contract-visible. ~80 staging E2E specs ride mock-auth
  (§8) — the "structural, behavior-identical" FIX is neither.
- **[EVID]** `server.ts:595-604` (fresh), `server.ts:638-647` (auto-membership), `server.ts:586-587`
  vs `mock-auth.ts:58` (courier default), `mock-auth.ts:98-104` (locationSlug); YAML body schema
  omits `fresh` (openapi-s2-auth.yaml:889-894, 954-959).
- **[MAPS]** Q11, Q-DUP. The collapse needs a **behavioral** diff gate (real E2E across BOTH paths),
  not openapi-diff, and a decision on which superset body to keep.

### H5 · B-SCALE / B-CONSIST / T-1,T-2 · Refresh benign-window: SQL says 5s, the comment says 10s — porting the wrong one, OR porting the value with the wrong clock, breaks in opposite directions

- **[BREAK]** The benign-concurrent-refresh window is SQL `interval '5 seconds'` (`auth.ts:277`) but
  the code **comment two lines up says "last 10s"** (`auth.ts:274`). Two distinct mis-ports:
  - **Wrong value (port 10s from the comment):** the window widens → a **genuine stale-token replay
    5–10s after a legitimate rotation is classified as benign 409 instead of triggering the family
    DELETE** (`auth.ts:280-285`). Reuse-detection develops a 5-second false-negative hole — exactly
    the T-1 replay window ADR-0004 exists to close.
  - **Right value, wrong clock:** if the port computes the window in **Rust app-time**
    (`Utc::now() - Duration::seconds(5)`) instead of leaving it in **SQL `now()`** (both the
    `created_at` default and the comparison are DB-clock today), then app-vs-DB clock skew + round-trip
    latency enters a **5-second security boundary**. Back-of-envelope: 200–500 ms of latency is
    4–10% of the window; a modest 1–3 s NTP drift **flips the classification** — a benign 2 s two-tab
    refresh computed as 5 s becomes "replay" → **family DELETE → self-DoS logout of all the user's
    devices** (T-2, the "expires too soon" regression the atomic claim was built to prevent).
- **[EVID]** `auth.ts:274` (comment "10s"), `auth.ts:277` (`interval '5 seconds'`, keyed on
  `family_id`, `created_at` from `now()`), `auth.ts:280-285` (409 vs family DELETE branch).
- **[MAPS]** Q-5S-COMMENT (marked "FIX cosmetic"), T-1, T-2. The register calls it *cosmetic* — it is
  not: it is a two-sided correctness boundary. Port DoD must pin **both** the value (5s) **and** the
  computation site (SQL `now()`, never app clock).

### H6 · B-SEC / T-7 · Dev-kid segregation gains a NEW failure mode (build flag) that Node lacks; if the runtime arm is collapsed to keypair-presence, staging→prod image promotion re-opens the ADR-0003 backdoor

- **[BREAK]** Node's prod defense has **three runtime arms, each independent**: `acceptDevKid =
  NODE_ENV !== 'production' && !!JWT_DEV_KID` (`jwt.ts:91`), prod holds no dev public key (`jwt.ts:37-40`
  return null), and boot fail-fast on any dev var (`config/index.ts:230-244`). The critical property:
  the `NODE_ENV !== 'production'` arm rejects dev-kid tokens **even when a dev keypair is present**.
  The Rust port replaces the primary gate with a **compile-time** `#[cfg(feature="dev-routes")]`. But
  the ~80-spec staging E2E backbone **requires that feature compiled IN** — so a `--features dev-routes`
  binary exists and is deployable. If that image is ever promoted to prod (this repo's own pattern:
  "deploying dark code to verify is fine"), the dev-kid **verify** branch is compiled in, and the
  *only* remaining gate is the runtime arm. If the port folds Node's two runtime arms into "dev keypair
  present" alone (dropping the NODE_ENV-independent check because "cfg already gates it"), then a prod
  env that **inherited `JWT_DEV_PUBLIC_KEY` via config drift** — the packet itself flags raw
  `process.env` schema-drift vars (`TELEGRAM_BOT_USERNAME`, Q-TG-BOTENV; PLISIO/PROVISION secrets, §4)
  — accepts dev-signed owner tokens. That is the ADR-0003 incident class verbatim (MEMORY: "LIVE prod
  owner-JWT backdoor").
- **[REPRO]** Build `--features dev-routes` (required for staging) → promote image to prod → set/leak
  `JWT_DEV_PUBLIC_KEY` in prod env → mint via any dev site → dev-kid token **verifies on prod** iff the
  runtime arm is keypair-presence rather than an env-mode check independent of the keypair.
- **[EVID]** `jwt.ts:91` (NODE_ENV arm, independent of keypair), `jwt.ts:37-40` (no dev pubkey → null),
  `config/index.ts:230-244` (boot fail-fast). Proposal §4 wording ("NODE_ENV != production && dev
  keypair present") is *correct* — the finding is that it is under-specified as a **testable gate**.
- **[MAPS]** Q (dev-kid segregation, port-blocking), T-7, AR-6. DoD §12 lists "dev-kid prod-rejection"
  but must specifically test the **dangerous build**: a `dev-routes`-compiled binary + `NODE_ENV=production`
  + a dev pubkey present in env still rejects a dev-kid token. Do not collapse the AND; keep cfg-flag
  strictly **additive**.

---

## MED

### M1 · B-COMPAT / Q10 · `jsonwebtoken` default 60 s leeway vs jose 0 → the two stacks verify the SAME token differently for a 60 s window

- **[BREAK]** Node's jose `jwtVerify` uses `clockTolerance = 0` (`jwt.ts:105-107` passes none). The
  idiomatic Rust `jsonwebtoken::Validation::default()` sets **`leeway = 60` seconds**. So for 60 s past
  `exp`, Rust **accepts** an access token that Node **rejects**. During Phase-B overlap the same token
  gets opposite verdicts on the two stacks — violating Q10's "both stacks verify the same tokens" — and
  it silently extends the ADR-0004 "≤24h leaked-owner-access" accepted-risk window by 60 s on the Rust
  arm.
- **[EVID]** `jwt.ts:105-107` (no clockTolerance); `jsonwebtoken` default leeway is a crate default.
- **[MAPS]** Q10; §7 residual-risk (≤24h window). Parity test must pin `leeway = 0`.

### M2 · B-FAIL / §3 · The proposed `CourierSession` extractor omits the jti-LESS dev pass-through → rejects every mock courier token → breaks the staging E2E backbone

- **[BREAK]** Node's courier bind (`plugins/auth.ts:62-70`) has a branch the proposal's §3
  `CourierSession` description does not carry: **a courier token with no `jti` is allowed through iff
  `devLoginAllowed(env)`** (dev/mock courier tokens carry no `jti` and have no `courier_sessions` row —
  `mock-auth.ts:45-49,61-65`). A faithful `CourierSession` extractor that *always* requires a
  `jti → courier_sessions` bind (as §3 states: "performed the live courier_sessions bind check") will
  **reject every mock courier token** → the ~80-spec staging courier E2E (AUTH-08) goes red — the
  parity oracle itself.
- **[EVID]** `plugins/auth.ts:63-70` (jti-less dev branch), `mock-auth.ts:45-49,61-65` (no jti minted),
  proposal §3 (CourierSession description).
- **[MAPS]** AUTH-08, §3. The extractor must carry the `jti.is_none() && dev_login_allowed → pass`
  branch, not just the bind.

### M3 · B-SEC / B3 · The extractor-seats-GUC design structurally cannot reach the PRE-AUTH mint paths → post-B3 the "identity-split × RLS-reliance" root stays open on exactly those paths

- **[BREAK]** The port's answer to fail-open RLS is GUC-seating inside the claims-extractor (§3, and
  `threat-model.md §6`: "set `app.user_id` on every connection that touches tenant tables — including
  the pre-auth mint paths"). But the pre-auth mint paths — `track/exchange` (`NO_AUTH_PATHS`,
  `server.ts:403`), courier redeem/login, claim — **never run the extractor** (they take no bearer), so
  an extractor-based GUC pattern **cannot seat their GUC by construction**. `track.ts:43-53` queries
  `customer_track_grants JOIN orders` on the operational pool with an explicit `WHERE` and **no
  `set_config('app.user_id', …)`**. Post-B3 (NOBYPASSRLS) with the fail-open NULL-context policy
  `USING(app_current_user() IS NULL)` (sweep #2, CRIT) table-wide TRUE on a GUC-less connection, these
  pre-auth queries run in a NULL identity context. Bounded **today** only by the explicit `WHERE` — so
  the proposal must **not** claim the extractor closes the identity-split root; it doesn't for the
  pre-auth surface, which is where most S2 operations live (TB-1).
- **[EVID]** `track.ts:43-53` (no GUC), `server.ts:403` (track in NO_AUTH_PATHS), proposal §3 / §11
  (extractor runs only on guarded routes), `threat-model.md §6`.
- **[MAPS]** Q-TRACK-POOL, threat-model §6 (B3-council-owned). The S2 DoD guardrail ("privileged pool
  queries have `WHERE location_id`", extended to `routes/**`) must include the **pre-auth** mint routes
  explicitly, and the packet must drop any assertion that the extractor covers them.

---

## LOW

### L1 · B-CONTRACT / Q-LOGOUT · `Claims<Owner>` extractor likely returns 403 where Node returns 401 on wrong-role logout

- **[BREAK]** `/api/auth/logout` is under `/api/auth/*` (not an `AUTH_PREFIXES` path), so a valid
  **courier** bearer passes `verifyAuth` (it even runs the courier session-bind DB query,
  `plugins/auth.ts:62-91`) and then hits `if (!userId) 401` (`auth.ts:329-330`) → **401**. A
  `Claims<Owner>` extractor rejects the wrong role earlier — but as a **403** ("Forbidden role",
  `requireRole` at `plugins/auth.ts:110-111`), diverging from the documented **401** (Q-LOGOUT, YAML
  logout 401).
- **[EVID]** `auth.ts:329-330` (401), `plugins/auth.ts:110-111` (403 for wrong role).
- **[MAPS]** Q-LOGOUT. Pin the extractor rejection to 401 on this route, or an E2E asserting 401 flips.

---

## Vectors probed and NOT broken (kept honest — no severity inflation)

- **alg-confusion / `alg=none`:** Node pins RS256 twice (`jwt.ts:105-111`). If the port carries the
  double-pin verbatim (proposal §1.1 mandates it), not a break. Green invariant.
- **base64url padding:** JWT/JWS mandates **unpadded** base64url; both jose and `jsonwebtoken` emit
  unpadded. Not a break — do **not** add padding-tolerance as a "fix."
- **`aud`/`iss` absence:** Node sets neither; jose requires neither; `jsonwebtoken` default requires
  only `exp`. Not a break **unless** the port sets `validate_aud`/required `iss` (which would reject
  *all* Node tokens — a self-inflicted vector, note in the vector set).
- **dev-kid on a *correctly-built* prod binary:** Node's three arms hold and the port keeps them →
  a dev token cannot verify. The residual is the **build-flag + config-drift compose** (H6), not a
  first-order hole.
- **AR-6 leaked `kid:1` token:** unchanged property (rotation-only kill); not introduced by the port.
  Carry as accepted risk (Q1a) — correctly dispositioned.

---

## SHARPEST — the council must not ship without fixing this

**C1 (`kid`-as-required-body-claim).** It is the single finding that most threatens the **Q10 cutover
seam** — the strangler's entire premise. `kid` is a required `.strict()` **body** claim
(`legacy.ts:162`) that Node writes into the body (`jwt.ts:50-54`) but consumes for key-selection only
from the **header** (`jwt.ts:87,94`), making the body copy pure parity-tax — maximally likely to be
dropped by an idiomatic `jsonwebtoken` port. If it is dropped, **Rust-minted tokens are rejected by
Node and rollback loses every session minted during the Rust window**; if the Rust struct mirrors
`.strict()` and also omits it, **Node-minted tokens are rejected by Rust**. Either way Q10(a)'s DoD
("Node-minted rotates on Rust, Rust-minted verifies on Node, rollback leaves every live session
valid") fails. The fix is the architect's — the council's non-negotiable is that the **body-`kid`
round-trip becomes a named cutover gate** (Node→Rust and Rust→Node), alongside the strict-parse
extra-claim vector (H1) and the customer-order-scope decision (H3, which has no register row today).
No 🔴 auth row builds until C1, H1, H3 have vectors and H2 (RETIRE) is settled.
