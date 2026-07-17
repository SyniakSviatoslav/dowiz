# BLUEPRINT — DEMO & MARKETING PIPELINE REFACTOR (offers + previews + funnel) — 2026-07-17

> Cross-cutting blueprint over the 19-phase master roadmap (`sovereign-roadmap-2026-07-16/
> R2-MERGED-PHASE-ROADMAP.md`). It does not add a 20th phase: it defines seven work units
> (**DM-1 … DM-7**) that slot into existing phases' dependency graph — consuming **P07**
> (money-law closure), **P16** (product UI rebuild), **P17 pre-unlock** (scripted wasm demo),
> **P18** (public flip) and feeding **P19** (growth engine). Built per the Detailed Planning
> Protocol (`AGENTS.md:160-231`): ground truth first, re-derived dependencies, inline DECART,
> falsifiable done-checks, 2-question doubt audit applied to this document itself (§9).
>
> **Planning document only — no code is written or edited by this blueprint.** DM-1/DM-2 touch
> red-line money math and earn their own careful implementation pass. Nothing here publishes,
> submits, or sends anything — every public-facing action is behind P18 + operator go, mirroring
> P19's boundary.
>
> **HARD CONSTRAINT honored throughout** (`hermetic-architecture-2026-07-16/
> HERMETIC-ARCHITECTURE-PRINCIPLES.md`; operator: all-Rust-native): every tool proposed is pure
> Rust (or a committed static asset), or a narrow typed Rust adapter to an external endpoint.
> No new Python/Node enters the tree. The one pre-existing external adapter reused (Telegram
> `sendMessage`) is already in-tree Rust (`tools/telemetry/topics/src/main.rs:55`).

---

## 1. Current-state evidence (file:line + live probes, read/verified 2026-07-17)

### 1.1 The old demo pipeline is deleted from HEAD but still LIVE on an orphaned deployment

- **Deleted:** the entire JS/TS product surface — `79ef316f6` + `db766de47` (2026-07-13,
  "remove legacy JS/TS thin-layer") deleted `apps/web`, `packages/ui`, `packages/domain`,
  `packages/shared-types`; `e1505e1d9` quarantined `apps/api` (with `spa-shell.ts`,
  `og-card.ts`, `preview-render.ts`), `apps/worker`, `packages/db`, `fly.toml` into `attic/`;
  `f9ab28ff1` ("drop ALL JS/TS (per operator)") + `a29aa219e` then purged the attic itself.
  `ls attic/` → does not exist at HEAD. `scripts/` contains no `demo-builder.mjs` /
  `acquisition-bulk-provision.mjs` — the demo-builder loop, the 12-venue provisioning scripts,
  and the OG-card renderer exist **only in git history**.
- **Still live (probed this session):** `curl -A "TelegramBot"
  https://dowiz-staging.fly.dev/s/apollonia` → HTTP 200 with `og:title "Apollonia — Menu
  Digjitale"`, `og:image https://dowiz-staging.fly.dev/og/apollonia.png`, `robots noindex,
  nofollow`. The 12 Durrës shadow demos and their rich per-venue OG unfurl (shipped `6a89d6e8`,
  2026-07-06) are served by a **staging Fly deployment whose source no longer exists at HEAD**.
- **Prod is effectively dark for demos:** `curl https://dowiz.fly.dev/s/apollonia` → HTTP 200,
  **63-byte body, zero OG tags** (root `/` serves 2727 bytes). Whatever prod runs now, it is
  not the rich storefront.

### 1.2 The memory claim "12 demos SHADOW; FULL unfurl = RED-LINE override, gated on counsel" is STALE in both directions

