# S2-AUTH Port — Council Packet · OPEN QUESTIONS

> **STATUS: DRAFT — NOT APPROVED.** Everything the live Triadic Council (architect / breaker /
> counsel / human) must decide before S2 auth is ported. Each question has options + a lane-R3
> recommendation. The recommendation is a *starting position for friction*, not a decision.
> Docs only.

Legend: **[SEC]** security-correctness · **[CONTRACT]** shape/parity · **[INFRA]** topology ·
**[PRODUCT]** feature scope. 🔴 = red-line, human sign-off required.

---

### Q1 🔴 [SEC] Key rotation & the pre-minted leaked-token residual (AR-6)
The port inherits ADR-0003 R-6: a leaked prod `kid:1` token is killable only by operator key rotation
(no per-token deny-list).
- **(a)** Carry as-is — rotation stays a runbook action; no deny-list. *(recommend for S2 scope)*
- **(b)** Add a `kid`-versioned rotation mechanism now (dual-kid verify window → retire old kid).
- **(c)** Build a token/`jti` deny-list (ADR-0004 DEFERRED Option A/B) — heavier, fail-closed-capable.

**R3 recommendation:** (a) for S2 + spec a documented rotation runbook; open (b) as a fast-follow
(dual-kid verify is cheap and makes rotation non-disruptive). Defer (c) to its ADR-0004 trigger.

### Q2 🔴 [SEC] Dead `/api/auth/courier/activate` — RETIRE or port? (AR-2 / Q-DEAD)
Zero FE callers, zero E2E, 7d TTL into the **owner** refresh table (latent role-confusion), and its
minted token violates strict `CourierClaims` (unverifiable).
- **(a)** RETIRE with proof-of-deadness (grep 0 callers, matrix RETIRE row). *(recommend)*
- **(b)** Port + fix (write to courier sessions, correct claims, unify TTL) — but no consumer exists.

**R3 recommendation:** (a) RETIRE. Porting resurrects a role-confusion path with no user. The matrix
row carries proof-of-deadness so the drop is deliberate, not lost.

### Q3 [CONTRACT] Courier TTL unification (AUTH-GAP-3 / AR-3)
Redeem mints **14d** JWT, login/refresh **24h** JWT, session row **30d**.
- **(a)** Carry verbatim — parity oracle sees identical `exp`. *(safest for cutover)*
- **(b)** Unify redeem JWT to **24h** (match login) at port, keep 30d session; E2E delta on the
  redeem-token-expiry assertion. *(recommend as a fast-follow, post-parity)*
- **(c)** Unify everything to one TTL pair (e.g. 24h JWT / 14d session).

**R3 recommendation:** (a) at cutover for byte-parity; schedule (b) immediately after green — a 14d
courier bearer is a large blast radius bounded only by the per-request `jti` bind.

### Q4 [CONTRACT] Divergent error shapes — carry vs migrate-to-envelope
S2 has four non-envelope shape families (claim bare `{error:CODE}`, courier manual-Zod
`{error,details}`, refresh `{error:'concurrent_refresh'}`, track/telegram status bodies). CONVENTIONS.md
defers these to post-Astro FE-lockstep.
- **(a)** Carry all verbatim; migrate later in an FE-lockstep pass. *(recommend)*
- **(b)** Migrate to the ADR-0010 envelope now, with synchronized FE branch changes + E2E.

**R3 recommendation:** (a). The current FE branches on these exact shapes; migrating during the port
couples two risky changes. Do it as a dedicated, FE-lockstepped contract change once Astro owns the FE.

### Q5 🔴 [SEC] AUTH-GAP-5 — keep localStorage tokens or move to httpOnly cookies?
Zero-cookie today; tokens XSS-exfiltrable.
- **(a)** Keep localStorage/Bearer for Phase-B byte-parity (both stacks, same transport). *(recommend
  for the port itself)*
- **(b)** Redesign to httpOnly cookies (+ CSRF strategy, SameSite, refresh-cookie rotation) as part
  of S2.
- **(c)** Hybrid — access in memory, refresh in httpOnly cookie.

**R3 recommendation:** (a) for the port (changing transport mid-strangler breaks the shared-token
seam and forces an FE-lockstep during cutover). Open (b)/(c) as an explicit fast-follow council row —
it is a transport-contract change with real CSRF design, not a port default.

### Q6 🔴 [SEC] Courier refresh — add ADR-0004-style per-location re-check? (T-6 / Q-COURIER-NORECHECK)
Owner refresh re-derives membership live (P-c); courier refresh only re-checks `couriers.status`, not
per-location membership.
- **(a)** Carry the asymmetry (parity). *(safest cutover)*
- **(b)** FIX at port — mirror P-c: re-check active courier_locations membership on refresh; 401 if
  the courier was removed from the location; E2E delta. *(recommend as fast-follow)*

