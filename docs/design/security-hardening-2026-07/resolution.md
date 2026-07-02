# Resolution — Security hardening batch (RESOLVE round, 2026-07-02)

Round: Architect resolves Breaker (`breaker-findings.md`) + Counsel (`counsel-opinion.md`) against the
FIX design (`proposal.md` + `ADR-security-hardening-2026-07.md`). Each breaker finding →
**fix** / **accept-risk** / **defer-flag**; each counsel point → **revise** or **flag for human**.
Proposal + ADR updated in lockstep. No production code (design-time).

---

## A. Breaker findings — disposition

### B1 [HIGH] Guardrail cannot prove isolation, cannot detect #7, excludes `locations` → **FIX**
The Breaker is correct: a substring sweep for `WHERE location_id` proves a *token is present*, not that the
*bound value is authorized*. `WHERE id=$1 AND location_id=$2` with `$2 = request.body.locationId` (the #7
class) passes GREEN; the JOIN shape `m.location_id = o.location_id` (WS order-room) satisfies the substring
without being a tenant filter; and `locations` (the actual #7 table, `couriers.ts:25`) was not in the scanned
set. **The DoD was resting on a gate that cannot certify the property it claimed.**

Resolution — restructure the DoD:
1. **Primary guardrail = per-finding BEHAVIORAL red→green tests** (real proof of the property):
   - #1 owner: owner-A token → `GET /orders/:id` for an owner-B order → **404**; owner-A → own order → 200.
   - #1 courier cross-tenant: courier bound at location X → order at location Y → **denied**.
   - #1 courier insider-window (B2): courier whose `courier_locations`/session is revoked → own-location
     order → **denied** (not just cross-tenant).
   - #7: owner-A posts `/couriers/invites` with owner-B's `locationId` in the body → **404**, no
     `courier_invites` row created; owner posts own `locationId` → invite created.
   - #4/#6/#5/#8/#9: the behavioral assertions already enumerated per finding.
   These are the DoD. A finding is not "done" until its behavioral test is proven red→green.
2. **Static scan = cheap secondary lint only.** Reframe its claim to *"flags obviously unscoped
   `SELECT/UPDATE/DELETE` on high-value tables (the pure `WHERE id=$1`-with-nothing-else omission)"* —
   explicitly **not** "proves tenant isolation." Add `locations` (and `couriers`, `courier_invites`) to the
   scanned table set. Stop claiming it proves #7; it is an anti-omission net, nothing more.
3. **Goodhart / escape-hatch discipline (Counsel §3.6):** any *new* `-- no-location-id` escape comment
   requires a named reviewer (enforced in the guardrail's baseline-diff, not aspirational) — the escape
   comment is the exact seam the class re-enters through.

Proposal §10 and ADR §Decision-3 rewritten accordingly.

### B2 [HIGH] #1 courier insider-removal read window (PII) open until token TTL → **FIX**
Correct and asymmetric with #4/#6. `GET /orders/:id` uses `softVerifyAuth` → no courier session/
`courier_locations` recheck; scoping to the **baked** `activeLocationId` lets an ex-courier read every live
order's `delivery_address`/phone at location X for up to 14d. Resolution:

- **Fold the live recheck into the authorizing read itself** (single round trip, closes both cross-tenant
  AND insider-window):
  - **Owner branch** → one query that authorizes-by-JOIN:
    `SELECT o.* FROM orders o JOIN memberships m ON m.location_id = o.location_id
     WHERE o.id = $1 AND m.user_id = $2 AND m.role='owner' AND m.status='active'` → 404 on no row. This
    live-checks active membership (ADR-0004), works for **multi-location** owners (fixes LOW-multiloc, see
    B-LOW-2), and does not trust the baked `activeLocationId`.
  - **Courier branch** → authorize-by-live-binding, reusing the ADR-0013 primitive
    (`courierReadVerdict(db, sub, activeLocationId, orderId)` — the same liveness the fan-out guard uses):
    require an `ALLOW` verdict (which checks a live `courier_assignments`/binding for THIS order) before
    returning the row; `UNAVAILABLE` → retryable 503, deny → 404. This closes the insider-removal window
    *and* the cross-customer read (OR-3) in one move, because binding-scoping is strictly narrower than
    location-scoping.

This makes #1 symmetric with #4/#6 (all three now do a live recheck, not a baked-claim trust). The courier
branch upgrade also **absorbs OR-3** (courier cross-customer read) into Tier 1 rather than deferring it —
see Counsel §3.2. Tier 1, no split needed; the insider-window is not a separate deferred OR-2.

### B3 [HIGH] #4 WS revocation enforced at subscribe only; fan-out never re-authz's owners → **FIX (residual stated)**
Correct. `ownerCanAccessRoom` runs once in the `subscribe` handler (`websocket.ts:215`); the fan-out
(`subscribeToRoom` handler, `:38-49`) routes every member through `relayGuard.relay(...)` which **only
re-validates couriers** (`relayGuard.check = courierReadVerdict`, `:64`). Owners/customers relay directly →
a persistent owner socket keeps streaming after revocation until disconnect. `status='active'` at subscribe
only gates *new* subscribes.

Resolution — **extend the existing fan-out guard to owners**, mirroring the courier pattern that already
exists:
- Add an owner re-validation to the fan-out path: before relaying an `order:`/`location:` frame to an
  **owner** member, re-check active owner membership (the same `ownerCanAccessRoom` query, cached with a
  short TTL like the courier guard) and **evict on fail** via the existing `evict` mechanism
  (`:65-79` — drop from room + `binding_revoked` notice, not socket-close).
- **Residual, stated honestly:** this bounds the window to **≤ the guard's re-check TTL** (the ADR-0013
  courier posture — "stops receiving within ≤TTL"), **not literally zero**. Zero would require a
  push-based revocation signal (emit a `membership_revoked` event on the owner's rooms at the moment
  `memberships.status` flips) that drops the socket immediately. Recommendation: ship the TTL-bounded
  fan-out re-authz now (parity with couriers, small change), and file the push-based zero-window signal as
  **OR-9 (tracked follow-up)**. The proposal will no longer claim #4 closes the window to zero — it closes
  it to ≤TTL at the fan-out, which is the same guarantee couriers already have.

### #9 [MED] Global `Fly-Client-IP` keyGenerator rekeys the whole auth brute-force surface onto a trusted header → **FIX (constrained) + operator precondition**
Correct that the stakes jump: `auth.ts` login/OTP/reset limiters carry no `keyGenerator`, so they inherit
the global key — setting it to `clientIp()` rekeys every brute-force limiter onto `Fly-Client-IP`. If that
header is spoofable on *any* reachable ingress (fly-replay, 6PN/private-network, WS upgrade, a future
direct route), header rotation fragments every limiter → brute-force evasion on money/auth. Resolution:
- **Constrain trust to the edge.** `clientIp()` must only trust `Fly-Client-IP` when the connection is
  actually from the Fly edge. Since the app cannot cryptographically verify the edge, this becomes an
  **operator precondition**: confirm the Fly edge sets/overwrites `Fly-Client-IP` on **all** ingress paths
  (HTTP, the WS upgrade, health, internal) and that no non-edge ingress can reach the app with an
  attacker-set header. Record the answer. **Never** trust `X-Forwarded-For`.
- **Robustness:** handle header-absent (already fail-closes to `shared:no-fly-ip` in prod) and **IPv6**
  (normalize; do not let `::ffff:` prefixes or casing fragment buckets).
- **Testability fix:** the fail-closed path is gated on `NODE_ENV==='production'` — the #9 guardrail case
  (b) is vacuous outside prod. The behavioral test must **force production mode** in the harness so the
  spoof-falls-to-shared-bucket assertion is real, not green-by-accident.
- **Scoping if the precondition is unconfirmed:** if the operator cannot guarantee edge-only ingress,
  **do not rekey the auth limiters onto the header** — keep them on `request.ip` (Fly socket, not
  client-controllable) and apply the `clientIp()` key only to the funnel/access-gate routes already using
  it. i.e. #9's global rekey is **conditional on the ingress guarantee**; the false-429 collapse fix must
  not buy a spoof-evasion hole on login/OTP. Marked in the ADR as operator-gated.

### #5 [MED] URL-token removal breaks cached PWA/service-worker WS clients → **FIX (deprecation window) + puddle remediation**
Correct — "FE-first" does not protect already-installed SW-cached clients (`apps/api/public/sw.js`); they
keep connecting with `?token=` and get `auth_timeout`-closed at 5s → silent WS lockout. Resolution:
- **Dual-accept deprecation window:** keep BOTH the `?token=` path AND message-auth working; **log**
  `?token=` usage (count, role, no token value); publish a SW cache-bust so installed clients update;
  remove the URL path **only after** access-log usage of `?token=` hits zero. During the window, the log
  redaction (drop `sub`/`role`) still applies, reducing (not eliminating) the leak surface.
- **Puddle remediation (Counsel §5) — OR-8:** closing the faucet does not drain the puddle. Bearer tokens
  (24h–14d) already written to Fly access logs / Referer / history during the `?token=` era are **valid
  now**. Name it: **owner = Operator + Security**; **action** = (1) rotate the JWT signing key (invalidates
  all outstanding bearer tokens — forces re-auth) *or* forced session/refresh invalidation for owner +
  courier roles, and (2) scrub `?token=` from historical Fly access logs. Decide rotate-vs-invalidate at
  council; this is **in scope as a named follow-up**, not silently omitted.

### #2 [MED] Anon-read GUC inventory incomplete → the flip 404s customer order-tracking → **FIX (handoff completeness) + anti-orphan**
Correct that the *Tier-1 sequencing is sound* (inert under bypass, no window opens) but the *handoff list to
B3 is wrong*. The customer `GET /orders/:id` reads on the **raw pool** (`orders.ts:762`, not `withTenant`,
no GUC); spa-proxy order reads and any status-poll are the same. Under NOBYPASSRLS with C1 narrowed, every
such path returns 0 rows → **customer tracking 404s for everyone at the flip** — the KNOWN TRAP. Resolution:
- **Complete the anon-read-path inventory before the flip:** enumerate EVERY path that reads the fail-open
  tables (`orders`, `order_items`, `customers`, `idempotency_keys`) without seating a GUC — customer
  `GET /orders/:id`, spa-proxy order reads, status-poll, track-exchange, any public order view — and assign
  each a scope-GUC (or route through the narrow definer, see next). This inventory is a **B3 Phase-2
  precondition**, added to `remediation-plan.md`'s worker table as an *anon-read* companion table. The
  flip-gate E2E must cover the *status-read* path, not just checkout.
- **Prefer the definer mechanism (Counsel §3.4):** nudge B3 to narrow C1 via a `SECURITY DEFINER` scoped by
  order-id/token-hash (pinned search_path — the same primitive #3 pins and the guardrail guards) rather
  than a *new* per-request `app.anon_order_id` GUC, which adds a fresh "forgot-to-set → fail-open" surface.
  One isolation primitive, not two.
- **Anti-orphan artifact NOW (Counsel §3.1) — the load-bearing fix:** #2 already died once in migration 077.
  From the moment Tier 1 ships, land a **tracked failing-or-pending artifact** that cannot be silently
  closed: (a) a `docs/regressions/REGRESSION-LEDGER.md` red-line row "C1 anon SELECT/UPDATE fail-open —
  narrow before NOBYPASSRLS flip"; (b) a **skip-registered, named** `verify:rls` probe
  (`test.skip('C1 anon policies fail-closed under dowiz_app', …)`) that is impossible to remove without
  narrowing C1; and/or (c) a `plane-guard.mjs` pending check. The gate must exist *before* the fix, not
  arrive bundled *with* it (which is circular — it cannot guard the interim).

### #3 [MED] proconfig probe lives in `verify:rls`, not in CI `verify:all` → **FIX (CI wiring) + honest limitation**
Correct: `verify:rls` is DB-backed (`--env-file=.env`) and NOT in `ci.yml`'s `verify:all --ci`. So the
"runtime probe" shares the exact skip-gated limitation §10 criticized. Resolution:
- The **static** definer gate (`guardrail-definer-search-path.mjs`, already `ci:true`) is the
  continuously-enforced **regression** net — a *new* migration re-`CREATE OR REPLACE`-ing the fn unpinned
  lands in a new file the gate catches. That is sufficient for *preventing new offenders* and is honestly
  CI-continuous.
- The **runtime** "prod actually has the pin" check cannot run in a DB-less CI job. Wire it where a DB
  exists: (a) into the **staging deploy validation** (the ship-discipline step 3 already runs against a
  live DB) and/or (b) as a **boot-guard** assertion at API start (like the pool-role boot-guard) that
  FATAL-exits if `app_member_location_ids` lacks a pinned `search_path`. Reframe §10: the static gate is
  CI-continuous for regressions; the runtime pin is verified at deploy + boot, not in DB-less CI. State
  this limitation honestly rather than implying CI proves the live pin.

### B-LOW-1 [LOW] #1 back-of-envelope wrong: `getOwnerLocationId` is a separate pool checkout → **FIX (doc) — moot after B2**
Correct as originally written. But the B2 resolution folds authorization into the read query (owner JOIN /
courier verdict), so the owner path is **one** `withTenant` checkout (the JOIN authorizes inside it), not
two. §2 corrected: the owner path no longer does a separate `getOwnerLocationId` pre-checkout for
`GET /orders/:id`. #6 (spa-proxy) still adds one recheck per owner request — noted accurately (it has ~14
call-sites through `getLocationId`/`getOwnerContext`, each gaining one indexed read; still minor vs pool 20,
but stated as +1 checkout, not "on the held path").

### B-LOW-2 [LOW] #1/#6 scope multi-location owners to single `activeLocationId` → 404 on own other-location order → **FIX**
Correct functional regression. Resolved by the B2 owner-JOIN form (`m.user_id=$2` across all their active
memberships, not a single baked location) — a multi-location owner reading their own order at any of their
locations gets 200. For #6 (spa-proxy writes), the resolver must likewise verify membership for the
*target* location, not only the baked one; where the operation is inherently single-location (settings for
the active location), the baked-location live-recheck is correct. Enumerated in the proposal now.

---

## B. Counsel points — disposition

| # | Counsel point | Disposition |
|---|---------------|-------------|
| §2 | Reframe pool-role as a **LIVE-EXPOSURE gate**: if NOBYPASSRLS today, #2 is a live table-wide PII siphon → **promotes to Tier 1** | **REVISE** — OR-1 + ADR Open-decision rewritten; drop "Tier 1 ships regardless" unqualified |
| §3.1 | Anti-orphan artifact for #2 must exist **from the moment Tier 1 ships** | **REVISE** — ledger row + skip-registered named probe (see #2 above); added to DoD |
| §3.2 | Elevate courier cross-customer read (OR-3) from footnote to tracked PII-minimization item | **REVISE** — absorbed into Tier 1 via the B2 courier binding-scoping fix (`courierReadVerdict`); OR-3 upgraded from "accept follow-up" to "fixed in Tier 1" |
| §3.3 | Time-box the dual-authority (M1) transitional state to a B3 trigger | **REVISE** — ADR: dual-authority is valid ONLY until the B3 flip; the anti-orphan artifact (#2) is the trigger that forces closure |
| §3.4 | Prefer the definer mechanism for #2 (reuse, not a new GUC) | **REVISE (nudge B3)** — recorded as the recommended #2 mechanism |
| §3.5 | Per-finding recorded decision, not one batch stamp; record human decision per tier + explicitly on OR-1 (pool) and the courier accept | **FLAG FOR HUMAN** — ADR marked APPROVED-pending-operator; per-tier + OR-1 sign-off required, not a single stamp |
| §3.6 | Enforce escape-hatch reviewer (Goodhart) on new `-- no-location-id` comments | **REVISE** — folded into B1 guardrail spec |
| §4 | Steel-man Option B (one atomic B3) | **ACKNOWLEDGE** — C still chosen (urgency + flip blast radius), but C is defensible ONLY with the #2 anti-orphan mechanism (§3.1). "Adopt C, pay its premium" recorded in ADR Consequences |
| §5 | Remediate already-leaked `?token=` credentials (the puddle) | **REVISE** — OR-8 named (owner = Operator+Security; rotate-or-invalidate + log scrub); in scope as named follow-up |
| §5-sec | Idempotency replay under shared IP / #8 key-shift window | **ACCEPT (benign, checked)** — phone is in the `requestHash`; #8 adds a stable per-customer id → strictly *more* unique, fewer collisions; the one-time key shift creates only fresh keys (no stored-row invalidation, no cross-party response). No security window. |

No ETHICAL-STOP was raised (Counsel §2 = 0 stops). The one grounded precondition (pool-role as live-exposure
gate) is handled as a hard human gate, not overridden.

---

## C. STOP-DESIGN-B — the build handoff

### Tier-1 build order (each with its behavioral guardrail as DoD)
1. **#3** pin `app_member_location_ids()` search_path (forward-only migration) + boot-guard/staging runtime
   proconfig assertion. *(independent, safe both pool cases)*
2. **#8** customer `sub` as throttle/idempotency identity. *(pure, unit-tested)*
3. **#7** explicit membership predicate on body `locationId` in `/couriers/invites`. *(behavioral: cross-tenant invite → 404)*
4. **#1** owner-JOIN authorizing read + courier binding-scoped read (`courierReadVerdict`) — absorbs OR-3 +
   B-LOW-2. *(behavioral: owner-B order → 404; ex-courier → denied; multi-loc owner → 200)*
5. **#6** spa-proxy resolvers routed through live ADR-0004 recheck (target-location membership). *(behavioral: revoked owner write → 401/404)*
6. **#4** WS `order:` subscribe `status='active'` + **fan-out owner re-authz** (mirror courier guard,
   ≤TTL residual). *(behavioral: revoked owner stops receiving frames within ≤TTL)*
7. **#5** dual-accept `?token=` + message-auth, log usage, redact logs; removal deferred until usage→zero. *(behavioral: message-auth works; ?token= logged)*
8. **#9** `clientIp()` global keyGenerator **iff** operator confirms edge-only ingress; IPv6-normalized;
   test in forced prod mode. Else scope to funnel routes only. *(behavioral, prod-forced: distinct Fly-Client-IP → distinct buckets; spoofed XFF → shared bucket)*
9. **Guardrail restructure (B1):** behavioral tests as primary DoD; static scan demoted + `locations`/
   `couriers`/`courier_invites` added + escape-hatch reviewer enforced; wire the static scan into
   `verify:all` `ci:true`.
10. **Anti-orphan for #2 (ships WITH Tier 1, before the #2 fix):** REGRESSION-LEDGER red-line row +
    skip-registered named `verify:rls` probe that cannot close until C1 narrows.
11. SAFE riders: `pnpm.overrides tmp@>=0.2.6` (operator, protect-path).

### Accepted risks + owners
| ID | Risk | Disposition | Owner |
|----|------|-------------|-------|
| OR-1 | Live pool role unconfirmed | **LIVE-EXPOSURE gate** — if NOBYPASSRLS today, #2 promotes to Tier 1; batch not "PII-resolved" without recorded confirmation | Operator + DB owner |
| OR-4 | #2 must land with courier/anon GUC seating + complete anon-read inventory before flip | Sequencing contract + anti-orphan artifact | DB owner |
| OR-5 | Connect-guard rejects only literal `postgres`, not BYPASSRLS | Harden to `rolbypassrls` check | DB owner |
| OR-8 | Already-leaked `?token=` bearer tokens valid now | Named follow-up: rotate/invalidate + log scrub | Operator + Security |
| OR-9 | #4 residual = ≤TTL, not zero | Push-based `membership_revoked` socket-drop as follow-up | Architect |
| M1 | Dual-authority (app-predicate + inert-RLS) transitional | Time-boxed to B3 flip; anti-orphan artifact forces closure | Architect |
| #9-ingress | Fly-Client-IP trust across all ingress | Operator confirms edge-only; else scope narrower | Operator |

### Threat-model items to carry into the build as tests
- Cross-tenant order read (owner + courier) → 404. Cross-tenant invite → 404.
- Insider-removal: revoked owner (WS subscribe + fan-out + spa-proxy write); ex-courier order read.
- Within-tenant PII minimization: courier reads only assigned-binding orders (not all venue orders).
- WS: `?token=` logged during window; message-auth works; revoked owner evicted from fan-out ≤TTL.
- Rate-limit (prod-forced): distinct Fly-Client-IP → distinct buckets; spoofed XFF → single shared bucket.
- Flip-gate (B3, deferred): C1 fail-closed under `SET LOCAL ROLE dowiz_app`; customer status-read path
  returns its own order with the scope GUC set (not 404).

### ADR status
**APPROVED-pending-operator** — all three HIGH (B1 guardrail, B2 courier insider-window, B3 WS fan-out)
resolved in design; remaining gate is the operator's recorded **pool-role confirmation (OR-1, live-exposure
gate)** + the **#9 ingress guarantee**. Per-tier + per-OR sign-off required (not a single batch stamp).