Verified against `og-preview-demo-upgrades-2026-07-06.md` + git: the counsel gate was **already
resolved** on 2026-07-06 — operator waived council ("full preview without additional consents"),
WS1+WS2 shipped and proven on staging (`6a89d6e8`). So "gated on counsel" is stale-closed. And
the substrate itself is stale-deleted (§1.1). What **still stands** is the substance of the
P6-2/P6-3/H3 honesty invariants the operator did *not* reverse: preview banner ("preview mockup —
not a live store"), `noindex`, never-orderable shadow spine (`owner_id NULL`, `status='closed'`),
claim/decline path. The reversal was scoped to OG richness only.

### 1.3 Offers: a shipped-then-deleted feature with its kernel slot deliberately empty

- **Historical:** promotions CRUD shipped in the old stack (`812d254e1` "feat: promotions CRUD")
  with `PromotionType = percentage | fixed | free_delivery`, min-order, validity window,
  max-uses, single-code no-stacking — deleted with the thin layer (`79ef316f6`). Survives only
  in a `node_modules` cache copy and in the P16 feature inventory.
- **Inventory-preserved:** the P16 26-page inventory carries the full page spec —
  `docs/design/dowiz-interfaces/RESEARCH-CONSPECT.md:101` ("PROMOTIONS: CRUD (code/type
  %-fixed-free_delivery/value/min-order/valid-from-until/max-uses …); NO discount-stacking
  single-code") and `BLUEPRINTS-DOWIZ-INTERFACES.md:266`. P16's "no feature is lost" rule means
  the *page* will be rebuilt; nothing yet gives it math to stand on.
- **Kernel:** `kernel/src/domain.rs:11` — "No discounts in this scope"; `domain.rs:89-90`
  mirrors the historical oracle `total = subtotal + deliveryFee + taxTotal - discountTotal`
  (orders.ts:565) but implements only `subtotal + tax + fee` (`compute_order_total`,
  `domain.rs:95-112`, integer-only, `checked_add`, non-negative assert). The discount slot is a
  **named, deliberate gap** — the oracle formula is cited in the doc comment but not restored.
- **Wasm surface:** `estimate_order_total_js(subtotal, cfg_json)` exists
  (`kernel/src/wasm.rs:407`) — the sanctioned UI money entry point to extend.

### 1.4 The Rust substrate the refactor builds ON

- **Deterministic frame compositor:** `engine/src/field_frame.rs:189` `compose(scene, eq, w, h,
  steps) -> Vec<u8>` (RGBA8), proven bit-identical (`compose_returns_deterministic_frame`,
  `:302`); engine `Cargo.toml` mandates **zero external crates** in the default build ("NO
  dependencies — offline-clean by mandate … cosmic-text OUT OF SCOPE here (added behind
  features later)").
- **Static serving:** `tools/native-spa-server/` (DK-04) — from-scratch Rust static server
  replacing nginx, TLS/HTTP2 capable, RED-test-locked.
- **Outreach adapter:** `tools/telemetry/topics/src/main.rs:55` already POSTs
  `api.telegram.org/bot{}/sendMessage` from Rust — an existing in-tree Telegram adapter, so
  reusing it is **not** a new integration (no DECART required; Integration Decart Rule applies
  to *new* adoptions/swaps).
- **Crate acquisition is BLOCKED:** R2 §4 O18a — `cargo add` fails, crates.io 403, verified RED
  2026-07-16. Grep of every `Cargo.lock` in-tree finds **no** `png`/`image`/`resvg`/
  `tiny-skia`/`fontdue`/`cosmic-text`. Any design that needs a new crate is gated on the same
  network unlock as `wgpu`. This constraint shapes the §5 DECART decisively.
- **P17 pre-unlock demo (blueprinted, unbuilt):** `BLUEPRINT-P17-demo-splat-gpu-unlock.md` §2 —
  a scripted, byte-deterministic wasm delivery demo (`delivery-01` scenario: courier along a
  real `router::route`, FSM ticking via `apply_event_js`, field composed around it), node ≡
  browser bit-identical. DM-6 **consumes** this; nothing in this blueprint re-plans it.
- **Money-law bug that would corrupt offer accounting:** `BLUEPRINT-P07-money-law-closure.md`
  §1.1 — `commit_after_decide` computes the dedup id **before** prev-chaining while `append`
  re-chains and stores under a different id, so a replayed event **misses dedup and re-runs
  `decide`**. An `OfferRedeemed` event committed through this path would double-count
  redemptions on replay. This is the real (not assumed) dependency edge DM-2 ← P07.

---

## 2. External research digest — what "best-in-class" actually is (sourced, thin-data flagged)

Web research pass 2026-07-17 (restaurant-vertical B2B SaaS: Toast, Owner.com, ChowNow, Flipdish,
Slerp; demo-tooling category: Navattic/Storylane/Arcade). Full citations inline; every number
below is labeled by evidence grade.

1. **The restaurant-SaaS category runs gated, sales-assisted demos — none of the four incumbents
   ship a self-serve interactive demo or sandbox** (verified against their live sites: Toast
   `pos.toasttab.com/request-demo` segmented request forms; Owner.com `owner.com/demo` gated
   form; ChowNow `get.chownow.com/demo/`; Flipdish differentiates on onboarding humans, per
   third-party reviews). **Self-serve interactive demo is category whitespace.**
2. **Personalization is the strongest documented demo pattern**: Owner.com's pitch is a
   *personalized* preview ("see how Owner could work for you"), results-framed — the closest
   analog to dowiz's per-venue shadow demos ("your own menu, live"). Named-human follow-up
   (ChowNow's "Restaurant Success Manager") is the category's other conversion lever.
3. **Interactive-demo format benchmarks (adjacent categories, vendor-published):** Navattic 2025
   report across 28k+ demos — top-quartile ~50% engagement / ~29% completion; top-1% ~84% / ~62%.
   No restaurant-vertical breakdown exists. Arcade's "7.2x vs video" is a vendor case-study
   claim. **Directional only; no decision below leans on a specific number.**
4. **Offer mechanics that exist in-category:**
   - *Referral:* Toast pays **$500 per referred restaurant after 30 days of usage**
     (`pos.toasttab.com/contact/refer-a-restaurant`) — clean restaurant-refers-restaurant.
   - *Trial-as-fee-waiver:* Uber Eats 0% marketplace fee first 30 days; DoorDash 30-days-
     no-commission promos. The in-category "trial" is **waived take-rate for N days**, not a
     feature-limited SaaS trial. No vendor publishes a literal "free until $X GMV" tier.
   - *No-lock-in as guarantee:* Owner.com month-to-month, cancel anytime — substitutes for a
     money-back guarantee.
   - *Trial-structure folklore (uncited aggregator blogs — treat as folklore):* opt-in trials
     ~18% conversion vs opt-out ~49%; SMB demo→close 30-40%. Methodology undisclosed everywhere.
5. **Vendor-facing offer management is table-stakes, not a differentiator**: ChowNow ships
   owner-controlled promos/scheduling; Slerp ships per-site loyalty tied to hero products;
   category-standard mechanics = discount codes, first-order discounts, day-of-week/BOGO,
   happy-hour push. A delivery platform without vendor promos is *below* baseline.
6. **The combination gap**: no single competitor combines owner-controlled promos + referral
   cash/credit + first-N-days fee waiver end-to-end. That combination — on a self-hosted,
   AGPL, PQ-protocol stack no competitor can match — is the honest positioning wedge, and it is
   consistent with P19 §4's mesh-consistent pricing (monetize services, never the protocol).

---

## 3. Design — the pipeline as seven work units

**Shape:** one pipeline, three lanes that meet at the public funnel: (a) *offers substrate*
(DM-1→DM-2→DM-3) gives both the vendor and the operator real offer mechanics; (b) *demo
substrate* (DM-4, DM-5) re-derives the deleted demo assets on the Rust stack; (c) *funnel
assembly* (DM-6, DM-7) wires them into the P17 wasm demo and P18 public surface. Nothing in lane
(c) goes public before P18 + operator go.

### DM-1 — Kernel offer math (restore the oracle's discount slot) — RED-LINE money

Extend `compute_order_total` (`kernel/src/domain.rs:95`) to the full historical oracle formula
it already cites: `total = subtotal + tax + fee − discount`, with `discount: Option<i64>`.
Typed domain in a new `kernel/src/offer.rs` (mirrors deleted `PromotionType` — feature-inventory
preservation):

- `enum OfferKind { Percentage { bps: u32 }, Fixed { amount: i64 }, FreeDelivery }` — percentage
  as integer **basis points** (no float touches money; P3-A4/S9 discipline), deterministic floor
  rounding, `checked_mul`/`checked_add` everywhere.
- `struct Offer { code, kind, min_order: i64, valid_from, valid_until, max_uses: u32, … }` —
  fields exactly per the inventory spec (`RESEARCH-CONSPECT.md:101`), including **single-code
  no-stacking** as a type-level invariant (an order carries `Option<AppliedOffer>`, never a Vec).
- `fn compute_discount(offer, subtotal, fee) -> Result<i64, String>` — fail-closed `Err` (P4:
  one decision function, typed deny) on: below `min_order`, outside window, over-discount
  (`discount > subtotal + fee` is an error, not a clamp), overflow. Total invariant unchanged:
  `assert_non_negative(total)`.
- Wasm: extend `estimate_order_total_js` cfg JSON with an optional `discount` field —
  additive, existing callers unaffected.

*Deps:* none hard (pure math; parallel-safe with everything). Red-line money ⇒ separate careful
implementation pass, invariant tests written RED first (AGENTS §8 TDD binding).
*Done-check:* unit+property tests proving (1) parity with the historical oracle formula on a
fixture table, (2) no input produces negative total or panics, (3) `FreeDelivery` discount
exactly equals the fee slot, (4) bps rounding is deterministic across two runs.

### DM-2 — Offer redemption accounting (event-sourced) — depends on P07

`OfferRedeemed { code, order_id, amount }` appended through the one event log (never a side
channel — AGENTS §8 event-driven binding). `max_uses` and the inventory's "usage badge
used/max" are **folds over the log**, not a mutable counter.
*Deps:* **P07 §1.1 dedup fix (HARD)** — under the current `commit_after_decide` id-divergence a
replayed `OfferRedeemed` re-runs `decide` and double-counts (§1.4); building redemption before
P07 lands would bake the corruption in. Also DM-1 (the types).
*Done-check:* the P07 replay test extended with an offer fixture — replaying a committed
`OfferRedeemed` is a no-op; a fold after N distinct redemptions of a `max_uses=N` code rejects
redemption N+1 with a typed deny.

### DM-3 — Owner Promotions page — owned by P16, bound here to its substrate

The page itself is P16's deliverable (already in the 26-page inventory with a full spec —
`RESEARCH-CONSPECT.md:101`, `BLUEPRINTS-DOWIZ-INTERFACES.md:342`). This blueprint adds exactly
one binding: the page's validation and price preview call **kernel exports** (DM-1 via
`estimate_order_total_js`), never JS re-implementations — enforced by P16's existing wasm-math
grep gate. No page planning is duplicated here.
*Deps:* P16 (admin shell), DM-1; usage badge additionally DM-2.
*Done-check:* P16's grep gate passes with the Promotions page present; a Playwright/e2e
assertion that the previewed discount equals the kernel-computed value byte-for-byte.

### DM-4 — Demo re-derivation: `tools/demo-forge` (static per-venue shadow demos)

Rebuild the deleted demo-builder as a native Rust bin producing **static, pre-rendered demo
storefront bundles** served by `native-spa-server` — no runtime DB, no Node, fits zero-OCI:

- **Input:** a versioned venue fixture (`demos/<slug>/venue.json`: menu, palette, photos,
  rating), one per demo venue. The 12 Durrës venues' data is recoverable from git history /
  the staging DB — **but see the OPERATOR DECISION below before any recovery.**
- **Output:** a static bundle rendering the P16 storefront components against fixture data,
  with the standing honesty invariants baked in: "preview mockup — not a live store" banner,
  `noindex,nofollow`, **no order path compiled in at all** (never-orderable by construction —
  stronger than the old runtime gate), claim/decline CTA.
- **Invariant carry-over:** the operator's 2026-07-06 ruling (rich per-venue OG) is honored;
  the un-reversed P6-2/P6-3/H3 substance (banner, noindex, decline) is preserved verbatim.

> **⚠ OPERATOR DECISION (named, blocking DM-4 data recovery):** the old fixtures were built
> from scraped Wolt/Google data under a council-conditioned consent posture. The operator
> reversed the *generic-OG* condition only. Reusing that scraped data in a NEW pipeline —
> versus re-provisioning demos from fresh, consenting, or synthetic venues — is a consent-scope
> question this blueprint does not decide. Options: (a) reuse the 12 as-is, (b) synthetic
> fixture venues only (zero consent surface; loses personalization), (c) fresh outreach-consented
> venues. DM-4's *tooling* is identical under all three; only the fixture content differs, so
> tooling may proceed while this is open.

*Deps:* P16 (storefront pages to render); fixture content additionally the operator decision.
*Done-check:* `demo-forge` run twice over the same fixture yields byte-identical bundles
(P6 determinism); a serving smoke on `native-spa-server` shows banner + noindex + zero order
endpoints; grep proves no ordering code in the bundle.

### DM-5 — Preview/OG card renderer: `tools/preview-card` (pure Rust, zero new crates)

The per-venue OG card (`/og/<slug>.png`, 1200×630) that the deleted `og-card.ts` produced,
re-derived: engine `compose` paints the branded field background (one compositor — P2: one
primitive, not a second raster path), an integer glyph-blit draws name/rating from a
**committed glyph atlas asset**, and an in-tree **stored-block PNG encoder** (zlib stored
blocks + CRC32, ~150 lines, no compression needed at this size) writes the file. Full DECART
in §5.1 — this is the only genuinely new tool decision in the pipeline.

- Glyph atlas = a committed asset (bitmap strikes + metrics sidecar) covering Latin +
  **Albanian diacritics (ç, ë — "Menu Digjitale", venue names)** + digits/★. Its one-time bake
  provenance and font license file are committed beside it; the build path is pure Rust
  consuming bytes.
- Cards are emitted at demo-forge time into the static bundle (no runtime rendering, no route
  code — `native-spa-server` serves them as files).

*Deps:* none (engine `compose` exists; atlas is an asset). Buildable **now**, pre-unlock.
*Done-check:* golden test — card bytes for a fixture venue are identical across two runs and
match a committed golden; the test itself re-parses the PNG (signature, IHDR, CRC per chunk,
inflate-stored round-trip) so validity is proven without an external decoder.

### DM-6 — Funnel assembly (the marketing pipeline proper)

The research-grounded funnel, each stage an existing planned artifact — this unit is *wiring*,
not new surface area:

```
QR / link  →  per-venue shadow demo (DM-4: personalization — "your menu, live")
           →  embedded P17 §2 wasm delivery demo (interactive tour — category whitespace, §2.1)
           →  claim CTA (standing claim/decline path)
           →  offer attach (DM-7 founding offer, computed by DM-1 math)
           →  operator contact (Telegram deep-link; notify via the existing Rust
              sendMessage adapter — §1.4, no new integration)
```

Funnel measurement = **typed local counters per demo tenant** (M8: local-only, typed, never
exfiltrated; the old channel-attribution E2E, `80b204452`, is the feature-inventory precedent).
No third-party analytics — rejected in §7.
*Deps:* DM-4 + P17 pre-unlock (both demo artifacts); **public embedding ← P18 + operator go**
(staging-scoped assembly may precede the flip; nothing public before it).
*Done-check:* on staging, one scripted pass walks all five stages and the local counter fold
shows exactly one event per stage; grep proves no external analytics origin in the bundle.

### DM-7 — Operator's own offer preparation/management (specs + drafts, publish-gated)

Applies §2.4/2.6 to dowiz's mesh-consistent pricing (P19 §4: protocol free forever, monetize
services on top). Three offer specs, each with a falsifiable trigger, all **draft-only** here:

