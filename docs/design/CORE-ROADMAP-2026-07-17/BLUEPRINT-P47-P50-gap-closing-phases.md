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
>
> **UPDATE 2026-07-18 (same day, later): all three operator rulings LANDED.** P47 rail
> sequencing (§2.2), P48 rendering + hub role (§3.2/§3.1), P49 identity deferral (§4.2) are
> each RESOLVED with dated notes appended next to the original open-question framing (framings
> preserved, per convention). D-items are green; B-item build-out is open.

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

> **RESOLVED (2026-07-18, operator ruling)** — verbatim intent: "у першу чергу зараз готівка,
> у планах крипта, та останнє уже платіжні системи, stripe, payoneer, google/apple pay — тут
> уже варто застосовувати готові і перевірені бібліотеки без власного нативного коду."
>
> - **Wave 0 (now) = cash.** Row 1's recommendation is CONFIRMED by the operator — no longer
>   merely recommended. No new work beyond this status change; B1–B3 stand as written.
> - **Wave 1 (planned, next after cash) = crypto** — explicitly ordered BEFORE conventional
>   processors. Recorded with its reasoning, not as arbitrary ordering: a crypto payment is a
>   signed transaction, which fits the mesh's own capability-cert / PQ-signature settlement
>   model (signed attestation → `decide` → `SettlementRecorded` fold, `verify_chain` /
>   `RevocationSet` reuse) far more naturally than a centralized-processor integration — the
>   rail extends trust machinery the stack already has instead of importing a foreign trust
>   model. This also retires row 3's "not proposed" hedge: crypto is now in scope, sequenced.
> - **Wave 2 (last) = Stripe / Payoneer / Google Pay / Apple Pay** — with a BINDING
>   constraint: OFFICIAL, PROVEN THIRD-PARTY LIBRARIES ONLY, no custom native
>   reimplementation. This is a deliberate, NAMED EXCEPTION to the repo's native-Rust /
>   re-derive-first default (memory: `rust-native-bare-metal-decision-2026-07-14`, whose own
>   rule is honest falsifiable comparison, not purity): payment-processor integration is
>   high-liability, PCI-DSS-adjacent compliance surface — reinventing audited, certified
>   handling in native code is a real security/liability risk. Official SDKs exist because
>   this territory is solved, audited, certified. **Live verification (crates.io,
>   2026-07-18):** Stripe publishes NO first-party Rust SDK; the de-facto crate is
>   community-maintained `async-stripe` (latest 1.0.0-rc.6, 2026-05-28, actively maintained,
>   ~4.6M total downloads). Wave-2 candidates to evaluate at build time: `async-stripe` OR
>   Stripe's official REST API directly; Google Pay / Apple Pay via their standard web/native
>   Payment Request APIs — never custom payment crypto. The final vendor pick stays a
>   build-time engineering choice within this constraint; this note picks none.

