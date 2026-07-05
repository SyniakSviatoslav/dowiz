# Rebuild Cutover Harness — Breaker Findings

- **Attacker:** System Breaker (DeliveryOS). Axis: competitive truth — where does the harness break, not is it pretty.
- **Target:** `docs/design/rebuild-cutover-harness/{proposal.md,open-questions.md,threat-model.md}` + `docs/adr/ADR-rebuild-cutover-harness.md`.
- **Method:** each finding is demonstrable — a concrete break scenario OR a back-of-envelope number OR a `file:line` ground-truth contradiction. No fixes proposed (architect fixes). Read-only verification against `apps/api/src/server.ts`, `apps/api/src/bootstrap/routes.ts`, `apps/api/src/routes/spa-proxy.ts`, `apps/api/src/routes/orders.ts`, `packages/platform/src/auth/jwt.ts`, `packages/shared-types/src/legacy.ts`, `apps/api/src/lib/client-ip.ts`, `rebuild/crates/api/src/main.rs`.
- **Severity tally:** CRITICAL 2 · HIGH 5 · MEDIUM 4 · LOW 2.

---

## CRITICAL

### [CRIT] B-CONSIST / B-ANTIPATTERN · The longest-prefix router cannot separate the 5 surface families that all live under one variable-segment prefix `/api/owner/locations/:locationId/*`

**Scenario.** The proposal (§4 "Path-ownership map") specifies routing "by **`(method, prefix)`**, longest-prefix wins." But ground truth (`apps/api/src/bootstrap/routes.ts:131-140`) mounts SIX plugins under the *single* prefix `/api/owner/locations`, and the real registered routes (verified via route census) interleave five distinct surface families beneath one variable UUID segment:

- `/api/owner/locations/:locationId/orders/:orderId/deliver|confirm|reject|pickup|verify|mark-no-show|metadata|reveal-customer-contact` → **S5** (orders/money 🔴) and reveal-contact (PII)
- `/api/owner/locations/:locationId/settlements*`, `/refunds*` → **S5** (money)
- `/api/owner/locations/:locationId/couriers*`, `/orders/:orderId/route`, `/courier-invites*` → **S7** (dispatch 🔴)
- `/api/owner/locations/:locationId/notifications*`, `/alerts*`, `/signals*`, `/push*` → **S8**
- `/api/owner/locations/:locationId/gdpr-requests*` → **S9** (irreversible erase 🔴)
- `/api/owner/locations/:locationId/theme`, `/theme/logo`, `/settings/dwell`, `/settings/fallback` → **S3**

The discriminator that tells S5 from S9 (`/orders/` vs `/gdpr-requests/`) sits **after** a variable UUID — it is an **infix, not a prefix**. Longest-PREFIX matching keys on a leading string; no prefix string can express "route on the segment after a variable UUID." So a longest-prefix router collapses every `/api/owner/locations/:locationId/*` route into whichever single surface owns the `/api/owner/locations/` prefix.

**Break.** Flipping S5 (money) to Rust would co-flip S9 (GDPR irreversible erase) and S7 (dispatch) and S8 — because they share the prefix. That is the exact inverse of Goal 3 ("S1..S10 flip independently") and the safe→risky ordering (Q5): the money flip drags the irreversible-erase surface with it. There is no `(method,prefix)` partition that isolates them.

**Two surviving interpretations, both break:** (a) *literal longest-prefix on the URL path* → can't isolate the families (above); (b) *silently meaning per-matched-route-template* (`/api/owner/locations/:locationId/orders/:orderId/deliver`) → then the "236-route partition" is a 236-row exact-route table, not the coarse prefix rows the doc shows (`/api/owner/menu/*`, `/api/owner/orders/*`), the disjointness proof is over a far more fragile artifact than advertised, and it is *still* wrong on the phantom paths (see next finding).

**Invariant violated:** per-surface isolation (proposal Goal 3); "provably-disjoint `(method,path)` ownership map" (§4); safe→risky irreversibility ordering (Q5).

---

### [CRIT] B-CONSIST · The map's crown-jewel S5 paths (and the S2 OTP paths) are phantom — they do not exist in the running routes

**Ground truth contradictions:**