1. **Founding offer** — managed-hosting/service fee waived for the first N claimed venues for
   30 days (the in-category trial shape: Uber Eats/DoorDash fee-waiver, §2.4), time-limited,
   N and window are operator knobs. Computed/displayed via DM-1 math (`FreeDelivery`-analog on
   the service-fee slot), so the offer the marketing shows is the offer the ledger enforces.
2. **Referral offer** — venue-refers-venue **service credit** (Toast's $500 pattern, adapted:
   credit not cash, ledger-visible, no surveillance scoring). BUILD-TRIGGER (grep-able):
   > **BUILD-TRIGGER (DM-7 referral):** first real *claimed* venue. Until then referrals are
   > hand-tracked; building referral plumbing before one venue exists violates YAGNI/C8.
3. **Seasonal campaign templates** — vendor-facing promo presets (day-of-week, happy-hour —
   §2.5 table-stakes) shipped as fixture `Offer` rows the Promotions page can clone; zero new
   mechanics beyond DM-1.

*Deps:* drafting ← none; any publication/sending ← **P18 + operator go** (P19 §8 boundary
mirrored verbatim: this unit submits nothing and publishes nothing).
*Done-check:* specs exist in-tree with grep-able BUILD-TRIGGER lines; `grep` finds zero
implementation for referral plumbing pre-trigger; founding-offer copy contains no claim the
DM-1 math cannot enforce.

