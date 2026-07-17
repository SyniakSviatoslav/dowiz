# DELIVERY FLOWS — BACKEND COMPLETENESS AUDIT + MULTI-TENANT MESH TESTING STRATEGY (2026-07-17)

> **What this is:** (1) a code-grounded audit of every flow a real food-delivery platform needs,
> checked against what the dowiz kernel/engine ACTUALLY implements and tests today — not what docs
> claim; (2) a testing/verification strategy for multi-tenant correctness in the M5 hub-autonomy
> mesh, grounded in how hub/tenant boundaries are (not) represented in `kernel/src/` today.
> **Operator's framing under test:** "the backend is where every genuinely-needed function must
> live; the frontend is just display and continuation of the backend." This doc verifies that claim
> against the live tree instead of assuming it.
> **Planning artifact only — no product code written or edited.** Branch: `feat/harness-llm-backend`
> (per the roadmap §8.3 finding, "landed" is branch-implicit — every claim below was read on THIS
> branch, this session).
> **Baseline reused, not re-derived:** market expectations come from
> `HUB-DESIGN-VENDOR-MARKET-RESEARCH-2026-07-17.md` (§2.5 table-stakes T1–T9, gaps G1–G8, addendum
> D1–D6). This doc extends that work down to the per-flow, per-test level and adds the
> multi-tenant dimension that doc did not cover.

---

## 1. Problem + non-goals

**Problem.** P16 (Product UI Rebuild) will project owner/customer/courier surfaces onto the kernel.
If a flow's business logic exists only as a UI-plan row, the rebuild will re-create the legacy
anti-pattern (logic in the frontend) under a new name. Separately, M5 makes every vendor hub a
sovereign tenant — but nothing in `kernel/src/` names, enforces, or tests a tenant boundary, so
"hub A can't leak into hub B" is currently an unstated process-topology assumption, not a verified
property.

**Non-goals:** no new phases (the HUB addendum's own scope rule holds — every fix extends P07/P10/
P13/P14/P15/P16); no re-derivation of market research; no operator rulings made here (O3, O20 are
flagged where they bite); no implementation, no test code written by this doc.

---

## 2. Ground truth + back-of-envelope

**What the kernel is:** a deterministic `std`-only core (`kernel/src/lib.rs:1-4`) with 423
`#[test]` occurrences across 48 files (grep-counted this session; the roadmap's "337 tests" is the
`cargo test` pass count on the default feature set — both numbers are honest, they count different
things). **None of these run in CI** (roadmap §1.3 — Phase 1 is still the epistemic gate on every
GREEN claim below; every "tested" verdict in §3 means "a `#[cfg(test)]` suite exists and asserts
the behavior in-source", verified by reading the test bodies, not by executing them, since this
audit pass has no Bash).

**Back-of-envelope (per my canon — sizes every decision below):**
- One hub = one restaurant: peak ~2–4 orders/min ≈ 300–600 orders/day ≈ ~5k lifecycle events/day.
- Kernel ops: `assert_transition` is a table lookup (~ns); `place_order_priced` is O(items)
  BTreeMap lookups (~µs); `MeshEvent::event_id` = SHA3-256 over ~1 KB (~1–3 µs pure-Rust).
  A hub's entire day of order traffic folds in **well under one second of CPU**. Nothing in the
  delivery domain justifies a heavier runtime than the current instance-scoped structs — the
  correct scaling unit is MORE HUBS (processes), never a bigger shared kernel.
- Mesh at N=1,000 hubs = 1,000 sovereign processes, zero shared mutable state by construction
  (verified below with one exception, §5.2) — no connection-pool math applies because there is no
  shared DB in the canonical build (D1: native store per hub).
- The twin-hub interleaving test proposed in §6 (2 hubs × 10⁵ seeded ops) costs < 1 s CPU —
  trivially CI-affordable once P01 exists.

---

## 3. PART 1 — Flow-by-flow audit (gap table)

Verdicts: **BUILT-AND-TESTED** (real logic + in-source test suite asserting it) · **PARTIAL** ·
**MISSING-FROM-BACKEND** · **WRONGLY-IN-FRONTEND-per-old-docs** (logic exists only in a UI/design
doc). Every claim carries file:line read this session.

