# S10-PLATFORM-ADMIN/PROVISIONING — Council RESOLVE (the LAST surface)

> **Verdict: PROCEED-WITH-REVISIONS. No ETHICAL-STOP (counsel).** Packet-status 🟡 — NOT
> COUNCIL-APPROVED until operator signs §3, incl. the HARD Phase-D gate (§3.Q5b). Seats: architect
> (packet) · breaker (1 CRIT / 1 HIGH / 2 MED / 1 LOW) · counsel (PROCEED-WITH-REVISIONS) · lead.
> The B4/P6 controls (3-role enum un-forgeable, SELECT-only allowlist, fail-closed gate, timing-safe
> ops-secret, provision ordering/GUC, CONTACT_REQUIRED, no restore-to-prod, fail-loud backup key) are
> all carried faithfully (verified, no regression). Two items need fixing; one is a factual correction.

## 1. Frozen revision set

- **REV-S10-1 (breaker C1 CRIT — the plane-gate must be RUNTIME, not compile-time).** Node's
  zero-detection fail-safe is a runtime matched-pattern root hook (`platform-admin.ts:76-83`, keyed on
  `routeOptions.url`). The packet's substitute (nested `Router` + clippy tripwire + one sibling-closure
  test) is STRICTLY WEAKER: the lint fires only on a literal `prefix:'/api/admin'`
  (`eslint-plugin-local:630-645`); ordinary axum `.nest`/`.merge` yields an `/api/admin/*` handler with
  no matchable literal → a FUTURE admin route ships UNGATED = cross-tenant breach. REV: build a
  **runtime** plane-gate — an axum middleware/extractor (or a catch-all fallback on the admin router)
  that checks the path is platform-admin-scoped AND the caller is on the `platform_admins` allowlist,
  at REQUEST time, 403 by default. Prove closure BY ATTACK: an ungated/unknown `/api/admin/*` path →
  403 (a test that a NEW route with no explicit gate still 403s via the catch-all), not just
  router-nesting. This is the load-bearing S10 build requirement.
- **REV-S10-2 (breaker H1 HIGH — Q4a false premise + false-green DoD, CORRECTED).** `locations` has
  `public_select USING(true)` (`1780338909301:7`, public for the storefront) — NOT RLS-restricted, so
  post-B3 the admin reads do NOT 0-row on locations; `r2-check` (locations-only) returns the full fleet
  and does NOT break. The REAL B3 blocker is `owner_notification_targets` (FORCE tenant-isolation,
  `1790000000077:99`) which `fallback/health` (`fallback.ts:13`) SILENTLY ZEROS post-flip. CORRECT the
  packet: the platform-read DEFINER (Q4a) targets **`owner_notification_targets`**, NOT `locations`;
  the DoD must assert the health COLUMNS return fleet data under NOBYPASSRLS (the locations rows pass
  trivially = the false-green). Build the platform-read as an auditable named grant (counsel:
  legibility — "who may read every tenant"), proven fleet-rows-under-NOBYPASSRLS before the flip.
  Gates the FLIP, not the dark build.
- **REV-S10-3 (breaker M1 — Plane D is verifyAuth, not "no session").** `/api/claim/accept`
  (`public/claim.ts:19-23`) requires `verifyAuth` and derives the recipient from `request.user.sub`;
  it is verifyAuth-only, so a courier/customer `sub` can reach `acceptClaim` (a potential IDOR /
  broken-transfer if the port builds to the wrong §2/§8 tables). REV: port to the RIGHT auth (derive
  recipient from the authed sub) + guard that only a valid claim-recipient sub can accept.
- **REV-S10-4 (breaker M2 — audit attribution across the Rust→Node drill boundary).** `auditCtx` actor
  falls back to `'unknown'` (`platform-admin.ts:104`); the Q3b Rust-trigger→Node-drill carve-out
  crosses a process boundary where `request.user` can go missing → destructive-drill attribution lost
  WITHOUT failing closed. REV: the drill trigger must carry + require the actor identity across the
  boundary; missing actor → fail closed (reject), never `'unknown'`.
- **REV-S10-5 (breaker L1).** Ops-secret brute-force belt: the secret gate 404s BEFORE per-route
  rate-limits fire (`route.ts:56`); only the global 100/min-per-IP applies. Document; if a tighter
  pre-routing limit on `/internal/*` is wanted, it's a small add.
- **REV-S10-6 (counsel — the residuals, named not silent).** (a) Backup manual runbook made genuinely
  safe (double-confirm + target-pin + audit + drilled once) — an ops item, any restore-to-prod endpoint
  is its OWN council. (b) Ownership-transfer "needs work" → the SPECIFIC deferred hardening
  (recipient-binding, invite lifecycle, shared/stale contact, mis-directed-invite revocation) + owner
  — not a "needs work" checkbox. (c) Preserve the Art-14 decline prominence in the port (erase stays as
  easy as claim). (d) `/internal` network-isolation defer = a named-threshold trigger + owner. (e)
  Register: the tenant read cross-tenant (`fallback/health`) has no read-signal — "who audits the
  auditor" — a register item, not S10 scope.
- **REV-S10-7 (counsel — Phase-D, HARD GATE).** Verified: the cutover-harness REV-C10 "record now" slot
  (`cutover-harness/resolution.md:94`) left the owner AND date BLANK. S10's flip IS the Node-decommission
  trigger — flipping with the slot blank fires it into a void → the Node front-door becomes the
  permanent incumbent (the exact captivity the rebuild exists to escape). REV: **S10 approval AND flip
  are GATED on the REV-C10 slot being filled with a concrete named owner + a concrete dated condition**
  (e.g. "all-ten stable ≥ N days ⇒ front-door migrates to Rust, Node shim deleted, by <owner> before
  <date>"). The agent CANNOT self-assign the owner — operator decision.

## 2. Question resolutions (ALL 🔴)
Q1 → port B4 authority intact (3-role enum, allowlist — verified). Q1a → REV-S10-1 (runtime gate,
attack-proven). Q2a → REV-S10-5 + REV-S10-6d. Q2b → REV-S10-6b (specific residual + owner). Q3 →
restore-runbook stays Node (thin Rust trigger, REV-S10-4) + backup-key carried. Q4a → REV-S10-2
(corrected: DEFINER on owner_notification_targets, real DoD). Q5b → REV-S10-7 (HARD GATE).

## 3. 🔴 OPERATOR SIGN-OFF (blocks approval/build/flip)
Q1 (B4 authority) · Q1a (runtime plane-gate) · Q2a (/internal boundary trigger) · Q2b (ownership
residual + owner) · Q3 (restore runbook safety + backup key) · Q4a (platform-read DEFINER on the RIGHT
table + real DoD) · **Q5b — HARD: fill the Phase-D owner + dated trigger; S10 flip is gated on it being
non-placeholder.**

## 4. Build/cutover DoD deltas
Runtime plane-gate attack-proof (a new ungated /api/admin route → 403, REV-S10-1) · platform-read
DEFINER on owner_notification_targets + fleet-columns-under-NOBYPASSRLS probe (REV-S10-2) · claim-accept
recipient-sub guard (REV-S10-3) · drill actor fail-closed across the boundary (REV-S10-4) · Phase-D slot
filled (REV-S10-7) before flip. This is the LAST surface — its cutover DoD is also the Phase-D
decommission gate.