### DM-8 (housekeeping, operator-gated) — decommission the orphaned deployments

The staging Fly deployment (§1.1) keeps serving deleted code; prod serves a 63-byte stub.
When DM-4+DM-5 reach visual/OG parity for the chosen fixture set, the orphaned Node staging
deploy is retired. Irreversible infra action ⇒ **operator gate**, never autonomous.
*Done-check (pre-parity):* a dated inventory of what the orphan serves (routes, OG assets) so
parity is checkable, not vibes; *(post-parity, post-go)* the Fly app is stopped and the demo
URLs resolve to the Rust-served bundles.

---

## 4. What this deliberately does NOT build

- **No interactive-demo SaaS embed** (Navattic/Storylane/Arcade) — §5.2 DECART rejects.
- **No third-party analytics/attribution service** — M8 no-surveillance; local typed counters.
- **No video demo before O18a fires** — E4/F47 LOCK stands exactly as P17 holds it.
- **No new outreach channel** — Telegram-primary via the existing in-tree adapter (IP-* stance).
- **No billing infrastructure** — E57 stays spec-only under its P19 trigger; DM-7's offers are
  ledger math + copy, not a payment stack.
- **No re-plan of P16 pages or P17 scenario** — DM-3/DM-6 bind to them, never duplicate them.

