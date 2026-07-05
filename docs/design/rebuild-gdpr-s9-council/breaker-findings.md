# S9-GDPR/COMPLIANCE Port — Council Packet · BREAKER FINDINGS

> **Seat: system-breaker.** Axis = ADVERSARIAL TRUTH: not "is the packet thorough" (it is) but
> "where does it break." Each finding is code-grounded (`file:line`) and demonstrable (a break
> scenario or a number). No fixes proposed — the architect fixes; I name the break + the violated
> invariant. Ranked CRIT / HIGH / MED / LOW, no severity inflation.
>
> **Method:** every packet claim was VERIFIED against ground truth (not taken on faith). GAP-A
> confirmed. The sharpest escalation is **new**: the packet's post-flip model is **internally
> contradictory** — it fixes the `customers` erase visibility (088) while its own N1 premise silently
> kills the worker's access to the `gdpr_erasure_requests` queue table (BRK-1, CRITICAL).

---

## Severity summary
- **CRITICAL: 2** — BRK-1 (queue-table RLS blindness / self-contradiction), BRK-2 (GAP-A, confirmed-live)
- **HIGH: 3** — BRK-3 (backstop scope < erasure scope), BRK-4 (retention silent no-op, no backstop),
  BRK-5 (subject_phone never erased; phantom "eventual retention")
- **MEDIUM: 2** — BRK-6 (dedup index vs 24h cooldown → 500), BRK-7 (retention `locationId` dead param)
- **LOW: 3** — BRK-8 (foreign-tenant attempt not logged post-flip), BRK-9 (`gdpr_claim_due` phantom
  precedent), BRK-10 (TOCTOU → duplicate audit rows)

---

## [CRITICAL] BRK-1 · B-CONSIST / B-FAIL — the worker cannot claim/complete `gdpr_erasure_requests` post-flip; the packet's own N1 premise contradicts its "table-driven recovery" guarantee

**The packet fixes the wrong half of the RLS boundary.** 088 makes the `customers` UPDATE
visibility-independent. But the worker's *entry point* — the scan that discovers which requests to
erase — runs on the same context-free connection against a table with **no anonymous policy arm**.

- **Ground truth.** The GDPR worker uses `deps.pool` (the operational RLS-subject pool,
  `bootstrap/workers.ts:100-102` → `server.ts:209 createOperationalPool()`), context-free — no
  `set_config`, no `withTenant`, no `app.user_id` anywhere in `anonymizer-gdpr.ts` or `index.ts`.
- Its claim scan: `SELECT ... FROM gdpr_erasure_requests WHERE status='pending' ... FOR UPDATE SKIP
  LOCKED` (`anonymizer-gdpr.ts:26-33`).
- `gdpr_erasure_requests` has **ONLY** the member-only `gdpr_tenant_isolation` policy under **FORCE
  RLS** (`1780421100060:46-51`: `USING (location_id IN (SELECT app_member_location_ids()))`). Unlike
  `customers` (`anonymous_select`/`anonymous_update`, `1780338981782:6-11`) and `orders`
  (`anonymous_select`, `1780338981783:5`), it has **NO anonymous arm and no `app.current_tenant`
  arm**. Today it works **only** because `dowiz_app` carries **BYPASSRLS** (`1790000000077:2`).

**Break scenario (the B3 NOBYPASSRLS flip the packet plans, §10/Q7):** the operational role loses
BYPASSRLS → the context-free worker's `app_member_location_ids()` = ∅ → the claim scan returns **0
rows**. Every Art.17 request sits `pending` **forever**: no erasure, no `failed`, **no
`ANONYMIZER_GDPR_FAILED` signal** — the #61 backstop (`:74-94`) never fires because **no job is ever
claimed**. The same context-free connection also cannot `UPDATE gdpr_erasure_requests SET status=…`
(`:39,56,81,97`) nor `INSERT INTO anonymization_audit_log` (member-only FORCE RLS,
`1780421100060:53-59`; written context-free at `anonymizer-gdpr.ts:115` **and**
`index.ts:183,270`) → even a DEFINER-088 customer erase leaves the **audit provenance (G7, a
red-line asset) unwritable**.

