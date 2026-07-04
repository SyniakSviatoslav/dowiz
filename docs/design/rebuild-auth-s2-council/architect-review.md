# S2-AUTH Port — Council Packet · ARCHITECT REVIEW

> Seat: **System Architect** (Triadic Council, dowiz complete-rebuild — Rust/axum port of the auth
> surface). This is a design-review artifact; no code. Verdict is a gate before any S2 auth row moves
> to `BUILT`. All findings are verified against live source at `fix/audit-remediation@ae9f5360`.

## VERDICT: **RATIFIED-WITH-REVISIONS**

The packet is the strongest port packet on the program to date: the RS256/kid double-guard, the
strict discriminated-union claims model, the ADR-0003/0004 parity, the dead-flow RETIRE, and the
Q10 shared-table strangler premise all hold against live source. I verified each load-bearing claim
(`jwt.ts`, `legacy.ts:161-175`, `local.ts`, `auth.ts:235-393`, `courier/auth.ts`, `plugins/auth.ts`,
`server.ts:405-427`, `track.ts`, ADR-0003/0004). The design is **sound in structure** but has **three
must-fix under-specifications** in the two most load-bearing seams (the courier extractor and the Q10
cross-verification encoding contract) plus the middleware tower's rate-limit position. None is a
redesign; each is a precise tightening the council must record before code. Ratification is
**conditional** on the six revisions in the final section.

---

## Section-by-section findings (verified against live source)

### §3 — Claims-extractor type-state: sound, with two under-reaches

**Sound / ratified.** `Claims<Owner|Courier|Customer>` role-narrowing by `FromRequestParts` correctly
makes the AUTH-09 logout quirk **structural** (Q-LOGOUT): `auth.ts:329` reads `request.user?.userId`,
which exists only on the owner variant (`legacy.ts:164` — owner has `userId`; courier `legacy.ts:165`
has `sub/activeLocationId/jti?`, no `userId`; customer `legacy.ts:166-173` has `orderId/locationId`).
A `Claims<Owner>`-bound logout handler is unconstructible from a courier/customer token — this closes
the class by type, not convention. `OwnerAt<Loc>` correctly re-homes the ADR-0004 P-d per-route
`status='active'` re-read (`plugins/auth.ts:148-154`) onto owner-write surfaces and off the hot path.

**UNDER-REACH 1 (MUST-FIX — courier extractor under-specified, this is a security regression risk).**
§3 describes `CourierSession` as "the live `courier_sessions` bind check (`jti` → session row,
`status='active'`)". That is **incomplete against live source.** The live bind is
`courierSessionValid` (`plugins/auth.ts:24-30`) driven by the query at `plugins/auth.ts:74-83`, which
validates **four** conditions, not one:
1. row present;
2. `revoked_at IS NULL` (`:26`);
3. `expires_at` not past (`:27`);
4. **`has_location`** — `EXISTS(SELECT 1 FROM courier_locations WHERE cl.courier_id = s.courier_id AND
   cl.location_id = $2)` (`:76-79`), i.e. the courier **still holds membership in the token's
   `activeLocationId`**, re-checked **on every request** (`:28` → `if (!row.has_location) return false`).

The bind is also keyed on `(jti, activeLocationId, sub)` (`:82`, params `$1/$2/$3`), so a
cross-courier session-id swap is rejected. And there is a **jti-less courier branch**
(`plugins/auth.ts:63-70`): a courier token with no `jti` is accepted **only when `devLoginAllowed(env)`
is true** (the mock path), else 401.

Consequence: if the Rust `CourierSession` extractor implements only the literal §3 text
("`status='active'`") and drops the `has_location` predicate and the jti-less dev branch, it
**regresses courier per-location revocation from ~1 request to the full 14d/24h TTL** and either breaks
dev-E2E or accepts a real jti-less token. This is the single most dangerous omission in the packet
because it is silent: the happy-path E2E stays green while the revoke control is gone. **Revision R1.**