- Map (§4, S5 row) says S5 owns **`POST /orders`**. Real path is **`POST /api/orders`**: `apps/api/src/routes/orders.ts:73` (`fastify.post('/orders', …)`) registered with `{ prefix: '/api' }` at `bootstrap/routes.ts:96`. Confirmed independently by the CORS hook `server.ts:151` (`request.url.startsWith('/api/orders') && request.method === 'POST'`). A router built literally from the map keys on `/orders`; the live URL is `/api/orders`, which does **not** start with `/orders` → **the order-create route never matches its S5 rule → S5 (the crown jewel) can never flip.**
- Map (§4, S5 row) says S5 owns **`/deliver`**. No top-level `/deliver` exists; the real route is `/api/owner/locations/:locationId/orders/:orderId/deliver` (`ownerDashboardRoutes`, prefix `/api/owner/locations`).
- Map (§4, S5 row) says S5 owns **`GET|PATCH /api/owner/orders/*`**. The owner order-*actions* live under `/api/owner/locations/*/orders/*` (see CRIT-1), not `/api/owner/orders/*`. Only the read list `GET /api/owner/orders` (`spa-proxy.ts:393`) matches that literal prefix.
- Map (§4, S2 row) says S2 owns **`POST /api/customer/otp/*`**. Real path is **`/api/customer/locations/:slug/otp/(send|verify)`** (`customerOtpRoutes` prefix `/api/customer` + `/locations/:slug/otp/*`, confirmed `server.ts:420` regex).

**Break.** The proposal's Phase-0 gate claims "a CI extractor proves the map is a **partition** — every one of the 236 census routes maps to exactly one surface" (§4). The *single worked example* for the money surface is off by the `/api` prefix and two of its three representative S5 paths are phantom. Either the CI census the gate ran against does not match the live Fastify route tree (the "provable partition" is unfalsifiable/unproven), or it was hand-written and never cross-checked against `printRoutes()`. This falsifies the disjointness claim that the entire safety argument (threat T1, T2) rests on, and blocks the money cutover the harness exists to enable.

**Invariant violated:** "provably-disjoint `(method,path)` ownership map" (ADR §Decision.1); map-coverage zero-diff gate G4 (§6); threat T1 mitigation.

---

## HIGH

### [HIGH] B-CONSIST / B-SCALE · "Atomic flip" is not atomic — it is TTL-bounded eventual convergence, and NOTIFY may never arrive through the pooler

**Scenario.** The flip is `UPDATE cutover_flags … + NOTIFY` (§4); every Node instance `LISTEN`s and refreshes; "a short TTL (1–5s) is the backstop if a notify is missed." On multi-machine Fly, convergence is not atomic — it is eventual, bounded by TTL. **The codebase documents that LISTEN/NOTIFY does not work through the transaction pooler:** `server.ts:220-221` ("Operational pool may use transaction pooler which doesn't support LISTEN/NOTIFY") and `server.ts:240-241` (pg-boss forced to session port 5432 for the same reason). The proposal never specifies which pool the `cutover_flags` LISTEN uses. If it reuses the operational (transaction-pooler, port 6543) pool — the natural choice for a cheap flag read — **NOTIFY never arrives and every flip degrades to pure TTL staleness: a guaranteed 1–5s split-brain window on every flip.**

**Number (S5 duplicate-order exposure during the window):** at today's peak ~0.17 orders/s (proposal §2), a 1–5s window spans ~0.2–0.9 orders; at the modeled 10× (~1–2 orders/s) it spans ~1–10 orders. During the window instance A routes `POST /api/orders`→Rust while instance B routes →Node. Safety then **fully reduces to R-3** (the shared `(key,location_id)` unique + request-hash byte-identity) — i.e. the doc's own top-risk, now exercised on *every* flip, not just S5's.

**Invariant violated:** "the flip is per-surface-atomic" (threat T1); "sub-second via notify" (§9 rollback claim); the split-brain-prevention premise.

---

### [HIGH] B-SEC · Cross-stack JWT verification parity breaks on the non-standard body-`kid` claim + `.strict()` schema (Q4 / R-2)

**Ground truth.** `packages/shared-types/src/legacy.ts:162-174`: `AuthToken` is a `z.discriminatedUnion('role', …)` where **every branch is `.strict()`** and `AuthBase` **requires `kid: z.string()` in the token body**. `packages/platform/src/auth/jwt.ts:50-56` duplicates `kid` into *both* the JOSE protected header and the body claims. `verifyAuthToken` (jwt.ts:105-114) ends with `AuthToken.parse(payload)` — a strict parse of the body.