---

## 5. DECART — the new tool/service choices (Integration Decart Rule, inline)

### 5.1 Preview-card renderer (DM-5)

| Criterion | **A: in-tree zero-dep** (engine `compose` + committed glyph atlas + stored-PNG encoder) | B: vendored Rust crates (`resvg`/`tiny-skia`/`fontdue` or `cosmic-text`) | C: hosted card-render SaaS (htmlcsstoimage / Bannerbear) | D: resurrect old Node `sharp` pipeline from git |
|---|---|---|---|---|
| Bare-metal / all-Rust fit | Perfect — zero deps, engine mandate intact | Good — pure Rust, but +~40 transitive crates | None — external service | **Violates the hard constraint** (Node) |
| Falsifiable correctness | Golden byte-identical PNG (P6-grade determinism) | Mostly deterministic; float AA paths need per-version goldens | Not falsifiable (remote render drift) | n/a |
| Measured performance | 1200×630 compose ≈ ms-scale on CPU (engine already proves frame-rate compose) | Fine | Network-bound | n/a |
| Supply-chain / license | Zero crates; font license committed beside atlas | **BLOCKED — crates.io 403 (O18a RED, §1.4)** until network unlock + DECART-per-crate | API key + ToS dependency | npm tree, purged stack |
| Maintainability | ~150-250 in-tree lines (encoder + blit); crude typography | Best long-term text quality (proper shaping) | Low code, high dependency | Dead stack |
| Reversibility-as-port | Clean: A's call-site becomes B's behind a feature flag post-unlock | Fine | Lock-in | None |

