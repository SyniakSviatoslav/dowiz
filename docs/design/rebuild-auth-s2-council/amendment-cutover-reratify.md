# S2-AUTH — Cutover Posture Amendment · RE-RATIFICATION PACKET

- **Date:** 2026-07-05 · **Surface:** S2 auth 🔴 · **Type:** superseding amendment to the signed S2 record.
- **Status:** 🟡 **PENDING-RERATIFICATION** — proposed, not yet adopted. It changes the S2 record only when the S2 seats ratify.
- **Re-ratification seats:** **S2-breaker**, **S2-counsel** (the two S2 seats this revision touches; architect drafts, operator signs the 🔴).
- **Amends:** `resolution.md` convergence-3 (cutover posture, `resolution.md:36-40`), the Q10 cutover gates (`resolution.md:50-56`), and the `COUNCIL-APPROVED` gate item "cutover canary plan" (`resolution.md:99-101`).
- **Origin:** cutover-harness council **REV-C4 / REV-C8b** (`docs/design/rebuild-cutover-harness/resolution.md:31-34,51-58`), routed here by counsel.

## Why this packet exists (the route matters more than the merits)

The cutover-harness (a **2-seat** mechanism council: breaker + counsel) proposed to overturn the S2 cutover posture — a decision that S2 reached as a **unanimous 4-seat convergence point** (architect, breaker, counsel, decorrelated security-sentinel; `resolution.md:4`, `resolution.md:36-40`) and that the operator **signed as a 🔴 item** (commit `515ee373`). Both S2 counsel and the harness counsel ruled that such a change **must not be settled inside the 2-seat mechanism council**; it must return to the S2 record and be re-consulted with the S2 seats that own it — "especially the Breaker who authored gate-iv" (harness `counsel-opinion.md:296-301`; `resolution.md:51-58` REV-C8b). This document is that return. It is a **decision packet**: the architect states the change, re-argues it on auth's terms, and recommends; **the S2 seats ratify or reject.** Nothing here is adopted by drafting it.

Scope guard (counsel): the harness generalized an **S3** conclusion ("catalog is an edit-session, so an atomic per-surface flip is fine") to auth. That reasoning **does not transfer** — auth's hazard is a concurrent-refresh race with an irreversible revoke, not an edit-session. Every recommendation below is re-derived from auth's own failure model, not inherited from S3.

---

## 1. What the original S2 record decided about cutover — and why

**The decision (verbatim, `resolution.md:36-40`, convergence-3, ≥3 seats = highest confidence):**

> **Cutover is irreversible where it deletes.** … a refresh-family DELETE from a parity-seam bug evicts a working vendor mid-shift and **routing back to Node does not un-delete the family**. The cutover must be a **canary flip gated on the family-revocation-rate matching the Node baseline, not a hard switch.**

**The reasoning (why a canary, not a hard switch):** the S2 threat model treats the refresh path as the one place where a parity-seam bug produces an **irreversible** effect. A mis-encoded reuse-detection (`sha256` hex-vs-bytes, the `interval '5 seconds'` benign window, or a non-atomic guarded UPDATE) trips the family-revoke branch (`auth.ts:280-285`) and evicts a working vendor from every device. Rollback reverts **routing** but not **committed state**: the deleted family stays deleted. So S2 chose the posture that makes a parity bug **observable at low blast radius before it is committed** — route a small % of vendor traffic to Rust, watch the family-revocation rate, widen only when it matches the Node baseline (S2 `counsel-opinion.md:159-163`).

**The binding gates the canary rides on (Q10, `resolution.md:50-56`):** four named cutover gates, of which **gate (iv)** is the load-bearing one for this amendment:

> (iv) cross-stack concurrent-refresh proven (safe via the shared-DB atomic UPDATE **iff** both stacks use identical refresh SQL, incl. the `interval '5 seconds'` window — the SQL is authority over the stale "10s" comment).

Gate (iv) was authored by the S2 breaker (H5, `breaker-findings.md:152-171`). **It is the gate whose safety the atomic-vs-canary choice actually turns on** — which is why the breaker seat, not the mechanism council, must re-verify the revision.

---

## 2. The proposed revision (from the cutover-harness)

**REV-C4a + Q3 (harness `resolution.md:31-34`; harness breaker MED, `breaker-findings.md:135-141`):** replace the per-request / %-traffic canary with an **atomic per-surface flip** (the whole S2 family moves to Rust in one flag write) **plus a family-revoke-rate auto-rollback trip-wire** (one metric → one threshold → auto-revert toward Node).