| # | Flow | Backend reality (evidence) | Verdict | What's needed (owner) |
|---|---|---|---|---|
| 1 | **Order creation (own edge)** | `place_order` (`domain.rs:122`) → `Pending`; `place_order_priced` (`domain.rs:163`) re-derives every price from `PriceCatalog`, fail-closed on unknown product (tested: `domain.rs:482-517`); overflow-safe subtotal (`domain.rs:62-72`); FSM boot/post-fold drift gates (`lib.rs:177`, `domain.rs:220-231`) | **BUILT-AND-TESTED** (the pricing+FSM core) — but see #2, #4 for what "intake" still lacks | — |
| 2 | **Order intake as a POLICY surface** (store-state check, auto-accept, prep-quote, idempotent submission) | Nothing consults store state (none exists, #12); no auto-accept policy; no idempotency binding between `place_order` and the event-log content-id in any tested path (`event_log.rs:580-609` composes `assert_transition` with the log, never `place_order`); NOTE: `kernel/src/intake.rs` is a **name collision** — it is the BP-08 spec-admission compiler (`intake.rs:1-19`), not order intake | **MISSING-FROM-BACKEND** | P13 §3 spine + HUB-D4 StoreState consult; rename or re-doc `intake.rs` to kill the collision |
| 3 | **Marketplace/phone channel intake** | `Order.channel: Option<String>` (`domain.rs:47`) is an attribution tag only; `ChannelBridge` port is design-only (HUB-D2); zero adapter code anywhere | **MISSING-FROM-BACKEND** | HUB-D2 port (P10/P13 seam), post-G11 adapters |
| 4 | **Payment / charge / settlement** | `cash_pay_with` is a pass-through `Option<String>` (`domain.rs:51`). `price_trusted` exists (`domain.rs:56`) with the comment "Downstream (charge/settlement) MUST refuse to charge an untrusted order" — **that downstream does not exist**: grep shows `price_trusted` is set (`domain.rs:150,202`, `wasm.rs:158`) and asserted in tests, but READ by no enforcement point in the repo. No charge, no capture, no settlement, no double-entry ledger (P13 §5 is blueprint-only) | **MISSING-FROM-BACKEND** (the entire money-movement plane) | P07 + P13 §5; the `price_trusted` enforcement point is the first brick |
| 5 | **Tax math** | `apply_tax` half-up, inclusive+exclusive, overflow → typed `Err` (`money.rs:94-112`; tests `money.rs:277-320`) | **BUILT-AND-TESTED** | — |
| 6 | **Delivery fee** | `compute_delivery_fee`/`estimate_order_total` (`money.rs:199-241`) are explicitly a **client-view mirror** — the header (`money.rs:158-165`) says "The SERVER (apps/api orders.ts fee ladder) stays the single source of truth for what is CHARGED"; **that server is deleted at HEAD** (roadmap §1.2). Distance-tiered fees return `None` ("server-only", `money.rs:208-210`) — so the authoritative distance-fee computation now exists **nowhere** | **PARTIAL, with a stale authority pointer** — the kernel holds the display mirror of a charging authority that no longer exists | P13 must promote a kernel fee ladder to THE authority (incl. distance tiers) and re-point the mirror comment |
| 7 | **Order lifecycle (accept/prep/ready/dispatch/deliver/complete)** | 10-state FSM, happy path + pickup + reject/cancel-from-Pending, terminal set, fold semantics, golden-signature drift gate (`order_machine.rs:64-78,472-483`; 23 tests `order_machine.rs:663-977`; 15 more in `domain.rs`) | **BUILT-AND-TESTED** (forward path) | — |
| 8 | **Cancellation / compensation edges** | `Cancelled` and `Rejected` are reachable **only from `Pending`** (`order_machine.rs:67`). A `Confirmed`/`Preparing`/`Ready`/`InDelivery` order **cannot be cancelled by anyone** — customer, owner, or system. The HUB doc names this: "compensation edges = Phase 7". `Scheduled` is a scaffold-disabled orphan (`order_machine.rs:63-64,80-82,835-839`) | **MISSING-FROM-BACKEND** — and the Dashboard UI plan sells "Reject→CANCELLED" as an owner action on live orders (`dowiz-interfaces/DOWIZ-INTERFACES-PLAN.md:328-329`) that the FSM would refuse for anything past Pending | P07 (money-law closure owns compensation); FSM edit must bump `FSM_GOLDEN_SIGNATURE` with rationale (the gate exists precisely for this) |
| 9 | **Refunds / error charges / disputes** | Refund: zero code (grep `refund` in `kernel/src` hits only retrieval-corpus fixture text, `retrieval/recall.rs:50-51`). Dispute: design-only, and the only spec contradicts M12+M6 — blocked on operator ruling **O3** (roadmap §1.8) | **MISSING-FROM-BACKEND** | P14, gated on O3 |
| 10 | **Courier assignment (dispatch)** | Exists in the OTHER repo: HRW rendezvous matcher, structurally scoring-free (`/root/bebop-repo/bebop2/proto-cap/src/matcher.rs:41` `hrw_weight`; `Courier` carries pubkey ONLY, `:33-36`) — verified live this session. dowiz has **zero code-level dependency on it** (roadmap §1.1: one stray comment at `domain.rs:524`) | **PARTIAL (built elsewhere, unintegrated)** | P13 wires it; HUB-D3 `DispatchProvider` port makes it adapter-0 |
| 11 | **Courier tracking / ETA** | Pure kinematics: `haversine_meters` (`geo.rs:15`), `ema_next` 1-D Kalman (`geo.rs:39`), `eta_seconds` (`geo.rs:153`), `is_arriving` (`geo.rs:194`); 11 tests. `TrustEstimate` Kalman threaded through the fold (`domain.rs:248-311`), deliberately OFF the `Order` struct (courier-scoring red line), fail-closed on missing observation (tested `domain.rs:593-612`) | **PARTIAL** — math is BUILT-AND-TESTED; no courier-location event ingest, no shift model, no prep-time estimation (travel-only ETA) | P13 ingest; HUB-D4 prep estimator reuses `ema_next` |
| 12 | **Store state (open/busy/paused/hours)** | Nothing in kernel. `KitchenBusyToggle`, "delivery pause/resume", working hours exist ONLY as UI rows (`DOWIZ-INTERFACES-PLAN.md:335,348-350`) | **WRONGLY-IN-FRONTEND-per-old-docs** | HUB-D4 `store_state.rs` (Wave-0-grade, no dependency) |
| 13 | **Menu / availability (86) / schedules** | `PriceCatalog` is a flat price map (`catalog.rs:30`), fail-closed `unit_price` (`catalog.rs:61-74`, 4 tests). No menu entity, no availability state, no schedule, no versioning. `Cart::reconcile` (`cart.rs:117-129`) drops delisted products — the closest thing to availability logic, and it is client-cart-shaped. MenuManager/86-toggle/MenuScheduleEditor are UI-only (`DOWIZ-INTERFACES-PLAN.md:333-336`) | **WRONGLY-IN-FRONTEND-per-old-docs** (HUB-G1, the #1 vendor surface) | HUB-D1 `menu.rs`, `PriceCatalog` demoted to projection |
| 14 | **Cart** | Single kernel authority: dedupe/add/remove/price/reconcile, integer money, overflow-safe (`cart.rs:35-129`, 6 tests) — a correct example of logic PULLED OUT of two JS carts into the kernel (`cart.rs:1-8`) | **BUILT-AND-TESTED** | — |
| 15 | **Promotions / discounts** | `compute_order_total` scope explicitly excludes discounts (`domain.rs:11`, `:89-91` — the legacy oracle's `- discountTotal` term was dropped in the port). Promotions CRUD is a UI-plan row (`DOWIZ-INTERFACES-PLAN.md:342-343`) | **WRONGLY-IN-FRONTEND-per-old-docs** — and money-adjacent: a discount slot re-enters `compute_order_total`, so it is red-line work | P13/P20 (DM-2 offer-redemption ledger already hard-depends on P07) |
| 16 | **Multi-location / multi-vendor** | Zero `location`/`org`/`tenant` identifiers in any kernel type (grep-verified, §5.1). Historical schema had organizations→locations; the mesh never ruled what a location IS | **MISSING-FROM-BACKEND + unruled** | Operator decision **O20** (HUB-D6), then D1's attenuation-shaped override format |
| 17 | **Notifications** | `messenger.rs` builds TG/WA/Viber deep links — construction only, "never sends" (`messenger.rs:7-8,33-53`, 5 tests). `Spool` is the crash-safe queue a notification drainer would ride (`spool.rs:36`, 6 tests). No notification content/dispatch/preference logic | **PARTIAL** — plumbing exists, the flow doesn't | P22 `SocialPoster`-pattern port; owner "preset messaging" (UI plan :329) needs a kernel template model |
| 18 | **Analytics** | `ChannelLedger`: `orders_by_channel`, fixed-shape `funnel`, `reduce_anomalies` folding through the real Law (`analytics.rs:40-165`, 5 tests) — the first deterministic attribution reader | **PARTIAL** — `ChannelEvent` carries **no money field** (`analytics.rs:27-32`), so the Dashboard's headline "Revenue" KPI (`DOWIZ-INTERFACES-PLAN.md:326`) has **no backend reader at all**; no item-level profitability (HUB-G7) | Extend the deterministic-reader pattern with a revenue reducer over P13 ledger entries (HUB-D5) |
| 19 | **Customer identity / OTP / CRM** | `customer_id: Option<String>` pass-through (`domain.rs:42`); no customer entity, no OTP, no PII handling in kernel. CRM page = UI-only (`DOWIZ-INTERFACES-PLAN.md:340-341`) | **WRONGLY-IN-FRONTEND-per-old-docs** | P16-adjacent kernel model; PII rules (claim-check, nothing to ШІ) apply |
| 20 | **Courier ops (invites/shifts/earnings)** | Nothing in kernel. Couriers page plans earnings/deliveries/shifts detail (`DOWIZ-INTERFACES-PLAN.md:351-352`) — **earnings is money** with zero backend; the plan itself flags a courier `rating` read to reconcile vs NO-COURIER-SCORING | **WRONGLY-IN-FRONTEND-per-old-docs** (money-adjacent) | P13 payout saga feeds earnings as a ledger projection; rating-read must die in the P16 rebuild |
| 21 | **Supplies/inventory** | UI plan itself admits `localStorage`-only (`DOWIZ-INTERFACES-PLAN.md:357`) — state living in the browser | **WRONGLY-IN-FRONTEND-per-old-docs** (self-flagged) | local-first event-log migration per the plan's own §4 note |

### 3.1 Where the operator's principle IS already honored (counter-evidence, for honesty)

The claim "frontend = thin projection" is not aspirational everywhere — three places enforce it
structurally today:

- `engine/src/money_guard.rs:15-68` — `Money` deliberately does NOT implement `FieldValue`, so
  `interpolate(money,…)` is a **compile error**; `TweenGuard::jump` is the only legal money
  transition. The render engine is type-incapable of inventing monetary values.
- `money.rs:219-241` `estimate_order_total` degrades to `None` rather than fabricate a number it
  can't back (incl. the P07 tax-overflow RED→GREEN at `money.rs:429-441`).
- `cart.rs` — two legacy JS cart implementations were consolidated INTO the kernel (`cart.rs:1-8`),
  the exact direction the operator mandates.

### 3.2 The sharpest single anti-pattern finding

**The only order-creation surface exposed to a JS frontend is the untrusted-price path.**
`wasm.rs:41` imports `place_order` (legacy, caller-priced), NOT `place_order_priced`; `wasm.rs:158`
hard-codes `price_trusted: false`; and `lib.rs:151`'s headline re-export also omits
`place_order_priced`. Combined with #4 above (no enforcement point reads `price_trusted`), the
current end-to-end reality is: **a frontend can set its own prices and nothing downstream would
refuse them** — the M1/M2 fix exists in the kernel but is not wired to the boundary. One-line-fix
class (export/bind the priced path; add the refuse-untrusted gate when charge code exists), but it
must be named: today the boundary trusts the display layer, which is the operator's anti-pattern
inverted at the exact seam that matters most (money).

Also load-bearing: `wasm.rs:51` `static ORDER_SEQ: AtomicU64` — a **process-global** order-id/
timestamp source on the order path (wasm feature). Harmless one-hub-per-process; a genuine
cross-tenant bleed if any runtime ever multiplexes hubs through one wasm instance (§5.2).

---

## 4. PART 2 — Multi-tenant ground truth: how the hub/tenant boundary is represented today

### 4.1 It isn't — and that is (mostly) the design, not an accident

Grep across all of `kernel/src` and `engine/src` for `tenant|hub_id|HubId|location_id|org_id|
multi-tenant|cross-hub`: **zero domain hits**. The only identity-bearing fields are
`MeshEvent.actor_pubkey` (`event_log.rs:139` — an ACTOR, not a tenant) and `hydra.rs`'s `node_id`
(self-witness alerts). No kernel type is hub-scoped; no test anywhere exercises two hubs.

Per canon this is coherent: **M5/M10 make the tenant boundary a PROCESS boundary** — one hub = one
sovereign node with its own kernel instance, own event log, own store (`ARCHITECTURE.md:14,19`).
Tenant isolation is intended to be *structural* (separate OS processes / microVMs —
`isolation/microvm.rs` already fail-closes native-process adapters on hosts without KVM), not
*row-scoped* (the legacy RLS model whose schema had `organizations→locations`). The kernel's
shared primitives are shared **code**, never shared **state**.

### 4.2 But the assumption is currently unstated, unenforced, and violated once

If the process-per-hub invariant is what carries all tenant isolation, three facts matter:

1. **Every shared primitive is single-tenant by construction and silently wrong if shared.**
   Verified against the live types:
   - `EventLog` has ONE chain tip (`event_log.rs:275-311`); `append` binds a zeroed `prev` to the
     current tip — two hubs sharing one log would interleave into ONE hash chain (hub B's event
     chains onto hub A's tip, changing B's content-ids: `event_log.rs:293-301`).
   - `PriceCatalog` keys on bare `product_id` strings (`catalog.rs:30-32`) — two hubs' `"p1"`
     collide; last insert wins, and hub A's customers get hub B's price.
   - `ChannelLedger.ingest` **silently ignores** a duplicate `order_id` and locks its channel at
     first sighting (`analytics.rs:60-66`, tested `:174-198`) — cross-hub order-id collision =
     silent misattribution with `false` as the only signal.
   - `Spool` is one queue, one capacity, one backpressure signal (`spool.rs:36-42`).
   - `wasm.rs:51` `ORDER_SEQ` is the one **actual global mutable** on the order path (the rest of
     the kernel has none — grep for `static mut|OnceLock|lazy_static` finds only the wasm counter,
     a retrieval cache `retrieval/recall.rs:306`, and instance-scoped `Mutex`es in
     `token_bucket.rs`/`retrieval/memory_store.rs`).
2. **Nothing verifies signatures inside dowiz.** `event_log.rs:22-23,274` says events "carry an
   actor_pubkey" and "the network layer never re-runs decide — it only verifies signatures"; that
   network layer lives in bebop2 and is not wired here (roadmap §1.1). Until P9, `actor_pubkey` is
   32 unverified bytes — a hub replaying another hub's events into its own log is undetectable at
   this layer. (Correctly sequenced: P3→P9 own this. Named here so the test strategy doesn't
   pretend the property exists early.)
3. **No test pins any of this.** The invariant "these primitives must be instantiated per hub" is
   documented in module headers ("per-node", `event_log.rs:1`) but has no RED test that fails if a
   future runtime multiplexes them.

### 4.3 The boundary-representation decision (two options, per protocol — recommendation flagged)

| | **Option A — process-per-hub stays THE boundary; tests enforce it structurally** (recommended) | **Option B — hub_id scoping inside kernel types** |
|---|---|---|
| Concept applied | Isolation-by-instance (shared-nothing); the mesh analog of one-DB-per-tenant | Row-scoping (the legacy RLS shape ported into struct keys) |
| Fits canon | Exactly M5/M10 + "schema rich, runtime minimal" — the seam (a `Hub` owning its instances) goes in the runtime's composition root, kernel types stay tenant-free | Contradicts M10 ("intra-hub = hub's own business") by baking a tenancy notion into the shared kernel every solo hub must carry |
| Failure mode | A multiplexing runtime violates the invariant silently → **closed by T1–T3 below** (the tests make the violation loud) | Key-discipline bugs (forgot the hub_id in one map) → the exact class RLS existed to catch, re-imported without RLS's enforcement |
| O20 interaction | Compatible with both O20 rulings (a location = sub-hub is just another process; = intra-hub row is hub-internal data, still one process) | Prejudges O20 toward rows |
| Cost | Test-only; zero kernel churn | Touches every shared primitive's key type |

**Decision (flagged, overridable): Option A.** Tenant isolation stays structural; the engineering
work is to make the invariant **tested and impossible to violate silently**, not to re-key the
kernel. If O20-a (location=sub-hub) is ruled and P15 sub-hubs later share a parent process, this
decision must be revisited explicitly — that trigger is named, not assumed away.

---

## 5. The multi-tenant test strategy (T-layers, each with a falsifiable done-check)

Ordered cheapest-first; T1–T4 are Wave-0-grade (plain `#[test]`, zero new deps, no operator
ruling); T5–T6 ride P9/P10 when those land. Every layer names the failure it makes loud.

### T1 — No-shared-state structural gate (CI, deterministic)
A CI check (Phase-1 lane, same genre as `ci-no-courier-scoring.sh`) that greps `kernel/src` for
process-global mutable state (`static mut`, `OnceLock`, `lazy_static`, `thread_local`, non-`const`
`static` with interior mutability) against an explicit allowlist. Today's honest allowlist:
`wasm.rs:51 ORDER_SEQ` (with an `innovate:` ceiling comment — upgrade trigger: wasm surface gains
an instance handle), `retrieval/recall.rs:306` (read-only memo). Anything new fails the gate with
"name it or scope it".
**Done-check:** deleting the allowlist entry makes the gate RED on the current tree (proves it
reads the real tree); the gate is GREEN with the allowlist.

### T2 — Twin-hub non-interference property test (the core proof)
In one test binary, instantiate **two complete hub cores** (two `EventLog<MemEventStore>`, two
`PriceCatalog`s with overlapping product ids at DIFFERENT prices, two `ChannelLedger`s, two
`Spool`s). Drive both with an interleaved op stream from the kernel's own seeded PRNG
(`rng.rs` SplitMix64→PCG64 — no new dep, reproducible by seed). Assert the **projection
property** (non-interference, the CS name for "no leak"): hub A's final state (chain tip, order
totals, funnel counts) is byte-identical to a solo run of A's op subsequence alone, for every
seed tried; and no content-id from A's log exists in B's store.
**Done-check:** the test exists, runs < 1 s at 10⁵ ops (per §2 envelope), and — RED-first — a
deliberately-shared single `PriceCatalog` variant of the same harness FAILS the price assertion
(A quotes B's price), proving the assertion has teeth. Failure prints the seed (hand-rolled
shrinking substitute — see DECART probe).

### T3 — Chain-confusion RED pin (documents today's hazard as a test)
One `EventLog` shared by two logical actors: assert that B's zero-`prev` append chains onto A's
tip and that B's content-id therefore DIFFERS from B's solo-log content-id (this is directly
derivable from `event_log.rs:293-301` + the existing test at `:614-636`). The test's doc-comment
states the invariant it pins: **`EventLog` is per-hub; a runtime that multiplexes hubs through one
log corrupts both hubs' content-addressing.**
**Done-check:** test passes today (it asserts current single-tenant behavior); any future "make
EventLog multi-tenant" change must consciously rewrite this pin, which is the point.

### T4 — Shared-primitive collision-semantics pins
- `ChannelLedger`: duplicate `order_id` across tenants → second silently ignored
  (`analytics.rs:60`) — pin it, and derive the stated requirement: **order ids must be globally
  collision-free** (content-address or UUID) before any cross-hub analytics aggregation exists.
  The wasm `ORDER_SEQ` (`seq`-based ids) fails that bar — named follow-up under P13's intake spine.
- `PriceCatalog`: overlapping-key insert overwrite — pin as "per-hub instance only".
**Done-check:** both pins in-suite; each doc-comment names the runtime rule it implies.

### T5 — Cross-hub protocol boundary (lands with P9; contract named now)
When the P9 wire arrives: (a) an event authored by hub A's actor key, replayed toward hub B, must
be rejected at signature-verify (bebop2 `proto-cap` owns this; dowiz's `commit_after_decide` never
re-runs decide on synced events, so verify-or-refuse is the ONLY gate — `event_log.rs:274`);
(b) M12 replay-nonce/expiry tests on capability frames; (c) the HUB-D3 `PeerHubProvider`
done-check (d) already requires the two-hub ledger-conservation test — reuse it as the money-leg
isolation proof (Σ==0 on BOTH ledgers, no phantom entries on a third hub).
**Done-check:** inherited from BLUEPRINT-P09's acceptance criteria + HUB-D3 §done-check (c); this
doc adds only the framing "these ARE the tenant-isolation tests at the wire".

### T6 — Cascade containment (lands with P10)
A hub whose local `decide` closure panics/rejects (the two poles are already typed —
`CommitError::Rejected` vs `Store`, tested `event_log.rs:563-574,724-745`) must not affect a peer
hub: at the runtime level this is process/microVM isolation (`isolation/microvm.rs` KVM probe,
fail-closed) plus P10's kill-switch scoping (M9: kill a hub/subtree, never the mesh).
**Done-check:** P10's AC set + one added two-process test: kill -9 hub A mid-fold, assert hub B's
order flow completes and A's log replays to a consistent tip on restart (the crash-safety
`spool.rs`/`event_log.rs` durability tests already cover the single-hub half —
`event_log.rs:701-716`).

### Consistency + idempotency notes (concepts named, per canon)
Idempotency is content-addressing (`event_id = sha3(prev‖actor‖seq‖payload)`, `event_log.rs:148`),
NOT TTL-dedup — a replayed duplicate is a structural no-op with `decide` not re-run (tested,
`event_log.rs:533-561`). Tenant isolation composes with this: two hubs are two chains, so
cross-tenant "duplicates" cannot exist BY ID unless ids collide (closed by T4's requirement).
Consistency model between hubs = local-first, eventual, signed (CAP: AP inter-hub, CP intra-hub via
single-writer fold) — nothing in this strategy invents a cross-hub transaction, and T5(c)'s
conservation check is deliberately per-ledger, not a distributed commit.

### Failure/degradation per layer
T1 fails → CI RED, zero runtime effect. T2–T4 fail → a real isolation regression; these are
kernel-crate tests, so the failure blocks merge at the same gate as the money tests (once P01 runs
them — see §7). T5/T6 failures degrade to their phase's own fail-closed behavior (unverified frame
dropped; hub island-continues per P13 AC-6). No layer introduces a runtime component that can
itself fail — the strategy is tests + one grep gate, nothing standing.

---

## 6. DECART — test-infrastructure choice (Integration Decart Rule; new-integration decision)

| Criterion | Hand-rolled seeded property harness on `kernel::rng` (chosen) | `proptest`/`quickcheck` crate | Model checking (`kani`) / `loom` |
|---|---|---|---|
| Bare-metal / hermetic fit | Zero new deps; `rng.rs` is already the kernel's reproducible-MC primitive | New dev-dependency + its transitive tree into the zero-dep kernel's dev graph | Heavy toolchain (kani needs its own solver install; network-gated here) |
| Falsifiable correctness | Seed printed on failure → exact reproduction; RED-first shared-catalog variant proves teeth | Better: automatic shrinking finds minimal counterexamples | Strongest: exhaustive within bounds |
| Measured performance | 10⁵ ops < 1 s (§2 envelope) | comparable | minutes-to-hours; wrong cost tier for a CI gate |
| Supply-chain / license | none | MIT/Apache, fine but nonzero | toolchain, not a crate |
| Maintainability | ~200 lines in-suite, mirrors existing test style (`domain.rs:531-589` already hand-rolls SplitMix64 in a test) | idiomatic but a second idiom next to 423 existing plain tests | specialist knowledge |
| Reversibility | delete the module | remove dep | remove toolchain |

**DECISION:** hand-rolled seeded harness — falsifiable reason: every property in T2–T4 is a
deterministic state-projection equality over a finite op alphabet (≤6 op kinds), where a printed
seed + op-log IS a full reproduction; shrinking's marginal value doesn't buy a dev-dependency into
a crate whose whole identity is zero-dep (`lib.rs:199-205` documents that gate).
**Older-as-adapter note:** n/a (nothing replaced).
**Probe (strongest case against):** without shrinking, a failure at op #73,412 is a miserable
debugging artifact; proptest would hand back a 3-op counterexample. Mitigation is real but partial:
keep the op alphabet tiny, log the op sequence on failure, and binary-search the prefix by re-run
(cheap at <1 s/run). If T2's failures in practice prove hard to minimize, adopting `proptest` as a
**dev-dependency only** is a one-line reversible upgrade — that trigger is named here so the
future decision cites this table instead of re-litigating.

*(T1's grep gate reuses the existing `scripts/ci-*.sh` pattern — in-repo reuse, no DECART needed;
recorded so the absence is a decision, not an oversight.)*

---

## 7. Operability + sequencing

- **Everything in §5 is unverifiable until Phase 1 (CI Truth Floor) lands** — kernel tests run
  nowhere in CI today (roadmap §1.3). T1–T4 should land WITH or immediately after P01's kernel-test
  job so they are born enforced, never aspirational. This is Ananke applied: an isolation test that
  CI doesn't run is a hope, not a property.
- Observability: T2's harness should emit its seed + op-count as a one-line deterministic telemetry
  record per run (kernel telemetry stays std-only, per the AGENTS.md native-telemetry doctrine) —
  a failing seed is then in the CI log within one read, no re-run archaeology.
- Rollback: all layers are additive test code + one CI script — revert = delete; zero runtime
  exposure, no flag needed.

---

## 8. Open / accepted risks (owner named)

| # | Risk | Status | Owner |
|---|---|---|---|
| R1 | `place_order_priced` unexposed at the JS boundary + no `price_trusted` enforcement consumer (§3.2) — the money-integrity fix exists but isn't wired | **OPEN — highest-severity Part-1 finding**; fix belongs to P13 intake (or a P07-adjacent quick wire) | P07/P13 implementer |
| R2 | Authoritative distance-tier delivery fee exists nowhere (§3 #6) — mirror points at deleted code | OPEN; P13 fee-authority | P13 |
| R3 | Compensation edges absent — no cancel past `Pending` (§3 #8), while UI plan promises owner reject on live orders | OPEN; P07 owns; requires a conscious `FSM_GOLDEN_SIGNATURE` bump | P07 + operator (lifecycle change is gated by design) |
| R4 | `wasm.rs ORDER_SEQ` global + seq-shaped order ids fail the global-uniqueness bar T4 derives | OPEN (small); P13 intake replaces id minting | P13 |
| R5 | Tenant isolation rests on process topology with zero tests until T1–T4 land | OPEN — this doc's whole Part 2 is the closure plan | next implementation pass |
| R6 | Cross-hub signature verification absent until P3→P9 (§4.2.2) | ACCEPTED for now — correctly sequenced by the roadmap's critical path; T5 names the contract so it can't be forgotten | P09 |
| R7 | O20 unruled — if location=sub-hub later shares a process, Option A's premise weakens | ACCEPTED with named trigger (§4.3) | operator (O20) |
| R8 | `intake.rs` name collision invites future confusion between spec-admission and order intake | OPEN (cheap: doc/rename) | any Phase-1 cleanup pass |

---

## 9. 2-question doubt audit (AGENTS.md ritual, applied to THIS document)

**Q1 — least confident about (6 items):**
1. **"Tested" ≠ "passing"**: no Bash this session, so every BUILT-AND-TESTED verdict rests on
   reading test bodies, not executing them. On a branch under active development that's a real
   (if small) gap — P01 is the systemic fix; a `cargo test` run should accompany any action taken
   on this audit.
2. **The 423-vs-337 test-count reconciliation** is my inference (occurrences vs. default-feature
   pass count incl. `#[cfg(feature="wasm")]`-gated suites); not proven by running both counts.
3. **`price_trusted` enforcement absence** was verified by grep in `*.rs` only — if an enforcement
   design exists in a blueprint I didn't open (P07's full text), the finding stands for CODE but
   the "nobody has even designed it" implication would soften.
4. **engine/src received a lighter pass than kernel/src** (read `money_guard.rs` fully, others by
   name/roadmap only) — I judged the render engine out-of-scope for delivery-flow logic; if
   business logic hides in `widget_store.rs`/`scene.rs`, this audit missed it.
5. **T2's projection property assumes op streams are per-hub-partitionable** — true for today's
   primitives (no cross-hub op exists in kernel), but the moment `PeerHubProvider` (HUB-D3) lands,
   non-interference must be restated as "interference ONLY through signed protocol frames," a
   materially harder property this doc names but does not design.
6. **bebop2 matcher evidence** is one file read in a sibling repo whose branch state I did not
   verify against ITS canon — the "built elsewhere" verdict for dispatch inherits that repo's own
   branch-implicitness hazard (roadmap §8.3.1).
7. **DOWIZ-INTERFACES-PLAN line numbers** (:326-357) are read-this-session but that doc is a plan
   under revision; treat as anchors, re-verify at implementation.

**Q2 — the biggest thing I might be missing:** this audit judges "backend completeness" against the
kernel-as-backend framing, but the honest current state is that **there is no runtime backend at
all** — no process serves orders today (apps/* deleted, node binary not built, P13 unlanded). The
gap table can therefore under-alarm: rows marked BUILT-AND-TESTED are *libraries awaiting a
server*, and the operator's principle ("backend owns every function") is currently satisfiable
vacuously because neither side is live. The 1-in-4 check on this: I re-read the roadmap's G11
flag (§7.2) rather than assuming — the tension is already named at the charter level
(quantum-substrate-first vs. first-real-order-first), so this is a known operator-level question,
not a new blind spot; but a reader of THIS doc should not mistake a green gap-table row for a
shippable flow.

---

## 10. Anu / Ananke check

- **Anu (derivable, not asserted):** every verdict in §3 carries file:line read this session; the
  two claims taken from other docs (dispute-spec contradiction, CI regression) are cited to the
  roadmap's own re-verified findings, not to memory; the one cross-repo claim (matcher) was
  re-verified by reading the file, with its residual branch-hazard named in Q1-6; the Option A/B
  decision is argued from canon text (M5/M10) + failure-mode comparison, and its weakening trigger
  (O20-a) is explicit.
- **Ananke (structure forces the outcome):** the strategy's core moves are structural, not
  disciplinary — a grep gate that fails CI (T1), a RED-first harness variant that proves the
  assertion can fail (T2), pins that force any future multiplexing change to consciously rewrite a
  named test (T3/T4), and the sequencing rule that T-layers land WITH the CI job that runs them
  (§7) so no isolation test can exist unenforced. Where structure can't force it yet (signature
  verify pre-P9; the vacuous-completeness hazard in Q2), the doc says so instead of hoping.

*This document plans; it changes no code and no canon. Follow-ups proposed to owners: R1–R4 into
P07/P13 execution; T1–T4 into the Phase-1/next kernel implementation pass; R7 rides O20; R8 is a
cleanup one-liner.*