**DECISION:** **A now** — it is the *only* candidate buildable pre-unlock, and the only one that
keeps the engine's zero-dep mandate; **B is the named post-unlock upgrade** (behind a feature,
with its own per-crate DECART when O18a fires — same discipline as `wgpu` in P17 §3). C rejected
(sovereignty, falsifiability). D rejected (hard constraint).
**Older-as-adapter note:** the deleted `og-card.ts` design (layout, card anatomy: name +
rating + dish photo + accent) is carried over as the *spec* A implements — the old artifact
serves as oracle, not as running code.
**MANDATORY PROBE (strongest honest case against A):** the atlas bake is a one-time non-Rust
step (recorded as provenance, outside the build path) — a wart; bitmap-strike typography at
1200×630 is visibly cruder than the old DejaVu vector text, and a marketing card is precisely
where typography is the product. If a fixture card looks bad enough to hurt the pitch, the
honest fallback is **ship no rich card until B unlocks** — "ugly card beats no card" is not
established by any evidence in §2, while the staging unfurl proves the rich-card value bar.

### 5.2 Interactive demo mechanism (DM-6 stage 2)

| Criterion | **A: own P17 §2 deterministic wasm demo** | B: interactive-demo SaaS (Navattic/Storylane/Arcade) | C: video walkthrough |
|---|---|---|---|
| Bare-metal / all-Rust fit | Perfect (kernel wasm, zero JS math) | Violates — third-party JS embed + their backend | n/a until GPU |
| Falsifiable correctness | Byte-deterministic node ≡ browser (P17 AC 1-2) | None (hosted, mutable) | — |
| Evidence it converts | Format benchmarks directional (§2.3); category whitespace (§2.1) | Same benchmarks — the format, not the vendor, is the signal | E4/F47 **LOCK** blocks it pre-unlock |
| Supply-chain | Zero | External script + data flow (M8 conflict) | — |
| Reversibility | Ours | Contract + embed removal | — |