---

## D. Resolution table (finding → disposition)

| Finding | Breaker/Counsel | Disposition |
|---------|-----------------|-------------|
| B1 guardrail too weak | HIGH | **FIX** — behavioral tests primary DoD; static scan demoted + tables fixed |
| B2 #1 courier insider-window | HIGH | **FIX** — courier branch live binding-recheck (`courierReadVerdict`) |
| B3 #4 WS fan-out no re-authz | HIGH | **FIX** — extend fan-out guard to owners; residual ≤TTL (OR-9 for zero) |
| #9 rate-limit header trust | MED | **FIX (constrained)** — edge-only ingress precondition + IPv6 + prod-forced test |
| #5 cached-client lockout | MED | **FIX** — dual-accept deprecation window; + OR-8 puddle remediation |
| #2 anon-read inventory | MED | **FIX** — complete inventory + anti-orphan artifact now + definer mechanism |
| #3 non-CI runtime probe | MED | **FIX** — static gate = CI regression net; runtime pin at deploy/boot (honest limitation) |
| #1 BOE two checkouts | LOW | **FIX (doc)** — moot after B2 owner-JOIN (one checkout) |
| #1/#6 multi-loc owner 404 | LOW | **FIX** — owner-JOIN across all active memberships |
| Counsel §2 pool live-exposure | precondition | **REVISE** — OR-1 reframed as live-exposure gate |
| Counsel §3.1 anti-orphan | advice | **REVISE** — artifact ships with Tier 1 |
| Counsel §3.2 courier PII | advice | **REVISE** — absorbed into Tier 1 (B2) |
| Counsel §3.3 time-box M1 | advice | **REVISE** — boxed to B3 trigger |
| Counsel §3.4 definer for #2 | advice | **REVISE** — recommended to B3 |
| Counsel §3.5 per-finding sign-off | advice | **FLAG FOR HUMAN** — per-tier + OR sign-off |
| Counsel §3.6 escape-hatch | advice | **REVISE** — reviewer enforced |
| Counsel §5 leaked tokens | open Q | **REVISE** — OR-8 named |
| Counsel §5-sec idempotency | open Q | **ACCEPT** — benign (phone in key; more unique) |

**HIGH still open: none.** All three HIGH resolved in design. Remaining gates are human/operator, not
architectural: **OR-1 pool-role confirmation (live-exposure)** and **#9 ingress guarantee** — both routed
to the operator, both required before the batch is declared PII-resolved / before #9's global rekey ships.
