# Reversible Cutover-Harness — Council RESOLVE

> **Verdict: PROCEED-WITH-REVISIONS (major). No ETHICAL-STOP (counsel).** Packet-status: **🟡 — NOT
> COUNCIL-APPROVED.** The S1 read-only slice may proceed once the map/matcher CRITs are fixed;
> EVERYTHING past S1 (any write/money/irreversible surface) is gated on the CRIT+HIGH fixes below AND
> on re-ratifying the S2-canon revision through the S2 seats. Seats: architect (packet) · breaker
> (2 CRIT / 5 HIGH / 4 MED / 2 LOW) · counsel (PROCEED-WITH-REVISIONS) · lead (this RESOLVE).

## 1. Frozen revision set

- **REV-C1 (breaker CRIT-2 — phantom map).** The path-ownership map is hand-authored and WRONG
  (`POST /orders`→ really `POST /api/orders`+`/api` prefix `server.ts:151`; deliver/otp mis-pathed).
  REV: **regenerate the map programmatically from the actually-registered routes** (`server.ts`/
  `bootstrap/routes.ts` walk), and make the map-coverage gate assert every registered route maps to
  exactly one surface (the "provable partition" must be machine-derived, not asserted).
- **REV-C2 (breaker CRIT-1 — prefix can't separate families).** S3/S5/S7/S8/S9 all live under
  `/api/owner/locations/:locationId/*`; the surface discriminator is an INFIX after a UUID. REV:
  the resolver keys on a **full (method, path-template) match with `:param` segment templating**, NOT
  longest-prefix. Prove disjointness over the templated set; a request that matches no template
  fails CLOSED to Node (logged as unmapped) — never silently co-flips S5 with S9-erase.
- **REV-C3 (breaker HIGH-1 — flip not atomic; NOTIFY blocked on pooler).** LISTEN/NOTIFY is documented
  blocked on the transaction pooler (`server.ts:220-221`); a pooled LISTEN never wakes → permanent
  1-5s split-brain per flip. REV: flag-change propagation uses a **dedicated session (non-pooled)
  LISTEN connection**, or a bounded-TTL poll with the window treated as real. For money/irreversible
  surfaces the flip must **quiesce** (drain: old stack stops accepting new writes for the window)
  — and the true duplicate guard is the DB `(key, location_id)` UNIQUE (see REV-C7), not the hash.
- **REV-C4 (breaker HIGH-2 + HIGH-3 — cross-stack auth parity = the S2 revision).** (a) `AuthToken`
  is `.strict()` + requires **body-kid** (`legacy.ts:162-174`, Node duplicates kid into body
  `jwt.ts:53`); a Rust minter must preserve body-kid + strict-compatible claims BOTH directions or
  Node rejects Rust tokens — gates S3/S4/S5. (b) G1 parity E2E uses dev-kid tokens accepted only
  non-prod (`jwt.ts:73,91`); the Rust staging deploy must carry the **S2 dev-kid gate**
  (`auth_dev.rs`, `#[cfg(dev-routes)]`+boot-guard-D — already built) so parity specs run WITHOUT
  re-opening the dev-login-backdoor cross-stack. Verification-parity is a hard prerequisite for
  flipping any authed surface ahead of S2.
- **REV-C5 (breaker HIGH-4 — auto-degrade manufactures split-brain).** The per-instance no-consensus
  health circuit-breaker lets instance A (rust) and B (node) serve one surface at once during a flap.
  REV: auto-degrade flips the **global flag** (one consensus action, with hysteresis/debounce), and is
  **DISABLED for money/irreversible surfaces** (S5/S7/S9 degrade only by human go/no-go). One
  metric → one action → one surface (counsel's scope-creep guard).
- **REV-C6 (breaker HIGH-5 — client IP lost across flycast).** The internal `flycast` hop drops
  `Fly-Client-IP` → Rust fails closed to one shared rate-limit bucket → S5 velocity collapses; §8's
  "set XFF" contradicts the never-trust-XFF invariant. REV: the front-door propagates the real client
  IP via a **trusted internal header** that Rust accepts **only** from the internal flycast source
  (source-verified), never from public ingress. Do NOT re-enable XFF trust.
- **REV-C7 (breaker MED — the real duplicate guard).** The S5 cutover gate over-indexes on request-hash
  byte-identity; the actual duplicate-PREVENTION is the `idempotency_keys(key, location_id)` UNIQUE.
  Reframe (shared with S5 REV-S5-2): hash drift → a legit retry FALSE-422s (recoverable), NOT a
  duplicate order. Gate = the UNIQUE holds cross-stack (it does — same table) + minimize false-422 via
  the canonicalization contract. Lowers the money-flip risk class from "double order" to "rejected
  retry."
- **REV-C8 (counsel #1+#2 — reversibility honesty + right route for the S2 revision).**
  (a) Promote the two-tier truth ("reversible ROUTING, not reversible cutover") into the goals + ADR
  title; each irreversible-effect flip (S2/S5/S9) = a distinct human go/no-go with a **pre-authored
  cleanup runbook as a PRECONDITION**, not an afterthought. (b) The Q3/Q4 revisions overturn a
  **signed unanimous 4-seat S2 decision** — they must be ratified as a **superseding amendment in the
  S2 record, re-consulting the S2 seats (esp. the S2 breaker)**, NOT settled inside this 2-seat
  mechanism council. S3's "atomic is fine" reasoning ("catalog = edit-session") does NOT transfer to
  auth — re-argue it on auth's terms.
- **REV-C9 (counsel #3+#4 — Goodhart + the person).** S1's read-only DoD proves ROUTING, not the
  safety machine (write-parity, idempotency, trip-wire, cleanup runbook). REV: **fire-drill the
  trip-wire + cleanup runbook on S3 under synthetic divergence BEFORE S5** — never let S5 be the first
  real fire. The flip-instant response to an in-flight person must be **truthful + retry-safe** (never
  "order failed" when it may have succeeded — the soft-confirm rule at the flip instant); courier
  in-flight state must survive the S6 WS reconnect (flip in a low-delivery window); owner
  read-after-write staleness closed visibly.
- **REV-C10 (counsel #5 — the un-priced lock-in).** The front-door shim has no Phase-D cut-trigger and
  no owner → the "temporary" vine becomes the permanent incumbent (the exact lock-in the rebuild exists
  to escape). REV: give the shim a **dated cut-trigger + named owner NOW**, recorded in the ADR.
- **REV-C11 (breaker LOW — undici 10s vs Rust 30s).** The 10s undici timeout can report a COMMITTED
  order as failed (Rust still processing at 10s). REV: front-door timeout ≥ the surface's real
  server-side budget; on timeout, the response must be retry-safe (REV-C9), never a bare failure.

## 2. Disposition
| Finding | Sev | Disposition |
|---|---|---|
| CRIT-1 prefix | CRIT | REV-C2 (template matcher) |
| CRIT-2 phantom map | CRIT | REV-C1 (machine-generate the map) |
| HIGH-1 atomic | HIGH | REV-C3 (session LISTEN + money-quiesce) |
| HIGH-2 JWT parity | HIGH | REV-C4a (= S2 C1; prerequisite) |
| HIGH-3 dev-kid oracle | HIGH | REV-C4b (S2 dev-kid gate on Rust) |
| HIGH-4 auto-degrade | HIGH | REV-C5 (global+hysteresis; off for money) |
| HIGH-5 client IP | HIGH | REV-C6 (trusted internal header) |
| MED ×4 | MED | REV-C7 + register (onboarding two-writer, refresh-split window, LISTEN conn budget) |
| LOW ×2 | LOW | REV-C11 + break-glass-not-instant note |
| Counsel process/lock-in/person | — | REV-C8/C9/C10 |
Confirmed sound: **S1 read-only holds — zero writes in the public read routes** (regression baseline).

## 3. 🔴 OPERATOR SIGN-OFF REQUIRED
1. **S1-first proof MAY proceed** once REV-C1 (real map) + REV-C2 (template matcher) land — read-only,
   staging, shadow-diff → flip → parity + sub-second rollback + chaos-degrade → then prod.
2. **Everything past S1 is gated** on REV-C3..C7 (atomicity, auth parity, client-IP, dedup reframe).
3. **The S2-canon revision (Q3/Q4) goes back to the S2 seats** as a superseding amendment (REV-C8b) —
   not ratified here.
4. **Phase-D cut-trigger + owner recorded now** (REV-C10).
5. **Each money/irreversible flip = human go/no-go + pre-authored cleanup runbook** (REV-C8a/C9).

## 4. Build ordering (post-sign-off)
Deploy Rust staging dark → REV-C1/C2 map+matcher (buildable now, non-routing) → S1 proof → S3
trip-wire/cleanup fire-drill (REV-C9) → S2 re-ratification → authed surfaces (REV-C4) → S5 (REV-C7 +
S5 RESOLVE) → S6 WS → … → Phase-D decommission on the REV-C10 trigger.