**The internal contradiction.** Proposal §9.2 / open-questions Q6(a) assert as a load-bearing
correctness property: *"the S8 fleet's … worker recovers any pending row from the table directly
(the scan is `WHERE status='pending'`, table-driven, stack-agnostic) … a Rust-created request is
never lost."* This directly contradicts the packet's own N1 premise (`proposal:180`: *"under
NOBYPASSRLS+MIG-2 that connection is invisible to `customers` RLS"*). **One context-free connection
cannot be RLS-blind to `customers` yet RLS-free on `gdpr_erasure_requests`** — both are member-only
FORCE RLS. Either the worker keeps BYPASSRLS (⇒ the context-free `customers` erase also works, N1 and
088 are moot) **or** it loses it (⇒ the queue scan, status write, and audit insert all fail). The
packet assumes a third, non-existent state.

- **The DEFINER asset list is incomplete:** only 088 (`gdpr_erase_customer`, customers-erase) is a
  named cutover asset (§10). There is **no** claim-DEFINER for the queue and **no** `gdpr_finalize`
  in the DoD (§12) or migration-drafts — `gdpr_finalize` appears only as a **conditional aside** in
  §8 (*"if the worker needs to write them under FORCE-RLS"*), never committed.
- **The DoD has a matching blind spot:** §12 lists an N1 P-proof (`customers.anonymized_at IS NOT
  NULL`) and a "`FOR UPDATE SKIP LOCKED` cross-stack single-flight probe" but **no probe that the
  worker can CLAIM a pending request under NOBYPASSRLS**. The 088 P-proof (`088:26-28`) asserts the
  customer end-state — a harness that inserts a `pending` row and calls the DEFINER directly is
  **green while the real worker scan is dead** (exactly the P5 false-green the packet warns against
  in §5).

**Timing is worse than the packet models:** the queue-scan failure triggers at the **NOBYPASSRLS
flip itself**, *before* MIG-2 (there was never an anonymous arm on `gdpr_erasure_requests` to scope).
The packet's "land 088 before MIG-2" sequencing (§10) does nothing for this — 088 is customers-only.

**Invariant violated:** silent-non-erasure liveness / fail-loud (an Art.17 request must reach a
terminal state; #61's whole point). This fails **silent** (stranded `pending`, no DLQ) — strictly
worse than N1, which fails loud. **New finding; not surfaced in the packet.**

---

## [CRITICAL] BRK-2 · B-DATA / B-CONSIST — GAP-A confirmed LIVE: GDPR erasure never touches the subject's orders (VERDICT: confirmed-live-gap)

The packet's headline, VERIFIED true at the call-graph level (this is a **pre-existing live Node
production compliance failure**, not introduced by the rewrite):

- `anonymizer-gdpr.ts:62-65` calls `anonymize({scope:'gdpr', subject:{customerId, locationId}})` —
  **no `orderId`**.
- `index.ts:83-88` runs `anonymizeCustomer` only. The `orderId` branch (`:90-95`) is skipped (no
  orderId); the retention fan-out (`:97-113`) is gated on `!options.subject` — **false** here
  (`subject` is set) **and** `scope==='retention'` — **false** (`'gdpr'`). So **no order row is ever
  reached** by the GDPR path.
- Consequently, for the data subject, `orders.delivery_address / delivery_instructions /
  receiver_name / receiver_handle / receiver_messenger_kind / customer_messenger_handle /
  client_ip_hash / delivery_photo_key / delivery_lat / delivery_lng` (`index.ts:242-249`, +
  lat/lng not even in the null-set — GAP-B) **survive** an Art.17 "erase me" until the retention
  TTL (default **365 days**, `1780421100060:8`) sweeps the order by age.
- The `delivery_photo_key` R2-object purge (#74, `index.ts:249,260-267`) lives **inside
  `anonymizeOrder`** — the branch the GDPR path never calls. **The just-landed doorway-photo fix is
  dead code for GDPR erasure.**

**Break:** "erase me now" leaves the subject's home address + doorway photo (more identifying than
the address) live for up to a year. **Severity CRITICAL as a compliance defect** (incomplete Art.17).
Credited to the packet — it correctly self-identifies this as the reddest finding (§1, §3.2, Q1a1).
My value-add is BRK-1/3/4/5, which the packet does **not** surface.

---

## [HIGH] BRK-3 · B-CONSIST — the completion backstop verifies ONLY `customers.anonymized_at`; the GAP-A fan-out fix re-introduces silent false-completion on the order carriers

The #61 backstop (`anonymizer-gdpr.ts:74-94`) re-reads **only** `customers.anonymized_at`. The
packet (§3.1 layer 2 / Q-N1-BACKSTOP) claims this "makes silent false-completion **structurally
impossible**." That is scoped to the **customer** carrier only.

