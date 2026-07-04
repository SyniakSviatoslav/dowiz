# S2-AUTH Port — Council Packet · THREAT MODEL

> **STATUS: DRAFT — NOT APPROVED.** Adversarial input to the S2 council. Enumerates assets,
> trust boundaries, carried accepted-risks, and the failure modes the Rust port must not
> silently introduce. Read alongside `proposal.md`. Docs only; no code.

- **Method:** STRIDE-lite over the S2 YAML surface + fold-in of the 2026-07-02 blue-team sweep
  (`docs/security/hardening-findings-2026-07-02.md`) + ADR-0003/0004 residuals.
- **Scope note:** three sweep findings are S2-adjacent but ship with other surfaces —
  #4/#5 (WS) → S6; #1/#2/#3 (RLS/definer) → B3. Their *disposition* is recorded here because they
  change S2's assumptions; their *fix* is in those councils.

---

## 1. Assets (what an attacker wants)

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| A1 | Prod signing key (`JWT_PRIVATE_KEY`, `kid`) | Fly secret / config | Forges any owner/courier/customer token. Sole kill = rotation |
| A2 | Owner access + refresh family | client localStorage; `auth_refresh_tokens` (sha256) | Full tenant read/write |
| A3 | Courier session + JWT | client localStorage; `courier_sessions` | Courier PII, GPS, delivery actions |
| A4 | Customer track JWT + grant | localStorage; `customer_track_grants` (sha256) | Order status, address, contact reveal |
| A5 | Claim/transfer token | `#token=` fragment, `dos_claim_token` | Atomic shadow-tenant ownership transfer |
| A6 | Dev-auth secret + dev keypair | staging config only | Mints staging tokens; MUST be prod-inert |
| A7 | Argon2 password hashes; encrypted courier PII | `users`, courier tables (AES-256-GCM) | Credential + PII compromise |
| A8 | Google OAuth client secret | config | Account-linking abuse |

---

## 2. Trust boundaries

- **TB-1 unauth → pre-auth mint** — login/redeem/exchange/track/claim/telegram-poll take *no* bearer.
  The request body/token IS the authority. Rate-limits + timing-safe compares + anti-enumeration are
  the only gates. (Highest-value boundary; most operations live here.)
- **TB-2 bearer → role** — the claims-extractor (proposal §3). A wrong-role token must be unable to
  reach a wrong-role handler (type-state).
