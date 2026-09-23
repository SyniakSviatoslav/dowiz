# The top of the pyramid: forecasting, franchise, retention, fintech, academy — what exists, what crosses a red line, the shape that does not, and whether Albania is a dozen venues or a Balkan SaaS

**Date:** 2026-09-23. **HEAD read:** `339f325f`. **Status:** research blueprint, read-only lane; no code
was changed. Every code claim below names a file:line or the command that produced it; every market or
legal number names a URL. Numbers that could not be read are in §8, not silently rounded.

**Companion:** a sibling lane is writing `BLUEPRINT-OPERATIONAL-BLIND-SPOTS-2026-09-23.md` (fiscal
offline, write-offs, KDS, multi-currency, procurement). It did not exist in the tree when this was
written (`ls docs/design/ | grep 2026-09-23` → two bebop files). Where a point below depends on
write-offs or procurement, it is referenced, not redone.

---

## 0. The answer in plain words, and the verdict table

**Neither "a dozen top venues in Tirana" nor "Balkan SaaS" is the first answer; the first answer is
a mass product in Albania that cannot yet print a receipt.** Albania's fiscalisation is
software-based, real-time and certification-gated (Law 87/2019; a POS application "does require
certification" — [fiscal-requirements.com/countries/17](https://www.fiscal-requirements.com/countries/17)),
and 53 producers already hold that certificate
([tatime.gov.al list](https://www.tatime.gov.al/c/424/494/lista-e-subjekteve-te-certifikuara)).
dowiz is not one of them, so today it is the room, the ordering, the stock and the CRM layer
*beside* a fiscal POS — the position the ebills blueprint already accepted (read-only poll,
`BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md` §0). That position sells to the ~34,000 accommodation and
food businesses (hypothesis: 14.3 % of 237,881 active legal units — §6.1) at a price between
fiscal-only software (DevPOS 368 lek/month, §6.3) and Square Plus ($49), on infrastructure that costs
$5.93/month for the *account* and about $0.65 per venue-month at the margin
(`/root/.claude/projects/-root/memory/dowiz-hub-unit-costs.md`, verified by reading the file).
It does not sell to the top dozen, who want one system with the receipt in it and are the accounts
the 53 certified integrators already hold. **Sequence:** Albania → Kosovo (same language; Electronic
Fiscal Software certification opened in 2026 — a greenfield rule-set; Wolt is alone there) → Montenegro
(software fiscalisation, cheap vendor testing, tiny market) → Serbia (the big market, 40,700 venues,
a heavier ESIR/PFR/security-element regime) → North Macedonia last (hardware fiscal devices with GPRS
— a Worker cannot drive a fiscal printer). **The answer flips to "a dozen top venues"** the day a Tirana
group pays for the chain design in §2 and that money funds the Albanian certification; it flips to
"partner only, never certify" if certification takes longer than two Kosovo-and-Montenegro entries.

The five points, in one table:

| # | Point | Exists today | Red-line collision? | Verdict | When |
|---|---|---|---|---|---|
| 1 | Forecast-driven purchasing | `analytics/fold.rs` folds by day / by hour / top products over 7 or 30 days (`fold.rs:36-49`, `handler.rs:1`); stock ledger with `Received/Wasted/Stocktake` (`stock.rs:58-68`); `lowAt` reorder threshold (`recipe.rs:6`); **0** hits for `weather\|forecast` in the product path (command §1.1) | No — C1 forbids AI in the *decision* path, not a deterministic back-office fold; a float/RNG model *inside* the hub would collide with C2 | Build the deterministic baseline (seasonal naive, m = 7, integer) as a fold; weather and events as owner-entered day tags first; ML never in the decision path | After stock is switched on (0 recipes live) and after the sibling's write-off design — Phase E1 |
| 2 | Franchise / HQ governance | One Durable Object per venue (`hubstore.rs:192,210`, `one-venue` gate at 0); platform registry object `__platform` (`platform_store.rs:1-25`); catalogue import with dry run (`import.rs:1-8`, `admin/menu.js:87`); **0** hits for `franchis\|royalt` | Yes if HQ *writes* into a franchisee's object (C4/C13, D1) or a venue *rank* is stored (`no-scoring.sh:58` lists `venue_tier/rank`) | A read-only federation over per-venue signed capabilities; brand menu as a signed bundle applied by dry-run import; royalty as a conservation-held fold over `Paid` | After Phase B3 (price per channel is the override mechanism) and after a second venue of one brand exists — Phase E2 |
| 3 | Automated retention ("Ivan missed three weeks") | Consent witness `Consented` with private fields (`consent.rs:171,252`), gate `tools/gates/consent.sh`; customer record with no derived fields (`record.rs:16-27` refuses `tier`); `last_at` as a fold (`roll.rs:10-20`); WhatsApp Cloud API (`channels.rs`); outbox; promo redemption fold; **campaigns not built** (`grep -rn not_seen_since workers/api/src` → tests of `Sort` only) | Yes as stated: a stored "comes Fridays with family" model is profiling under Law 124/2024; a send without a `Consented` witness is unrepresentable by type | CRM §3.6 as designed: `not_seen_since(days)` evaluated at send time from the fold, to consented keys only, promo-code attribution; drop the household/weekday inference | Phase C6, in the roadmap already |
| 4 | Embedded fintech | Wallet = double-entry journal, "balance is replayed, never stored" (`wallet.rs:9`); till with `PayIn/PayOut` (`till.rs:1-13`); `Paid` carries `by` (`pay.rs:20-34`); Stripe is card-in only (`grep -niE "connect\|transfer" stripe.rs` → 0); **0** hits for `supplier\|loan\|lending` | Tips "by digital ratings" — **yes, the exact pattern the gate refuses** (`staff_rating`); micro-loans — regulated credit (Bank of Albania Reg. 1/2013) and a capture of the economic control point (MANIFESTO §2) | Tips by a declared integer rule over hours and role weights with largest-remainder rounding (Σ shares == pool); supplier payables as a fold that *exports* a payment order to the venue's bank; lending → a venue-signed revenue statement a lender of the venue's choice may read, dowiz never holds funds | Tips: with A7/A8. Payables: after B (VAT-correct numbers). Statement: Phase E |
| 5 | Staff academy / simulator | `lib/guide.js` first-run tour + per-control hints, one declarative table, imported by the courier app (`courier/app.js:11`); i18n hints in three languages (`admin/i18n.js`); **0** hits for `tutorial\|academy\|lesson\|simulat` | Only if completion becomes a per-employee *score*; completion as a capability grant fits `CLAUDE.md:102` | Tour tables for the waiter surface now; a simulator = the pure `command/*` deciders run client-side against a throwaway log (they are already pure and tested); completion → the owner grants a `Cap` | Tour: with A4–A8. Simulator: after the command layer is callable from wasm (`BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md`) |

---

## 1. Predictive forecasting → forecast-driven purchasing

### 1.1 What exists

- **The only analytics fold** is `workers/api/src/services/analytics/fold.rs`: a `Report` with `by_day:
  Vec<Day>`, `by_hour: [i64; 24]`, `top_products` (`fold.rs:36-49`), over a window of 7 or 30 days
  (`handler.rs:1`, `fold::window`). It buckets by the venue's local day using `dowiz_hub::tz`
  (`storefront.rs:261` shows `local_weekday_minute`; `tz.rs:289`). It is orchestration over the
  order fold; "every number in it is decided in `fold`, where it has a test" (`handler.rs:3-5`).
- **The stock ledger** is a real event fold: `Received`, `Reserved`, `Consumed`, `Released`, `Wasted {
  reason }`, `Stocktake { observed }` (`crates/dowiz-hub/src/stock.rs:58-68`), with an `OutOfStock`
  refusal. It is **inert in production: 165 dishes, 0 recipes, 0 supplies on both venues**
  (memory `dowiz-stock-ledger-works-but-is-off`, proved live 2026-09-21).
- **A reorder threshold** exists as a supply field, `lowAt` (`workers/api/src/recipe.rs:6`). Nothing
  reads it to suggest an order.
- **Weather, events, forecast: nothing.** `grep -rniE "weather|forecast" workers/api/src
  crates/dowiz-hub/src --include=*.rs | wc -l` → `0`. The 63 hits for `predict` across the tree are
  the gauge comments ("a reading near full predicts a doubling", `gauges.rs:46`) and research
  modules in `crates/dowiz-core` that are not on the product path.
- **A nightly alarm exists** to run a fold in: `crons = ["17 3 * * *", "* * * * *"]`
  (`workers/api/wrangler.toml:138`).

### 1.2 The red line, precisely

MANIFESTO C1: "No AI in protocol/runtime logic — deterministic Rust/WASM only; AI only for R&D /
back-office" (`MANIFESTO.md:14`). C2: no clock / RNG / float reaches the kernel (`MANIFESTO.md:15`),
and `tools/gates/float-money.sh` counts floats on money. A purchasing *suggestion* is back-office: it
decides nothing in the order FSM and moves no money. So a forecast does not collide with C1 **provided
it is advisory and a person places the order.** It collides with C2 the moment a float or a random
initialisation lives in the hub or kernel, because then two replicas fold the same log to two
different numbers, which is exactly the property the whole system exists to prevent. That is the
engineering reason — not taste — for the integer-only method below.

### 1.3 What a single venue actually accumulates, measured

- The live venue on ebills issues **452 bills in 30 days** (`BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md`
  §1.6), ≈ 15 a day. dowiz's own log on `dubin-durres` holds 217 orders in total
  (`ROADMAP-2026-09-22.md` §6). Per **dish** per **day** the series is therefore mostly zeros and
  ones: 165 dishes over 15 bills. Per **supply** per **day** it is better (salmon is in many dishes)
  — but only once recipes exist, and today none do.
- **The daily series is one week old in dowiz's own log for a week, a month old in a month.** The
  ebills poll could backfill a longer history if the API answers ranges older than 30 days;
  whether it does is **not verified** (§8).
- **The annual signal is enormous and unobserved:** Albania's accommodation nights were 2.26 million
  in August against 226,431 in February — a tenfold peak-to-trough
  ([Albanian Daily News](https://albaniandailynews.com/news/foreign-tourists-arrivals-speed-up-in-august),
  [ALTAX](https://altax.al/en/transformation-of-tourist-demand-in-albania-2021-2025/)). No venue on
  dowiz has one summer in its log. Any "yearly seasonality" claim before summer 2027 is a hypothesis.

### 1.4 The baseline to beat, and the minimum history

The method the operator's own sentence implies — "same weekday last 4 weeks" — **is** the seasonal
average with m = 7. Its cheaper cousin is the **seasonal naive**: "we set each forecast to be equal
to the last observed value from the same season"
([Hyndman & Athanasopoulos, FPP3 §5.2](https://otexts.com/fpp3/simple-methods.html)). Both are
integers over integers.

| Method | History before it can answer | History before it should be trusted (hypothesis) | Integer-safe |
|---|---|---|---|
| Seasonal naive, m = 7 | 1 week | 4 weeks | yes |
| Seasonal mean of last k weekdays (k = 4) | 4 weeks | 8 weeks | yes (Σ/k with a stated rounding) |
| Simple exponential smoothing on the weekday-adjusted series | 2 weeks | 12 weeks | yes in fixed point: `level += (α_num × (y − level)) / α_den`, α as a ratio like 1/8, round-half-even stated as an equation (the B1 tax-rounding discipline) |
| Holt-Winters, weekly | 2 seasons (14 days) | 8–12 weeks | yes in fixed point, but three parameters to fit and nothing to fit them on |
| Yearly seasonality (tourism) | 1 year | 2 years | — no venue has it |
| Weather as a regressor | 1 season of paired days | 2 seasons | rain/heat as a *day tag*, not a coefficient, until there is data |

**The rule:** nothing is promoted past seasonal naive until it beats seasonal naive on a rolling
holdout the venue's own log can compute — mean absolute error over the last 28 days, per supply, in
integers. That comparison is itself a fold and should ship with the first forecast, so the product
cannot claim an improvement it did not measure (the `bebop-instruments-that-measure-nothing` lesson).

### 1.5 Weather and events, honestly

- **Weather:** Open-Meteo serves forecast and the historical archive with no API key for
  non-commercial use, under 10,000 calls/day, and requires a paid plan for commercial use
  ([open-meteo.com/en/pricing](https://open-meteo.com/en/pricing), [terms](https://open-meteo.com/en/terms));
  the commercial price was not read (§8). One call per venue per day is nothing. But a coefficient
  ("+30 % beer when > 30 °C") needs a summer of paired data; before that, weather is a **day tag**
  the fold groups by (`hot`, `rain`, `normal`), so the owner sees "on the 6 hot Thursdays you had, beer
  was X" — a table, not a prediction.
- **Events (the national team plays):** no source is worth wiring. The honest first step is a
  one-tap **owner-entered day tag** ("tomorrow is an event day"); after N tagged days the fold shows
  the measured uplift ratio, and only then does a forecast use it. This mirrors how tables and hours
  already work: the venue declares, the fold derives (`tables.rs:1-12`).
- **The sentence "cut the salmon order or you'll write it off"** needs three things in order: a
  recipe (salmon per dish), a `Wasted` history (the ledger has the event kind, `stock.rs:66`, and
  the sibling blueprint owns the write-off workflow), and a forecast. The first two are worth more
  than the third and exist; the third is worthless without them.

### 1.6 The smallest design that fits this architecture

1. **A `demand` fold, not an image.** `demand(orders, catalogue, bom, zone, day_tags) → per supply per
   local day: consumed qty`, derived through the BOM the way `stock.rs` already derives reservations.
   No stored counter (the `promo.rs` law).
2. **A `suggest` fold over it:** `forecast(supply, horizon) = seasonal_naive` with the rolling MAE of
   the two baselines beside every number; `order_qty = max(0, Σ forecast over (lead + cover days) −
   on_hand − on_order)`, integers, `lowAt` as the floor. Shown on the stock screen; nothing is
   ordered by software.
3. **Day tags as a small `LogImage`** (`kinds: tag`), owner-written, weather-written by the nightly
   cron once a commercial Open-Meteo plan is paid for (or never — the owner can type "hot").
4. **Where it runs:** in the Durable Object's nightly alarm, memoised per generation like
   `/fold/orders` (memory `dowiz-hub-seven-phases`, phase 2), so a phone reads a number rather than
   folding 30 days of log.
5. **Where ML would go, if ever:** outside the hub, as an *advisory* call behind the venue's own AI
   endpoint (`rail.rs` already breakers "the venue's AI endpoint"), never producing a number the hub
   stores. C1 keeps it there.

### 1.7 Order

After stock is switched on at one venue (recipes are the empty middle), after the sibling's write-off
flow, and after B1 fixes rounding as an equation — because a forecast's rounding rule should be the
same sentence. **Phase E1**, not before D.

---

## 2. Franchise / multi-tenant governance

### 2.1 What exists

- **Tenancy is structural.** A venue is a Durable Object addressed by its id; `Place::of_authorised`
  authorises first and then names the object (`workers/api/src/hubstore.rs:192`), `Place::must_be`
  refuses a request that names two (`:210`), and `tools/gates/one-venue.sh` holds that at 0
  (`ROADMAP-2026-09-22.md` §0). This was learned the hard way: 37 owner routes once authorised one
  venue and acted on another (memory `dowiz-tenant-isolation-hub-image`).
- **The platform has its own object**, `__platform`, holding "who the people are, which venues exist,
  which host answers for which venue" (`workers/api/src/platform_store.rs:1-16`) — an id "no venue can
  have". One owner may already own two venues (the operator does).
- **Roster is per venue, by design:** "no profiles, no preferences, no history, and no cross-hub
  identity" (`crates/dowiz-hub/src/roster.rs:3-6`); the endgame named there is the P67 anchor roster
  with signed capability delegations (`DECISIONS.md` D10).
- **Capabilities are a closed set with no ordering** (`crates/dowiz-hub/src/caps.rs:9-12`; variants
  `Advance, TakeOrders, TakePayment, Void, OpenTill`, `:45-53`).
- **Menu import with a dry run** exists: `crates/dowiz-hub/src/import.rs:1-8` (pure parser; "ambiguity
  is refused, not guessed"), driven by `admin/menu.js:87` ("a dry run first, with what would change,
  then apply").
- **Brand tokens** are five owner-editable values (`crates/dowiz-hub/src/brand.rs:1-5`).
- **A signed-id-set merge** already exists for replication: the G-Set CvRDT in
  `crates/dowiz-core/src/mesh_replication.rs:209`.
- **Franchise, royalty, HQ: nothing.** `grep -rniE "franchis|royalt" workers/api/src
  crates/dowiz-hub/src crates/dowiz-core/src kernel/src --include=*.rs | wc -l` → `0`.

### 2.2 Where it collides, and where it does not

- **"HQ sees the whole chain live"** does not collide *if it is a read* granted by each franchisee.
  It collides the moment HQ **writes** into a franchisee's object (a pushed menu, a forced price) —
  that is the central server D1 dropped (`DECISIONS.md:14-24`, MANIFESTO C4/C13).
- **"Each franchisee sees only its own finances"** is not a feature to build; it is what `one-venue`
  already guarantees.
- **A league table of venues** — "Sarandë is our best unit" — is `venue_rank`, which the gate's
  regex names (`tools/gates/no-scoring.sh:58`). The CRM blueprint's test applies: a *query* HQ runs
  and stores nowhere is fine (`Sort::Spent`, `roll.rs:24-35`); a persisted rank that anything branches
  on is refused. Venues are participants in the gate's list, and the same wording applies.
- **Royalty** is money: integer, a stated rounding rule, a conservation law. No collision; it is a
  fold nobody has written.

### 2.3 The design: a read-only federation

1. **The brand is an object, not a tenant of tenants.** A brand principal owns a `__brand:<id>`-style
   object (the `__platform` precedent: two underscores cannot be a venue) holding a **signed catalogue
   bundle** — items, base prices, allergens, photos' content ids — and the roster of venues that
   carry it. Signing is the hybrid policy already wired for capability chains
   (`DECISIONS.md` D4 audit line: `HybridSignPolicy` in cap-chain verification).
2. **A venue adopts a bundle by import, never by push.** The bundle arrives through the same pure
   `import.rs` path and the same dry run the owner already uses for a CSV. The venue's catalogue keeps
   `brand_ref = (bundle digest, item id)` on each adopted line and **its own price**, which is B3's
   "price per channel" mechanism with one more axis (`BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md`,
   in flight). "Sarandë sells seafood cheaper" is a local price on a brand line; "what differs from
   brand" is a fold, `diff(venue catalogue, bundle)`, never a stored exceptions list.
3. **HQ reads through per-venue signed capabilities.** One new `Cap` variant, `ReadReports`
   (the closed set grows by one row, as `caps.rs:20-24` says it should), minted by the *franchisee*
   owner to the brand principal, with a period, revocable. The HQ console fans out N reads (one per
   object — `Place::of_authorised` per grant, so the gate still holds) and folds client-side. There
   is no shared store, no cross-tenant query, and the franchisee can revoke the read on the day the
   contract ends. This is "trust is a signed capability" applied to organisations.
4. **Royalty is a conservation-held fold over `Paid`.** `royalty(period) = Σ_paid (took_money ×
   rate_bp) / 10_000` per venue with the rounding rule written as an equation (B1's discipline;
   `venue_took` at `status.rs:68` already defines the base and excludes the tip). The ninth-law shape
   from the till (`till.rs:8-12`) applies: `Σ per-order royalty − Σ period royalty` is bounded by
   `N × 1 minor unit`, checked by the conservation audit, and the franchisee's object emits a signed
   **royalty statement** that HQ verifies against the chain (`Hub::chain_check`, `lib.rs:701`).
   Payment of the royalty is a bank transfer the statement supports, not a movement dowiz makes
   (§4.3, D14 D-money).
5. **What is deliberately not built:** a brand-side write route, a "sync now" button, a stored
   ranking of units, a single HQ login that "is" every venue (the roster's "no cross-hub identity"
   stands; HQ is a *different* principal holding *read* caps).

### 2.4 Order

After B3 lands (the override mechanism) and after a real second venue of one brand exists — KAN's
purchase of KFC Albania (Oct 2025, [Balkanweb](https://www.balkanweb.com/en/kfc-ne-shqiperi-behet-pjese-e-kan-grupit-me-te-madh-ballkanik-te-kfc/))
shows the buyer profile, but no such venue is on dowiz. **Phase E2.** Building it earlier produces
an object with no reader, which `unreached.py` would rightly count.

---

## 3. Automated marketing / retention

### 3.1 What exists

- **Consent as evidence, by type.** `Consented` has private fields and no constructor
  (`crates/dowiz-hub/src/consent.rs:171`); only `consent::state(entries, key, purpose, channel)`
  produces one (`:252`); a send site that takes a bare string is counted by
  `tools/gates/consent.sh`. The header cites Law 124/2024 and Meta's per-business opt-in rule
  (`consent.rs:3-9`).
- **The customer record holds what a paper card would** — note, closed-list tags, EU-14 allergens,
  `usual_table`, language, birthday as MM-DD (`services/customers/record.rs:16-27`); it **refuses**
  `spent` or `tier` at the door (`record.rs:16-18`). Orders, spend and **`last_at` are folds**
  (`roll.rs:10-20`).
- **Erasure** exists (`services/customers/forget.rs`, `crates/dowiz-hub/src/forget.rs`).
- **Channels:** WhatsApp Cloud API and Instagram through one Meta webhook (`workers/api/src/channels.rs`;
  memory `dowiz-integrations-2026-09-19`), the outbox with retry/dedupe (`outbox.rs`), Telegram
  (`notify.rs`). **Promo redemption** is a fold (`crates/dowiz-hub/src/promo.rs`).
- **Campaigns and segments: not built.** `grep -rn "not_seen_since\|everyone_consented"
  workers/api/src crates/dowiz-hub/src` → hits only in `customers/tests.rs` for `Sort`.

### 3.2 The collision, and the version that meets the need

The operator's sentence has four claims: *Ivan usually comes Fridays* (a behavioural pattern),
*with family* (an inferred household), *has missed three weeks* (a recency fact), *send a personalised
offer at his favourite table* (a send, a promo, a record field).

- **Recency is a fold, not a profile.** `not_seen_since(21)` over `last_at` at send time is exactly
  the predicate the CRM blueprint lists (`BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22.md` §3.6); stored
  nowhere, it goes stale with nothing and an erasure has nothing to find.
- **"Usually Fridays" and "with family" are profiling.** Law 124/2024 (in force January 2025;
  [Clym summary](https://www.clym.io/regulations/law-no-1242024-on-personal-data-protection-albania),
  [CMS](https://cms.law/en/int/expert-guides/cms-expert-guide-to-data-protection-and-cyber-security-laws/albania))
  gives the data subject the right to object to profiling and restricts automated decisions; the
  CRM blueprint's own line is "a stored behavioural profile" is what erasure would have to find. A
  household inference from party sizes is a stored belief about a person nobody asked. **Refused as
  stored data.** If a venue wants "your usual Friday table", the honest field is the one that exists
  — `usual_table`, typed by the owner, on the card.
- **"Automatically sends"** is allowed exactly as CRM §3.6 designs it: a campaign the owner defined,
  a closed predicate list, a preview with the COUNT and the cost, each `Entry` minted only through a
  function that takes `&Consented`, drained by the minute cron, attributed by promo code — no open or
  click tracking. Automation of a consented, owner-authored campaign is not the collision; an
  unattended inference engine is.
- **Cost is real and metered by Meta.** Marketing templates are the most expensive WhatsApp category;
  Germany is €0.1131 per marketing message and countries without their own line fall in a regional
  bucket ([Blueticks](https://blueticks.co/blog/whatsapp-business-pricing-europe-2026)); the Albania
  rate was not read (§8). The preview must show it.

### 3.3 Design and order

Nothing beyond CRM §3.6: a `campaign` LogImage (`def`, `sent`), predicates `everyone_consented`,
`not_seen_since(days)`, `tag(x)`, `birthday_this_week`, the `Consented` witness on every entry,
promo-code attribution. The only addition this document makes is a **refusal list in the predicate
enum itself**: no `usual_weekday`, no `party_size_over`, no `spent_over` — the last is a tier by
another name. **Phase C6**, after C1–C4, as the roadmap orders; the tree already has C1–C3's parts.

---

## 4. Embedded fintech

### 4.1 What exists

- **Money is a double-entry journal** whose balance "is replayed, never stored" (`workers/api/src/
  wallet.rs:9-16`), validated by the kernel's `ledger_account` before anything is written; `TxKind`
  includes `Payout` (`wallet.rs:69,80`).
- **The till** is a five-event LogImage — `Opened, Counted, Closed, PayIn, PayOut` — with a blind count
  and `over_short` never adjusted away (`workers/api/src/command/till.rs:1-13`).
- **`Paid` names its signer**: `by` is "mandatory, and never waived" (`command/pay.rs:27-28`), and the
  tip is "THE COURIER'S, passing through" (`services/orders/status.rs:61-68`).
- **Stripe is a card-in rail only:** `grep -niE "connect|transfer|payout|destination"
  workers/api/src/stripe.rs` → one comment about a flaky connection, nothing else.
- **Suppliers, loans, lending: nothing.** `grep -rniE "supplier|\bloan\b|lending" workers/api/src
  crates/dowiz-hub/src --include=*.rs | wc -l` → `0`. The stock ledger's `Received` (`stock.rs:58`)
  is the only supplier-shaped fact.
- **Red-line policy:** ledger/money is deny-by-default for any agent (`crates/dowiz-core/src/ports/
  agent/scope.rs:265-268`); D14 D-money scopes settlement to "the single-hub pilot surface only (no
  cross-hub auto-settlement in v1)" (`DECISIONS.md:385-387`).

### 4.2 Supplier payment orders on an accepted e-delivery-note

**The need:** stop retyping supplier invoices, pay on time. **What Albania gives you:** every B2B
invoice is fiscalised through the tax administration's system since July 2021
([Thomson Reuters / Pagero](https://europe.thomsonreuters.com/compliance/regulatory-updates/albania)),
so the supplier's e-invoice exists as structured data the venue already accepts in the tax portal.
**Design:** a `payable` LogImage (`kinds: invoice, paid`) written from an imported e-invoice — lines
become `Received` movements (stock) and one payable (money); `outstanding = Σ invoice − Σ paid` is a
fold; a **payment order is an exported file** (the bank's import format) the owner uploads to their
bank, or an instruction the owner approves in their banking app. dowiz never initiates a transfer:
there is no open-banking mandate in Albania this lane could verify (§8), and D14's fence stands.
**Order:** after Phase B — a payable without VAT handled is a wrong number, and rule 2 of the roadmap
("a wrong number is worse than a missing one") applies. The procurement side (who orders what, when)
belongs to the sibling blueprint.

### 4.3 Tip split "based on digital ratings" — refused; the version that works

A rating of a waiter or a cook is `staff_rating` / `waiter_score` — literally the pattern
`tools/gates/no-scoring.sh:58` refuses, and `caps.rs:9-12` states the principle for staff: "who is
the better waiter is not a question this type can answer". `DECISIONS.md:350` deleted `reputation.rs`
for couriers; the wording does not change for kitchen staff. **Refused, permanently.**

The need — a fair, automatic split — is met by **a declared rule over facts the log already has:**

- **The pool** is `Σ tip` over the till period, folded from `Paid` (POS blueprint §7 item 8 builds
  `tips_by_person`; the tip term is missing from conservation law 3 and is item G0 there).
- **The basis** is hours and role, both facts: the till's `Opened/Closed { by, at }` and the shift
  record (`courier.rs:214-215` has `started_at_ms/ended_at_ms`; the waiter analogue is A7). The owner
  declares integer weights per role (kitchen 2, floor 3 — a venue's own policy).
- **The equation:** `share_i = pool × (w_i × minutes_i) / Σ_j (w_j × minutes_j)`, integer division,
  **largest-remainder** assignment of the leftover minor units so that `Σ share_i == pool` exactly —
  a tenth conservation law, testable on a synthetic log. Refuse a rule whose weights are not
  integers or whose Σ is zero.
- The POS blueprint's re-entry condition for tip distribution is "a venue asks, with its rule
  written as an equation over integers" (§5 E); the equation above is what that request would
  produce. Albanian tax treatment of pooled tips was not researched (§8).

### 4.4 Micro-loans against future order flow — refused as dowiz-the-lender; one honest shape

- **Regulated:** lending and microcredit in Albania require a Bank of Albania licence under
  Regulation no. 1/2013 on non-bank financial institutions
  ([bankofalbania.org](https://www.bankofalbania.org/Supervision/Regulatory_Framework/Licensing_Regulations/Regulation_no_1_On_the_granting_of_license_to_non_bank_financial_institutions.html));
  the capital minimums were not readable from the page (§8).
- **Structural:** a loan "against future order flow" means the platform holds, pledges or routes the
  venue's revenue — the matcher/settlement "economic control point" MANIFESTO §2 says must not sit
  with one party; a lender-platform is "DoorDash with extra steps" in a suit.
- **The shape that serves the need without either problem:** the venue's object emits a
  **signed revenue statement** — `Σ took_money` by week for N weeks, chain-verified (`lib.rs:701`),
  signed by the venue's key — and the owner grants a **time-boxed read capability** to a lender of
  their choosing (the same `ReadReports` cap as §2.3). The lender underwrites on a statement it can
  verify offline from the log's chain; dowiz never sees the loan. That is local-first credit: the
  venue owns the record that makes it creditworthy.
- **Order:** Phase E, after B (a lender wants VAT-correct revenue) and after the cap exists.

---

## 5. Staff academy / LMS

### 5.1 What exists

- **`workers/api/public/lib/guide.js`** — "the first-run tour and the per-control hints, ONE module for
  every surface": a declarative `{ at, title, body }` table per surface, the tour as an ordered list
  of keys into it, persistence in `localStorage dw_guide_<key>` with `open / paused` states
  (`guide.js:1-27`). Imported by the courier app (`courier/app.js:11`) and precached by its service
  worker. The admin console has per-screen hint strings in three languages (`admin/i18n.js:42,149,240`)
  but does not import the guide (`grep -rln "lib/guide.js" workers/api/public` → courier only).
- The customer kit has an `onboarding.js` with placeholder copy (`kit/screens/onboarding.js:10-11`,
  "Lorem ipsum") — not staff training.
- **No simulator, no lessons, no sandbox venue.** `grep -rniE "tutorial|academy|lesson|simulat|walkthrough"
  workers/api/public workers/api/src` → `0`.

### 5.2 Collision

Only one, and it is avoidable: an LMS that stores a per-employee **score** or **completion rank** is
a participant rating. A module *completed* is a binary fact and belongs where authority already
lives — the owner grants the capability (`Cap::TakePayment`) when they are satisfied. No quiz scores
are stored; the tour's state stays in the device's `localStorage`, as it does now.

### 5.3 The design that costs least

1. **Tour tables for the waiter surface** (open a table, add a round, void with a reason, split,
   pay) written in the same `{ at, title, body }` shape, in the three languages the console already
   has. This is copy, not code, and lands with A4–A8 because a waiter who exists needs it that day.
2. **The simulator is the command layer, run on the device.** `command/place.rs`, `amend.rs`,
   `pay.rs`, `till.rs` are pure deciders with tests (`command/pay.rs:77`, `till.rs:183`); every
   refusal is a typed `Refused` with a sentence. A practice mode loads the waiter screen against an
   **in-memory log** and calls the same deciders — no venue, no network, no fake data on a live
   object. The lesson *is* the refusal: "a cash `Paid` outside any open till is a breach" (`till.rs:13`)
   teaches opening the till better than a slide would. This requires the command layer to be callable
   from the browser, which is what `BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md` and the kernel's existing
   wasm build (`scripts/build-kernel-wasm.sh`, `CLAUDE.md` "Common commands") are about.
3. **"In 10 minutes without a manager"** is met by 1 + 2 plus the capability model: the owner
   issues the waiter token with `TakeOrders` first and adds `TakePayment` and `Void` when ready —
   the closed set in `caps.rs` is already the curriculum's chapter list.
4. **Not built:** completion tracking on the server, badges, leaderboards, "top waiter this week".

### 5.4 Order

Item 1 with Phase A's tail; item 2 after the wasm command surface exists (Phase D territory). Both
are cheap; neither needs a new image family.

---

## 6. The market, with sources

### 6.1 Albania — size and count

- **Active enterprises (structural business statistics, 2024): 119,870**, employing 551,189, turnover
  3,532,889 million ALL; 99.9 % are SMEs; micro-enterprises hold **47.3 % of turnover in
  accommodation and food services** (INSTAT, *Statistics on SMEs 2024*,
  [nvm-2024-angl.pdf](https://www.instat.gov.al/media/kfhl5iic/nvm-2024-angl.pdf), Table 1 — decoded
  locally from the PDF's text streams; the sector's absolute count is in a chart, not a table).
- **Active legal units (business register, end 2024): 237,881**
  ([China-CEE Institute briefing on INSTAT](https://china-cee.eu/2025/09/16/albania-monthly-briefing-structure-and-dynamics-of-albanian-businesses-a-2025-profile/),
  page 403 to a direct fetch; cited from the search index). Accommodation and food service
  = **14.3 % of active enterprises in 2023**
  ([SCAN TV on INSTAT](https://scantv.al/english/scan-intel/analiza/ritet-numri-i-ndermarrjeve-me-10-kryesojne-bizneset-ne-tregti-i23921),
  same caveat). **Hypothesis:** ≈ 34,000 venues (0.143 × 237,881 mixes a 2023 share with a 2024
  base). INSTAT's own 2025 register tables are downloadable Excel files
  ([business registers page](https://www.instat.gov.al/en/themes/industry-trade-and-services/business-registers/),
  published 2026-06-03) and would settle it.
- **Demand shape:** 12.47 million foreign arrivals in 2025 (11.7 million in 2024); 2.38 million in
  August alone
  ([Albanian Daily News](https://albaniandailynews.com/news/albania-attracts-6-6-more-foreign-tourists-in-2025),
  [August](https://albaniandailynews.com/news/foreign-tourists-arrivals-speed-up-in-august)).

### 6.2 Albania — fiscalisation (Law 87/2019) and what it costs to enter

- **Regime:** software-based since 2020 ("no obligation for certification of fiscal devices"), but
  "a POS application does require certification"; real-time XML exchange with the tax authority
  ([fiscal-requirements.com/countries/17](https://www.fiscal-requirements.com/countries/17)).
  Phases: B2G 1 Jan 2021, B2B 1 Jul 2021, all cash 1 Sep 2021
  ([Pagero/Thomson Reuters](https://europe.thomsonreuters.com/compliance/regulatory-updates/albania),
  [DDD Invoices](https://dddinvoices.com/learn/fiscalization-and-real-time-reporting-in-albania)).
- **Certified producers: 53** on the tax administration's list
  ([tatime.gov.al](https://www.tatime.gov.al/c/424/494/lista-e-subjekteve-te-certifikuara));
  49 on a January 2024 mirror ([fature.al](https://fature.al/b/lista-e-kompanive-te-licensuara-nga-tatimet-per-fiskalizimin)).
  Applications go through the National Agency for Information Society via e-Albania
  ([tatime.gov.al notice](https://www.tatime.gov.al/d/8/45/0/1401/fillojne-aplikimet-per-certifikimin-e-prodhuesve-apo-mirembajtesve-te-zgjidhjes-software)).
  The fee and the calendar time were not found (§8).
- **Law 79/2025 tightens the screws in dowiz's favour:** POS terminals mandatory for tourism and
  transport businesses by **30 May 2026**, all others by **31 Dec 2026**; cash caps of 100,000 lek
  (B2B) and 500,000 lek (individuals)
  ([fiscal-requirements 5314](https://www.fiscal-requirements.com/news/5314),
  [5525](https://www.fiscal-requirements.com/news/5525),
  [VATupdate](https://www.vatupdate.com/2025/12/30/albania-sets-new-cash-payment-limits-under-fiscal-package-effective-january-2026/)).
  Every coastal bar-restaurant now has a card terminal and a fiscal app; a second screen that runs the
  room is a smaller ask than it was in 2024.
- **Incumbents (all on the certified list):** BNT Electronics / **eBills** (the venue's own system;
  JHipster/Spring behind Azure Front Door, `BLUEPRINT-EBILLS-INTEGRATION` §1.1), Image & Communications
  Development / **DevPOS** (368 lek/month or 4,415 lek/year per a 2021 interview,
  [hashtag.al](https://www.hashtag.al/en/index.php/2021/07/07/drejtuesi-i-dev-al-shpjegon-devpos-sistemin-online-te-fiskalizimit-qe-lehteson-burokracite/),
  page 403 to fetch — treat as dated), Prisma / **ProFisc** ([profisc.al](https://profisc.al/en/home/)),
  **Elif** ([elif.com.al](https://elif.com.al/en/)), **easyPos** ([easypos.al](https://easypos.al/en)),
  Bilanc, Asseco SEE, Infosoft, and — notably — **Dega Kosova Information Technology**, a Kosovo
  company certified in Albania: the two markets already share vendors.
- **Aggregators:** Wolt (Tirana, Durrës, Vlorë, Dhermi, Fier, Elbasan, Sarandë, Shkodër —
  [wolt.com/en/alb](https://wolt.com/en/alb), [exittoalbania](https://exittoalbania.com/15-apps-you-will-need-in-albania/)),
  Glovo ("dominates Tirana"), Bolt Food ("growing")
  ([Nomada, May 2026](https://nomada.tools/directory/food-delivery/albania)), Baboon (Tirana, Durrës,
  Vlorë — [App Store](https://apps.apple.com/al/app/baboon-delivery-food-and-more/id1069121718)).
  Three to four aggregators in one small market is the strongest argument for a venue-owned
  storefront — which dowiz already is.

### 6.3 The four neighbours, keyed to cost of fiscal entry

| Country | Regime (what software must do) | Entry cost signal | Venues | Aggregators | Sources |
|---|---|---|---|---|---|
| **Kosovo** | Hardware fiscal devices until 2025; tax blocks electronic-only from **21 Jul 2025**; **AI MoF 01/2026** adds *Electronic Fiscal Software* with certification applications now open; transitional period for existing devices; €2,000 cash rule from 1 Jun 2026 | A greenfield software rule-set in Albanian; certification body ATK; fees/deadlines not readable (ATK page refused the connection, §8) | not found | **Wolt only** (Prishtina Jan 2025, Ferizaj Sep 2025, Prizren Mar 2026); Glovo absent | [ATK](https://www.atk-ks.org/en/notice-to-taxpayers-fiscalization-of-tax-blocks-will-be-done-only-electronically/), [fiscal-requirements 5652](https://www.fiscal-requirements.com/news/5652), [5653](https://www.fiscal-requirements.com/news/5653), [SeeNews](https://seenews.com/news/finlands-wolt-launches-service-in-kosovo-1270099), [Gazeta Express](https://www.gazetaexpress.com/en/wolt-from-March-26-in-the-beautiful-city-of-Prizren-download-the-app-and-start-with-your-first-orders/) |
| **Montenegro** | Software fiscalisation from 1 Jan 2021, mandatory 1 Jun 2021; ENU + qualified seal; cash fiscalised within 2 s, non-cash within 48 h | Vendor registers in CRPS, gets a tax id, passes Tax Administration testing; seal **€30** per entity (Pošta CG or Core IT); competitors at **€13/month** (Fiskal MN) and €7.90/month | small (not found) | **Glovo monopoly** (bought Donesi); no Wolt | [fiscal-requirements](https://www.fiscal-requirements.com/countries/14), [TiramisuERP](https://tiramisuerp.com/en/electronic-fiscalization-in-montenegro), [fiskalmn.me](https://www.fiskalmn.me/), [TripLinkHub](https://www.triplinkhub.com/en/blog/best-travel-apps-montenegro) |
| **Serbia** | Law on Fiscalization in force **1 Jan 2022**, transition to 30 Apr 2022; ESIR (POS) + PFR (local L-PFR or the state's V-PFR) + a security-element card from the Tax Administration; ESIR approved and numbered (Otkucaj: "record number 1667") | Approval of the ESIR by the Tax Administration, a per-taxpayer security card, an L-PFR for offline; free ESIRs exist (Otkucaj: **≤ 500 receipts/month free, €100/month priority**) — price competition is at zero | **40,700** hospitality businesses, 78,200 F&B workers (PKS, 2025) | Wolt and Glovo both | [efaktura.gov.rs](https://www.efaktura.gov.rs/tekst/en/5639/fiscalization-law.php), [Gecić Law](https://www.geciclaw.com/e-fiscalization-in-serbia/), [otkucaj.com](https://otkucaj.com/en/), [bezsajta.rs](https://bezsajta.rs/en/blog/restoran-trziste-srbija-2026/), [wolt.com/en/srb](https://wolt.com/en/srb) |
| **North Macedonia** | **Hardware-based**: certified fiscal device with fiscal memory and a GPRS module sending Z reports; POS software not certified but must drive the device; e-Faktura for non-cash B2B: pilot Jan 2026, mandatory Q3 2026 per one source, April 2027 per another (conflict noted) | A Worker cannot talk to a fiscal printer on the venue's LAN — the same LAN-bridge the POS blueprint refused for printers (§5 F). Highest entry cost of the five | 199.5 bars per 100k inhabitants | Glovo dominant; Wolt since Apr 2025 | [fiscal-requirements 44](https://www.fiscal-requirements.com/countries/44-north-macedonia), [VATupdate](https://www.vatupdate.com/2025/09/04/north-macedonia-introduces-e-faktura-reform-to-modernize-tax-system-and-enhance-fiscal-discipline/), [SeeNews](https://seenews.com/news/wolt-launches-service-in-n-macedonia-1273243), [Nomada](https://nomada.tools/directory/food-delivery/north-macedonia) |

### 6.4 Price points

| Product | Price | Source |
|---|---|---|
| DevPOS (AL, fiscal software) | 368 lek/month, 4,415 lek/year (2021) | hashtag.al above |
| Fiskal MN (ME, fiscal software) | €13/month, €156/year, first month free | [fiskalmn.me](https://www.fiskalmn.me/) |
| Otkucaj (RS, ESIR) | €0 to 500 receipts/month; €100/month priority | [otkucaj.com](https://otkucaj.com/en/) |
| Loyverse (global, free core) | $0 + add-ons $5–25/month per store | [Loman](https://loman.ai/blog/loyverse-pricing) |
| Square for Restaurants | $0 / $49 / $149 per location per month | [squareup.com](https://squareup.com/us/en/point-of-sale/restaurants/pricing) |
| Lightspeed Restaurant | $69 / $189 / $399 per month; KDS $30/screen | [UpMenu](https://www.upmenu.com/blog/lightspeed-pos-pricing/) |
| eBills (AL) | not published; not read | §8 |

### 6.5 Unit economics, and the recommendation

- **Cost:** one account is $5.93/month all-in (the $5 Workers Paid plan plus the domain); ten venues
  = $1.88 each ≈ 150 lek; the corrected marginal cost of a venue-month at overage rates is ≈ $0.65,
  and the binding limit is request COUNT, not duration (memory `dowiz-hub-unit-costs`, read in full;
  the artifact calculator is linked there). **Assumption:** the per-venue image growth stays inside
  the phase-2..5 numbers (≈ 1.2 KB per delivered order after deltas, memory `dowiz-hub-seven-phases`).
- **Price (assumption, to be tested on the first ten):** 1,500–3,000 lek/month (€15–30) for room +
  storefront + stock + CRM beside the venue's fiscal app. That is four to eight times DevPOS's
  fiscal-only price and a third of Square Plus, on an infrastructure margin above 90 %; the real cost
  is people — support in three languages and the certification project.
- **Why not "a dozen top venues":** they buy one system with the receipt in it; that system today is
  one of the 53. Winning them means certification first, a Tirana sales cycle, and a bespoke chain
  build — the highest cost of entry for the smallest count, before a single unit of the product is
  validated at volume.
- **Why not "Balkan SaaS" now:** four fiscal regimes, three of which need a certification or an
  approval in a different language, and one (North Macedonia) that needs hardware. Entering all five
  is five certifications for a product that cannot yet issue an Albanian receipt.
- **Therefore:** mass in Albania first, beside the fiscal POS, on the Law 79/2025 wave; certify in
  Albania when the tenth paying venue asks for one screen (that is the trigger, not a date); then
  **Kosovo** — the same language, the shared vendors (Dega Kosova), a rule-set written in 2026 for
  software, a single aggregator to differentiate against; then **Montenegro** because the entry is a
  €30 seal and a test suite; **Serbia** when there is a Serbian-speaking team, because it is the only
  neighbour large enough to change the numbers; **North Macedonia** only via a LAN bridge, i.e.
  after the POS blueprint's printer re-entry condition is met anyway.
- **Conditions under which the answer flips:** (a) a Tirana group signs for §2's chain design and
  funds certification → top-dozen first, because the certification is paid for; (b) certification
  in Albania proves slower than two neighbours' entries → stay beside the fiscal POS and never
  certify; (c) Kosovo's EFS fee or bond turns out prohibitive (§8) → Montenegro before Kosovo.

---

## 7. Recommended AGAINST, with the condition to reopen

| Item | Why not | Reopen when |
|---|---|---|
| Tips split by any rating of staff | a participant rating; `no-scoring.sh:58`, `DECISIONS.md:350`, `caps.rs:9-12` | never |
| A stored behavioural profile (usual weekday, household, propensity) | profiling under Law 124/2024; a record erasure must find; the fold answers recency without it | never; `usual_table` typed by the owner is the allowed form |
| dowiz as lender, escrow or holder of venue revenue | Bank of Albania licence; MANIFESTO §2 economic control point; D14 D-money | a licensed partner underwrites on the venue-signed statement (§4.4) and the operator reopens D-money |
| HQ write-through to franchisee objects | the central server D1 dropped | never; import-by-dry-run is the channel |
| A persisted ranking of venues or units | `venue_rank/tier` is in the gate | never; a query at read time, stored nowhere |
| ML or an LLM producing a number the hub stores | C1/C2; two replicas would disagree | an advisory model outside the hub beats seasonal naive on ≥ 12 weeks of rolling holdout, measured by the fold in §1.4 |
| Per-employee training scores or leaderboards | a staff rating | never; completion → a capability grant |
| Weather as a fitted coefficient before one summer of paired data | nothing to fit it on (§1.3) | summer 2027 at the earliest, per venue |
| Entering North Macedonia before a LAN fiscal bridge exists | hardware regime (§6.3) | the POS blueprint's printer re-entry (§5 F) is met for another reason |

---

## 8. What could not be verified

- **INSTAT's absolute count of accommodation and food service enterprises.** The SME PDF carries it in
  a chart; the register page links Excel tables; the two secondary pages returned 403. The 34,000
  figure is a hypothesis of a 2023 share on a 2024 base.
- **The Albanian certification fee and elapsed time** for a software producer (AKSHI/e-Albania path).
- **eBills' price.** Not published where this lane could read it.
- **Kosovo EFS fees, deadlines and the length of the transitional period:** the ATK notice page
  refused the connection (`ECONNREFUSED 178.132.222.4:443`); the summary comes from a secondary source.
- **Serbia's law articles** on ESIR approval: the efaktura.gov.rs page exposes only a PDF link.
- **Meta's WhatsApp marketing rate for Albania** and the neighbours; only Germany's €0.1131 was
  readable as an anchor.
- **Open-Meteo's commercial price** (the pricing page renders it client-side).
- **Bank of Albania minimum capital** for an NBFI / microcredit institution (inside an 851 KB PDF).
- **Whether ebills answers sale ranges older than 30 days** — decides whether a forecast can be
  backfilled from the venue's own fiscal history rather than waiting a year.
- **Whether any live order carries `tip > 0`** — the POS blueprint's open question (§8 there); it
  decides whether §4.3's pool fold has a breach to fix first.
- **Two dates disagree on Law 124/2024's entry into force:** `consent.rs:3` says 31 January 2025;
  the CMS/Clym summaries say 17 January 2025. Whichever is right, it is in force.
- **North Macedonia's e-Faktura mandatory date:** Q3 2026 (VATupdate) vs April 2027
  (fiscal-requirements). Not material to the sequence, since the cash regime is the blocker.
- **The sibling blueprint** (`BLUEPRINT-OPERATIONAL-BLIND-SPOTS-2026-09-23.md`) had not been written
  when this was; §1.5, §4.2 reference it by name and may need their cross-references checked once
  it lands.
- **Every price in §6.4** is a vendor's public page on 2026-09-23; none was confirmed by a quote.
