# HUB DESIGN — VENDOR-SIDE MARKET RESEARCH + GAP-CLOSING ADDENDUM TO P10/P15 (2026-07-17)

> **What this is:** external market research (how the best digital delivery services serve the
> RESTAURANT OWNER today) + a code-grounded comparison against dowiz's own hub-autonomy design
> (M5, `BLUEPRINT-P10-hub-runtime-kill-switch-boot.md`, `BLUEPRINT-P15-living-organism-unbounded.md`)
> + a gap-closing design addendum. **Planning artifact only — no code is written or edited by this
> document.** It does not duplicate P10/P15; every design item below names the existing blueprint it
> extends and the seam it plugs into.
> **Method:** three parallel web-research passes (US marketplace platforms; POS/restaurant-OS +
> aggregator middleware + Olo; EU/regional players + white-label dispatch + direct-ordering), run
> 2026-07-17, sources cited inline — plus live repo reads this session (file:line cited below).
> **Constraint honored throughout:** all-Rust-native (`hermetic-architecture-2026-07-16/
> HERMETIC-ARCHITECTURE-PRINCIPLES.md`); any external tool appears only as a **thin Rust
> adapter/bridge behind a Trait-as-Port**, never an absorbed ecosystem — the exact pattern
> `HARNESS-LLM-BACKEND.md` already set for LLM backends.
> **Research coverage:** all three passes completed in full (the marketplace pass was interrupted
> once by an API error and re-run to completion; its findings are folded into §2.1). One residual
> caveat carried from that pass: Deliveroo publishes far less than DoorDash/Uber — its "not found"
> points below are **unconfirmed** absence, not confirmed absence.

---

## 1. Ground truth — what dowiz's hub design currently is (live-verified this session)

### 1.1 The hub, per canon and P10/P15, is a TRUST/TRANSPORT/COMPUTE entity — not yet a VENDOR-OPS entity

- **M5** (`ARCHITECTURE.md:14`): *"Every HUB = autonomous HYDRA: may change OWN rules, open
  ports/bridges, use any models/API/MCP/agents at its discretion."* The realization of M5 is P10's
  `HubPolicy`-as-data (`BLUEPRINT-P10 §2.1`): `{revision, policy_sha3, listeners, bridges,
  model_endpoints, access_lists, red_line_policy, rate_limits}`. Every field is about **transport,
  trust, and model access**. Nothing in `HubPolicy` — or anywhere in P10/P15 — models the things a
  restaurant owner actually operates: a menu, store state (open/busy/paused), kitchen capacity,
  external sales channels, courier-fleet choice, or financial statements.
- P15 generalizes *self-modification of that same policy surface* (sub-hubs, model manifests,
  per-agent capability minting, eqc-gated self-update) — again agent/trust machinery, not vendor ops.
  P15's sub-hubs (§4) carry **agent-recursion** semantics (depth-capped delegation), not
  **organization-structure** semantics (a chain's second location).
- P13 builds the order spine (intake → signed envelope → DoD gate → fold), PoD k-of-n, and the payout
  saga on a double-entry ledger; its AC-6 **solo-hub island test** (full order→delivery flow with
  ZERO other services running) is the strongest offline-resilience guarantee in this genre — stronger
  than anything in §2. P16 rebuilds the owner UI (16 owner pages incl. MenuManager, Analytics,
  Settings with pause/resume) — but as UI over kernel primitives **that do not exist yet** (§1.2).

### 1.2 Kernel reality — what exists under the owner-facing plans (read this session)

| Vendor-ops concern | What the kernel has today | What it does NOT have |
|---|---|---|
| Menu / pricing | `kernel/src/catalog.rs` `PriceCatalog{product_id → PriceEntry{base, modifiers}}` — trusted price re-derivation, fail-closed on unknown product (`unit_price()` errs, never falls back to client price) | No menu entity: no availability/86 state, no schedules/dayparting, no versioning, no per-channel price dimension, no publish/draft. `PriceCatalog` is a flat price map |
| Orders | `kernel/src/order_machine.rs:8` `OrderStatus` FSM (happy-path; compensation edges = Phase 7); `domain.rs:39` `Order` carries `channel: Option<String>` (:47) — an **attribution tag** | No channel *intake* concept — nothing produces orders except dowiz's own edge; no auto-accept policy, no prep-time quote |
| Store state | — | No open/busy/paused/closed state anywhere in kernel; `DOWIZ-INTERFACES-PLAN.md:335` plans a `KitchenBusyToggle` + `:350` "delivery pause/resume" as **UI**, with no kernel model under them |
| Kitchen load / ETA | `geo.rs:39 ema_next` (1-D Kalman), `:153 eta_seconds`, `:194 is_arriving`; `kalman.rs` | Nothing estimates **prep time** or kitchen load; ETA math is courier-travel only |
| Analytics | `analytics.rs` `ChannelLedger` — `orders_by_channel()`, `funnel(channel)`, `reduce_anomalies` (first deterministic attribution reader) | No item-level profitability / menu-engineering, no owner statement projection, no benchmark surface |
| Multi-location | — (historical schema had `organizations → locations → memberships`, `DeliveryOS-As-Built-Summary-v1.md:80`; idempotency was scoped by `location_id`, :119) | The mesh architecture never says what a *location* IS (hub? sub-hub? intra-hub row?). Grep across `docs/design/` finds no ruling |
| External channels | — | Grep across `docs/design/` for Deliverect/Otter/Glovo/Bolt/DoorDash/aggregator: **zero hits** except a UI tap-target note (`DESIGN.md:193`). No design doc anywhere contemplates ingesting or feeding an external sales channel |
| Dispatch | `bebop2/proto-cap/src/matcher.rs` HRW courier assignment (structurally NO-COURIER-SCORING) over **the hub's own couriers** | No concept of renting an external fleet, quoting multiple fleets, or lending the hub's fleet to another hub |
| Money / payout | P13 §5 double-entry ledger port (`conserved()` Σ==0), payout saga, cash reconciliation; P14 dispute/escrow (gated on O3) | No owner-facing statement/reconciliation projection; no per-order fee-transparency line items |

**The one-sentence diagnosis:** dowiz's hub design is, today, a *sovereign node* design (identity,
policy, kill-switch, self-mod, order spine) with an owner UI planned on top — while the market's
"hub" is a *vendor operations cockpit* (menu → channels → orders → dispatch → money → insight). The
two are complementary, not in conflict; the addendum in §4 closes the vendor-ops half **using the
node machinery P10/P13/P15 already specify**, which is why this is an addendum and not a new phase.

---