**Break.** A Node-minted token carries body-`kid`; a Rust owner surface (S3/S4/S5, flipped before S2 per Q4/Q5) must verify it, and a Rust-minted token must verify on Node. An **idiomatic Rust JWT minter** (`jsonwebtoken` crate) puts `kid` only in `Header` and does **not** replicate the body-`kid` quirk, and/or emits standard registered claims (`iss`/`aud`/`nbf`). Against Node's `.strict()` `AuthToken.parse`:
- missing body-`kid` → parse fails → **Node rejects every Rust-minted token**;
- any extra claim (`iss`/`aud`/`nbf`/etc.) → `.strict()` unknown-key rejection → **Node rejects it**.

So Q4's stated precondition ("body-`kid` round-trip both directions") is not a checkbox — it requires Rust to reproduce an exact, non-standard claim shape (kid mirrored into body, zero extra registered claims, exactly one of three role shapes). Any deviation blocks the entire safe→risky ordering, because verification parity gates S3/S4/S5 (R-2).

**Invariant violated:** cross-stack token-verification parity (R-2, Q4(a)); "a token minted by either stack verifies on the other" (§8).

---

### [HIGH] B-SEC · The parity oracle (G1) cannot run authenticated specs against Rust without either breaking or re-opening the dev-login-backdoor class

**Scenario.** G1 ("E2E parity slice green … against the Rust-served paths, flag ON in staging", §6) is *the* language-independent parity oracle. Staging E2E authenticates via `/api/dev/mock-auth` → `signDevToken` (jwt.ts:73-80), which signs under `JWT_DEV_KID` with a dev keypair. `verifyAuthToken` accepts a dev-kid token **only** in non-prod **and** only when the dev public key is present (jwt.ts:91-99). For any authenticated parity spec to pass on Rust, the Rust app must (a) hold `JWT_DEV_PUBLIC_KEY` and (b) replicate the dev-kid-only-in-non-prod segregation.

**Break.** Two-horned: (a) if Rust does **not** implement dev-kid acceptance, *every* authenticated parity spec (S2/S3/S4/S5/S7/S9/S10) fails on Rust → G1 is unprovable → no authenticated surface can pass its `readiness_ok` gate; (b) if Rust accepts dev-kid **without** faithfully reproducing the `NODE_ENV !== 'production'` gate (ADR-0003), a dev-signed owner/courier token is honored on a Rust surface — the **dev-login-backdoor incident class reborn cross-stack** (see memory `dev-login-backdoor`). `rebuild/crates/api/src/test_support.rs` uses "throwaway RSA keypairs generated at runtime" — a *test* fixture, not the staging dev-kid path; the parity-oracle dependency is unaddressed.

**Invariant violated:** G1 parity oracle provability (§6); ADR-0003 dev-kid segregation (dev tokens cryptographically rejected in prod).

---

### [HIGH] B-FAIL / B-OPS · Health-gate auto-degrade is a per-instance, no-consensus circuit breaker that MANUFACTURES cross-instance split-brain

**Scenario.** §7 / ADR §4: "a `rust` surface whose upstream is unhealthy **degrades to Node automatically**." Each Node machine independently polls Rust `/healthz` and makes this decision locally. The flag row says `rust` for all instances, but the *effective* route is `flag AND local-health`. During a Rust flap, a partial 6PN partition, or a rolling Rust deploy, instance A sees `/healthz` green (routes Rust) while instance B sees it red (auto-degrades to Node) → **the same surface is simultaneously `rust` on some instances and `node` on others**, with no flag change and no operator action. This defeats the per-surface-atomic invariant that the whole design rests on — and it is triggered *by the safety mechanism itself*, not by a flip.

**Break.** For S5 this reintroduces the exact duplicate-order split-brain the atomic flip exists to prevent. No hysteresis/debounce/quorum is specified for the health-gate, so a marginal upstream produces continuous oscillation. The circuit breaker has no cross-instance consensus — it is N independent breakers making divergent routing decisions on shared money writes.

**Invariant violated:** "a surface is wholly `node` OR wholly `rust` — never split within a surface" (threat T1); failure-first "zero cascade" (§7).

---

### [HIGH] B-SEC / B-SCALE · Real client IP is lost across the internal hop → S5 velocity/throttle collapses to one bucket, and the stated mitigation contradicts the #9 invariant

