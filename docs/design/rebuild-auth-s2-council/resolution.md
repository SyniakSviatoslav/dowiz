# S2-AUTH Port — Council RESOLUTION

- **Date:** 2026-07-04 · **Surface:** S2 auth 🔴 (REBUILD-MAP §3 Phase B strangler) · **Lane:** R3
- **Seats reported:** `system-architect`, `system-breaker`, `counsel`, decorrelated `security-sentinel`.
- **Inputs:** `proposal.md`, `threat-model.md`, `open-questions.md`, `packet-status.md`,
  `architect-review.md`, `breaker-findings.md`, `counsel-opinion.md`.

## VERDICT: **PROCEED-WITH-REVISIONS — CONDITIONALLY RATIFIED, NOT YET `COUNCIL-APPROVED`.**

The port design is sound; the crypto core (RS256 double-pin, dev-kid crypto-segregation, PII-clean
customer token) ports cleanly and every load-bearing packet claim was verified against live source.
**No ETHICAL-STOP.** But the two seams the packet itself calls load-bearing — strict-claim parse and
Q10 cross-verifiability — are under-specified, and the breaker demonstrated a live break in each. The
status ratchet stays at **`MAPPED`**; it advances to **`COUNCIL-APPROVED`** the instant the frozen
revision set below is folded into the packet **and** the operator signs the 🔴 items. No AUTH row
moves to `BUILT` before that.

## Convergence (where ≥3 seats agree — highest confidence)

1. **RETIRE `/api/auth/courier/activate` — UNANIMOUS, hard port-blocker.** All four seats independently:
   security-sentinel (HIGH), counsel (condition 1), architect (Q2 RATIFY-RETIRE), breaker (H2). It is
   dead (0 callers / 0 E2E), writes a **courier** refresh into the **owner** table (a live-reachable
   privilege-escalation via `/auth/refresh` role-rotation, only partially mitigated by ADR-0004 P-c),
   **and** mints a token its own `.strict() CourierClaims` cannot represent (unverifiable). Carrying it
   verbatim is strictly more dangerous than deleting it — the one place carry-verbatim is the wrong
   default. **Disposition: RETIRE with proof-of-deadness (matrix RETIRE row), re-verified at port time.**

2. **Q10 is not "byte-compatible" — it is cross-verifiable + identical-claim-shape + identical-hash-format
   + body-`kid` round-trip.** Architect R2 + breaker C1 (CRITICAL) + counsel condition 2 converge. The
   single sharpest finding (breaker **C1**): `kid` is a **required body claim** (`legacy.ts:162`), written
   to the body (`jwt.ts:50-54`) but consumed for key-select only from the **header** (`jwt.ts:87,94`). An
   idiomatic Rust `jsonwebtoken` mint puts `kid` in the header only → **Rust-minted tokens fail Node's
   `.parse()`** (missing required body claim) and, if the Rust struct mirrors `.strict()`, **Node-minted
   tokens fail on Rust** as an unknown/edge field. Either direction destroys the Q10(a) rollback premise.

3. **Cutover is irreversible where it deletes.** Counsel condition 2 + architect Q10 gates: a
   refresh-family DELETE from a parity-seam bug evicts a working vendor mid-shift and **routing back to
   Node does not un-delete the family**. The cutover must be a canary flip gated on the
   family-revocation-rate matching the Node baseline, not a hard switch.

## Port-blocking questions — resolved on the record

| Q | Resolution | Authority |
|---|---|---|
| **Q2** RETIRE courier-activate | **RATIFIED RETIRE** (unanimous). Re-verify proof-of-deadness at port. | council + operator sign-off (🔴) |
| **Q10** session migration | **RATIFIED (a) no-migration**, conditional on the four cutover gates (i)–(iv) below + the body-`kid` round-trip gate (C1). | council + operator go/no-go (🔴) |
| **Q11** dup mock-auth collapse | **NOT ratified as drafted.** Breaker H4: the two handlers diverge in 4 concrete ways (`fresh` mode, `locationSlug`, owner auto-membership, courier default location) and the named `openapi-diff` gate is **blind** because `fresh` isn't in the YAML. **Re-scope:** collapse only after the 4 divergences are captured in the contract + an E2E rides each path; otherwise keep two handlers. | council (send-back) |
| **dev-kid segregation** | **RATIFIED** — four ADR-0003 layers + `#[cfg(feature="dev-routes")]`; primary lock is key-material isolation (prod holds no dev pubkey), NODE_ENV/cfg are belt. Make the compile-out a **proven release-artifact test**. | council |