**UNDER-REACH 2 (should-fix — extractor's own error shapes not in the quirk register).** §10 captures
the global bearer-gate shape (Q-BEARERGATE, `server.ts:424` `{error:'Unauthorized'}`) but not the
**per-route** bare bodies the extractor itself emits: `{error:'Missing or invalid token'}`
(`plugins/auth.ts:47`), `{error:'Token expired or invalid'}` (`:55`), `{error:'Session revoked or
access removed'}` (`:85`), `{error:'Forbidden role'}` (`requireRole`, `:111`), and the
`requireLocationAccess` `{error:'Not found'}` 404s (`:129,137,153`). These are additional divergent
shapes an FE branch or E2E may assert. They must be enumerated in the quirk register and carried
verbatim. **Revision R4.**

**No material over-reach.** The type-state does not force runtime state the schema doesn't already
carry ("schema rich, runtime minimal" is respected: `OwnerAt<Loc>` re-read stays confined to write
surfaces per ADR-0004 §38, the owner hot path stays pure sig+exp).

### §4–§5 — Mint/verify/TTL parity: correct, but the Q10 encoding contract is under-specified

**Verified correct.** `jwt.ts:82-115` matches the packet verbatim: `decodeProtectedHeader` picks the
key by `kid` before verify (`:87`), `algorithms:['RS256']` (`:106`) **and** the second throw on
`protectedHeader.alg !== 'RS256'` (`:109-111`) — port both, do not collapse. `acceptDevKid = NODE_ENV
!== 'production' && !!JWT_DEV_KID` (`:91`); unknown kid → `Invalid Key ID` (`:101`). `signDevToken`
throws if the dev keypair is absent (`:76-78`) — no silent prod-key fallback. PEM `\n`-unescape at
`:16,23,34,40`. The TTL matrix is accurate: owner argon2 24h + 7d family (`local.ts:146,153`), dev
bypass 7d no-refresh (`local.ts:69-70`), OAuth/TG 24h + 7d (`auth.ts:149,155`), courier redeem 14d
(`courier/auth.ts:136`), courier login/refresh 24h (`:465`) + 30d session (`:450`), customer 7d
(`jwt.ts:131`). Customer token carries **no phone** (`jwt.ts:122-131`, `legacy.ts:170-171`) — P0-PII
invariant confirmed.

**The load-bearing gap (MUST-FIX — Q10 cross-verification encoding).** §4 says "strict claim parse"
and §5 says "carry exact TTLs / reproduce sub-defaulting" but **does not pin the concrete
serialization facts that decide whether a Node-minted token verifies on Rust and vice-versa.** These
are not cosmetic; they are the actual failure modes of the strangler seam. Named, per the charge:

- **E1 — `kid` is a body claim, not just a header field.** `jwt.ts:50-56` puts `kid` into **both** the
  protected header (`.setProtectedHeader({alg,kid})`) **and** the payload (`jwtPayload.kid = kid`).
  `AuthToken`/`AuthBase` (`legacy.ts:162`) requires `kid: z.string()` as a **required body claim** and
  is `.strict()`. A Rust signer that sets only the header `kid` (the `jsonwebtoken` crate default)
  produces a token whose **body lacks `kid`** → Node's `AuthToken.parse` (`jwt.ts:114`) rejects it.
  **Rust must emit `kid` in the body payload too.**
- **E2 — `kid` must serialize as a STRING.** The verifier compares `header.kid === getKid()`
  (`jwt.ts:94`), a strict `===` against the env string. If Rust emits a numeric `kid` in the header
  JSON (e.g. `1` vs `"1"`), `"1" === 1` is false → `Invalid Key ID`. Both header and body `kid` must
  be the env string value exactly.
- **E3 — NO extra registered claims.** The claims union is `.strict()` (`legacy.ts:164-173`). The
  `jose` mint here sets only `iat`/`exp` (via `setIssuedAt`/`setExpirationTime`) plus the payload — no
  `nbf`, `iss`, `aud`, `typ`-in-payload. If the Rust minter auto-adds any of these (some crates add
  `nbf`), Node's strict parse rejects the Rust token. Rust must emit **exactly** the variant claims +
  `sub/iat/exp/kid` and nothing else. (This is the mint-side dual of threat-model T-3, which only
  covers the parse side.)
- **E4 — `sub`-defaulting.** Owner OAuth/Telegram/refresh mints pass **no** `sub`
  (`auth.ts:149`, `refreshedOwnerClaims` `auth.ts:21` — `{role,userId,activeLocationId?}` only); the
  signer fills `sub = payload.sub ?? userId` (`jwt.ts:49`). Courier/customer pass `sub` explicitly
  (`courier/auth.ts:132,462`, `jwt.ts:130`). Rust must reproduce `sub = sub ?? userId` or owner
  OAuth/TG/refresh tokens get a wrong/random `sub`. (The final `?? randomUUID()` fallback is dead — no
  live path reaches it — but harmless to keep.)
- **E5 — refresh-hash format (the shared-table seam).** Owner refresh hash is
  `sha256( hex-string ).hex` where the input is `randomBytes(32).toString('hex')` — a **64-char ASCII
  hex string**, not 32 raw bytes (`local.ts:147-148`, `auth.ts:150-151,308-309`). A Rust impl that
  hashes the raw 32 bytes produces a different digest → a Node-minted refresh token cannot be rotated
  on Rust. Same class for the track grant: `sha256( base64url-string )` (`track.ts:41`). Courier
  refresh is argon2 over `tokenPlain` with the session-id prefix format `sessionId.tokenPlain`
  (`courier/auth.ts:412,469`) — PHC-self-describing, cross-verifies if argon2id + the stored PHC
  string are used.
- **E6 — "byte-compatible" is the wrong frame; the requirement is cross-verifiable + identical
  claim-shape + identical hash-format.** Header key **order** and a `typ` field differ harmlessly
  between stacks (verification recomputes the signature over the actual header bytes present). Do
  **not** spend effort matching serialization byte-for-byte — that is over-reach. Pin E1–E5 instead.

**Revision R2** turns E1–E6 into named DoD test vectors.

### §6 — ADR-0004 revocation parity: verified present and accurate

Every clause checks out against live source: atomic single-use claim `UPDATE … WHERE used=false
RETURNING` (`auth.ts:265-268`); the benign-window SQL is `interval '5 seconds'` (`auth.ts:277`) while
the code comment says "last 10s" (`auth.ts:273`) — **the SQL is authority, port 5s, fix the comment**
(Q-5S-COMMENT confirmed); genuine replay → `DELETE … family` + 401 (`:283-285`); P-c live owner
re-derive → 401 OWNER_REVOKED (`:293-301`); deterministic `ORDER BY created_at, location_id`
tenant-preserve (`:296,304-306`); logout user-wide DELETE + 204 (`:331-332`). P-d owner-write re-read
present (`plugins/auth.ts:148-154`), 404-not-403 (`:153`). Courier refresh reuse→family-revoke
committed (`courier/auth.ts:418-428`), status-only re-check (`:436-440`). Ratified.

**Accuracy correction (feeds Q6/T-6).** §6 and threat-model T-6/Q6 state courier refresh "has no
per-location membership re-check" and frame T-6 as "keeps a valid 24h/14d JWT". That is true of the
**refresh boundary** but materially misleading about the **access path**: the courier hot-path bind
re-checks `courier_locations` membership on **every request** (`has_location`, `plugins/auth.ts:76-79,
28`). So a courier removed from a location is locked out of that location's resources on the **next
request**, bounded to ~1 request — not the TTL. Even after a refresh (which does mint a new token with
the baked `active_location_id`), that new token fails the next `has_location` bind. **Net: Q6/T-6 is
low-severity today, already ~closed on the access path.** The honest disposition is: carry (parity)
is safe; the fast-follow refresh-boundary re-check (Q6 option b) is defense-in-depth, low value —
**provided R1 lands** (if the Rust extractor drops `has_location`, this overstated risk becomes a real
full-TTL hole). Record the correction so the council isn't paying premium attention to a closed gap.

### §7 — Session TTL/storage table: accurate, carry verbatim. Ratified.

### §8 — dev-login prod-exclusion (ADR-0003): all four layers verified. Ratified.

Layer 1 flag+secret `devLoginAllowed` (`dev-guard.ts:30-31`) + `isDevRequestAuthorized`
(`:54-62`). Layer 2 path-404 existence-hiding (`server.ts:412-416`, bare `{error:'Not found'}`).
Layer 3 dev-kid segregation (`jwt.ts:91-102`; `signDevToken` throws without dev keypair `:76-78`).
Layer 4 boot fail-fast (ADR-0003 §D, `config` boot-guard). The `#[cfg(feature="dev-routes")]`
compile-out is the correct Rust addition. **Primary lock is key-material isolation** (prod holds no
dev public key, `jwt.ts:97-98`) — NODE_ENV and the cfg flag are belt-and-suspenders; carry ADR-0003
R-10's "by construction under secret hygiene, not absolute" as the accepted-risk framing. The Q-DUP
collapse (one handler, two routes) is correct and removes the fix-one-miss-other hazard (`mock-auth.ts`
+ `server.ts:549`, two live bodies confirmed).

### §9 — Transport (header-only, zero cookies): confirmed. Ratified.

No `Set-Cookie`; opaque codes in URL fragments only (`auth.ts:166` `#code=`, `track.ts` `?t=` opaque
grant, never the JWT). AUTH-GAP-5 correctly deferred to Q5.

### §10 — Quirk register: complete except the extractor error shapes (R4, above). Otherwise ratified.

Q-DEAD verified: `auth.ts:374` mints `signAuthToken({role:'courier', userId}, '7d')` — this violates
strict `CourierClaims` (`legacy.ts:165` requires `activeLocationId`, forbids `userId` under `.strict()`),
so the minted token is **unverifiable** by `AuthToken.parse`. Proof-of-deadness (0 callers/0 E2E)
plus unverifiable-token = RETIRE is correct (see Q2 below).

### §11 — Middleware tower ordering: correct core order, three gaps (MUST-FIX for one)

**Verified correct core:** the global `onRequest` hook (`server.ts:405-427`) runs dev-gate → then the
AUTH_PREFIXES bearer-presence gate, before per-route preHandlers (`verifyAuth` → `requireRole` →
`requireLocationAccess`). The dev-gate-before-bearer-gate order is load-bearing (existence-hiding
must win) and the packet preserves it.

**GAP 1 (MUST-FIX — rate-limit position is unspecified; the charge asks exactly this).** §11's tower
lists seven layers but **omits rate-limiting entirely.** In the Node stack `@fastify/rate-limit`
installs its own `onRequest` hook; its order relative to the bearer-gate (`server.ts:405`) decides
whether an **over-limit unauthenticated** request to `/api/owner/*` returns **429 or 401**. That
observable ordering must be reproduced in the axum/tower stack (tower layer order is explicit, so this
must be a deliberate decision, not an accident). The packet must (a) determine the live precedence and
(b) name a test vector (over-limit + no bearer → assert the current status). Auth-vs-tenancy is fine
(tenancy/GUC seating is necessarily post-identity), but note the pre-auth mint paths
(login/redeem/track) touch tenant tables with **no** authenticated identity — the GUC-always-seated
requirement (threat-model §6, sweep #2) applies to them and is a B3 dependency, correctly acknowledged.

**GAP 2 (should-fix).** The tower omits the **OPTIONS/CORS-preflight short-circuit** that is the very
first thing the live hook does (`server.ts:406` `if (request.method === 'OPTIONS') return`). The Rust
tower must short-circuit preflight before the dev-gate/bearer-gate or it will 401/404 a preflight.

**GAP 3 (should-fix).** The `NO_AUTH_PATHS` + OTP-regex bypass (`server.ts:417-420`) is a discrete
ordered step **between** the dev-gate and the bearer-gate — it exempts the pre-auth mint routes
(`/api/courier/auth/`, `/api/customer/track/exchange`, the OTP send/verify regex) from the bearer
gate. §11 folds it into step 3; make it an explicit tower node, because a mis-order here would
401 the pre-auth mint surfaces (a cutover-breaking regression). **Revisions R3 (GAP 1/2/3).**

---

## Recommendation per port-blocking question

### 1. Q2 — RETIRE `/api/auth/courier/activate`. **RATIFY RETIRE.**
Verified dead **and** structurally broken: `auth.ts:374` mints `{role:'courier', userId}` which the
strict `CourierClaims` variant cannot represent (missing required `activeLocationId`, extra `userId`
under `.strict()`) → the token is **unverifiable** the moment it is issued; 0 FE callers, 0 E2E.
Porting resurrects dead code with no consumer. RETIRE with a proof-of-deadness matrix row (grep 0
callers + the unverifiable-claims proof). **One correction to the record:** threat-model AR-2's "latent
role-confusion — `/api/auth/refresh` would rotate it as `role:'owner'`" is now **mitigated** by
ADR-0004 P-c: refresh re-derives from `memberships WHERE role='owner' AND status='active'`
(`auth.ts:293-301`); a courier-activated user has a courier membership, not owner, so refresh returns
401 OWNER_REVOKED. The residual live risk is "unverifiable token + a stray courier row in the owner
refresh table (`auth.ts:378-382`)", not an active privilege-escalation path. RETIRE still dominates —
just fix the stated rationale so the drop is recorded honestly.

### 2. Q10 — Live-session migration at cutover. **RATIFY (a) no-migration — conditionally.**
No data migration is required: `auth_refresh_tokens`, `courier_sessions`, `customer_track_grants` are
shared tables; the seam is a routing flip; rollback routes back to Node. **But (a) is safe only under
four gates the DoD must name:**
- **(i) Encoding contract E1–E5 proven** (Revision R2) — else a Node-minted token won't verify on
  Rust or vice-versa. Add both directions as gates: *Rust verifies a Node-minted token* and *Node
  verifies a Rust-minted token* (owner + courier + customer variants).
- **(ii) Hash-format parity** (E5) — `sha256(hex-string)` for owner/track, argon2id+PHC for courier —
  proven by rotating a **Node-minted** refresh token **on Rust** against the shared table.
- **(iii) kid frozen + claim-set frozen for the whole overlap** (ties Q1 and Q9). Both stacks share
  the one prod `kid`. If the operator rotates the kid (R-6) mid-overlap, a Rust-only-new-kid verifier
  rejects Node's old-kid tokens (and vice-versa) — so **no kid rotation during Phase-B overlap** unless
  the dual-kid verify window (Q1 option b) lands in **both** stacks first. Likewise Q9(a) freeze: any
  additive claim breaks the other stack's `.strict()` parse. Make "kid + claim set frozen until
  decommission" an explicit cutover constraint.
- **(iv) Cross-stack concurrent-refresh proven.** During overlap two devices refreshing one family can
  hit different stacks. This is safe **because the serialization point is the shared DB** — the
  atomic `UPDATE … WHERE used=false RETURNING` (`auth.ts:265-268`) resolves the race regardless of
  which stack issues it — **iff both stacks use the identical refresh SQL** (5s window + family-delete
  conditions). Add a cross-stack concurrent-refresh test (one tab→Node, one→Rust, same family →
  exactly one 200, family intact for the <5s benign case).

No refresh-family / revocation-table / courier-session / kid schema step is needed. The council must
confirm gates (i)–(iv) as **cutover gates**, per packet §12 and status DoD item 5.

### 3. Q11 — Collapse duplicate mock-auth. **RATIFY (a): one handler, two routes.**
Two independent live bodies confirmed (`routes/dev/mock-auth.ts` and inline `server.ts:549`).
Structural FIX, behavior-identical; the alias contract entry keeps `openapi-diff` green by proving
both paths answer. Lowest-risk of the four; no reservation.

### 4. Dev-kid segregation (the fourth port-blocker). **RATIFY the four ADR-0003 layers + compile-out.**
All four layers present and verified (§8 above). The Rust prod binary must hold **no dev public key**
(the primary, key-material lock — `jwt.ts:97-98`); `#[cfg(feature="dev-routes")]` compiles the dev
handlers out of release; the runtime dev-kid gate (`NODE_ENV != production` equivalent) and boot
fail-fast are belt-and-suspenders. Carry ADR-0003 R-10 ("by construction under secret hygiene, not
absolute") and R-6 (the pre-minted leaked `kid:1` token is killable only by operator rotation) as
named accepted-risk rows — the port cannot and should not try to reject a pre-existing prod-kid token.

---

## Design revisions required before code (the ratification conditions)

**R1 (MUST — security regression risk).** Re-spec the `CourierSession` extractor to replicate
`courierSessionValid` **in full**: `revoked_at IS NULL` **and** `expires_at` not past **and**
`has_location` (live `courier_locations` membership for the token's `activeLocationId`) **and** the
`(jti, activeLocationId, sub)` keying **and** the jti-less→dev-gate-only branch
(`plugins/auth.ts:24-30,63-92`). Red→green test: courier removed from `courier_locations` → next
request 401 (proves per-location revoke stays ~1-request, not full-TTL).

**R2 (MUST — the Q10 seam).** Add the JWT cross-verification encoding contract E1–E6 as explicit DoD
test vectors: body-and-header `kid` as a **string** (E1/E2); no extra registered claims under the
strict union (E3); `sub = sub ?? userId` defaulting (E4); `sha256(hex/base64url-string)` refresh/grant
hashing + argon2id-PHC for courier (E5); and record that cross-verifiability + identical claim-shape +
identical hash-format — **not** byte-identical serialization — is the actual requirement (E6).

**R3 (MUST for GAP 1; should for GAP 2/3).** Pin the middleware tower fully: (a) position
rate-limiting relative to the bearer-gate/extractor with a 429-vs-401 test vector for an over-limit
unauthenticated request; (b) add the OPTIONS/preflight short-circuit as the first tower node; (c) make
the `NO_AUTH_PATHS` + OTP-regex bypass an explicit ordered node between dev-gate and bearer-gate.

**R4 (should).** Enumerate the extractor's own bare 401/403/404 bodies
(`plugins/auth.ts:47,55,85,111,129,137,153`) in the §10 quirk register as carry-verbatim divergent
shapes, alongside Q-BEARERGATE.

**R5 (should — record correction).** Correct §6 / threat-model T-6/Q6 to state that per-location
courier revocation is already enforced on the **access path** every request via `has_location`; the
refresh-boundary re-check (Q6 b) is low-value defense-in-depth, **contingent on R1**.

**R6 (should — record correction).** Correct threat-model AR-2's rationale: the role-confusion path is
mitigated by ADR-0004 P-c; the residual is an unverifiable token + owner-table row pollution. RETIRE
recommendation unchanged.

**Ratification is granted the moment R1, R2, and R3(GAP 1) are folded into the packet and the Q10
gates (i)–(iv) are named as cutover gates.** R3(GAP 2/3), R4, R5, R6 are should-fix on the same pass.
No 🔴 auth row builds before that.