**Ground truth.** `apps/api/src/lib/client-ip.ts` (the #9 fix): `clientIp()` trusts **only** `Fly-Client-IP` and **never** `X-Forwarded-For` ("NEVER trust `X-Forwarded-For` — it is client-controllable"). In production, header absent → fail-closed to a single shared bucket `shared:no-fly-ip`. `orders.ts` velocity/reputation and the per-IP order throttle key on `clientIp()`.

**Break.** The Node→Rust forward goes over Fly 6PN `flycast` (TB-2) — it does **not** traverse the Fly edge, so **`Fly-Client-IP` is absent** on the forwarded request. §8 proposes to set `X-Forwarded-For = clientIp` so "S5's velocity throttles key on the customer." But §8 also says the traversal/IP guards are "carried **verbatim** in the Rust port" — and the verbatim #9 invariant is *never trust XFF*. So:
- If Rust ports #9 verbatim (never trust XFF) → it ignores the forwarded XFF, `Fly-Client-IP` is absent → `clientIp()` on Rust returns `shared:no-fly-ip` → **every S5 order on Rust collapses into one global velocity/throttle bucket** (either false-positive throttling of all customers, or the abuse signal goes blind).
- To make it work, Rust must trust XFF *only from the internal Node peer* — an unspecified, security-sensitive trust carve-out that directly contradicts the invariant §8 claims to preserve.

The design cannot simultaneously (a) carry the #9 "never trust XFF" guard verbatim and (b) recover the real client IP across the hop via XFF. This is unresolved.

**Invariant violated:** #9 real-client-IP integrity (T8); "S5 velocity throttles key on the customer, not the Fly edge socket" (§8).

---

## MEDIUM

### [MED] B-CONSIST · Invisible un-migratable routes + an UNTRACKED cross-stack two-writer on `products`/`location_themes`

**Ground truth.** Many live Node routes are absent from the §4 map and, by "unmapped → Node default," stay on Node forever: `/api/owner/analytics` + `/analytics/product-orders` (read `orders`/`order_items` money tables — `spa-proxy.ts:296,375`), `/api/owner/couriers`, `/api/owner/customers` + `/:id/analytics`, `/api/owner/courier-invites`, `/api/owner/onboarding`, `/api/owner/brand/generate`, `/api/owner/promotions`, `/api/funnel`, `/api/telemetry`, plus infra (`/metrics`, `/livez`, `/health`, `/internal/*`, `/webhook/payments/plisio`, `/api/dev/*`).

**Break.** (a) Surfaces can never fully cut over → Phase-D Node decommission is blocked (contradicts "the vine, built to be cut", §3). (b) **`POST /api/owner/onboarding`** (`spa-proxy.ts:758`) WRITES `products`, `locations`, `location_themes` on Node while S3-Rust owns catalog CRUD → an **untracked cross-stack two-writer** on `products`/`location_themes`, beyond the *only* acknowledged writer (menu-import, REV-7). (c) `/api/owner/analytics` reads the orders money tables on Node while S5-Rust owns them → S5's "whole family flips atomically" (§4) is false; analytics stays Node.

**Invariant violated:** S3 two-writer completeness (REV-7); "whole family flips atomically" (§4 S5); Phase-D decommission.

---

### [MED] B-CONSIST · The S5 gate guards the wrong invariant — hash byte-identity is not what prevents the duplicate paid order

**Ground truth.** `CreateOrderInput.idempotency_key` is a **required** uuid (`legacy.ts:68`, `.strict()`). `orders.ts:395-420`: on a request the handler `SELECT`s `idempotency_keys WHERE key+location_id`; the `(key, location_id)` UNIQUE + a `23505`→409 path (orders.ts:718) is the concurrent guard; `request_hash` is only compared to decide replay (return same order) vs `422 IDEMPOTENCY_KEY_REUSED` (orders.ts:402-404).

**Break.** Proposal §6 states "the **only** guard against a cross-stack retry producing a duplicate paid order is the shared unique constraint, effective **iff** the `request_hash` is **byte-identical**." That conflates two things:
- The `(key,location_id)` UNIQUE prevents concurrent cross-stack duplicates **regardless of hash** (both stacks insert the same key → one wins, one 409). Hash byte-identity is **not** required for duplicate prevention.
- request-hash byte-identity governs only the *replay branch*: a drift turns a legitimate cross-stack retry into a spurious `422 IDEMPOTENCY_KEY_REUSED` (broken checkout during the flip window), **not** a duplicate.
- The genuinely CRITICAL money-dup requires a Rust **atomicity** drift — order INSERT and `idempotency_keys` INSERT not in one transaction, or the idem row written post-commit — which the hash golden-vector gate does **not** cover.

So the flip gate (request-hash golden-vector) is scoped to the replay-UX failure, while the real duplicate-money failure (transaction-scope parity on Rust) is named only implicitly. `JSON.stringify` (Node, `order-canonical.ts:40-51`) vs `serde_json` (Rust) will also differ on key order / non-ASCII escaping / number formatting — so hash drift is *likely*, producing spurious-422 storms during any S5 flip window.

**Invariant violated:** correct identification of the money-duplicate guard (proposal §6 / threat T5); R-3 gate scope.

---

### [MED] B-SEC · Q3 atomic-flip does not remove the concurrent-refresh-split hazard — it relocates it into the flip window, and the trip-wire only detects irreversible damage

**Scenario.** Q3/R-1 argues atomic-flip is *safer* than a per-request canary because it "keeps a family wholly on one stack." But during the 1–5s convergence window (HIGH-1), two concurrent refreshes of **one** family can hit instance A (rust) and instance B (node) → the cross-stack concurrent-refresh split the atomic flip was supposed to eliminate, now scoped to the flip window instead of "always." Residual safety still depends on **byte-identical refresh SQL incl. the `interval '5 seconds'` window** — and proposal §6 itself flags a drift ("the SQL is authority over the stale '10s' comment"). The family-revoke-rate trip-wire (§7, R-1) is **reactive**: it auto-rolls-back *after* the revoke-rate exceeds baseline — but a revoked refresh-token family is an irreversible session kill ("routing is reversible; committed side-effects are not", threat-model §Prime insight). The trip-wire detects damage it cannot undo.

**Invariant violated:** R-1 "argued safer" claim; "routing is reversible; committed side-effects are not."

---

### [MED] B-SCALE · Connection-budget omits the per-machine session-pool LISTEN connection

**Scenario.** The §2 `overlap_conns` budget enumerates api/worker/analytics/migrations pools but omits the flag-store's LISTEN. Because the transaction pooler blocks LISTEN/NOTIFY (server.ts:220-221), each Node **machine** needs a dedicated **session-mode** connection to `LISTEN cutover_flags_changed`. That is `N_machines × 1` session connections competing on Supavisor's session-mode budget with the existing pg-boss + messageBus session pools. Unbudgeted; at scale-out (the modeled 10×, more machines) this is a real slice of the session-pool ceiling the §2 table does not account for.

**Invariant violated:** §2 connection budget completeness; the "sum never exceeds the pooler ceiling" contract.

---

## LOW

### [LOW] B-FAIL · undici front-door timeout (10s) < Rust tower timeout (30s) → false-failure on a committed order

**Scenario.** §7 sets the front-door forward timeout "tighter … e.g. 10s" while Rust's own tower timeout is 30s (referenced §7). The front-door can time out at 10s and return a failure while Rust **commits the order at 12–30s**. The customer sees "order failed" and retries; the idempotency key (required) makes the retry a 200-replay or 409, so no duplicate — but a *succeeded* order was reported as failed, and the customer may abandon or double-attempt via a fresh cart (new key → genuine duplicate). Also: a flip re-warms a cold per-stack 30s menu cache (S1) → a thundering-herd DB read spike on Rust cold-start (latency, not divergence). Note: S1's "read-only, no DB write" claim otherwise **verified** — `apps/api/src/routes/public/{ssr,menu,theme,seo,client-flow,rates,fallback-config,voice-config}.ts` contain zero INSERT/UPDATE/DELETE.

**Invariant violated:** timeout-fallback fidelity (§7); "no request torn mid-flight" (T3) holds, but the response contract does not.

### [LOW] B-OPS · The break-glass `CUTOVER_FORCE_ALL_NODE` is an env var → its activation is NOT the "instant, no redeploy" the design sells

**Scenario.** §4 / Q7: the defense for "the flag store read is itself impaired" is a front-door env `CUTOVER_FORCE_ALL_NODE=1`. Setting a Fly env/secret requires a machine restart or redeploy. So the one recovery path for a corrupted-flag-value failure (the case where the DB-unreachable auto-default does *not* fire because the read *succeeds* with a wrong value) is precisely the redeploy the design's headline claim ("Rollback … no redeploy, ever", §9) says it avoids. The instant lever (the DB flip) is unavailable in exactly the scenario the break-glass exists for.

**Invariant violated:** "instant, no redeploy" reversibility (§9); T6 break-glass as a fast recovery.

---

*Zero fixes proposed. Each finding names the break and the violated invariant; the architect owns remediation.*
