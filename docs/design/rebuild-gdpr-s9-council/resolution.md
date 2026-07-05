# S9-GDPR/COMPLIANCE — Council RESOLVE

> **Verdict: PROCEED-WITH-REVISIONS + a scoped, liftable ETHICAL-STOP (counsel).** Packet-status 🟡 —
> NOT COUNCIL-APPROVED until operator signs §3 (ALL 7 Qs — reddest surface). Seats: architect (packet)
> · breaker (2 CRIT / 3 HIGH / 2 MED / 3 LOW) · counsel (PROCEED-WITH-REVISIONS + ETHICAL-STOP) · lead.
> GAP-A is CONFIRMED-LIVE by all three seats (architect claim → counsel read → breaker call-graph
> verify) — a live Art.17 under-erasure on production TODAY, pre-existing (not introduced by the port).

## 1. Frozen revision set

- **REV-S9-1 (counsel ETHICAL-STOP + breaker BRK-2 — GAP-A/B/C, the live Art.17 gap).** The GDPR
  customer-erasure path (`anonymizer-gdpr.ts:62-65` → `index.ts:83-88`) calls `anonymizeCustomer` ONLY
  and NEVER fans out to the subject's orders → `anonymizeOrder` (which carries the #74
  `delivery_photo_key` purge, address, receiver) is UNREACHABLE from Art.17; `delivery_lat/lng`
  (precise home GPS, GAP-B) and `order_ratings.feedback` (GAP-C) are nulled by NO path (forever). The
  system writes `completed` + fires `gdpr.erasure_completed` while all of it survives. **STOP lift =
  counsel option 1 (fix-on-Node-NOW, standalone):** fan the customer-erasure out to the subject's
  orders (reaching #74) + `delivery_lat/lng = NULL` + `order_ratings.feedback = NULL`. App-side, no
  migration (`orders.customer_id` FK + columns confirmed to exist); carried-fixed in the port.
  **[Dispatched as a live fix lane in parallel with this RESOLVE.]**
- **REV-S9-2 (breaker BRK-1 CRIT — the worker can't read its own queue post-flip).** Post-NOBYPASSRLS,
  the context-free worker (`bootstrap/workers.ts:100-102`, operational pool) cannot scan/claim
  `gdpr_erasure_requests` — that table has ONLY member-only FORCE RLS (`1780421100060:46-51`), no
  anonymous/service arm, unlike customers/orders. → every request stranded `pending`, no `failed`, no
  signal (#61 never fires — no job runs). The port's erasure worker must run under a context that CAN
  claim the queue (a `SECURITY DEFINER` claim fn with pinned search_path, or a dedicated service role),
  and the DoD MUST include a claim-under-NOBYPASSRLS probe. Migration-adjacent (queue-table policy /
  claim DEFINER). Cutover-blocker.
- **REV-S9-3 (breaker BRK-3 HIGH — the completion gate + the orders RLS arm).** The #61 backstop
  re-reads ONLY `customers.anonymized_at` → the GAP-A order fan-out (REV-S9-1) would silently NO-OP
  post-flip (`orders` has no `anonymous_update` arm) yet the worker still writes `completed` = a
  false-complete. REV: extend the completion gate + the pre-flip P-proof to the WHOLE subject-graph
  (customer + orders + ratings), AND add an `orders`/`order_ratings` erasure RLS arm (or run the
  fan-out via the same DEFINER as REV-S9-2) so the erasure actually applies post-flip. The Node live
  fix (REV-S9-1) works TODAY under BYPASSRLS; the post-flip RLS correctness is the migration half.
- **REV-S9-4 (breaker BRK-4 HIGH — retention has no fail-loud).** `anonymizer-retention.ts` silently
  returns 0 post-flip (no `anonymous_update`) → Art-5(e) storage-limitation stops INDEFINITELY with no
  alert. REV: a fail-loud backstop (0-rows-while-due → alert), mirroring the #61 anonymizer backstop.
- **REV-S9-5 (breaker BRK-5 HIGH — the erasure record is permanent PII).** `gdpr_erasure_requests`
  `subject_phone` is stored plaintext (`gdpr.ts:111-114`) and erased by NO path — an erasure request
  mints a PERMANENT PII record. REV (folded into the live fix): erase/mask `subject_phone` on
  completion (or store a hash at request time). The erasure record must not itself be un-erasable PII.
- **REV-S9-6 (breaker BRK-6/7 MED).** The dedup index covers `completed` permanently → a re-request
  after the 24h cooldown collides → unhandled 500 (not the cited 409/429) — carry-fix. Retention passes
  `locationId` to an option `anonymize()` never reads → a GLOBAL sweep N times (the per-location
  description is dead) — fix to per-location or document the global sweep.
- **REV-S9-7 (counsel Q5 — retention legal basis).** Default is 365 (not the 2555 max — no silent
  7-year hoard; verified). Residual: the legal basis is un-captured → a DPA clause + NO dark-pattern
  nudge to the max + GAP-B/C in the shared null-set. Owner-set retains financial fields under LI;
  delivery-PII carriers get NO cost justification for 7-year retention (Art.17 "without undue delay").
- **REV-S9-8 (counsel Q3 — restore-resurrection).** A pre-erasure encrypted backup can resurrect an
  erased subject. Bounded/owned: encrypted-window + lifecycle expiry + a re-erase-on-restore runbook
  tied to the FIXED erasure. Not a stop.
- **REV-S9-9 (counsel — the courier has NO erasure path).** `gdpr_erasure_requests` can't even
  represent a courier subject, yet the courier is the most-surveilled (continuous GPS, DPIA-flagged).
  Register item + trigger — NOT S9 scope to build, but owned, not silent.

## 2. Question resolutions (ALL 🔴 — reddest surface)
Q1 → REV-S9-1 (GAP-A fan-out) + REV-S9-3 (completion gate). Q2 → IDOR masked-404 + fail-closed scope
(verified sound) + BRK-8 log post-flip. Q3 → REV-S9-8. Q4 → 088 DEFINER pin+REVOKE (verified) +
REV-S9-2 claim DEFINER. Q5 → REV-S9-7. Q6 → cutover human-gate alongside S5 + REV-S9-2/3 proven under
NOBYPASSRLS BEFORE flip (erasure has NO cleanup — irreversible). Q7 → 088 + MIG-2 + the new
orders/queue policy arms sequencing.

## 3. 🔴 OPERATOR SIGN-OFF (blocks build) + the ETHICAL-STOP lift
1. **ETHICAL-STOP lift** — pick: (1) the Node live fix [recommended, dispatched], (2) fix-in-port +
   recorded interim accepted-risk w/ owner+trigger, (3) recorded accepted-risk (human override).
2. Q1/Q3 erasure completeness + subject-graph gate. Q2 IDOR. Q4 DEFINER + queue-claim. Q5 retention
   basis. Q6 cutover human-gate + NOBYPASSRLS-proven-before-flip. Q7 migration sequencing.

## 4. Build/cutover DoD deltas
Subject-graph fan-out + completion-gate red→green (REV-S9-1/3, live + port) · claim-under-NOBYPASSRLS
probe (REV-S9-2) · retention fail-loud (REV-S9-4) · subject_phone erased assert (REV-S9-5) · re-request
409-not-500 (REV-S9-6) · retention basis capture (REV-S9-7) · re-erase-on-restore runbook (REV-S9-8) ·
courier-erasure register row (REV-S9-9). Cutover: S9 flip = human go/no-go alongside S5, irreversible,
correctness proven under NOBYPASSRLS before the flip.