**Break:** adopt the packet's own recommendation (Q1a1: fan out to `orders`). Post-NOBYPASSRLS the
order UPDATE silently no-ops — `orders` has `anonymous_select`/`anonymous_insert` but **NO
`anonymous_update`** arm (`1780315000000`, `1780338981783`; only member/`app.current_tenant`
arms, `1790000000077:47-53`), so a context-free order UPDATE matches 0 rows. The worker **still
writes `completed`** because the customer row IS anonymized. Result: a partial erasure (customer
erased, orders skipped) reported as a successful Art.17 completion — the exact silent-false-completion
class #61 was built to kill, relocated to the carriers the fix adds. Carrier completeness is enforced
**only by a DoD test** (§12), never by a runtime gate whose scope matches the erasure's scope.

**Invariant violated:** completion-gate scope must equal erasure scope.

---

## [HIGH] BRK-4 · B-FAIL / B-OPS — the retention sweep has NO fail-loud backstop and silently no-ops post-flip

The packet's entire fail-loud posture (#61) is **GDPR-worker-only**. `anonymizer-retention.ts` has
no data-level backstop.

- Post-NOBYPASSRLS: `findExpiredOrders` SELECT works (`orders.anonymous_select`), but the subsequent
  `anonymizeOrder` UPDATE fails (no `anonymous_update` on orders) → 0 rows → `skipped++`,
  `anon:false`, **no error** (`index.ts:106-112`) → logs `"0 orders anonymized"`.
- Post-MIG-2: the `customers` `anonymous_*` arms get scoped → `findExpiredCustomers` /
  `anonymizeCustomer` also see ∅ → `"0 customers anonymized"`.