- **TB-3 owner → location** — ADR-0004 P-c (refresh boundary) + P-d (per-route `status='active'`).
  The current gap (sweep #6): not enforced at the hook; `spa-proxy` trusts baked `activeLocationId`.
- **TB-4 dev-kid → prod verifier** — ADR-0003. Must be crypto-impossible on prod, not just gated.
- **TB-5 app WHERE-predicate → RLS** — many privileged reads/writes lean on RLS *or* an explicit
  `location_id` predicate. B3 (NOBYPASSRLS flip) moves the load-bearing boundary — see §5.
- **TB-6 client localStorage → XSS** — zero-cookie architecture; any XSS exfiltrates A2/A3/A4
  (AUTH-GAP-5 / AR-5).

---

## 3. The 5 carried auth gaps — explicit ACCEPTED-RISK rows

These are **carried into the Rust port by decision, not fixed silently** (inventory 14 §4: "do not
silently fix during port"). Council ratifies each disposition; the port reproduces current behavior
unless a row is explicitly promoted to FIX-with-E2E-delta.

| AR | Gap | Live evidence | Risk in the Rust port | Disposition (recommend) |
|---|---|---|---|---|
| **AR-1** | **AUTH-GAP-1** — FE "Exit" never calls `POST /api/auth/logout` and never clears `dos_refresh_token`; refresh family survives UI logout | `AdminRoutes.tsx:145-150`; ADR-0004 P-b route exists but is unwired | User believes they logged out; refresh family (7d) still mints access tokens on a shared device | **FIX in the FE port** (wire Exit → logout + clear storage). Server logout already correct; this is an FE-wiring gap, safe to close with an E2E (logout → all-families-401). Recommend FIX-with-delta |
| **AR-2** | **AUTH-GAP-2** — `/api/auth/courier/activate` dead: 7d TTL, courier refresh written to the **owner** `auth_refresh_tokens` table (`auth.ts:378-382`) → `/api/auth/refresh` would rotate it as `role:'owner'` (latent role-confusion); **and** its minted token violates strict `CourierClaims` so it is unverifiable | `auth.ts:339-393`; 0 FE callers, 0 E2E | Porting by inertia carries a live-reachable role-confusion path into Rust | **RETIRE** with proof-of-deadness (grep 0 callers). Do NOT port. This is the one gap where carry-verbatim is *more* dangerous than fix |
| **AR-3** | **AUTH-GAP-3** — courier TTL matrix inconsistent (redeem **14d** JWT / login+refresh **24h** JWT / session **30d**) | `courier/auth.ts:136` vs `:219+` | A 14d courier JWT is a 14d bearer with only a per-request `jti` bind between it and revocation | **Council unify** target values at port (`open-questions.md` Q3); contract records reality until then. Recommend unify to 24h JWT / 30d session |
| **AR-4** | **AUTH-GAP-4** — no password-reset flow exists at all (grep-confirmed zero hits) | — | Absent feature; a locked-out owner has no self-service recovery → support-mediated | **Greenfield product decision**, not a port target. New `auth::password_reset` = its own council row, post-S2. Recommend backlog, do not invent in the port |
| **AR-5** | **AUTH-GAP-5** — zero-cookie architecture; A2/A3/A4 XSS-exfiltrable from localStorage | inventory 14 §4; CONVENTIONS.md | Any storefront/admin XSS = silent token theft; no httpOnly barrier | **Council keep-vs-httpOnly** at *this* port, not mid-implementation (`open-questions.md` Q5). Recommend: carry localStorage for Phase-B byte-parity; open httpOnly as a *fast-follow* council row (it changes the transport contract → FE-lockstep) |

**Plus one ADR-0003 residual (not one of the 5, but carry it):**

| **AR-6** | **ADR-0003 R-6** — the historic leaked `kid:1` prod owner token is killable **only by operator key rotation**; the design prevents recurrence but cannot reject a pre-minted leaked prod-kid token | `jwt.ts`; ADR-0003 Open items R-6 | The Rust port inherits the same property: a leaked prod key/token is only revocable by rotation (no per-token deny-list) | **CARRY** — rotation stays an operator runbook action. Consider a `kid`-versioned deny-list as a DEFERRED upgrade (ADR-0004 Option A/B trigger) |

---

## 4. JWT-in-URL finding — disposition for S2

The sweep's finding **#5 (HIGH)**: a 24h–14d bearer JWT read from the WebSocket `?token=` query URL
(`websocket.ts:154-167`) leaks to access logs / history / Referer; an off-URL path already exists at
`:186` (ADR-0013 addendum: `Sec-WebSocket-Protocol` carriage).

**Disposition:** this is **AUTH-12 (WS auth) → S6**, *not* an S2 route. But it clarifies an S2
invariant worth stating so the port does not regress it:

- **In S2, no actual JWT ever travels in a URL.** Every S2 URL-carriage is an **opaque, short-TTL,
  single-use-or-hashed code**, not a bearer token:
  - OAuth handoff: one-time opaque **code** in the `#code=` fragment, 60s TTL, exchanged server-side
    for the pair (`auth.ts:160-166`). The JWT pair is returned in the exchange *response body*.
  - Claim: opaque **token** (≥16 chars) in the `#token=` fragment, sha256-hashed server-side.
  - Track: opaque **grant code** in `?t=` (base64url 32 bytes), sha256 lookup, raw never logged.
- **Fragments (not query) are deliberate** — anti-Referer-leak (`#` never leaves the browser). The
  Rust port MUST keep fragment carriage (proposal §9, rows Q-FRAG-*).
- **Recommendation:** the S2 council records "no JWT-in-URL in S2" as a green invariant with a
  guardrail (a test asserting login/exchange/track/claim responses carry the JWT in the *body*, and
  that no route echoes a bearer into a redirect `Location`). The WS `?token=` fix is deferred to the
  **S6 WS-authz council** (land the ADR-0013 addendum, retire the deprecated query param — do not
  port `?token=` forward). The customer-JWT-mint↔WS linkage (REBUILD-MAP §7 item 1) is folded there.

---

## 5. Port-specific failure scenarios (Rust) — token theft / replay / tenant-confusion

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| T-1 | **Refresh replay** — stolen stale refresh token replayed after rotation | If the Rust guarded-UPDATE isn't atomic (e.g. SELECT-then-UPDATE), two requests both mint → single-use defeated (the exact bug ADR-0004's atomic claim fixed) | Port the `UPDATE … WHERE used=false RETURNING` atomic claim; test: concurrent double-refresh → one 200 + one 409/401, family intact for the <5s benign case, family DELETE for the genuine replay |
| T-2 | **Concurrent-refresh family self-DoS** — two tabs race, port wrongly treats the loser as reuse and revokes the family | Getting the `<5s` benign-window wrong (or copying the stale "10s" comment) | Port SQL 5s window verbatim; test the two-tab race stays 409, no family kill; keep Web-Locks single-flight contract |
| T-3 | **Role/tenant confusion via claim strictness** — a token with mixed/extra claims accepted | A lenient Rust deserializer (serde default allows unknown fields) | **Deny-unknown-fields + required-claim** parse mirroring `AuthToken.parse` (.strict discriminated union). This is *also* why AR-2's activate token is unverifiable today — the strictness is a live control, port it |
| T-4 | **Downgraded-owner roll-forward** — demoted owner keeps minting owner tokens | Dropping ADR-0004 P-c live role re-derive on refresh | Port P-c: 401 OWNER_REVOKED when no active owner membership; never hard-code `role:'owner'` on re-mint |
| T-5 | **Insider write-window** — removed owner still writes for ≤24h | Not re-homing sweep #6 (spa-proxy baked `activeLocationId`); dropping P-d | `OwnerAt<Loc>` extractor does the per-route `status='active'` re-read on write surfaces; test removed-owner blocked immediately on an owner-write route |
| T-6 | **Courier location-revoke lag** — courier removed from a location keeps a valid 24h/14d JWT (no per-location re-check) | Carrying AUTH-GAP courier-no-recheck verbatim (Q-COURIER-NORECHECK) | Document as accepted (parity) OR council-FIX to mirror P-c with an E2E delta (Q6). Session `status` re-check is present; per-location is the gap |
| T-7 | **Dev-token accepted on prod** — a staging/dev token verifies on the prod binary | Compiling dev routes into release, or a Rust verifier that accepts an unknown/dev `kid` | `#[cfg(feature="dev-routes")]` compile-out + runtime dev-kid gate + prod holds no dev pubkey (three layers, ADR-0003). Test: dev-signed token → prod verify rejects |
| T-8 | **Customer-token PII leak** — phone embedded in customer JWT | Convenience-adding a phone claim during the port | Assert minted customer token has NO phone claim (P0-PII, `jwt.ts:122-125`); customer claims schema forbids extra fields |
| T-9 | **Claim/transfer replay or cross-identity** — token replayed, or authenticated ≠ invited identity | Dropping single-use / contact-binding on `claim_transfer()` | Keep `claim_transfer()` SECURITY DEFINER atomicity, CONTACT_REQUIRED/CONTACT_MISMATCH; token single-use 72h; `reauth:true` forces re-derive |
| T-10 | **Enumeration** — attacker probes `/api/claim/request` to discover claimable shadows | Port returning a distinguishable response for real-vs-fake slug | Keep the byte-identical uniform 202; never reveal shadow existence |
| T-11 | **Timing oracle** — courier/owner login reveals identity existence by timing | Dropping the dummy-argon2-on-unknown-identity path (`courier/auth.ts:254-256`) | Port the constant-work path; INVALID_CREDENTIALS indistinguishable for unknown-identity vs bad-password |
| T-12 | **Customer-token order-scope drift** (breaker H3 / architect · = WS-authz council A1) — a per-order 14d tracking token (`?t=` query, referer-leakable) is authorized customer-wide, so a token minted for order A can read/cancel/rate any of the customer's other orders | `orders.ts:752` binds the token's `orderId` claim (per-order) but `customer/orders.ts:50` binds `customer_id = sub` (customer-wide, ignoring the claim). A lenient `Claims<Customer>` port silently unifies to the broader arm | **COUNCIL REV-3 — FIX-IN-PORT (not carry).** Unify customer authorization to the minted `(orderId, locationId, sub)` tuple; E2E delta: token for order A → `POST /customer/orders/B/cancel` must 403 (today it 200s). Resolve jointly with the WS-authz council (same A1 finding). Added to the quirk register as a FIX row. |

> **Governance note (breaker H3):** T-12 was ABSENT from both the quirk register and this threat-model
> in the drafted packet — a real cross-order authz gap with no "carry-vs-FIX" disposition. The council
> RESOLUTION (`resolution.md` REV-3) adds it here and dispositions it FIX-IN-PORT; it must also land as
> a quirk-register row in `proposal.md §10` before `COUNCIL-APPROVED`.

---

## 6. What the B3 RLS flip changes for auth assumptions

Today the operational write pool is **likely still BYPASSRLS** (sweep §"determinant" — migration
`1790000000015` made `deliveryos_operational_user` NOBYPASSRLS+SELECT-only, but checkout/status/invite
routes INSERT/UPDATE on it, so the deployed role is probably still a superuser). Consequences for S2:

- **Track-exchange** explicitly "runs on BYPASSRLS operational pool with explicit WHERE"
  (Q-TRACK-POOL). Several auth-adjacent reads/writes rely on the **app-layer `WHERE` predicate** as
  the *only* tenant boundary while RLS is bypassed.
- **B3 (NOBYPASSRLS) flip** makes RLS the enforced boundary. Two S2-relevant hazards the port must
  carry the fix for (both are B3-council rows, but they gate auth correctness):
  - **Sweep #2 (CRIT, blocks B3):** anonymous-order RLS `USING(app_current_user() IS NULL)` is
    table-wide TRUE on any connection with no `app.user_id` GUC → **fail-OPEN**, reachable via the
    courier/customer path (which carry no `userId` to seat the GUC). Under B3 this becomes a full
    cross-tenant siphon unless the GUC is *always* seated. **The Rust port's txn-scoped GUC pattern
    must set `app.user_id` on every connection that touches tenant tables — including the pre-auth
    mint paths** — or a NONE-context policy fails open.
  - **Sweep #3 (CRIT):** the keystone definer `app_member_location_ids()` has an unpinned
    `search_path` (all member policies depend on it). The Rust port CALLS these definer functions
    (KEEP disposition, REBUILD-MAP §8) — the search_path pin is a B3/RLS-council fix that must land
    before the flip.
- **Auth assumption shift:** pre-flip, an auth handler that forgets a `location_id` predicate is
  saved by nothing (BYPASSRLS). Post-flip, RLS catches it — *if* the GUC is seated. The port should
  **not** rely on the flip to fix missing predicates: the `OwnerAt<Loc>` / explicit-`WHERE`
  discipline (sweep structural root: "identity-split × RLS-reliance") must hold **independent of**
  which pool role is live. Belt (explicit predicate) AND suspenders (RLS) — the sweep's recommended
  guardrail (extend `rls-adversarial.test.ts` "privileged pool queries have WHERE location_id" from
  `workers/` to `routes/**`) is the S2 DoD gate for this.

---

## 7. Residual risks accepted at this port (summary for the human)

- **≤24h leaked-owner-access window** (ADR-0004 accepted; immediate <1min kill DEFERRED).
- **AR-5 localStorage XSS-exfiltration** (carry for parity; httpOnly = fast-follow council).
- **AR-6 leaked prod key/token → rotation-only kill** (no per-token deny-list).
- **T-6 courier per-location revoke lag** (carry vs FIX — Q6).
- **Google id_token signature unverified** (Q-GOOGLE-IDTOK — direct-TLS-fetch rationale; council may
  promote to JWKS-verify with delta).

None of these are introduced by the rewrite; each is a *current* property. The port's job is to carry
them **visibly** (matrix row + test) so none is lost or silently altered, and to let the council
promote any of them to a FIX-with-E2E-delta on the record.