**DECISION:** **A** — already blueprinted (P17 §2), zero new deps, and §2.1 shows no incumbent
has any self-serve demo, so the differentiation comes from *having one*, not from tour-tooling
polish. B rejected; C blocked by the standing LOCK.
**MANDATORY PROBE:** tour-SaaS demos convert because they walk the *real admin UI* with guided
tooltips; a physics render of a delivery shows the owner what their *customer* gets, not what
*they* will operate. Mitigation is structural in the funnel: the personalized DM-4 storefront
("your menu") is stage 1 and carries the §2.2 personalization pattern — the wasm demo is the
wow-stage, not the whole pitch. If claim-rate data later shows stage-2 drop-off, the correction
is a fixture-data P16 admin demo as a new stage, not a SaaS embed.

*(No third DECART: Telegram reuse is not a new integration (§1.4); demo-forge/preview-card are
in-tree code, not adopted dependencies.)*

---

## 6. Sequencing — re-derived dependency graph (per Protocol step 2)

Drafted order was DM-1→…→DM-8; re-derivation shows three of the seven are mutually independent
roots. Real edges only:

```
DM-1 offer math      ← ∅            (red-line pass, but no structural dep)
DM-5 preview-card    ← ∅            (engine compose exists; atlas = asset)
DM-7 offer specs     ← ∅            (drafting)   [publish ← P18 + operator]
DM-2 redemption      ← P07 (HARD: §1.4 replay double-count) + DM-1
DM-4 demo-forge      ← P16 (storefront components) [+ fixture content ← OPERATOR DECISION §3]
DM-3 promotions bind ← P16 + DM-1   [usage badge additionally ← DM-2]
DM-6 funnel          ← DM-4 + DM-5 + P17-pre       [public embed ← P18 + operator]
DM-8 decommission    ← DM-4 ∧ DM-5 parity + OPERATOR GO (irreversible)
```

**Waves:** **Wave α (now, parallel, collision-free):** DM-1 ∥ DM-5 ∥ DM-7-drafts — none touches
the others' files. **Wave β (after P07 / P16 respectively):** DM-2; DM-4 + DM-3.
**Wave γ:** DM-6 assembly on staging (after P17-pre). **Wave δ (operator-gated):** DM-6 public
embed + DM-7 publication (after P18); DM-8.
This slots under the master roadmap without bending it: nothing here accelerates P18/P19 or
jumps their gates; Wave α is exactly the substrate work that is safe *before* the flip (P19's
own rule: growth work before the flip has nothing real to point at — Wave α builds the thing
to point at, publishes nothing).

---

## 7. Preserved invariants & rejections (none resurrected, none weakened)

1. **Honesty invariants on every demo** — banner, `noindex`, never-orderable (by construction
   in DM-4), claim/decline. The 2026-07-06 reversal stays scoped to OG richness.
2. **No courier scoring / no surveillance (M8, E58)** — funnel metrics are local typed counters;
   no third-party analytics; no per-customer tracking in offers (`max_uses_per_customer` from
   the old model is carried as a field, enforced as a log fold, never as a profile).
3. **Money red-lines** — integer-only, checked, fail-closed, non-negative; discounts never
   touch a float or an easing channel (P3-A4); no discount-stacking (type-level).
4. **E4/F47 LOCK** — no demo video before O18a; DM-6 uses the wasm demo only.
5. **Never-bypass human gates** — DM-4 fixture consent, DM-8 decommission, all publication:
   operator decisions, named where they block.
6. **Zero-dep default builds** — engine/kernel default dependency graphs stay byte-identical;
   DM-5 candidate B enters only post-unlock behind features with per-crate DECARTs.
7. **Nothing published/sent by this blueprint's execution** — P19 §8 boundary, verbatim.

---

## 8. Acceptance criteria (falsifiable, numbered)

1. **DM-1:** kernel tests prove oracle-formula parity, no-negative-total, deterministic bps
   rounding, `FreeDelivery ≡ fee`; `estimate_order_total_js` accepts the optional discount cfg
   with all pre-existing calls green.
2. **DM-2:** replaying a committed `OfferRedeemed` is a proven no-op (extends P07's replay
   test); `max_uses=N` rejects redemption N+1 with a typed deny.
3. **DM-3:** P16 wasm-math grep gate green with the Promotions page present; UI-previewed
   discount equals kernel-computed value.
4. **DM-4:** double-run byte-identical bundles; banner + noindex present; grep finds zero
   ordering code in any demo bundle; the named operator decision on fixture data is recorded
   in-tree before any scraped-data recovery.