**Break:** the entire storage-limitation (Art-5(e)) mechanism silently stops, indefinitely, with
**nothing alerting** — customers/orders age past `retention_days` and are never anonymised. The
packet acknowledges the orders no-op (§8) but frames it as a seating fix and provides **no retention
backstop and no retention P-proof** (§12's retention bullet assumes the sweep runs: "the sweep
anonymises by TTL"). A silent compliance failure on the second erasure path.

**Invariant violated:** fail-loud on non-erasure (must apply to retention, not just GDPR).

---

## [HIGH] BRK-5 · B-DATA — `gdpr_erasure_requests.subject_phone` plaintext is never erased by any path; the packet's "eventual retention" mitigation does not exist

- The create route stores `subject_phone` **plaintext** (`gdpr.ts:111-114`, `phone || null`).
- **No path anywhere** nulls or anonymises it: verified by grep — the retention worker touches
  `customer_track_grants`, `funnel_events`, `customers`, `orders` only
  (`anonymizer-retention.ts:45-91`); no migration, worker, or script ever `UPDATE`/`DELETE`s
  `gdpr_erasure_requests.subject_phone`.

**Break:** a phone-initiated Art.17 erasure leaves the subject's phone in plaintext in
`gdpr_erasure_requests` **forever** — the very act of requesting erasure mints a permanent PII record
that erasure never cleans (data-map lists it as carrier #13). The packet asserts the opposite:
threat-model **G4** and proposal §4 claim the request row "must live under the same RLS + **eventual
retention** as the subject," and open-questions §5-residual repeats it. **That eventual-retention
path is phantom** — the packet cites a mitigation that isn't in the code.

**Invariant violated:** erasure completeness (every carrier of the subject); the erasure record is
itself an un-erased carrier.

---

## [MEDIUM] BRK-6 · B-DATA / B-FAIL — the dedup unique index covers `completed` permanently, contradicting the 24h-cooldown design → an unhandled 500 after 24h

- `gdpr_dedup_per_customer` = `UNIQUE (location_id, customer_id) WHERE status IN
  ('pending','in_progress','completed') AND customer_id IS NOT NULL` (`1780421100060:27-28`) —
  a `completed` row is in the index **permanently**.
- The route's cooldown check only looks back 24h (`gdpr.ts:99-107`, returns 429). After 24h it
  **passes**, then the `INSERT ... status='pending'` (`gdpr.ts:110-115`) collides with the still-
  present `completed` row on `(location_id, customer_id)` → **unique_violation → unhandled 500**, not
  the intended clean 429/409.

**Break:** the packet (§4, Q-COOLDOWN) cites "the unique index on those statuses" as the backing for
the 409 but never notes it also covers `completed` forever, contradicting the 24h-cooldown semantics
(which imply re-requests are allowed after 24h). A faithful port inherits a 500-on-re-request. Not a
security/erasure break — robustness + an inaccurate packet description of the guard.

---

## [MEDIUM] BRK-7 · B-ANTIPATTERN — the retention worker passes `locationId` at an option the anonymizer never reads; the per-location scoping the packet describes is dead

- `anonymizer-retention.ts:80-85` calls `anonymize({scope:'retention', locationId: loc.id, ...})`.
- `AnonymizeOptions` (`index.ts:11-25`) has **no top-level `locationId`** — only `subject.locationId`.
  `anonymize()` reads `options.subject?.locationId` (`index.ts:98,106`) = `undefined` →
  `findExpiredCustomers(undefined, …)` → `($1::uuid IS NULL OR …)` with null → **scans ALL
  locations**. (`@ts-nocheck` at `index.ts:1` hides the dropped field.)

**Break:** the per-location loop runs a **global** batch once per location. Functionally tolerable
today (each row is judged by its own `retention_days` via the correlated subquery,
`index.ts:307,320`), but the `locationId` argument is **dead**. The packet (§7) describes retention
as "per location runs `anonymize({scope:'retention', locationId})`" — a scoping the code silently
ignores. A Rust port reading the packet would either implement real per-location scoping (a behavior
change) or copy a dead parameter — a port-fidelity trap on a red surface.

---

## [LOW] BRK-8 · B-SEC / B-OPS — `cross_tenant_attempt` is NOT logged for a truly-foreign tenant post-flip (RLS masks the existence probe)

`gdpr.ts:69-83` runs `SELECT location_id FROM customers WHERE id=$1` inside `withTenant(db,
user.userId)` (member RLS). Post-NOBYPASSRLS, a `customerId` at a tenant the owner is **not** a member
of → RLS hides it → `existsRes=0` → **no `cross_tenant_attempt` log** (still a masked 404, no leak).
The packet (§4, Q2) claims a cross-tenant attempt "is security-logged before the 404 so it stays
detectable" — true only when the target is at **another of the actor's OWN** locations; the truly-
adversarial foreign probe is **silent** post-flip (it IS logged today under BYPASSRLS). Observability
regression, not a leak.

---

## [LOW] BRK-9 · B-ANTIPATTERN — the packet leans on a phantom `gdpr_claim_due` DEFINER precedent

`proposal §3.1.1` (via the inherited 088 draft, `088:21`) cites `gdpr_claim_due` as an existing
"tiny auditable DEFINER ingress-resolver convention" justifying "the worker calls a tiny DEFINER."
grep finds `gdpr_claim_due` **only inside 088's own comment** — no such function exists in migrations
or code. The cited pattern that would resolve BRK-1 (a claim-DEFINER for the queue) is presented as
precedent but was never built.

---

## [LOW] BRK-10 · B-CONSIST — the COMMIT-then-mark TOCTOU produces duplicate audit rows

The packet flags the claim TOCTOU (Q-CLAIM-TOCTOU: `run()` COMMITs the `FOR UPDATE SKIP LOCKED`
select `:34` before marking `in_progress` `:39`) and correctly notes idempotency covers *erasure*
correctness. Un-noted consequence: two workers processing the same `pending` row both reach the audit
INSERT (`anonymizer-gdpr.ts:115-132`, no unique constraint on `anonymization_audit_log`) → **two
audit rows for one erasure**. They agree on tenant (so §8's "never disagree" holds), but the append-
only forensic trail is doubled — audit-count-based assertions and any "one erasure = one audit row"
reasoning break. Minor; the recommended claim-before-work CAS also resolves this.

---

## Regression note (verified-true packet claims — do NOT re-litigate)
- **088 search_path pin + REVOKE PUBLIC + GRANT dowiz_app** — VERIFIED (`088:40,78-79`). Q4 holds.
- **`retention_days` DB default = 365 NOT NULL, CHECK 30–2555** — VERIFIED (`1780421100060:8-9`);
  the packet's "confirm default is 365, not max" (Q5) is correct.
- **IDOR masked-404 + fail-closed scope (`|| row.location_id` deleted)** — VERIFIED
  (`gdpr.ts:63-86`; `index.ts:129-132,218-221`). Q2 same-tenant proof holds (caveat BRK-8).
- **Status masking (`maskName`)** — VERIFIED (`gdpr.ts:184,237,247-250`). Q-STATUS-MASK holds.
- **`customers` has no `app.current_tenant` arm; RC4 is orders-only** — VERIFIED
  (`1790000000077:44-67`); the N1 class is real.

---

**council seat: breaker** · **verdict on GAP-A: CONFIRMED-LIVE-GAP** · **new CRIT that holds: BRK-1**
· **packet-status input: 🟡 DRAFT — 2 CRIT / 3 HIGH open.**
</content>
</invoke>