**Q10 cutover gates (named, all four required before flip):** (i) encoding contract E1–E5 proven both
directions incl. body-`kid`; (ii) hash-format parity proven by rotating a Node-minted refresh token on
Rust (sha256 of hex/base64url **string**, not raw bytes); (iii) `kid` + claim-set **frozen** for the
whole overlap (no rotation mid-overlap without dual-kid verify shipped in both stacks first — ties Q1/Q9);
(iv) cross-stack concurrent-refresh proven (safe via the shared-DB atomic UPDATE **iff** both stacks use
identical refresh SQL, incl. the `interval '5 seconds'` window — the SQL is authority over the stale "10s"
comment).

## Frozen revision set — MUST fold into the packet before code (each an architect/breaker requirement)

- **REV-1 (architect R1, new finding):** the CourierSession extractor must re-check live
  `courier_locations` membership (`has_location`) on **every request** and key on `(jti, activeLocationId,
  sub)` — the live bind (`plugins/auth.ts:24-30,76-83`) does. Dropping `has_location` silently regresses
  courier per-location revocation from ~1 request to the full 14d/24h TTL while happy-path E2E stays green.
  Add the design + an E2E (courier removed-from-location mid-session → next request 401).
- **REV-2 (breaker C1):** body-`kid` round-trip becomes an explicit cutover gate (see Q10-i).
- **REV-3 (breaker H3 / WS-lane A1 — GOVERNANCE HOLE):** customer-token **order-scope drift** —
  `orders.ts:752` binds the token's `orderId` claim (per-order) but `customer/orders.ts:50` binds
  `customer_id = sub` (customer-wide). A per-order 14d tracking link (`?t=`, referer-leakable) yields
  whole-customer read/cancel/rate. **Absent from the quirk register AND the threat-model**, so the
  packet's carry-vs-FIX rule can't be applied to it. Add it as a quirk row + threat row; **disposition
  FIX-IN-PORT** (unify `Claims<Customer>` authorization to the minted `(orderId, locationId, sub)` tuple)
  with an E2E delta (token for order A → cancel order B must 403). Same finding the WS-authz council
  (Q3) raised — resolve them together.
- **REV-4 (architect R3):** the middleware tower must pin rate-limit-vs-bearer-gate order (429-vs-401
  precedence), the OPTIONS/preflight short-circuit (`server.ts:406`), and the `NO_AUTH_PATHS`+OTP-bypass
  node (`server.ts:417-420`) — each a named test vector.
- **REV-5 (breaker H4):** the Q11 collapse premise is false as drafted — re-scope per the table above
  (capture `fresh`/`locationSlug`/owner-auto-membership/courier-default divergences before collapsing).

## Fast-follow-eligible (carry-at-cutover, tighten-after-green — do NOT block the flip)

Q1(a) rotation runbook + open dual-kid as fast-follow · Q3(a)→(b) courier 14d→24h after green ·
Q5(a) localStorage for the port, **but** counsel re-orders the httpOnly queue: the **customer/track
token** (an unconsented 7d bearer in storefront localStorage keyed to the holder's own address) is the
ethically-weighted head, ahead of the owner token → open Q5(c) refresh-in-httpOnly as the first
fast-follow · Q6(a)→(b) courier per-location re-check on refresh · Q8(a)→(b) Google id_token JWKS verify.

## Deferrable

Q4 error-shape → post-Astro FE-lockstep · Q9 claim versioning → post-decommission (freeze until Node gone)
· Q12 password-reset (AUTH-GAP-4) → own product council, out of S2. **Counsel condition 3:** every deferred
accepted-risk row (AR-4/AR-5/AR-6) gets a named owner + a trigger, so deferral can't become permanent by
inattention.

## What clears the gate to `COUNCIL-APPROVED` (and then `BUILT`)

1. REV-1…REV-5 folded into `proposal.md`/`threat-model.md` (docs — lead can do; REV-3 also updates the
   quirk register + adds the FIX-IN-PORT E2E delta).
2. **Operator sign-off (🔴, human-gated — cannot self-approve):** Q1, Q2 (RETIRE), Q5, Q6, Q10 go/no-go,
   the AR-6 key-rotation runbook, and the cutover canary plan. (packet-status §"Proposed council
   composition" reserves these for the operator seat.)
3. Then the AUTH rows flip `MAPPED → COUNCIL-APPROVED` in `traceability.csv`, and only then may a Rust
   S2 lane start — building to the cutover DoD (empty openapi-diff · auth E2E slice green · AUTH
   invariant-cluster red→green incl. the RS256 double-pin vector, body-`kid` round-trip, courier
   has_location revocation, customer-scope 403 · cross-stack hash/encoding parity · canary rollback plan).

---
*This resolution is the council record. It ratifies a design direction and freezes a revision set; it
does not authorize code. The `BUILT` transition is a separate, human-gated act recorded against
`traceability.csv`.*