### 2.3 Pre-named types (standard item 4 — names fixed now; shapes illustrative until build-out)
- `PaymentPort` — the kernel-ports trait; `RailKind` — `enum { CashOnDelivery, /* card variants only after 2.5-D1 */ }`
  *(2026-07-18 addendum, per ruling: planned variants now named — `Crypto` (Wave 1) and
  `Processor` (Wave 2, official-library adapters only); still illustrative shapes, added at
  their wave's build-out, never before the preceding wave is green)*
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
  — *✅ RESOLVED (2026-07-18): the §2.2 ruling note IS D1 — waves fixed (cash confirmed →
  crypto → processors last), official-libraries-only binding for Wave 2. The specific Wave-2
  vendor is delegated to build time within that constraint; geography/fee-model specifics
  still surface to the operator when a concrete vendor is proposed.*
- **B1:** `PaymentPort` in kernel-ports; `cargo tree` shows kernel has no payment-adapter
  dependency; committed red-proof (a direct adapter import fails the build).
- **B2:** cash rail end-to-end over P37's wire: place → deliver → attestation → fold; all `i64`.
- **B3:** reconciliation property test — folded settlements ≡ fold-derived order totals, exact
  integer equality, arbitrary order sequences.
- **B4** *(added 2026-07-18, per ruling — Wave 1)***:** crypto rail design note maps crypto
  settlement onto the existing signed-event model (signed transaction as attestation →
  `decide` → `SettlementRecorded` fold; `verify_chain`/`RevocationSet` for authorization; all
  amounts `i64`) BEFORE any crypto adapter lands. Gated behind B2 (cash green first). No
  chain/token/custody choice is made here — that is its own build-out decision when Wave 1
  starts.
- **B5** *(added 2026-07-18, per ruling — Wave 2)***:** processor adapters wrap an
  official/proven third-party library only. Candidates to evaluate (not final picks):
  `async-stripe` (no first-party Stripe Rust SDK exists — verified crates.io 2026-07-18) or
  Stripe's official REST API directly; Google Pay / Apple Pay via their standard Payment
  Request APIs. RED check: any custom native implementation of processor-side payment
  cryptography or card-data handling is the fail condition. Gated behind Wave 1.

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
*(2026-07-18 addendum: "no custom payment processor" is extended by the Wave-2 ruling — no
native reimplementation of processor-SDK territory either. Official libraries are BINDING for
Wave 2, a named exception to the native-Rust default; see §2.2's RESOLVED note. Wave ordering
is also binding: no processor adapter before crypto, no crypto adapter before cash is green.)*

---

## 3. P48 — Owner/Admin operational surface

> **➡ PROMOTED TO A STANDALONE BLUEPRINT (2026-07-18, same day, later still):**
> **`BLUEPRINT-P48-owner-hub-surface.md`** (this directory) is now P48's authoritative
> blueprint. A follow-up operator directive expanded the hub's scope past what this shared
> section carries: two-way messenger ORDER flow (place/manage an order by chatting, not just
> notifications), adaptive per-customer notification-channel choice (one or several), and
> read-only reviews ingestion starting with Google Maps (GBP API — free, approval-gated;
> Places API rejected with 2026 numbers), all unified by the event-log-first hub thesis.
> The standalone file carries this section's B1–B6 + adversarials 1–3 forward UNCHANGED
> (its §3.0) and re-litigates nothing. The text below is preserved as the record of the
> original 2026-07-18 rulings (rendering + hub role) — read it for provenance; build from
> the standalone file.

### 3.1 Scope & role
Menu editing, live order visibility, courier/staff roster — the workflow menu-as-data +
capability certs imply and nobody owns (P37 anti-scope excludes admin CRUD; P38b is
customer-facing). Concrete, closed operation list; not a framework.

> **RESOLVED — role re-centered (2026-07-18, operator ruling; original paragraph above
> preserved, this note is now the authoritative role statement).** Verbatim intent: "адмін
> поверхня власника це архітектура хабу, що дозволяє керувати та обробляти фуд вендор і
> замовлення з різних входів (соцмережі, сайти, боти, і тд) у одному хабі з агентською
> підтримкою… тут власне уся суть, що замовити може будь-хто і з різних входів."
> **The admin surface IS a HUB architecture:** the owner manages and processes the food vendor
> and its orders arriving from MULTIPLE INTAKE CHANNELS — social media DMs, websites, bots,
> etc. — all funneling into ONE hub, with agentic support. The core requirement this reveals:
> ANYONE can order from ANY input channel. Omnichannel order intake is therefore not a
> P22/P43 nice-to-have — it is what P48 actually IS. So the role = (a) a multi-channel intake
> hub — every channel maps into the SAME order pipeline, the
> `DeliveryEvent::OrderPlaced(OrderPlacedPayload)` vocabulary P34 already defines
> (`bebop2/proto-cap/src/event_dict.rs:279` variant, `:106` payload — verified live
> 2026-07-18); (b) a physics-rendered management view — same interface logic as everywhere
> else, "продовження рендер бекенду через фізику" (see §3.2's resolution); (c) agentic
> support — ties to P40's tool loop, an agent helping the owner triage/process orders across
> channels. Boundary: INBOUND intake is P48's; the OUTBOUND send path stays P43's. The
> menu/order/roster operation list above survives unchanged inside (b).

### 3.2 Open question #1 — rendering (⚠ OPERATOR; the blueprint's first decision, not decided here)

| Option | For | Against |
|---|---|---|
| WebGPU-rendered, same as customer surface | one render stack, §10.3 invariant 4 holds uniformly, no DOM creep precedent | admin UIs are data-dense/form-heavy — tables, text inputs, bulk edits are exactly what field-render is worst at today (MSDF text is itself still 0% per P38a) |
| DOM exemption, FE-15-adjacent reasoning | FE-15 already establishes DOM survives where WebGPU genuinely cannot serve (a11y mirror); an owner-only surface never touches the customer brand experience | first real breach of "never DOM-first" beyond the invisible mirror; risks becoming the precedent every future surface cites |

Both options keep capability-cert auth and thin-shell law identical — the ruling changes pixels,
never authority. Fail condition for the phase: surface code landing before a dated ruling.

> **RESOLVED (2026-07-18, operator ruling): WebGPU — NO DOM exemption.** "логіка для
> інтерфейсу така ж сама як і всюди — це продовження рендер бекенду через фізику": the admin
> surface's interface logic is the same as everywhere else in the product, a continuation of
> the backend rendered through physics (WebGPU field-render). §10.3 invariant 4 holds
> uniformly; FE-15's a11y mirror remains the only DOM survivor and no precedent for exemptions
> is created. The row-2 "Against" concern (data-dense/form-heavy, MSDF text at 0% per P38a) is
> acknowledged as a real build-out difficulty, not a reopened decision — it lands on P38a's
> critical path, which this ruling makes an unconditional P48 dependency. The dated-ruling
> fail condition above is hereby satisfied; surface build-out is unblocked.

### 3.3 Pre-named types
- `OwnerScope` — the capability scope an owner cert carries (distinct from courier/device scopes)
- `MenuEdit` intent → a dowiz-kernel event (note: P34's "no new event variants" anti-scope
  governs bebop2's five **wire** variants, not dowiz's internal kernel event vocabulary; whether
  menu edits need a wire representation at all is a build-time technical design note, not an
  operator ruling)
- `RosterAction { Grant, Revoke }` — thin wrappers over existing proto-cap issuance/revocation

### 3.4 DoD
- **D1 (decision):** dated rendering ruling (WebGPU vs DOM exemption).
  — *✅ RESOLVED (2026-07-18): WebGPU, no exemption — §3.2's ruling note. B-items unblocked.*
- **B1:** owner edits a menu item; a subsequently placed order's fold-derived state carries the
  change (the roadmap's missing sentence, as a test).
- **B2:** live order list = read-only projection of fold state; a review-gate check proves no
  shadow state in the surface crate.
- **B3:** roster grant/revoke via existing `RevocationSet`; revoked courier's next mutating
  request rejected.
- **B4:** negative test — no password-based admin login path exists; owner auth is the same
  capability-cert flow as P37.
- **B5** *(added 2026-07-18, per ruling — omnichannel intake, Wave 0)***:** at least TWO
  concrete non-native intake channels land as Wave-0 candidates: (i) a social-media DM/message
  intake adapter and (ii) a simple web-form intake. BOTH map into the same
  `DeliveryEvent::OrderPlaced(OrderPlacedPayload)` vocabulary
  (`bebop2/proto-cap/src/event_dict.rs:279`/`:106`, verified live 2026-07-18) — an intake
  channel minting its own order representation instead of `OrderPlaced` is the fail condition.
  Channels differ; the pipeline does not. Channel-specific transport choices (which social
  platform, which bot API) are build-time engineering picks, not rulings.
- **B6** *(added 2026-07-18, per ruling — agentic support)***:** a design note ties hub triage
  to P40's tool loop (agent-assisted processing/triage of orders arriving from different
  channels). Advisory at Wave 0 — it does not gate B1–B5.

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
*(2026-07-18 correction, per ruling: the P22/P43 deferral above covers dashboards and the
outbound send path only — INBOUND omnichannel order intake is P48's own hub scope now, not
deferred territory. Also now in-scope-adjacent: no DOM fallback surface may be built "just for
the hard parts" — §3.2's ruling closed that door; hard parts land on P38a.)*

---

## 4. P49 — Customer identity, notification & tracking UX

### 4.1 Scope & role
(a) Anonymous ordering + re-identification without device-bound cert enrollment; (b) the
customer-side consumer of P43's to-be-built send path; (c) tracking UX over the existing
Kalman/EMA math through P38's pipelines. The old stack's `softVerifyAuth` (commit `c3bd16cf9`)
proves the problem is real and solvable — this phase re-solves it natively, it does not port TS.

> **Urgency context (2026-07-18, operator — recorded as context, not a new decision):**
> "потрібен, перший клієнт тестував і чекає на оновлену частину, ще декілька клієнтів також
> ЧЕКАЮТЬ" — a first real client has already tested the product and is WAITING for the updated
> version, and several more clients are also waiting. This is not abstract planning; there is
> real, current demand. It is why §4.2's resolution ("simple default now, don't perfect it")
> is the right call: the roadmap needs a working simple version FASTER than a perfect one.
> Cross-reference: §5.3's first-real-order gate — that milestone is not hypothetical, real
> clients are already waiting on it.

### 4.2 Identity decision table (⚠ OPERATOR — options presented, none picked)

| Candidate | Enrollment burden | Re-identification strength | Privacy/GDPR surface | Offline-first fit | Notable failure mode |
|---|---|---|---|---|---|
| 1. Short-lived session token bound to a device fingerprint | zero | medium (fingerprint drift, shared devices) | fingerprinting itself is a GDPR-relevant technique — feeds P50's audit | good (token minted locally) | fingerprint collision/drift locks a customer out of their own order |
| 2. Lighter capability grant scoped to ONE order | zero visible (grant minted at order placement, carried by the client) | high (same signature machinery as the rest of the mesh) | minimal — grant dies with the order | best (pure proto-cap reuse, works solo-island) | grant loss = tracking loss unless a recovery leg is added (which drifts toward option 3) |
| 3. Magic-link via email/SMS | customer surrenders a contact channel | high while the channel is live | a stored contact identifier — retention/deletion obligations, feeds P50 | worst (needs an egress channel to send the link) | delivery dependency on exactly the send path P43 has not built yet |

Interaction note (stated, not resolved): option 2 reuses proto-cap without hardware enrollment —
if chosen, the blueprint's build-out must prove the grant is NOT a device identity (no
linkability across orders). Option 3 hard-depends on P43 DoD-2. Ruling required before B-items.

> **RESOLVED (2026-07-18, operator ruling):** "варто спланувати, та узагалі некритично і
> відкладається до перших 5/50 реальних клієнтів" — worth planning at design level, NOT
> critical, and the mechanism decision is DEFERRED until the first 5–50 real clients exist.
> Effect on this table: the operator gate is LIFTED — "Ruling required before B-items" no
> longer holds. Instead, build picks ONE simple pragmatic default from the three candidates as
> a Wave-0 minimal default WITHOUT extensive validation (a build-time engineering choice,
> recorded as a dated note when made — this table's own rows already carry the tradeoffs; row
> 2 is pure proto-cap reuse and best offline-fit, but the pick belongs to the build, not this
> note), and the proper mechanism decision is re-opened with real usage data at 5–50 clients.
> Do not over-engineer identity now and do not block anything on perfecting it. The
> interaction-note obligations (no cross-order linkability if option 2; P43 DoD-2 dependency
> if option 3) bind whichever default is picked.

### 4.3 Pre-named types
- `OrderTrackingGrant` / `CustomerSession` (which survives depends on the ruling — both named so
  neither arrives stringly-typed)
- `TrackingView` — the P38a-rendered projection of Kalman/EMA courier state, read-only
- `NotificationBinding { order_id, channel_ref }` — the order↔channel link, dies with the order

### 4.4 DoD
- **D1 (decision):** dated identity ruling among §4.2's candidates (or operator-supplied better).
  — *✅ RESOLVED (2026-07-18): the ruling is a DEFERRAL, not a mechanism pick — see §4.2's
  note. D1's replacement obligation: build records a dated note naming its simple Wave-0
  default (from the three candidates) when it lands, and a follow-up decision item is opened
  at 5–50 real clients. B-items are unblocked now.*
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

> **Note (2026-07-18, operator context — see §4.1's urgency note):** this milestone is not
> hypothetical. A first real client has already tested the product and is waiting for the
> updated version; several more clients are also waiting. The gate's prerequisites (P47 cash
> rail, P48 hub Wave 0, P49 simple identity default) should be built as their SIMPLE Wave-0
> versions for exactly this reason — real demand is queued behind this checklist today.

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

**RESOLVED 2026-07-18 — all three rulings landed (original framings preserved above and in
each section, per convention):**
1. **P47 (§2.2) ✅** — waves fixed: cash (Wave 0, confirmed) → crypto (Wave 1, before
   processors — fits the mesh's signed-transaction/capability-cert model) → Stripe / Payoneer
   / Google Pay / Apple Pay (Wave 2, last, OFFICIAL LIBRARIES ONLY — named exception to the
   native-Rust default; no first-party Stripe Rust SDK exists, `async-stripe` or the REST API
   are the candidates, verified crates.io 2026-07-18). Wave-2 vendor pick delegated to build
   time within the constraint.
2. **P48 (§3.2/§3.1) ✅** — WebGPU, no DOM exemption; role re-centered as a multi-channel
   intake HUB (social/web/bot inputs → one `DeliveryEvent::OrderPlaced` pipeline) with
   physics-rendered management view and agentic support (P40 tool loop).
3. **P49 (§4.2) ✅** — deferred until 5–50 real clients; simple Wave-0 default picked at build
   time from the three candidates, no further operator gate; urgency: real clients already
   waiting (§4.1 note, §5.3 cross-ref).
The P50 legal-judgment flag class remains open by design — it is per-row counsel routing, not
one of the three, and no ruling here touches it.

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
| 9 | Linux-discipline verdicts | REINFORCES: fail-closed defaults, no second auth path; GAP honestly named: three operator rulings block build-out — reduced-with-reason, not skipped *(2026-07-18: all three rulings landed — the gap is closed, build-out open; see §2.2/§3.2/§4.2 RESOLVED notes)* |
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