**The harness's argument (steel-manned fairly, harness `counsel-opinion.md:270-282`):** a per-request canary **routes concurrent refreshes of one family to different stacks** — token minted/rotated on Rust for request N, rotated on Node for request N+1 of the *same* family. That is precisely the cross-stack concurrent-refresh race S2 gate-iv worries about (the **S6-pooler-race class**: two writers, one family, no shared in-process lock). The canary would therefore **manufacture, on live vendors, the exact divergence it exists to detect.** Atomic-flip keeps a family wholly on one stack; the trip-wire preserves the canary's *intent* (watch revoke-rate, back off on divergence) and degrades toward the incumbent (fail-safe direction). Framed as: "canary's intent kept, canary's one self-inflicted hazard removed."

---

## 3. Re-argued on AUTH's terms (the crux the seats must judge)

The harness's argument is **half-right, and the missing half is decisive.** Two auth-specific facts break the clean "atomic is strictly safer" claim.

### 3a. Atomic-flip does not eliminate the concurrent-refresh split — it relocates it into the convergence window

The harness's own breaker found this (MED, `docs/design/rebuild-cutover-harness/breaker-findings.md:135-141`): **the flip is not atomic.** It is `UPDATE cutover_flags + NOTIFY`, and the harness documents that **LISTEN/NOTIFY does not work through the transaction pooler** (`server.ts:220-221`) — so convergence degrades to a **1–5s TTL-bounded split-brain per flip** (harness HIGH-1, `breaker-findings.md:50-57`). During that window instance A routes the family's refresh to Rust while instance B routes it to Node — **the same cross-stack same-family split the atomic flip was supposed to remove**, now scoped to the flip window instead of "always." And the trip-wire is **reactive**: it auto-reverts *after* the revoke-rate exceeds baseline, i.e. *after* families are already deleted. Against an irreversible effect, a detector that fires post-commit "detects damage it cannot undo."

So on auth's terms the honest ledger is not "canary bad, atomic good." It is a **trade of split geometry**:

| Posture | Cross-stack same-family split | Blast radius | Detection vs commit |
|---|---|---|---|
| Per-request canary (signed) | **Continuous**, on the canary % | **Low** (1% of vendors) | **Before** commit (front-loaded) |
| Atomic-flip + trip-wire (proposed) | **Bounded** to the 1–5s convergence window | **All** vendors | **After** commit (trip-wire reactive) |

Neither dominates. The canary trades a wider *time* exposure for a smaller *population* and pre-commit detection; atomic trades a narrower time window for full-population exposure and post-commit detection. On a surface whose committed effect is **irreversible**, "low blast radius + detect-before-commit" is not obviously the weaker choice — which is exactly why this cannot be a rubber-stamp.

### 3b. The family-revoke-is-not-rollback-recoverable constraint is the tie-breaker — and it points at a THIRD option

Both S2 counsel and the S2 record are explicit: **family-DELETE is not rollback-recoverable** (`resolution.md:38-39`; S2 `counsel-opinion.md:146-153`, "routing back does not un-delete a family row"). Given that constraint, the correct move is not to pick between two postures that **both** leave a residual cross-stack split — it is to **eliminate the split**, because the split is the only path to the irreversible revoke. Two auth-specific ways to do that, either of which dominates both A and B:

- **Option C1 — family-sticky canary (keeps the signed decision, removes the harness's objection).** Route the canary by **consistent-hash(family_id) → stack**, so *every* refresh of one family pins to one arm. This preserves the canary's low-blast-radius, detect-before-commit virtue **and** removes the concurrent-refresh split the harness objects to — the harness's entire case rests on the canary being routed *per-request*, which was an implementation assumption, not the S2 decision's letter ("canary flip gated on the family-revocation-rate", `resolution.md:39-40`). A family-sticky canary is a canary.
- **Option C2 — atomic-flip WITH refresh-write quiesce.** Adopt atomic only if the flip **quiesces the refresh write path** for the convergence window (Node stops accepting refresh rotations for the ~1–5s TTL, matching the harness's own money-quiesce remedy REV-C3, `docs/design/rebuild-cutover-harness/resolution.md:24-26`). Quiesce converts the bounded-window split to **zero split**, which is what makes atomic genuinely — not rhetorically — safer than the canary. **Atomic-flip WITHOUT quiesce merely relocates the hazard (3a) and is NOT strictly safer than the signed canary.**

**In every posture, S2 gate-iv is a hard prerequisite:** both stacks must run **byte-identical refresh SQL including the `interval '5 seconds'` window** (`resolution.md:53-56`), and the reuse-detection window must be computed in **SQL `now()`, never Rust app-clock** (S2 breaker H5, `breaker-findings.md:160-166`). No cutover posture is safe if gate-iv is open.

---

## 4. The courier-JWT-outside-S2 finding (a genuine scope gap in the original S2 record)

The route-surface-map (`docs/design/rebuild-cutover-harness/route-surface-map.generated.md:199-204,310-314`) surfaced a fact the original S2 scope did not account for: **`apps/api/src/routes/courier/auth.ts` mints, refreshes, and revokes S2-shaped tokens, but is path-owned by S7, outside S2's cutover gate.** Verified live:

- It signs via **`signAuthToken` from `@deliveryos/platform`** (`courier/auth.ts:7,131,330,460`) — the **identical body-kid RS256 signer** S2 C1 is about (`jwt.ts:50-56`). Every courier token therefore carries the same `.strict()` + required-body-`kid` shape (`legacy.ts:162-174`). **S2's C1 cross-stack verification obligation applies verbatim to S7-minted tokens.**
- It runs the same family model: `courier_sessions` keyed on `family_id` + a `jti` bind (`courier/auth.ts:124,330,401`), with a reuse→revoke branch `UPDATE courier_sessions SET revoked_at = now() WHERE family_id = $1` (`courier/auth.ts:420`). **So the concurrent-refresh split hazard of §3 recurs for the courier refresh path too.** (Asymmetry worth recording: the courier path *soft-revokes* via `revoked_at` UPDATE, marginally less irreversible than the owner path's family DELETE — but evicting a working courier mid-shift is still real harm to the least-powerful actor, S2 `counsel-opinion.md:265-273`.)

**Is this a gap in the original S2 scope?** Yes. S2 reasoned about owner/customer mint paths under `/api/auth/*` and never asserted authority over `/api/courier/auth/*`, because those routes are path-owned by S7. The C1 body-kid parity invariant is **surface-independent** — it must hold before *any* token-minting surface flips — but the S2 record scoped it to S2's own routes. The clean fix is to **elevate C1 from an "S2 gate" to a cross-surface auth invariant** that binds S2 **and** S7, rather than re-homing 5 routes across surfaces (which would fight the template matcher). Then S7's cutover DoD inherits the parity gate **and** the §3-ratified cutover posture by reference, and the courier `courier_sessions` family flip is treated as an auth-family flip (§3 posture + gate-iv byte-identical SQL) — not a generic S7 dispatch flip.

**Cutover-sequence consequence:** S7 must not flip its `courier/auth.ts` routes until (a) verification-parity (§5) is green and (b) the §3 posture is applied to `courier_sessions`. Record the obligation in **both** the S2 record (as the invariant's home) and the S7 record (as the inheritor), so no future reader flips S7 auth thinking S2's gate did not reach it.

---

## 5. Verification-parity (harness Q4) as a hard ordering gate

**The prerequisite (harness REV-C4, `docs/design/rebuild-cutover-harness/resolution.md:31-34`; = S2 C1, `breaker-findings.md:21-48`):** a **Node-minted token must verify on Rust, and a Rust-minted token must verify on Node** — body-`kid` present in the body (not only the JOSE header), `.strict()`-compatible claim set (no extra registered claims `iss`/`aud`/`nbf`), RS256 double-pinned, and **`leeway = 0`** to match jose (S2 M1, `breaker-findings.md:204-213`). Both directions, proven by golden vectors.

**Why it gates the ordering (not just S2):** the cutover build order flips **S3/S4/S5 before S2 mints on Rust** (harness `resolution.md:97-100`). But an S3-Rust surface still **verifies Node-minted owner tokens**, and once S2 mints on Rust while any surface is still Node, those Node surfaces **verify Rust-minted tokens**. Verification is a shared cross-stack contract regardless of who mints. Therefore:

> **No authed surface (S3, S4, S5, and the S7 `courier/auth.ts` routes of §4) may flip before the body-`kid` RS256 round-trip is proven green in BOTH directions.** This is a hard gate on the whole authed-surface sequence, decoupled from which surface mints.

This is a *lower-risk* gate than the mint/delete posture of §3 — the verify path is stateless and commits nothing (S2 counsel would rate it "verify path, no irreversible write"). It is nonetheless **binding**: a single missing body-`kid` field or one extra registered claim makes every cross-stack authed request 401 mid-session (S2 C1). Owner of the gate: architect + S2 lead + operator (harness `counsel-opinion.md:132-134`).

---

## Decision packet — for the S2 seats to ratify or reject

Each item: the change, the architect recommendation, the seat that owns the call. **Ratification = all four resolved + operator signs the 🔴.**

### AQ1 — Cutover posture: supersede the signed per-request canary?
**Change:** replace "canary flip gated on family-revocation-rate" (`resolution.md:39-40`) with atomic per-surface flip + revoke-rate trip-wire.
**Architect recommendation: RATIFY-WITH-CONDITION → adopt Option C (§3b), not bare atomic.** Bare atomic-flip only *relocates* the concurrent-refresh split into the 1–5s convergence window (§3a) and is not strictly safer than the signed canary. Ratify **either** C1 (family-sticky canary — preserves the signed decision, removes the harness's per-request objection) **or** C2 (atomic **with** refresh-write quiesce). Reject bare atomic-without-quiesce. In all cases **gate-iv (byte-identical refresh SQL incl. `interval '5 seconds'`, SQL-clock) remains a hard prerequisite.** Record on the S2 line: "canary superseded by [C1|C2], ratified [date], operator-signed; family-revoke is not rollback-recoverable."
**Owner of the call: S2-breaker** (authored gate-iv; must confirm the chosen posture genuinely closes the split, not just reshapes it) + operator 🔴.

### AQ2 — Verification-parity as a hard ordering gate on S3/S4/S5/S7?
**Change:** make the Node↔Rust body-`kid` RS256 round-trip (both directions, `leeway=0`, strict-claims) a hard precondition for flipping *any* authed surface.
**Architect recommendation: RATIFY (unconditional).** This is the safest and least controversial item — it is S2 C1 already in the record (`breaker-findings.md:282-295`), merely promoted to an ordering gate. Lower risk than AQ1 (stateless verify path, zero committed effect), but binding: no green round-trip → no authed-surface flip.
**Owner of the call: S2-breaker** (parity vectors) + architect/S2-lead/operator for ordering.

### AQ3 — Elevate C1 + the AQ1 posture to bind S7's `courier/auth.ts`?
**Change:** treat C1 body-`kid` parity and the AQ1 cutover posture as a **cross-surface auth invariant** binding S2 and S7's 5 courier-auth routes; S7 DoD inherits by reference; `courier_sessions` flip = an auth-family flip.
**Architect recommendation: RATIFY.** This is a real gap in the original S2 scope (§4): S7 mints S2-shaped tokens via the identical signer with the identical family-revoke hazard, outside S2's gate. Close it by making the invariant surface-independent (not by re-homing routes). Cross-reference in both the S2 and S7 records.
**Owner of the call: S2-counsel** (scope/legibility — this is a "who does the gate bind" governance question) + S7 lead notified.

### AQ4 — Record the irreversibility + require a pre-authored cleanup runbook per auth-family flip?
**Change:** record on the S2 line that **family-revoke is not rollback-recoverable**, and make each auth-family flip (S2 owner/customer refresh; S7 courier session) a **distinct operator go/no-go with a pre-authored, reviewed cleanup runbook as a flip precondition** (not invented under incident pressure).
**Architect recommendation: RATIFY.** Already the S2 record's own truth (`resolution.md:38-39`) and the harness counsel's §C-1 condition; this amendment simply binds it as a named precondition. The runbook answer for auth is thin-but-real (a wrongly-revoked family requires a support-mediated re-auth / vendor re-login), which is *why* the pre-commit posture of AQ1 matters more here than for reversible surfaces.
**Owner of the call: S2-counsel** (irreversibility/person-cost lens) + operator 🔴.

---

## The sharpest re-ratification question (for the S2-breaker)

**The harness's case for atomic-flip rests entirely on the canary being routed *per-request*. But the S2 decision said only "canary gated on family-revoke-rate" — it never mandated per-request routing.** So the real fork is not "canary vs atomic." It is:

> **Does a family-sticky canary — `consistent-hash(family_id) → stack`, so every refresh of one family pins to one arm — keep the signed decision's low-blast-radius, detect-before-commit virtue while eliminating the cross-stack concurrent-refresh split the harness objects to? And if it does, is bare atomic-flip (which the harness's own breaker shows only *relocates* that split into the un-atomic 1–5s convergence window, detected by a *reactive* trip-wire *after* the irreversible revoke) actually the weaker posture on the one surface where the committed effect cannot be un-done?**

If the answer is yes, the correct amendment is **not** to overturn the signed canary — it is to **tighten it to family-sticky routing** (Option C1) and keep the 4-seat decision substantially intact. That is the breaker's call, because it turns on gate-iv, which the breaker authored.

---

**Re-ratification seats: S2-breaker, S2-counsel.**
**Status: 🟡 PENDING-RERATIFICATION** — not adopted until both seats resolve AQ1–AQ4 and the operator signs the 🔴 items against the S2 record. Until then, the signed per-request/%-traffic canary (`resolution.md:39-40`) remains the standing S2 cutover posture; no authed surface flips.

---
*This is an amendment packet routed back to the S2 record per REV-C8b. It re-argues a mechanism-council revision on auth's own terms and recommends; it does not itself change the S2 decision or authorize code. Ratification is a separate, human-gated act recorded against the S2 resolution and `traceability.csv`.*