## 2. Market research — what the best vendor-facing delivery services actually give an owner

### 2.1 Marketplace partner portals (Glovo / Bolt Food / Wolt / Just Eat; DoorDash/UberEats genre)

- **Glovo Manager Portal** ([portal.glovoapp.com](https://portal.glovoapp.com/)): real-time order
  management, menu editing (manual or via the documented OAuth2 [Partners API](https://api-docs.glovoapp.com/partners/index.html)
  with menu bulk-update + order webhooks), one-tap item availability, **"Set to busy (30 mins)" and
  "Close for the day"** demand throttling, sales/ops reports, campaigns, staff access. Commission
  25–35% (~30% typical), with packaging fees, forced-discount campaigns, and VAT-on-commission on top
  ([Menuviel](https://blog.menuviel.com/glovo-fees-and-commissions-for-restaurants/)).
- **Bolt Food Merchant Portal** ([foodpartner.bolt.eu](https://foodpartner.bolt.eu/)): performance/AOV,
  **menu-view→order conversion**, top-item analytics, promotions, menu editor, ratings
  ([2025/26 merchant guide PDF](https://static.food.bolt.eu/menuEditorGuideline/6Pd8i2UyV4nXUD4tJhWTD/EN%20Bolt%20Food%20Merchant%20Portal%20+%20Menu%20Editor%20Guide%202025_2026.pdf)).
  Commission 15–30%.
- **Wolt** — the sovereignty benchmark among majors: **Wolt Hybrid Delivery** (mix own couriers with
  Wolt's, order-by-order) and **Wolt Drive** (white-label B2B logistics API: any merchant plugs
  Wolt's fleet into their *own* checkout, controlling which orders/areas use it)
  ([developer.wolt.com/docs/wolt-drive](https://developer.wolt.com/docs/wolt-drive)).
- **Just Eat Takeaway Partner Hub**: revenue/order/review dashboard, near-instant menu propagation,
  a "Performance Score" with recommendations. Its commission ladder is the market's own admission
  that self-delivery is structurally cheaper: platform-delivery ~25–30%, **self-delivery ~13–14%**,
  click-&-collect ~7–10% ([Menuviel](https://blog.menuviel.com/just-eat-fees-and-commissions-for-restaurants/)).
- **DoorDash Merchant Portal / Business Manager** — the deepest published vendor cockpit:
  **AI photo tooling** (AI Retouch / AI Replate / Match-the-Style; plus AI onboarding that scrapes
  the merchant's own site — [TechCrunch 2026-05](https://techcrunch.com/2026/05/04/doordash-adds-ai-tools-to-speed-up-merchant-onboarding-edit-photos-of-dishes/));
  **auto-confirm** with post-accept prep-time adjust; **"Busy" status auto-pads prep time** on new
  orders; store pause per-channel with duration; **Tablet Heartbeat** — platform auto-pauses the
  store if the tablet is unreachable **5+ min** and auto-resumes within **10 s** of reconnection;
  **published error-charge rules** (25–100% of item price, 100% of subtotal for a wrong order,
  no charge if reported ≥72 h late, **14-day dispute window**); **Self-Delivery at ~6% commission**
  (vs 15/25/30% tiers) with "Flexible Fulfillment" auto-routing between own fleet and Dashers —
  whose **rule changes take 24–72 business hours**, not self-serve; 4-tier role hierarchy
  (Business Group Admin > Business Admin > Store Manager > Store Operator); Report Builder + a
  Reporting API; second-price-auction Sponsored Listings, pay-per-order
  ([merchants.doordash.com/pricing](https://merchants.doordash.com/en-us/pricing),
  [error adjustments](https://help.doordash.com/en-us/merchants/article/what-are-order-error-adjustments),
  [self-delivery](https://merchants.doordash.com/en-us/products/self-delivery)).
- **Uber Eats Manager / Orders**: Menu Maker (photo approval takes **3–5 days**); Busy Mode
  (prep-time inflation) + Pause Mode; **auto-pause after >1 h offline** or repeated unaccepted
  orders, manual resume; **BYOC** (own couriers) is **mutually exclusive** with Uber couriers per
  order; commission Lite/Plus/Premium 20/25/30% + 7% pickup, Self-Delivery 15%; **Webshop**
  commission-free site at 2.5%+$0.29; and an *officially admitted* multi-location gap: SSO
  "does not provide consolidated or additional reporting at this time"
  ([merchants.ubereats.com/pricing](https://merchants.ubereats.com/us/en/pricing/),
  [help.uber.com multi-location](https://help.uber.com/en/merchants-and-restaurants/article/adding-multiple-locations-to-uber-eats-manager-)).
- **Deliveroo Partner Hub**: bulk-CSV menu workflow (8 MB cap, stock updates ≤10 sites/batch,
  5 locales, **non-undoable**); Marketer promotions with the most granular audience segmentation
  found (new / lapsed-56d / students / Plus subscribers) but **no paid-ads auction product at all**;
  the most procedurally transparent refund adjudication of the three — **7-day dispute window,
  72-h re-escalation, photo/CCTV evidence timestamped within 20 min of order-ready**; **no
  white-label dispatch product** (no Drive/Direct equivalent)
  ([help.deliveroo.com bulk management](https://help.deliveroo.com/en/articles/13833812-how-to-use-bulk-management-in-menu-manager),
  [refund disputes](https://help.deliveroo.com/en/articles/6457561-how-to-manage-and-dispute-refunds-in-partner-hub)).
- **White-label dispatch productized**: **DoorDash Drive** ($7.49 ≤5 mi + $0.50/mi) and **Uber
  Direct** (from $7.99/delivery, shareable tracking links + masked-number "Courier Connect"),
  consumed by restaurants through their POS (Toast Delivery Services) rather than the marketplace
  ([support.toasttab.com](https://support.toasttab.com/en/article/Toast-Delivery-Services),
  [developer.doordash.com Drive](https://developer.doordash.com/en-US/docs/drive/overview/faqs/)).

**Genre invariants** (every portal in this class): three-channel order intake (web portal + tablet
+ POS/API injection); per-channel menu editor + availability toggles; busy/pause store control with
bounded snooze; auto-accept configuration; prep-time adjustment; **connectivity-heartbeat
auto-pause** (DoorDash 5 min-pause/10 s-resume; Uber 1 h) so a dead tablet never silently strands
customers; promotions self-serve; ratings/feedback; payout statements; a bounded-window
error/refund dispute flow; and **courier dispatch as a walled garden** (owner sees a map and an
ETA, never assignment logic or fleet control — own-fleet options are either/or per order, never a
live mix).

### 2.2 Restaurant-OS / POS layer (Toast, Square) — the owner's actual operating system

- **Menu single-source-of-truth with hierarchy.** Toast's Menu Manager versions every entity (menu,
  group, item, modifier) with a **target/owner pair on a location tree** — chains share a base menu
  and layer location-specific overrides without duplicating entities; the platform *enforces* that a
  target is a descendant of its owner ([doc.toasttab.com](https://doc.toasttab.com/doc/platformguide/platformMenuManagerMenuAndMultiLocationRestaurants.html)).
  86'ing an item pushes real-time to every connected channel **plus a nightly safeguard sync**
  ([pos.toasttab.com](https://pos.toasttab.com/products/multi-location-management)).
- **Offline mode, precisely bounded.** Toast: card payments queue encrypted-locally
  (background processing), in-house ordering + hardwired KDS keep working, but devices can't sync
  with each other offline and **inbound digital orders die entirely**
  ([doc.toasttab.com/offlineMode](https://doc.toasttab.com/doc/platformguide/platformOfflineMode.html)).
  Square: offline payments queue with a merchant-set cap and are **declined if the device doesn't
  reconnect within 72 h**, merchant bears the risk ([squareup.com](https://squareup.com/help/us/en/article/7777-process-card-payments-with-offline-mode)).
  SpotOn claims the broadest offline surface (POS+KDS+handhelds+kiosks); Lightspeed's bridge does
  orders but **no offline card processing**.
- **Analytics depth.** Toast: live hour-by-hour owner app (Toast Now), **xtraCHEF** invoice-OCR →
  live food-cost → recipe/plate costing → the Stars/Plowhorses/Puzzles/Dogs **menu-engineering
  matrix** as a "real-time menu health barometer" ([xtrachef.com](https://xtrachef.com/food-cost-management-solution/),
  [menu-engineering](https://pos.toasttab.com/blog/on-the-line/menu-engineering-matrix)); **Toast
  Benchmarking** anonymously aggregates ~171,000 locations for peer comparison. Square: profitability
  tool (margin, food-cost %, labor %, break-even), COGS, labor-vs-sales.
- **APIs are narrow and rate-capped**: Toast = **20 req/s global across all APIs** (5 req/s for bulk
  orders) ([doc.toasttab.com](https://doc.toasttab.com/doc/devguide/apiRateLimiting.html)); Square
  won't publish numbers (100 req/h on one endpoint). The "open platform" of the incumbent OS is a
  keyhole.
- **Cost to the owner**: Toast realistic all-in $1,500–2,500/mo mid-volume; Square $250–1,500+/mo;
  every middleware layer below adds $85–$500/mo/location on top.

### 2.3 Aggregator middleware (Otter, Deliverect, Chowly, ItsaCheckmate) and Olo — the unification tax

- The category exists because marketplaces don't interoperate: manual re-keying of tablet orders has
  a measured **15–20% error rate per shift** (~$30/error) ([orderout.co](https://www.orderout.co/blog/order-entry-errors/)).
- **Deliverect**: POS order **injection** (validate items/modifiers/taxes, dedupe, map to
  menus/printers, status back to channel) at a cited **99.8% success rate**; single-edit menu push to
  all channels ([deliverect.com](https://www.deliverect.com/en-us/order-management)). **Chowly**:
  150+ channels, 86'd items propagate to all in real time. **ItsaCheckmate**: the POS is the menu
  source of truth — it **polls the POS every 5 minutes for 86'd items** and pushes instantly to all
  platforms; hourly full-menu sync ([support.itsacheckmate.com](https://support.itsacheckmate.com/hc/en-us/articles/8166742779419-Toast-POS-Direct-Sync-Manual)).
  **Otter**: per-channel **auto-accept vs manual-accept** configuration; expanded into its own
  POS/KDS/dispatch ([helpdesk.tryotter.com](https://helpdesk.tryotter.com/hc/en-us/articles/360052114914-Auto-vs-Manual-Order-Accept)).
- **Olo** (enterprise, ~90k locations): **Rails** injects 25+ marketplaces into the POS with
  real-time menu/price/availability sync; **Dispatch** auto-pairs first-party orders with a
  **27+ delivery-provider network** — 96% of locations have ≥2 providers, the customer sees the
  best-matched **fee+ETA quote**, operators set fee limits and max transit times per store
  ([olo.com/dispatch](https://www.olo.com/dispatch)). This is the only production system in the
  research doing genuine **multi-fleet competitive dispatch** — the POS vendors offer a binary
  Uber-Direct-or-DoorDash-Drive choice, no live quoting.
- **A further sync layer exists in several regional markets**: platforms like **GetOrder**
  ([getorder.biz](https://getorder.biz/)) and **Venus Hub** sync Glovo/Bolt/Wolt/UberEats ↔ local POS
  systems — an owner pays a *third* vendor just to see their own multi-channel business coherently.

### 2.4 Additional cross-cutting findings (most consequential for dowiz)

1. **Flat-fee subscription as an alternative to commission.** **Choice QR** ($7.1M Series A
   2026-03): flat $13–25/mo subscription instead of commission, 30k registered restaurants — but
   positioned as leverage *within* the marketplace relationship (it also plugs into Glovo/Bolt/Wolt),
   not an exit from it.
2. **Capital, not product, decides some regional challengers' survival.** A full-stack delivery
   challenger, **Rocket**, suspended operations — a funding/exit failure against marketplace-incumbent
   capital, not a documented product failure ([babel.ua](https://babel.ua/en/news/77258-rocket-delivery-service-suspends-its-activities-in-ukraine)).
3. **Dispatch SaaS** (Onfleet ML routing over 100M+ deliveries, auto-assign, POD photo/signature;
   Shipday commission-free white-label from $39/mo; Tookan with paid add-ons): all assume the owner
   already originates the demand. **Nothing bridges marketplace-sourced orders into an
   owner-controlled fleet** — the research's flagged build-gap: "*technically possible given the open
   APIs on both sides, but no packaged product does it — it would have to be built.*"
4. **Data ownership requires leaving the marketplace**: in markets where marketplace volume
   dominates, customer identity, order history, and re-engagement stay platform-owned; no reviewed
   portal exports that relationship back to the owner.
5. **Open-source stacks (TastyIgniter, Enatega, …) solve the wrong problem** — they are
   marketplace-clone kits, not a drop-in sovereign module for one existing restaurant.

### 2.5 The vendor-side table-stakes bar (synthesis of all passes)

| # | Table stake (every credible 2025-26 platform has it) | Best-in-class expression |
|---|---|---|
| T1 | Menu single-source-of-truth, real-time 86-sync to every channel + safeguard re-sync | Toast target/owner hierarchy; Checkmate 5-min poll; Chowly update-once |
| T2 | Direct order injection — no tablet re-keying | Deliverect 99.8% injection; Olo Rails |
| T3 | Store-state control: busy mode w/ bounded snooze, pause, hours, prep-time adjust, per-channel auto-accept, heartbeat auto-pause | Glovo "busy 30 min"; DoorDash busy-auto-pads-prep + 5-min tablet heartbeat; Otter per-channel auto-accept |
| T4 | Offline continuity with explicit bounds (queue-and-forward payments, local ordering) | Toast/Square store-then-forward; Square 72 h SLA; dowiz's solo-island test **exceeds all of these** |
| T5 | Item/menu-engineering profitability analytics; owner mobile pulse | xtraCHEF + menu matrix; Toast Now |
| T6 | Multi-location: menu inheritance/override on an org tree, location-scoped roles | Toast target/owner; Square Advanced Access |
| T7 | Dispatch optionality: white-label fleets, ideally multi-provider quoting | Olo Dispatch 27+ providers; Wolt Drive/Hybrid |
| T8 | Financial surface: payouts/statements/reconciliation, per-order fee transparency, error/dispute adjudication | DoorDash published error-charge rules + 14-day dispute; Deliveroo 7-day + 72-h evidence rubric; JET invoices |
| T9 | Marketing levers: promotions CRUD, visibility boosts, loyalty | JET StampCards/TopRank; P16 Promotions page already covers CRUD |

---

## 3. Gap analysis — best-in-class vendor UX vs dowiz's current hub plan

Verdicts: **GAP** (nothing in any blueprint), **PARTIAL** (UI planned or primitive exists, no
kernel/policy design), **COVERED** (an existing phase owns it), **AHEAD** (dowiz's design exceeds the
market).

| # | Capability (market ref) | dowiz today (evidence) | Verdict |
|---|---|---|---|
| G1 | **Menu as a versioned, scheduled, channel-aware, availability-carrying entity** (T1) | `PriceCatalog` flat price map (`catalog.rs`); MenuManager/MenuScheduleEditor/86-toggle exist only as P16 **UI** inventory (`DOWIZ-INTERFACES-PLAN.md:333-336`); no kernel menu module, no HubPolicy field, no 86-sync event | **GAP** — the #1 vendor surface has no kernel model |
| G2 | **External channel bridges** — ingest marketplace orders + push menu/86/store-state out (T1/T2; §2.4 finding #3: the unbuilt bridge) | Zero mentions in any design doc (grep-verified); `Order.channel` is an attribution string (`domain.rs:47`); P10 `bridges` = mesh-peer bridges only | **GAP** — M5 grants the right ("any API at its discretion") but no blueprint gives the mechanism |
| G3 | **Multi-fleet dispatch port** — quote/select among own couriers + rented fleets by fee/ETA; lend own fleet to peers (T7; Olo Dispatch, Wolt Drive) | HRW matcher over own couriers only (`matcher.rs`); no DispatchProvider concept anywhere | **GAP** — and the mesh makes the *inter-hub* half (a hub lending its fleet) protocol-native, something no incumbent can do |
| G4 | **Store-state + kitchen-load model** — busy w/ auto-expiry, pause, hours, prep-time estimation feeding quotes + auto-accept policy (T3) | `KitchenBusyToggle`/`pause/resume`/working-hours are P16 UI rows with no kernel state machine; `ema_next`/`kalman.rs` exist unused for prep-time | **PARTIAL** — primitives + UI planned, model missing |
| G5 | **Owner financial statement surface** — statements, per-order fee lines, reconciliation, dispute links (T8) | P13 double-entry ledger + payout saga + cash reconciliation (substrate ✓); P14 dispute/escrow (gated O3); **no owner-facing projection** designed | **PARTIAL** — derive-only work, substrate is strong |
| G6 | **Multi-location semantics** — org tree, menu inheritance/override, location-scoped roles (T6) | Historical schema had `organizations→locations` (As-Built:80); mesh canon never rules what a location IS; P15 sub-hubs are agent-recursion, not org structure | **GAP (ruling)** — needs an operator decision more than code |
| G7 | **Menu-engineering / profitability analytics + owner pulse** (T5) | `ChannelLedger` (attribution/funnel) is the only reader; P16 Analytics page plans revenue/top-products/ingredient-reorder UI | **PARTIAL** — deterministic-reader pattern exists to extend |
| G8 | **Cross-restaurant benchmarking** (Toast's 171k-location comparison) | M8 LOCK: local-only telemetry, never exfiltrated | **REJECTED-BY-DESIGN** — honest stance: dowiz refuses the surveillance trade; a future *opt-in, aggregate-only* mesh statistic would need its own DECART + operator ruling, not assumed here |
| A1 | **Offline resilience** (T4) | P13 AC-6 solo-island FULL order flow with zero services; local-first event-log + outbox (`spool.rs`); vs market: Toast/Square offline kills all inbound digital orders | **AHEAD** — no reviewed vendor-facing platform's offline mode matches a full order→delivery flow with zero external services; this is a structural, not incidental, differentiator |
| A2 | **Customer-data ownership** | Hub owns its event log by construction; no platform intermediary (F50 no-central-service invariant) | **AHEAD** — the thing ChowNow/Choice sell back to owners piecemeal is structural here |
| A3 | **Commission structure** | Protocol has no take-rate; JET's own ladder (self-delivery 13-14% vs platform 25-30%) prices the incumbent incentive dowiz eliminates | **AHEAD** (economics, not code) |
| A4 | **Courier dignity** | NO-COURIER-SCORING structural gate (M12, `scripts/ci-no-courier-scoring.sh`) vs marketplace driver-rating regimes | **AHEAD** — and a hard constraint on G3's design (fleet selection must rank *quotes*, never couriers) |

**The five most important gaps, ranked:** **G1** (menu-as-data kernel entity), **G2** (external
channel bridges), **G3** (multi-fleet dispatch port), **G4** (store-state/kitchen-load model),
**G6** (multi-location ruling). G5/G7 are smaller derive-only extensions of already-planned work.

---

## 4. Gap-closing design addendum (extends P10/P13/P15/P16 — no new phase)

Each item names its phase home, the existing seam it rides, exact target files, and a falsifiable
done-check. Ordering: D1/D4 are pure-kernel and parallel-safe **now** (Wave-0-grade, no operator
ruling needed); D2/D3 depend on P10's `HubPolicy` pipeline + P13's spine; D6 is an operator ruling.

### D1 — `kernel/src/menu.rs`: the menu-as-data entity (closes G1; extends P10 §2 + P16 §2)

Menu becomes an **event-sourced, content-addressed entity** with `PriceCatalog` demoted to a
*projection* of it (P2-CORRESPONDENCE: one concept, one primitive — today's `PriceCatalog` would
otherwise drift into a second menu authority).

```
MenuRevision {                 // mirrors HubPolicy's revision discipline (P10 §2.2) verbatim
  revision:  u64,              // monotonic
  menu_sha3: [u8;32],          // canonical-TLV hash — audit + cross-channel consistency proof
  items:     Vec<MenuItem>,    // id, name-key(i18n key, not text), base_price, modifiers,
                               //   prep_time_s, allergens, tags
  schedules: Vec<ScheduleWindow>, // dayparting: {item_set, dow_mask, open_s, close_s}
  channel_rules: Vec<ChannelPriceRule>, // {channel_id, markup_ppm | override_price} — per-channel
                               //   pricing the marketplaces force on owners (§2.1 commissions)
}
AvailabilitySet { eighty_sixed: BTreeSet<ItemId>, until: Option<u64> }  // 86/stop-list — SEPARATE
  // from MenuRevision: 86ing is a high-frequency operational event, not a menu edit (Checkmate's
  // 5-min-poll exists precisely because incumbents conflate these; dowiz emits it as an event)
```

- **Apply pipeline = P10's.** `apply_revision` semantics (parse+validate → floor-gate → atomic
  `Arc` swap → reconcile side-effects → typed telemetry event) are **reused, not reimplemented**;
  the floor-gate here = schema validity + i18n-key existence + price-integer discipline (money
  red-line: prices are integer minor units, `checked_add`, per `money.rs`).
- **86-sync fanout**: an `AvailabilityChanged` event appends to the hub's event log and is *pushed*
  to every registered channel bridge (D2) — push-on-event beats the market's poll-every-5-min; the
  market's "nightly safeguard sync" becomes a P5-RHYTHM return-swing: a scheduled full-menu
  reconcile per bridge, structurally fired (registered job, not remembered).
- **`PriceCatalog` projection**: built from the active `MenuRevision` + `ChannelPriceRule` for the
  order's channel; `place_order` keeps its exact fail-closed contract (`catalog.rs` unchanged
  callers). P16's MenuManager/MenuScheduleEditor/AI-import UI rows now have a kernel surface to bind.
- **Done-check (falsifiable):** (a) an order for an 86'd item is **refused** at intake with a typed
  error; (b) the same `menu_sha3` yields byte-identical projections on two hubs (M10 consistency);
  (c) a `ChannelPriceRule` for channel X changes X's quoted price and no other channel's; (d) a
  malformed menu revision is rejected and the last-good revision keeps serving (mirrors P10 AC-10).

### D2 — `ChannelBridge` port: external sales channels as thin Rust adapters (closes G2; extends P10 §2.1 + P13 §3)

The `LlmBackend` pattern (`HARNESS-LLM-BACKEND.md`) applied to sales channels — **one trait, one
transport, N thin adapters; the kernel never imports an adapter crate** (compile firewall).

```
trait ChannelBridge {           // ports/channel.rs; adapters in a separate crate
  fn id(&self) -> ChannelId;
  fn push_menu(&self, rev: &MenuRevision) -> Result<PushReceipt, ChannelErr>;
  fn push_availability(&self, av: &AvailabilitySet) -> Result<PushReceipt, ChannelErr>;
  fn push_store_state(&self, st: &StoreState) -> Result<PushReceipt, ChannelErr>;
  fn poll_or_webhook_orders(&self) -> Result<Vec<ExternalOrderIntent>, ChannelErr>;
  fn ack_order(&self, id: &ExternalOrderId, verdict: AcceptVerdict) -> Result<(), ChannelErr>;
}
```

- **HubPolicy carries the registry**: add `channel_bridges: Vec<ChannelBridgeSpec>` to `HubPolicy`
  (P10 §2.1) — `{channel_id, adapter_bin_sha3, endpoint, credentials_ref (EnvFile, S3 — never
  in-policy), token_bucket, auto_accept: AcceptPolicy}`. Rides the existing hot-reload +
  floor-gate + audit pipeline unchanged (Ananke: a bridge cannot exist outside the policy's
  fail-closed apply path). The adapter binary is **sha3-pinned** — the same verify-or-deny stance as
  P15 §5 model manifests, applied to bridge code.
- **Intake normalization**: an `ExternalOrderIntent` enters P13 §3's spine through the SAME thin
  intake edge as a native order — `place_order` against the D1 catalog projection (channel-priced),
  then `OrderTransition` frames, DoD gate, fold. **No second order path** (P2-CORRESPONDENCE);
  `Order.channel` (`domain.rs:47`) stops being decorative and becomes the bridge's `ChannelId`.
  Deliverect's 99.8% injection rate is the market bar; dowiz's version is *structural* — an intent
  that fails catalog/price validation is refused, typed, never re-keyed by a human.
- **Auto-accept policy** (Otter's per-channel pattern): `AcceptPolicy = Manual |
  Auto{max_load: KitchenLoad, within_hours: bool}` — evaluated against D4's store state; auto-accept
  degrades to Manual when busy/paused (fail-toward-human, P4 safe pole).
- **Sequencing honesty:** adapters for Glovo/Bolt (documented OAuth2 Partner APIs, §2.1) are natural
  first targets given their public API surface, but **any concrete adapter is post-G11 commercial
  work**; this addendum designs the port + policy seam only. The port must exist before the first
  adapter; the reverse order (ad-hoc Glovo glue) is how incumbent POS ecosystems grew their keyhole
  APIs.
- **Done-check:** (a) a mock-channel adapter (in-repo test double speaking the trait over stdio)
  delivers an order intent that folds to a `Delivered` terminal through the standard spine, visible
  in the owner dashboard with `channel = mock`; (b) an 86 event reaches the mock bridge in <1 s
  (push, not poll); (c) removing the bridge from `HubPolicy` hot-stops its intake without restart
  (P10 AC-8 pattern); (d) an adapter binary with a wrong sha3 is refused at spawn.

### D3 — `DispatchProvider` port: multi-fleet dispatch + mesh-native fleet lending (closes G3; extends P13 §2/§4)

```
trait DispatchProvider {
  fn quote(&self, job: &DeliveryJob) -> Result<DispatchQuote, DispatchErr>; // {fee_minor, eta_s, expiry}
  fn dispatch(&self, job: &DeliveryJob, q: &DispatchQuote) -> Result<DispatchHandle, DispatchErr>;
  fn track(&self, h: &DispatchHandle) -> Result<DispatchStatus, DispatchErr>;
  fn cancel(&self, h: &DispatchHandle) -> Result<(), DispatchErr>;
}
```

- **Adapter 0 (default, always present): `MeshCourierProvider`** — wraps the existing HRW matcher
  (`matcher.rs::assign`) over the hub's own couriers. Zero behavior change for a solo hub; the
  solo-island test (P13 AC-6) MUST still pass with only this provider (F50: no new mandatory
  external service).
- **Adapter 1 (mesh-native, the incumbent-impossible half): `PeerHubProvider`** — a neighboring
  hub's fleet, spoken to over the P9 wire as capability-scoped frames (a `DispatchOffer`/`Quote`
  frame pair; frame-kind request to P9's registry, same procedure as P10's `OperatorKill` request).
  This is Wolt-Drive-as-protocol: fleet lending between sovereign hubs with no platform in the
  middle, settled through P13 §5's ledger + P14 escrow. PoD stays k-of-n (P13 §4) regardless of
  whose courier signs.
- **Adapter 2+ (external SaaS/LaaS, DECART-gated per adapter):** Shipday/Onfleet-style or Glovo-LaaS
  — thin `ureq` adapters, post-G11, each behind its own DECART.
- **Market context sharpening the win:** among the majors, own-fleet/platform-fleet mixing is
  either/or per order (Uber BYOC is mutually exclusive with Uber couriers) or rule-changes take
  **24–72 business hours** (DoorDash Flexible Fulfillment); order-by-order live quoting across
  fleets exists only in Olo's middleware (§2.3). D3's chooser does it as hot-reloadable HubPolicy
  config — instant, per-order, no vendor ticket.
- **Selection = quote ranking, never courier ranking.** The chooser ranks `DispatchQuote`s by
  `(fee, eta)` under `HubPolicy` limits (max fee, max ETA — Olo's operator controls) —
  **M12/NO-COURIER-SCORING holds structurally**: a quote prices a *job offer from a fleet*, carries
  no per-courier identity or history, and the CI gate (`ci-no-courier-scoring.sh`) continues to fence
  the `Courier` type. Deterministic tie-break (lowest fee, then lowest ETA, then provider id) —
  P6-CAUSE-AND-EFFECT: same quotes in, same choice out.
- **Done-check:** (a) with two providers quoting, the chooser picks the cheaper-within-ETA-limit
  quote deterministically (property test over quote permutations); (b) with zero providers
  available, dispatch fails **typed** and the order stays `Ready` (never silently stuck
  `InDelivery`); (c) a two-hub test: hub A's order is delivered by hub B's courier via
  `PeerHubProvider`, PoD verifies k-of-n, and both hubs' ledgers conserve (Σ==0) across the
  cross-hub settlement; (d) grep proves no courier-scalar enters the quote struct.

### D4 — `StoreState` + prep-time estimator in the kernel (closes G4; extends P10 §2.1 + P16 Dashboard/Settings)

- **State machine** (`kernel/src/store_state.rs`): `Open | Busy{until_ms} | Paused{reason} |
  Closed{schedule}` — one mechanism, named poles (P4-POLARITY). `Busy` **requires** a bounded
  `until_ms` (the market's "busy 30 min" pattern; an unbounded busy is the failure mode where an
  owner forgets and silently loses a day of orders — the bound is structural, P5-RHYTHM's return
  swing: expiry re-opens automatically). Working hours (`Closed{schedule}`) come from
  `HubPolicy`-adjacent data the P16 Settings page edits. Intake consults StoreState *before*
  `place_order`: paused/closed ⇒ typed refusal; busy ⇒ quoted ETA inflated by the current
  prep-time estimate, and D2's auto-accept degrades to Manual.
- **Prep-time estimator**: reuse `geo.rs:39 ema_next` (the 1-D Kalman the repo already owns —
  P2-CORRESPONDENCE, no new estimator) over observed `Confirmed→Ready` durations per item-class,
  producing `prep_estimate_s` (a) added to courier ETA for the customer quote, (b) fed to D3's
  `DeliveryJob` so dispatch times courier arrival to food readiness (Olo's make-time logic), (c)
  compared against actual as a **typed telemetry line** (P8 sink) — the owner sees their own
  prep-accuracy the way Wolt shows it, computed locally (M8: never exfiltrated).
- **Done-check:** (a) `Busy{until}` expires and intake re-opens with no operator action (clock-driven
  test); (b) an order placed while `Paused` is refused typed; (c) after N synthetic
  `Confirmed→Ready` observations the quoted ETA moves toward the observed mean (convergence property
  of `ema_next`, already tested at `geo.rs:257` — extend, don't re-prove); (d) prep-accuracy
  telemetry row appears in the local sink per completed order.

### D5 — Owner statement projection (closes G5; extends P13 §5 + P16 Analytics)

Pure derivation, zero new primitives: a deterministic reader over the P13 double-entry ledger + P14
dispute events producing `OwnerStatement{period, per_channel: Vec<{channel, orders, gross, fees,
net}>, payouts, disputes_open}` — the JET-style invoice surface, computed from conservation-checked
entries instead of a platform's opaque PDF. Per-order fee lines derive from D2 channel metadata +
D3 dispatch fees. On dispute *evidence*, dowiz is structurally ahead of the market bar: Deliveroo's
adjudication rubric asks owners for a photo/CCTV frame timestamped within 20 min of order-ready
(§2.1) — a dowiz hub already holds a hybrid-signed, hash-chained PoD claim + event log (P13 §3-4),
which IS that evidence, cryptographic instead of photographic; D5 only has to surface it on the
statement row. Falsifier: statement totals reconcile **exactly** with `conserved()` ledger sums
for the period (any drift is a RED, because both derive from the same entries); a disputed order
appears with its P14 state, never silently netted.

### D6 — Multi-location semantics: a new operator decision (closes G6) — **proposed O20, not ruled here**

Two coherent mappings exist; they cannot both be default, and the choice shapes D1's inheritance
model (this is the Toast target/owner insight made sovereign). The market bar G6 must clear:
org-tree roles (DoorDash documents 4 tiers), menu inheritance/override on a location tree (Toast),
and consolidated rollup reporting — which even Uber Eats officially admits it lacks (§2.1):

- **O20-a: location = sub-hub.** Reuses P15 §4's spawn + capability-attenuation mechanism with org
  semantics: a chain's HQ hub mints an attenuated child capability per location; menu inheritance =
  parent `MenuRevision` + child override diff (content-addressed, narrow-only — a location can
  86/reprice locally, never widen the parent's red-line fields). Subtree-kill (O13) then covers
  "close the whole chain." Cost: sub-hub machinery becomes load-bearing for the product earlier than
  P15 planned.
- **O20-b: location = intra-hub entity.** One hub, `location_id` scoping inside kernel state
  (the historical schema's shape, As-Built:80). Cheaper now; forfeits per-location sovereignty
  (a franchisee can never take their location's hub and leave — which M5's spirit arguably demands).
- **Recommendation (flagged, overridable):** O20-a as target with O20-b as the explicit v1 bridge —
  the historical schema proves single-hub multi-location worked; the mesh makes the sub-hub upgrade
  a migration, not a rewrite. The menu-override diff format (D1) should be designed
  attenuation-shaped from day one so it serves both rulings unchanged.

---

## 5. DECART tables (Integration Decart Rule — new-integration choices made by this addendum)

### DECART-1 — How dowiz meets external sales channels (G2)

| Criterion | Thin Rust `ChannelBridge` adapters (chosen) | Integrate a middleware SaaS (Deliverect/GetOrder) | Ignore marketplaces entirely |
|---|---|---|---|
| Bare-metal / hermetic fit | Port + sha3-pinned adapter binary; kernel never imports adapter crate | A standing external SaaS dependency on the order path — violates F50 no-mandatory-central-service | Perfect fit, zero surface |
| Falsifiable correctness | Mock-channel done-check in-repo; intent validated by the same catalog/Law as native orders | Vendor's 99.8% claim unverifiable by us; their outage = our outage | Trivially correct |
| Measured performance | Push-on-event 86-sync (<1 s target) vs market's 5-min poll | Adds a hop + vendor rate limits | — |
| Supply-chain / license | Per-adapter `ureq` + OAuth2 against documented public APIs (Glovo Partners API et al.) | Closed SaaS, $85–500/mo/location (§2.3), ToS churn | — |
| Maintainability | N small adapters, one trait; each deletable | One vendor relationship, but their channel catalog may not match the target market, and the relationship is closed | — |
| Reversibility | Adapter = config row in HubPolicy; remove = hot-reload | Contract + data egress | — |
| Honest economics | The bridge captures the majority marketplace volume share (§2.4) an owner cannot yet walk away from | Same, rented | Sovereign but starves the owner during transition — Rocket's fate (§2.4) shows demand can't be wished over |

**DECISION: build the `ChannelBridge` port + mock adapter now (design), real adapters post-G11,
each behind its own per-channel DECART.** The port is what makes M5's "any API at its discretion"
mechanically true for sales channels; middleware SaaS is rejected as a *mandatory* path but any hub
MAY run one behind the same trait (SCOPE RULE — hub's own business).
**Probe (strongest case against):** marketplace Partner APIs are permissioned — Glovo/Bolt can
revoke or never grant API access to a self-hosted integrator, and ToS may forbid non-certified
middleware; the port could sit adapter-less in some markets. Mitigation is honest, not technical: the
port also serves direct-ordering channels (own site, Telegram bot — the existing Telegram surface is
the obvious first real adapter) so it is not dead weight even at zero marketplace adapters; and
precedents like GetOrder show that regional marketplace API access is grantable to third-party
integrators in practice, at least in some markets.

### DECART-2 — Multi-fleet dispatch mechanism (G3)

| Criterion | `DispatchProvider` port: HRW default + mesh peer-fleet + optional SaaS adapters (chosen) | Own-fleet only (status quo) | Adopt a dispatch SaaS as THE dispatcher |
|---|---|---|---|
| Bare-metal fit | Trait + frames on the existing P9 wire; solo hub unchanged | Simplest | External service on the delivery hot path — F50 violation |
| Falsifiable correctness | Deterministic quote-ranking property test; two-hub ledger-conservation test | Already proven (HRW tests) | Vendor black-box auto-assign |
| Performance | Quote round-trip bounded by expiry; HRW path byte-identical to today | — | Vendor SLA |
| Supply-chain | Zero new deps for adapters 0–1; SaaS adapters DECART-gated later | Zero | $39–299/mo + per-task fees (§2.4) |
| M12 red-line | Quotes rank fleets' offers, never couriers — structurally scoring-free | Same | Onfleet auto-assigns **by driver performance** — imports the exact scoring regime M12 bans |
| Reversibility | Providers are HubPolicy config rows | — | Contract |

**DECISION: the port, with `MeshCourierProvider` as the irremovable default and `PeerHubProvider`
as the first real second adapter.** This is the one gap where dowiz can leapfrog rather than catch
up: Olo needed 27 commercial contracts to create dispatch competition; a mesh of sovereign hubs gets
it from the protocol.
**Probe:** peer-fleet lending only has value at ≥2 adjacent hubs with couriers — before that density
exists (pre-G11 there is not even ONE real order), `PeerHubProvider` is capability theater. Answer:
D3 ships the port + adapter-0 refactor only when P13 lands anyway; adapter-1 is explicitly
density-gated (a named unlock: two real hubs in one city), not built speculatively.

*(D1/D4/D5 introduce no new dependency or external service — per the Integration Decart Rule and the
`HARNESS-LLM-BACKEND.md` §5 precedent table, no DECART is required for pure in-repo reuse; recorded
here so the absence is a decision, not an oversight.)*

---

## 6. Consolidated falsifiable done-checks (addendum-level)

1. **86-refusal:** order for an 86'd item refused typed at intake (D1-a).
2. **Cross-hub menu consistency:** same `menu_sha3` ⇒ byte-identical channel projections on two hubs (D1-b).
3. **Bridge round-trip:** mock-channel order intent → standard spine → `Delivered`; 86 push <1 s; hot-unregister stops intake without restart; wrong adapter sha3 refused (D2).
4. **Deterministic quoting:** cheaper-within-limits quote wins under permutation; zero providers ⇒ typed failure, order stays `Ready`; cross-hub delivery conserves both ledgers; no courier-scalar in quotes (D3).
5. **Busy self-heals:** `Busy{until}` re-opens unattended; `Paused` refuses typed; prep estimate converges on synthetic data; prep-accuracy telemetry emitted locally (D4).
6. **Statement ≡ ledger:** owner statement reconciles exactly with `conserved()` sums; disputes surface, never net silently (D5).
7. **Solo-island preserved:** P13 AC-6 still passes with ALL addendum features present and zero external services — every D-item degrades to sovereign-solo (F50 standing invariant).

---

## 7. 2-question doubt audit (AGENTS.md ritual, applied to THIS document)

**Q1 — least confident about (7 items, honestly):**
1. **Deliveroo's public documentation is thin.** The marketplace pass completed (DoorDash/Uber
   portal detail in §2.1 is now first-hand-sourced), but several Deliveroo dimensions (busy mode,
   tablet-offline behavior, role taxonomy, exact commission %) returned zero fetchable results across
   multiple search angles — treated throughout as *unconfirmed* absence, per that pass's own caveat.
2. **Marketplace API accessibility for a sovereign integrator** (DECART-1 probe) — Glovo Partners
   API docs are public, but approval policy for non-certified partners is unverified; could nullify
   the first commercial use of D2. Named as the port's biggest external risk.
3. **Line numbers in §1.2** for `catalog.rs`/`analytics.rs` are read-this-session but the files are
   under active development on this branch; treat as anchors, re-verify at implementation.
4. **`ChannelPriceRule` (per-channel markup) brushes the money red-line** — it changes what a
   customer is charged. D1 keeps it inside the integer-money floor-gate, but the implementation pass
   must treat menu channel-rules as money-adjacent (test-integrity red-lines apply), which this doc
   flags rather than proves.
5. **Prep-time estimator per item-CLASS granularity** is a guess; per-item may be too sparse,
   per-store too coarse. The done-check only pins convergence, deliberately leaving granularity to
   measured data — but that deferral is a real unknown, not a resolved choice.
6. **G8's rejection reading of M8** (no benchmarking) is my inference from the LOCK text; an opt-in
   aggregate could be canon-compatible. Flagged as needing an operator view only if ever wanted.
7. **The market itself is moving** (e.g. Olo launched a consumer app 2026-03; commission tiers and
   partner-portal features across every reviewed platform revise on their own cadence) — this
   research is a 2026-07 snapshot; the gap TABLE ages faster than the port DESIGNS, which is why the
   addendum binds to traits/seams, not to any vendor's current shape.

**Q2 — the biggest thing I might be missing:** the addendum assumes the vendor-ops surface should
live INSIDE the existing phase structure (extending P10/P13/P15/P16) rather than being its own
phase with its own wave slot. If the operator reads G1–G4 as "the actual product" rather than
"an addendum," the honest consequence is a resequencing question this doc does not decide: D1/D4
are buildable in Wave 0 *today* (pure kernel, no mesh dependency) and arguably belong BEFORE parts
of the crypto-first critical path if G11 ("first real order") is the true north — the same tension
SELF-CRITIQUE-2Q already flagged for the roadmap as a whole. Flagged, not resolved: it is the
operator's charter call, and this document deliberately gives D1/D4 no dependencies so either ruling
can execute without rework.

---

## 8. Anu / Ananke check (AGENTS.md doctrine)

- **Anu (is every decision derivable, not asserted?):** each gap row cites both sides (market source
  URL + repo file:line); each D-item names the existing mechanism it reuses (D1→P10 apply pipeline,
  D2→LlmBackend port pattern + P13 spine, D3→HRW matcher + P9 frames + P13 ledger, D4→`ema_next`,
  D5→P13 ledger) so the design is checkable against the live tree; the two genuinely new
  integration choices carry DECART tables with probes; the two things NOT derivable from evidence in
  front of me (marketplace API approval policy; multi-location semantics) are surfaced as a named
  risk and a proposed operator decision (O20) respectively, not silently chosen.
- **Ananke (does structure force the good outcome?):** every D-item ends in a falsifiable
  done-check (§6) rather than a description; the 86-safeguard re-sync is specified as a structurally
  fired job (P5), the Busy state *cannot* be unbounded (type carries `until_ms`), auto-accept
  *cannot* stay on while busy (evaluated against StoreState in the accept path), bridges *cannot*
  exist outside the fail-closed HubPolicy pipeline, quotes *cannot* carry courier scalars (type +
  existing CI fence), and the solo-island test (§6.7) is the standing structural proof that no
  addendum feature reintroduces a mandatory central service. Where structure can't force the outcome
  (external API approval), §7.2 says so instead of hoping.

---

## 9. Sources / provenance

- **Research pass A (US marketplaces):** completed after one interrupted run — DoorDash
  (merchants.doordash.com, help.doordash.com, developer.doordash.com, TechCrunch 2026-05), Uber Eats
  (merchants.ubereats.com, help.uber.com, developer.uber.com), Deliveroo (help.deliveroo.com,
  api-docs.deliveroo.com, merchants.deliveroo.com). URLs inline in §2.1; Deliveroo caveat in §7 Q1-1.
- **Research pass B (POS/restaurant-OS/middleware/Olo):** Toast (doc.toasttab.com, pos.toasttab.com,
  support.toasttab.com), Square (squareup.com), Otter (tryotter.com, helpdesk.tryotter.com),
  Deliverect (deliverect.com), Chowly (chowly.com), ItsaCheckmate (support.itsacheckmate.com), Olo
  (olo.com, developer.olo.com), Lightspeed/SpotOn (posusa.com, spoton.com), plus orderout.co /
  intouchinsight.com error-rate studies. URLs inline in §2.2–2.3.
- **Research pass C (EU/regional/dispatch/direct):** Glovo (api-docs.glovoapp.com), Bolt
  (foodpartner.bolt.eu, static.food.bolt.eu), Wolt (developer.wolt.com, press.wolt.com), JET
  (partner-hub.takeaway.com, menuviel.com, esmmagazine.com), GetOrder/Venus Hub, Choice QR (tech.eu),
  Rocket (babel.ua), Onfleet/Shipday/Tookan, ChowNow/Flipdish/Slerp. URLs inline in §2.1/2.4.
- **Repo evidence:** `ARCHITECTURE.md` §0; `BLUEPRINT-P10/P13/P15/P16`; `R1-B` gap analysis;
  `HARNESS-LLM-BACKEND.md`; `HERMETIC-ARCHITECTURE-PRINCIPLES.md`; `kernel/src/{catalog.rs, cart.rs,
  analytics.rs, order_machine.rs, domain.rs, geo.rs}`; `DOWIZ-INTERFACES-PLAN.md:325-357`;
  `DeliveryOS-As-Built-Summary-v1.md:80` — all read live this session (2026-07-17).

*This document plans; it changes no code and no canon. Proposed operator items: O20 (multi-location
semantics, §4-D6); density-unlock trigger for D3 adapter-1; per-channel DECARTs before any real
external adapter.*