5. **DM-5:** golden byte-identical card; the test independently re-verifies PNG structure
   (signature/IHDR/CRC/stored-inflate); atlas covers ç/ë and renders a fixture Albanian venue
   name; zero new entries in any default `Cargo.lock`.
6. **DM-6:** one staged pass traverses all five funnel stages with exactly one local counter
   event each; grep finds no external analytics origin; nothing publicly embedded pre-P18.
7. **DM-7:** BUILD-TRIGGER lines grep-able; zero referral plumbing pre-trigger; founding-offer
   copy asserts nothing DM-1 math cannot enforce.
8. **DM-8:** dated orphan-inventory exists before parity work; decommission occurs only after a
   recorded operator go.

---

## 9. Self-critique — the 2-question doubt audit (run on this document)

**Q1 — least confident (not rounded down):**
1. **Prod's 63-byte body was never inspected** — I probed sizes/tags, not content; what prod
   actually serves now is unknown. *(Routine: DM-8's inventory done-check covers it before any
   action.)*
2. **Staging Fly orphan's ownership/cost/expiry unverified** — no `flyctl status` run this
   session; the deploy could vanish on its own before parity. *(Real risk to DM-8's "keep until
   parity" assumption → elevated into DM-8's first done-check: the dated inventory must be
   taken EARLY, not at parity time.)*
3. **Glyph-atlas font licensing + ç/ë coverage asserted, not checked** — DejaVu's license
   permits embedding, but I did not verify a specific font file or its diacritic strikes.
   *(Routine: AC 5 makes it falsifiable before merge.)*
4. **All §2 conversion numbers are vendor/aggregator-grade** — flagged in place; no §3 decision
   depends on a specific number, only on verified site-behavior patterns (§2.1) and shipped
   competitor mechanics (§2.4-2.5).
5. **Scraped-fixture consent scope** — the "1 in 4" class item: building DM-4 content on the
   old Wolt data without a fresh ruling would repeat the P6 red-line dance. *(Acted on:
   promoted to a named blocking OPERATOR DECISION in §3, and AC 4 requires it recorded
   in-tree.)*
6. **Interaction with in-flight agentic-mesh settlement/budget work (B2/B3) unexamined** —
   offer redemption is a money-adjacent ledger fold; whether the mesh WorkReceipt/Settlement
   design constrains `OfferRedeemed`'s shape was not checked against the sibling worktree.
   *(Real risk for DM-2 → recorded as a pre-implementation check in DM-2's pass, alongside its
   P07 dependency.)*
7. **The exact P16/DZ work-unit id owning the Promotions page** — I cite the inventory lines,
   not the DZ number; the implementer must bind DM-3 to the right unit at implementation time
   (named gap, per Protocol step 4 — not papered over with an invented id).

**Q2 — the biggest thing I'm missing:** this plan's gravity assumes demo/marketing substrate is
worth building *before* P18's flip and G11's first real order — but the roadmap's own
sequencing wisdom (P19: "growth work before the flip has nothing real to point at") cuts the
other way, and the operator's actual highest-leverage marketing asset today may simply be the
staging orphan that already works. The honest resolution baked into §6: Wave α contains only
substrate the *product* needs regardless (offer math is a P16-inventory obligation; the card
and demo tooling replace deleted capability), and every outward-facing action sits behind the
same gates P19 already respects. If the operator rules that even Wave α should wait behind the
critical path (P3→P9→P10→P13), nothing in this blueprint resists that — it re-slots without
edits because its roots are dependency-free.

---

## Anchor / phase coverage

| Touches | How |
|---|---|
| P07 | DM-2 hard-depends on its §1.1 dedup fix; extends its replay test |
| P16 | DM-3 binds the inventoried Promotions page to kernel math; DM-4 renders its storefront |
| P17 | DM-6 consumes the §2 pre-unlock wasm demo unchanged; LOCK honored |
| P18 | Sole gate for every public embed/publication in DM-6/DM-7 |
| P19 | DM-7 instantiates §4 mesh-consistent pricing as concrete offers; boundary mirrored |
| M8/E58 | Local-only funnel metrics; no scoring/surveillance in offers |
| S5/S9 | DM-1/DM-2 integer, fail-closed, event-sourced money |

**Boundary:** this blueprint writes no code, recovers no fixture data, publishes no copy, sends
no message, stops no deployment, and adds no crate. It is the plan those passes will follow.
