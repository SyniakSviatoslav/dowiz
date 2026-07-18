# BLUEPRINT — P47–P50 gap-closing phases (payment rails · owner surface · customer identity · compliance/first-order gate)

> Written 2026-07-18 by the end-state-vision follow-up pass. Phase-index authority:
> `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §11 (this file is its
> referenced blueprint). Measured against `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2
> (the 20-point contract) — compliance map in §7 below.
>
> **Why ONE combined blueprint for four phases:** each of these phases is smaller and more
> exploratory than P34/P37/P38/P40/P41, and three of the four (P47, P48, P49) are gated on an
> operator ruling before deep build-out. Their DoD is therefore mostly "make an operator-facing
> decision, then here is the falsifiable build-out once decided." One lighter-weight blueprint
> that presents those decisions clearly is more honest than four padded full ones. Where a real
> judgment call belongs to the operator, this document says so explicitly and stops there — it
> invents no vendor choices, no geography assumptions, and no legal conclusions.

---

## 0. Ground truth (every cite verified live 2026-07-18, this pass — not inherited)

| Claim | Evidence (live) |
|---|---|
| `SettlementRecorded` exists as a wire event | `bebop-repo/bebop2/proto-cap/src/event_dict.rs:122` (payload), `:279` (`DeliveryEvent::OrderPlaced` variant block; settlement variant in same enum) |
| Money is range-checked integer math | `kernel/src/money.rs:8` ("`i128 → i64` casts are range-checked via `i64::try_from`") |
| Zero payment code anywhere | grep `payment|stripe|liqpay|paypal|cash_on_delivery|cash-on-delivery` across `kernel/src`, `engine/src`, `web/src`, `llm-adapters/src`, `bebop2/delivery-domain/src`, `bebop2/proto-cap/src`: **zero non-test hits** |
| Kernel-ports convention P47 mirrors | `kernel/src/ports/{mod.rs, llm.rs, agent/}` — plain structs, no HTTP/serde in kernel |
| KernelFacade seam (firewall pattern) | `bebop2/proto-cap/src/facade.rs:123 pub fn submit_intent` — ⚠ drift note: §10 cites `:64`; the live line is `:123` (file grew). Symbol unchanged. |
| Revocation machinery P48's roster reuses | `bebop2/proto-cap/src/revocation.rs:49 pub struct RevocationSet` (append-only invalidate set + `merge` anti-entropy) |
| Geo math P49's tracking renders | `kernel/src/kalman.rs` (exists); `kernel/src/geo.rs:39 pub fn ema_next` (its 1D special case) |
| Order-total organ P48's menu edits feed | `kernel/src/domain.rs:129 pub fn compute_order_total` |
| Telegram send path does NOT exist | `kernel/src/messenger.rs:33 telegram_link()` — non-sending deep-link builder only (§10.5.5's corrected-false claim holds) |
| Old-stack GDPR surface (P50's audit inventory) | git history: `attic/apps-api/src/routes/owner/gdpr.ts`, `attic/apps-api/src/workers/anonymizer-gdpr.ts`, `attic/apps-api/src/public/admin/gdpr.html` (deleted `f9ab28ff1`); `packages/shared-types/src/contracts/owner/gdpr.ts` (deleted `79ef316f6`). `attic/` and `apps/` are NOT on disk — history is the only source. |
| Old-stack anonymous-customer precedent | commit `c3bd16cf9` "softVerifyAuth for anonymous order tracking, phone throttle on orders, GDPR 409" — the old stack solved P49's identity problem once; deleted with the purge |

---

## 1. Proportionality — how this blueprint applies the 20-point standard

Three of the four phases pivot on a decision only the operator may make (rail vendor, rendering
exemption, identity mechanism). For those, the standard's build-heavy items (benchmarks,
mesh payload budgets, tensor representation) are **proportionately reduced with stated reason**
rather than padded with speculation — §7 maps all 20 points explicitly. Every DoD below is split
into **(D)** "decision recorded" — falsifiable today (a dated note exists or it doesn't) — and
**(B)** "build-out once decided" — falsifiable the normal RED→GREEN way after the ruling.

---

## 2. P47 — Payment & settlement rails

### 2.1 Scope & role
Own the boundary where money physically enters or leaves. The kernel already records settlement
(`SettlementRecorded`) and computes totals in airtight `i64`; what is missing is the rail: a
`PaymentPort` in `kernel/src/ports/` (mirroring `llm.rs` conventions), adapters outside the
kernel behind the same compilation firewall as KernelFacade/ToolPort (§10.3 invariant 5).

### 2.2 Rail decision table (⚠ OPERATOR — this blueprint recommends row 1 and decides nothing else)

| Candidate rail | External dependency | Custody | Failure mode | Who decides |
|---|---|---|---|---|
| **Cash-on-delivery** (recommended Wave-0) | none — no vendor, no network, no central authority; matches the mesh's local-first stance | courier holds cash; signed attestation is the settlement source | courier misreports → caught by reconciliation invariant (2.5-B3) + attestation signature | build-out needs no ruling |
| Card / digital wallet | a real payment processor (vendor, geography, fee model, settlement delay, chargeback law) | processor custody until payout | processor outage, chargebacks, KYC obligations | **operator — not made here** |
| Anything else (crypto/mesh-native settlement, invoicing, …) | varies | varies | varies | **not proposed** — out of this blueprint's scope; raising it is itself an operator call |

### 2.3 Pre-named types (standard item 4 — names fixed now; shapes illustrative until build-out)
- `PaymentPort` — the kernel-ports trait; `RailKind` — `enum { CashOnDelivery, /* card variants only after 2.5-D1 */ }`
- `CashAttestation { order_id, amount_i64, courier_cert_ref, sig }` — the signed cash-collected claim
- `SettlementOutcome` — typed result; a rail failure is a value, never a panic or silent retry
- `const SETTLEMENT_IDEMPOTENCY_KEY: order_id` — one settlement fold per order, enforced in `decide`

### 2.4 Spec/event/TDD shape (standard item 3)
Spec first: settlement is an **event append**, never a mutation — `CashAttestation` →
`decide` → `SettlementRecorded` → fold. Tests assert on the event sequence (placed → delivered →
settled), not end-state only.

### 2.5 DoD
- **D1 (decision):** dated operator note on the card/digital rail (or an explicit "cash-only for
  now"). Fail condition: any card-rail adapter code existing without the note.
- **B1:** `PaymentPort` in kernel-ports; `cargo tree` shows kernel has no payment-adapter
  dependency; committed red-proof (a direct adapter import fails the build).
- **B2:** cash rail end-to-end over P37's wire: place → deliver → attestation → fold; all `i64`.
- **B3:** reconciliation property test — folded settlements ≡ fold-derived order totals, exact
  integer equality, arbitrary order sequences.

### 2.6 Adversarial cases (standard item 5 — each lands as a test designed to break the invariant)
1. **Double settlement:** the same order's attestation submitted twice → second is rejected
   (idempotency by `order_id`), fold unchanged.
2. **Settle-before-deliver:** attestation for an order not in delivered state → `claim_machine`
   transition rejected; no settlement event appended.
3. **Amount mismatch:** attested amount ≠ fold-derived total → typed reject; the system must
   NEVER silently adjust either number.
4. **Forged attestation:** invalid/revoked courier cert chain → fail-closed reject
   (reuses `verify_chain`/`RevocationSet`, no new crypto).

### 2.7 Isolation / mesh / rollback (standard items 11–13)
Bulkhead: a rail-adapter failure yields a typed `SettlementOutcome` error and leaves the order
flow complete-with-settlement-pending — it can never block placement or delivery (money writes
stay degrade-closed: no partial fold). Mesh: an attestation is a node-local signed event,
gossip-propagated exactly like the other `DeliveryEvent`s — no new transport, payload well under
existing frame budgets (one order id + `i64` + sig). Rollback: event-sourced by construction —
snapshot re-entry (the standard's third recovery class), nothing bespoke.

### 2.8 Anti-scope
No custom payment processor. No touching the money integer law (CORE's, correct). No geography
coupling without a ruling. No card adapter before D1. No settlement math outside `decide`/fold.

---

## 3. P48 — Owner/Admin operational surface

### 3.1 Scope & role
Menu editing, live order visibility, courier/staff roster — the workflow menu-as-data +
capability certs imply and nobody owns (P37 anti-scope excludes admin CRUD; P38b is
customer-facing). Concrete, closed operation list; not a framework.

### 3.2 Open question #1 — rendering (⚠ OPERATOR; the blueprint's first decision, not decided here)

| Option | For | Against |
|---|---|---|
| WebGPU-rendered, same as customer surface | one render stack, §10.3 invariant 4 holds uniformly, no DOM creep precedent | admin UIs are data-dense/form-heavy — tables, text inputs, bulk edits are exactly what field-render is worst at today (MSDF text is itself still 0% per P38a) |
| DOM exemption, FE-15-adjacent reasoning | FE-15 already establishes DOM survives where WebGPU genuinely cannot serve (a11y mirror); an owner-only surface never touches the customer brand experience | first real breach of "never DOM-first" beyond the invisible mirror; risks becoming the precedent every future surface cites |

Both options keep capability-cert auth and thin-shell law identical — the ruling changes pixels,
never authority. Fail condition for the phase: surface code landing before a dated ruling.

### 3.3 Pre-named types
- `OwnerScope` — the capability scope an owner cert carries (distinct from courier/device scopes)
- `MenuEdit` intent → a dowiz-kernel event (note: P34's "no new event variants" anti-scope
  governs bebop2's five **wire** variants, not dowiz's internal kernel event vocabulary; whether
  menu edits need a wire representation at all is a build-time technical design note, not an
  operator ruling)
- `RosterAction { Grant, Revoke }` — thin wrappers over existing proto-cap issuance/revocation

### 3.4 DoD
- **D1 (decision):** dated rendering ruling (WebGPU vs DOM exemption).
- **B1:** owner edits a menu item; a subsequently placed order's fold-derived state carries the
  change (the roadmap's missing sentence, as a test).
- **B2:** live order list = read-only projection of fold state; a review-gate check proves no
  shadow state in the surface crate.
- **B3:** roster grant/revoke via existing `RevocationSet`; revoked courier's next mutating
  request rejected.
- **B4:** negative test — no password-based admin login path exists; owner auth is the same
  capability-cert flow as P37.

### 3.5 Adversarial cases
1. **Scope escalation:** a customer- or courier-scoped cert attempting `MenuEdit` → fail-closed
   reject.
2. **Revoked-owner mid-session:** owner cert revoked between two edits → second edit rejected.
3. **Edit-vs-in-flight-order race:** price change lands while an order is mid-flight → the order
   folds against menu state **at placement time**; no retroactive price change on any placed
   order (money red-line adjacent — property test, not prose).

### 3.6 Anti-scope
No separate admin-password system (rejected as anti-pattern — a weaker path for the most
privileged user). No general-purpose admin framework. No analytics/marketing dashboards
(P20/P22/P43). No new auth machinery — proto-cap only.

---

## 4. P49 — Customer identity, notification & tracking UX

### 4.1 Scope & role
(a) Anonymous ordering + re-identification without device-bound cert enrollment; (b) the
customer-side consumer of P43's to-be-built send path; (c) tracking UX over the existing
Kalman/EMA math through P38's pipelines. The old stack's `softVerifyAuth` (commit `c3bd16cf9`)
proves the problem is real and solvable — this phase re-solves it natively, it does not port TS.

### 4.2 Identity decision table (⚠ OPERATOR — options presented, none picked)

| Candidate | Enrollment burden | Re-identification strength | Privacy/GDPR surface | Offline-first fit | Notable failure mode |
|---|---|---|---|---|---|
| 1. Short-lived session token bound to a device fingerprint | zero | medium (fingerprint drift, shared devices) | fingerprinting itself is a GDPR-relevant technique — feeds P50's audit | good (token minted locally) | fingerprint collision/drift locks a customer out of their own order |
| 2. Lighter capability grant scoped to ONE order | zero visible (grant minted at order placement, carried by the client) | high (same signature machinery as the rest of the mesh) | minimal — grant dies with the order | best (pure proto-cap reuse, works solo-island) | grant loss = tracking loss unless a recovery leg is added (which drifts toward option 3) |
| 3. Magic-link via email/SMS | customer surrenders a contact channel | high while the channel is live | a stored contact identifier — retention/deletion obligations, feeds P50 | worst (needs an egress channel to send the link) | delivery dependency on exactly the send path P43 has not built yet |

Interaction note (stated, not resolved): option 2 reuses proto-cap without hardware enrollment —
if chosen, the blueprint's build-out must prove the grant is NOT a device identity (no
linkability across orders). Option 3 hard-depends on P43 DoD-2. Ruling required before B-items.

### 4.3 Pre-named types
- `OrderTrackingGrant` / `CustomerSession` (which survives depends on the ruling — both named so
  neither arrives stringly-typed)
- `TrackingView` — the P38a-rendered projection of Kalman/EMA courier state, read-only
- `NotificationBinding { order_id, channel_ref }` — the order↔channel link, dies with the order

### 4.4 DoD
- **D1 (decision):** dated identity ruling among §4.2's candidates (or operator-supplied better).
- **B1:** anonymous place → later re-identify → track, over P37's wire, no durable account — one
  integration test.
- **B2:** one real notification reaches the customer channel on a state change (RED until P43
  DoD-2 transmits; honest cross-phase dependency, not double-counted work).
- **B3:** tracking view renders real Kalman/EMA output through P38a with a deterministic test
  against kernel math (P38's own convention).

### 4.5 Adversarial cases
1. **Token/grant guessing:** re-identification secret must carry stated entropy; a brute-force
   test proves guessing an active order's handle is infeasible at wire rate limits.
2. **Replay after completion:** grant/token used after order terminal state → rejected (expiry
   is part of the type, not a cron job).
3. **Cross-order leak:** a valid grant for order A requesting order B's tracking → fail-closed.
4. **Notification misbinding:** state change on order A must never notify order B's channel —
   property test over the binding.

### 4.6 Anti-scope
No customer account/profile beyond one order's needs — no loyalty, CRM, marketing identity. No
conflation with courier/operator device-bound certs. No second notification transport (P43 owns
the send path). No porting old TS — precedent cited, code re-derived natively.

---

## 5. P50 — Legal/compliance audit + first-order validation gate

### 5.1 Scope & role
Two bundled gates, both "did-we-forget" checks rather than builds. **(a) Compliance audit:** the
pivot deleted a real legal surface (§0's four GDPR files; `attic/` is gone from disk — git
history is the source); prove nothing legally obligatory was silently dropped. **(b) First-order
gate:** promote "first real order, real money, real courier, real customer" (G11) from a late
done-test to a first-class, dated, operator-visible milestone.

### 5.2 Audit method (concrete, executable by an agent with zero session context — standard item 18)
1. Recover inventory: `git log --all --diff-filter=D --name-only -- '*gdpr*' '*consent*'
   '*anonymiz*' '*tax*' '*invoice*' '*food*'` (starting set = §0's four files; extend by grep,
   record every hit).
2. For each recovered item, read its deleted content at its last live commit (`git show
   <commit>^:<path>`) and write ONE row: obligation it served → new-stack status.
3. Classification contract — every row exactly one of: **ported** (MUST cite new-stack
   file:line — a "ported" claim without a live cite is the audit's own fail condition, CI-lintable),
   **deliberately-dropped-with-reason** (dated reason), **genuinely-missing** (becomes a tracked
   item, not silently absorbed).
4. Any row whose disposition requires legal judgment (retention periods, tax reporting duty,
   food-safety applicability — anything jurisdiction-dependent) is flagged **⚠ OPERATOR/COUNSEL**
   and left undecided. This document reaches no legal conclusions and self-certifies nothing.

### 5.3 First-order gate definition
A milestone record (named, dated, in this repo's design docs) with a go/no-go checklist:
P34 wired · P37 wire live · P38 renders · P47 B2 (a way to pay — cash rail suffices) ·
P48 B1 (a managed menu) · P49 B1 (a customer who can order and track) · P50 audit rows all
classified with zero unresolved genuinely-missing legal blockers. Go/no-go is called by the
operator, not self-certified. Scale-out work (P46, P45-beyond-floor) does not start before GO.

### 5.4 DoD
- **B1:** the written audit exists; every row classified per §5.2-3; zero unclassified.
- **B2:** every legal-judgment row carries the ⚠ OPERATOR/COUNSEL flag; grep for a "compliant"
  self-claim without a counsel reference is the RED check.
- **B3:** the milestone record exists with the §5.3 checklist, each item mapped to its owning
  phase's DoD — a checklist item with no phase owner is the fail condition.

### 5.5 Anti-scope
Audit-and-gate only: no compliance framework, no policy generators, no legal-department process,
no implementation work smuggled in under "while we're here." No self-certified compliance
claims, ever — the standing anti-self-certification rule binds hardest exactly here.

---

## 6. Cross-phase dependency map

```
P37 (wire) ──┬──► P47 (pay: cash Wave-0) ──┐
             ├──► P48 (owner surface)  ─────┤
             ├──► P49 (customer id/track) ──┼──► P50 gate (first real order, GO/NO-GO)
P38 (render) ┘         ▲                    │         │
P43 DoD-2 (send path) ─┘                    │         └──► gates P46 / scale-out
P50 audit half: depends on NOTHING — startable today from git history
```

Operator decisions (3): P47 card-rail (§2.2) · P48 rendering (§3.2) · P49 identity (§4.2).
P50 additionally routes every legal-judgment row to operator/counsel — a flag class, not one
decision.

## 7. Standard-compliance map (CORE-ROADMAP-STANDARD §2, all 20 points)

| # | Point | Disposition here |
|---|---|---|
| 1 | Ground truth, file:line, this pass | §0 — all verified live, incl. one drift correction (`facade.rs:123` vs §10's `:64`) |
| 2 | Falsifiable DoD | every phase: D-items (dated note exists or not) + B-items (RED→GREEN) |
| 3 | Spec/event-driven TDD | §2.4 (settlement as event append); §3.4-B1/B2, §4.4, §5.2's lintable rows |
| 4 | Pre-named types/consts | §2.3, §3.3, §4.3 — named now, shapes deferred to post-ruling build-out (stated) |
| 5 | Adversarial/breaking cases | §2.6, §3.5, §4.5, §5.4-B2 (the audit's own anti-self-certification RED) |
| 6 | Safety from structure, not prose | money: settlement only via decide/fold + idempotency in the type (§2.3/§2.6); auth: fail-closed cert scopes (§3.5, §4.5) |
| 7 | Links to docs & memory | §8 |
| 8 | Scaling axis stated | §2.7 (settlements/sec ≪ existing event-log axis); P48/P49 surfaces scale on P37/P38's stated axes — no new axis introduced |
| 9 | Linux-discipline verdicts | REINFORCES: fail-closed defaults, no second auth path; GAP honestly named: three operator rulings block build-out — reduced-with-reason, not skipped |
| 10 | Benchmarks + telemetry | proportionately reduced: no hot path exists pre-ruling; B-item builds inherit P37/P38's bench duties; §2.7 notes settlement volume is trivially low |
| 11 | Isolation/bulkhead | §2.7 (rail failure never blocks orders); §3.4-B2 (no shadow state); §4.3 (binding dies with order) |
| 12 | Mesh awareness | §2.7 (attestation = node-local signed event, existing gossip, tiny payload); P49 option-2 is solo-island-fit by construction |
| 13 | Rollback/self-healing as math | event-sourced snapshot re-entry (§2.7); expiry-in-the-type self-termination (§4.5-2) |
| 14 | Bug-class → compile/CI-time | firewall red-proofs (§2.5-B1), CI-lintable audit rows (§5.2-3), forbidden-path negative tests (§3.4-B4) |
| 15 | Living-memory awareness | P50 audit is a temporal/history access pattern (git archaeology, §5.2); others: flat event-log access, nothing new |
| 16 | Tensor/spectral where applicable | N/A with reason: no closed-form math introduced; P49 tracking consumes existing Kalman/EMA, adds none |
| 17 | Regression tracking | every B-item test is permanent; reconciliation (§2.5-B3) and no-retroactive-price (§3.5-3) go to `docs/regressions/REGRESSION-LEDGER.md` on landing |
| 18 | Zero-context executability | explicit file targets + commands throughout (§5.2 is fully mechanical); rulings are the ONLY unmechanizable steps and are marked |
| 19 | Reuse-first | proto-cap issuance/revocation, verify_chain, Kalman/EMA, event fold, P43 send path — all reused; new machinery = one port trait + thin types |
| 20 | Hermetic principles | fail-closed authority boundaries (P48/P49 scopes), finite anchored authority (owner cert, not password), event-sourced correspondence (settlement mirrors delivery events) |

## 8. Links (docs & memory)

- `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §11 (phase entries), §10.3
  (binding invariants 3/4/5), §10.5.3 (P37/P38 templates), §10.5.5 (P43's corrected-false
  Telegram claim), §7 (G11 origin)
- `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` — the 20-point contract §7 maps against
- `docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` — D3 device-bound-primary model P48/P49
  must not fork
- `docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` — event vocabulary + claim_machine P47 folds
  through
- Memory: `never-bypass-human-gates-2026-06-29.md` (operator rulings are gates, not
  formalities) · `test-integrity-rules-2026-06-27.md` (money red-lines binding on §2) ·
  `ground-truth-over-proxy-2026-07-07.md` (§5's audit posture) ·
  `integration-ports-reactive-arc-2026-07-13.md` (P43 lineage P49 consumes) ·
  `verified-by-math-2026-07-07.md` (falsifiability bar)