**R3 recommendation:** (a) at cutover, (b) as a fast-follow with an E2E (courier removed-from-location
→ refresh 401). This closes a real revoke-lag but changes behavior → needs its own delta + council.

### Q7 [INFRA] OAuth/Telegram ephemeral state store — Redis vs Pg vs in-proc (decision A19)
Today OAuth state/nonce/PKCE (600s) and the handoff code (60s) and telegram tokens (5min DB) use
Redis/Pg.
- **(a)** Keep Redis (Upstash) — carries an external dependency into the Rust hot path.
- **(b)** Move ephemeral state to Postgres (TTL rows + sweep) — one fewer moving part; matches the
  "hand-rolled SKIP LOCKED + PgListener" queue decision. *(recommend, per REBUILD-MAP A19)*
- **(c)** In-process (single-node) — fails on multi-machine Fly deploys; rejected for OAuth state.

**R3 recommendation:** (b). REBUILD-MAP already flags "Redis state cache → Pg/in-proc per decision
A19"; Pg-backed short-TTL rows remove Redis from the auth critical path and unify durability.

### Q8 [SEC] Google `id_token` — keep unverified-signature (direct-TLS trust) or verify via JWKS?
`auth.ts:105-106` decodes the id_token without signature verification, trusting the direct-TLS fetch
from Google (nonce is checked).
- **(a)** Carry verbatim — documented rationale (TLS to `oauth2.googleapis.com`). *(parity)*
- **(b)** Verify against Google JWKS in the Rust port (defence in depth) — behavior-compatible on the
  happy path, an E2E delta only on tampered tokens.

**R3 recommendation:** (b) is low-risk and strictly stronger; but it is a security *tightening* → do
it with council + a test, not silently. Recommend (a) at cutover, (b) as a named fast-follow.

### Q9 [CONTRACT] Claims schema versioning during overlap
Both stacks verify the same strict discriminated union today. If Rust needs any additive claim (e.g. a
`ver`), how is it introduced without breaking the Node verifier mid-overlap?
- **(a)** Freeze the claim set for the whole overlap — no new claims until Node is gone. *(recommend)*
- **(b)** Additive-optional claims only, both stacks tolerant (but Node's `.strict()` rejects unknown
  claims — this would require a Node change first).

**R3 recommendation:** (a). Node's `.strict()` union makes any new claim a breaking change for the old
verifier. Freeze the claim set until decommission (Phase D); version only after Node is retired.

### Q10 🔴 [INFRA] Live-session migration at cutover
When the auth surface flips from Node to Rust behind the proxy, in-flight owner refresh families,
courier sessions, and customer grants are live in shared tables.
- **(a)** No data migration — both stacks read/write the same tables with the same hashing; a token
  minted by Node verifies on Rust and vice-versa (the load-bearing seam). Flip is a routing change,
  rollback = route back to Node. *(recommend — this is the strangler premise)*
- **(b)** Drain/re-issue — force re-login at cutover (unacceptable UX + defeats the seam).

**R3 recommendation:** (a). This is the whole point of byte-compatible tokens + shared tables. The DoD
must prove: Node-minted refresh rotates correctly on Rust, Rust-minted verifies on Node, sha256 hash
formats are identical, and rollback (route back to Node) leaves every live session valid. **Council
must confirm the hash/encoding parity test is a cutover gate.**

### Q11 [CONTRACT] Duplicate `/dev/mock-auth` vs `/api/dev/mock-auth` collapse (Q-DUP)
Two independently-maintained near-identical bodies (`mock-auth.ts:14` vs `server.ts:549`).
- **(a)** One Rust handler registered at both paths; openapi-diff proves both answer. *(recommend)*
- **(b)** Keep two handlers (carries the "fix-one-miss-other" hazard forward — rejected).

**R3 recommendation:** (a). Behavior-identical, removes a security-fix-divergence hazard. Named as a
structural FIX (not a behavior change) so the parity oracle stays green.

### Q12 [PRODUCT] Password-reset (AUTH-GAP-4 / AR-4) — in or out of S2?
No password-reset flow exists anywhere.
- **(a)** Out of S2 — it's a greenfield feature, its own council + product decision, post-cutover.
  *(recommend)*
- **(b)** Add it during S2 (scope creep on a red-line surface — rejected).

**R3 recommendation:** (a). Backlog it as a named product decision; do not invent it in the port.

---

## Decision-ordering note for the council
Q2 (RETIRE dead activate), Q10 (session migration = the strangler premise), Q11 (dup collapse), and
the four-layer dev-kid segregation (proposal §8) are **port-blocking** — they must be settled before
any code. Q1/Q3/Q5/Q6/Q8 are **fast-follow-eligible** (carry-at-cutover, tighten-after-green) so they
do not block the parity flip. Q4/Q9/Q12 are deferrable to post-Astro / post-decommission.
