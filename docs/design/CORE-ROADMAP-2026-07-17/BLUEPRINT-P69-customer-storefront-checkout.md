# BLUEPRINT P69 — Customer storefront & checkout journey (`/s/:slug`, the M1 critical path) (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Component:
> **DELIVERY / customer surface**. Wave **W2** (assembly) of the launch-blocker build sequence
> (`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5, row **P69**; build order §3.1). Structural
> template + rigor precedent: `BLUEPRINT-P51-open-map-routing.md`; the four W1 contracts this
> blueprint **consumes verbatim** (cited, never redefined): **P57** (canvas text input), **P58**
> (a11y-mirror-everywhere), **P60** (payment adapter core), **P62** (catalog & multi-vendor data
> model), plus **P64** (intent engine / friction / voice) and **P63** (shell & floor-parity spike).
>
> **This is the M1 (first real order) critical-path blueprint — treated with the highest rigor in
> the program.** M1 is defined (SYNTHESIS §3): *a real customer pays real money for a real pickup
> order on one hand-onboarded, dowiz-operated hub, through the full-wgpu intent UI, with status
> notifications delivered.* P69 is the surface that makes M1 reachable; its DoD (§6) carries the
> **end-to-end falsifiable M1 test** — a real order placed and paid through this exact flow.
>
> **Binding scope is fixed by SYNTHESIS §5's P69 row and its cross-cutting resolutions** — nothing
> here re-litigates a closed decision. Full-wgpu UI (§16.30), intent-only navigation (§16.35/
> §16.40), the narrative-cinematic checkout arc (§16.37), the Path-C hosted-redirect card moment
> (§4-A, CLOSED), honest hub-offline status (§16.14), the X7 catalog leaf invariant, the X6
> idempotency contract, and schema.org-JSON-LD-as-load-bearing-AEO (R1 §7) all stand exactly as
> ruled. This document makes them one buildable customer journey.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**the storefront/checkout surface is entirely greenfield, but every hard part it needs already
exists as a W1 contract or landed kernel code** — P69 is *assembly and choreography*, not new
mechanism. It renders through the existing bit-deterministic field oracle, submits over P37's
order route, reads P62's catalog, hands the card moment to P60, and imports P58's a11y gate and
P63's floor-parity gate. P69 invents **one** genuinely new thing: the **suspend/resume journey
state machine** that survives the Path-C redirect out of the canvas and back (§4.5).

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| **Zero storefront/checkout code anywhere.** grep `storefront\|/s/:slug\|:slug\|robots\.txt\|sitemap\.xml\|json-?ld\|open.?graph\|manifest\.json\|llms\.txt\|checkout` over `kernel/ engine/ wasm/ web/ apps/` (excl. tests) → **0 hits** | repo-wide grep this pass | **VERIFIED — P69 is greenfield; only the mechanisms it composes are landed** |
| Web app tree is a console-only kernel driver + a FieldSim render path; **no storefront route, no a11y mirror, no keyboard handling, no `web/tests/`** | `web/src/{app.mjs,pages/fieldsim.astro,render/fieldsim.smoke.mjs,components/FieldSim.svelte,lib/kernel/{contracts,kernel_client}.mjs,lib/fieldsim/{shader,buffer}.mjs}`; `ls web/tests` → absent | VERIFIED — P69 adds `web/src/storefront/` + `pages/storefront.astro`, **extends** `app.mjs`, does NOT rewrite it |
| `app.mjs` binds the kernel `_js` exports console-only; its header defers the DOM/FieldSim pass to a "separate work unit" | `web/src/app.mjs:1-12` (header), `:14-40` (24 binds) | VERIFIED — the journey rides this beachhead + P58's mirror + P57's keyboard source |
| **Bit-deterministic field oracle** `compose(scene, eq, w, h, steps) -> Vec<u8>`, asserted identical across calls — every storefront screen renders THROUGH this frame | `engine/src/field_frame.rs:255` (`compose`), `:430` (`compose_returns_deterministic_frame`), `:447-450` (bit-equality assert) | **VERIFIED — the storefront journey is a sequence of `compose()` frames, never bespoke render** |
| **CPU floor is a live wasm export** `compose_field(circles,w,h,steps) -> Vec<u8>` (+ `frame`, `vertex_field`) — the terminal rung of the FE-16 ladder the storefront must render on | `wasm/src/lib.rs:57` (`compose_field`), `:96` (`frame`), `:112` (`vertex_field`) | VERIFIED — P69's floor-parity DoD (§4.8) protects the ~18% without WebGPU |
| Scene + SDF primitives (menu cards, cart rows, step chrome are Scene geometry) | `engine/src/scene.rs:29-44` (`SdfShape` variants), `:71` (`Scene`), `:88,168` (`add`/`render_to_bridge`) | VERIFIED — journey screens are Scene compositions; glyph quads ride FE-06 (P57/P38) |
| **Money-never-tween guard LANDED and binding (🔴 RED-LINE)**: `Money` implements NO `FieldValue`; a monetary value is presented via `TweenGuard::present_money` (integer→integer), never interpolated | `engine/src/money_guard.rs:18` (`Money`), `:22-25` (`FieldValue` deliberately not for `Money`), `:60` (`present_money`), `:72` (`jump`) | **VERIFIED — every price P69 shows is TEXT from `present_money`; a count-up/animated cart total does not compile (§5.1)** |
| Kernel money authority `Money { minor: i64, currency: Currency }`; `Currency::Eur` present; cross-currency + overflow fail-closed | `kernel/src/money.rs:58-62`, `:29` (`Currency`), `:33` (`Eur`), `:71-121` (`checked_*`) | VERIFIED — the storefront never re-derives money; it displays P62's resolved `Money` |
| Integer minor→decimal price string, **no float**, `€0.01` never `€0.00` | `kernel/src/cart.rs:134-152` (`format_money`) | VERIFIED — the price string the JSON-LD `Offer.price` and the on-canvas price label both reuse |
| Order aggregate + trusted-price path: `place_order` and `place_order_priced` (re-derives every line's `unit_price` from the trusted `PriceCatalog`, ignoring the caller value) | `kernel/src/domain.rs:156` (`place_order`), `:198-240` (`place_order_priced`), flag `:55-59` | **VERIFIED — P69 submits a cart; the kernel re-prices it; a client cannot forge line prices (§5.1 security property)** |
| Order status FSM `OrderStatus` = Pending/Confirmed/Preparing/Ready/…/PickedUp/… + P07 compensation states; `fold_transitions` | `kernel/src/order_machine.rs:8-21`, `:156` | VERIFIED — the narrative pacing arc (§4.1) keys off these states; P69 mutates none of them |
| **P37 order HTTP surface is the submit rail:** dynamic order routes (place / advance / read) merged into `native-spa-server`'s router, one binary serving static `web/` AND the API; JSON authority = `place_order_logic`/`apply_event_logic` extracted to a feature-independent module | `BLUEPRINT-P37-order-http-surface.md:23-29` (ground), `:50-51,61-62` (W37-1/W37-2 routes) | **VERIFIED — P69 POSTs the order to P37's route; the hub reachability probe (§4.6) hits this same surface** |
| **W1 contracts P69 consumes exist on paper, in this same directory, written this wave** (SYNTHESIS §5 write-order: W1 contracts land first, W2 cites by number) | `BLUEPRINT-P57-canvas-text-input.md`, `BLUEPRINT-P58-a11y-mirror-everywhere.md`, `BLUEPRINT-P60-payment-adapter-core.md`, `BLUEPRINT-P62-catalog-multivendor-data-model.md`, `BLUEPRINT-P63-shell-platform-spike.md`, `BLUEPRINT-P64-intent-engine-friction-voice.md` (all present, `ls` this pass) | **VERIFIED — P69 cites their named types/sections; it does not redefine one** |
| **P66/P65 not on disk yet** (W2 siblings, written in parallel) — cited by their SYNTHESIS-defined contract, exactly as P57/P60 cite them | `ls …/BLUEPRINT-P6{5,6}*` → absent; contracts in `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md:441` (P66), `:440` (P65) | VERIFIED — P66's draft/idempotency and P65's dispatch are named-not-forked; M1 is pickup (no P65 on the M1 path) |
| P38 §12.1 canon-diff **names P69** as importing FE-15 (mirror base) + FE-16 (fallback ladder) + the tightened no-`<input>` grep gate; §12.2 pins the AR/VR view-matrix / `FieldPos`-3D / `InputSource` constraints P69 inherits | `BLUEPRINT-P38-webgpu-render-engine.md:744-750` (§12.3 names P57/P58/P69/P70/P71/P73), `:694-737` (§12.2) | VERIFIED — P69 carries the FE-16 floor line + routes all input through `InputSource` |
| P38's **narrative-cinematic order-lifecycle pacing arc** with named beats + amplitude budget lives in `BLUEPRINTS-DOWIZ-INTERFACES.md` Додаток C.2; §16.37 rules the checkout wizard **is this arc applied to the order flow, not a new design** | `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:427-450` (C.2 beats table), `:466-474` (C.4 camera language); `MASTER-ROADMAP-…-2026-07-16.md:2197-2204` (§16.37) | **VERIFIED — P69 cites the C.2 beats VERBATIM (§4.1); it invents no pacing model** |
| §16.14 honest hub-offline: **client holds zero server-side order state**; when a hub is unreachable, show an honest "hub offline" status — no disguised retry, no central queue; resilience lives on the venue hub or the customer device, never a dowiz server | `MASTER-ROADMAP-…-2026-07-16.md:1971-1983` (§16.14) | **VERIFIED — §4.6 is this ruling made a typed `HubStatus`; no central fallback exists to build** |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Design verdicts — the load-bearing decisions, argued not asserted

### 1.1 The checkout journey IS P38's existing order-lifecycle arc — cite the beats, invent no pacing (§16.37 ↔ Додаток C.2)

§16.37 is explicit and CLOSED: the multi-step wizard (menu → cart → delivery/pickup → payment)
is *"невелика пригода"* (a small adventure) and is **the already-named narrative-cinematic
order-lifecycle arc applied concretely to the order flow — not a separate new design.** The arc
is defined, with named beats and a falsifiable amplitude budget, in `BLUEPRINTS-DOWIZ-INTERFACES.md`
**Додаток C.2** (`:435-450`). P69 does **not** author a pacing model; it maps its journey steps
onto those existing beats and their existing instruments (the ζ-presets `motion.rs:66-76`, the
FE-14 settle gate as the held-stillness instrument, spectral attending speed as tempo, the energy
scalar as loudness). The load-bearing consequence: **the checkout culminates in the arc's first
beat.** The C.2 beats, cited verbatim (§4.1 builds on this table, it does not restate it as new):

| C.2 beat | Order event | Field VOCAB (existing) | Tempo (naming only, no new mechanic) |
|---|---|---|---|
| **Establishing shot** (C.4) | storefront arrive | full-bleed Море, hero = deepest well (3-second hierarchy) | Act 1 arrive — this is DZ-01 named as a cinematic term |
| — *(checkout build-up)* | menu browse → cart → fulfillment → pay | Act 2 Menu Sheet re-diffuse; cart ripple; Sheet progressive disclosure | the wizard steps ride existing dive/sheet-rise montage grammar (C.4), no new "cut" |
| **Inciting** | order placed / paid | amber burst 1.4 | SNAP instantly (invariant B-7: consequence ≤ `--motion-instant`), then a **held beat**: Море settles ~600-900 ms with NO new impulses — the weight of the decision is the silence AFTER, not a delay of the confirmation |
| **First answer** | confirmed | ember drift 0.3 | one low FLUID wave — quiet reassurance, not fanfare |
| **Long middle** | preparing / ready | ember drift 0.3 | quietest segment: TIDE ζ>1 ambient, settle gate active (0 rAF wake-ups) |
| **Climax** | delivered / picked up | gold bloom burst 1.8 | the single amplitude maximum, earned by prior restraint |
| **Tragedy (honest)** | rejected / cancelled | blood turbulence 3.4 | no softening AND no shaming; then a 300-600 ms recovery beat of silence before the next CTA |

**Amplitude budget (falsifiable, from C.2):** on the success branch no intermediate beat is
louder than the climax — max amplitude per state accumulates monotonically. P69's checkout is the
build-up whose payoff is the **Inciting** beat (order placed = amber burst + held beat). This is
why "place order" must SNAP instantly (never spin a fake progress bar) and then fall into
measured stillness — the pacing is dowiz-fixed T2, **no brand pacing token exists** (the 5-token
Sheet limit of DZ-02 is unchanged; a sixth "drama/pacing" token is a gate violation, C.1).

### 1.2 The card moment is Path C — a redirect out of the canvas and back, and that handoff is the real work (§4-A CLOSED)

§4-A is CLOSED (SYNTHESIS §0.2 / task): **Path C hosted redirect** on web + desktop (Path B native
SDK sheet on Tauri mobile, pending P63). The wgpu canvas **never** renders a card field — enforced
structurally by P60's no-card-data firewall (there is no card type in hub-core to bind, P60 §4.1).
The consequence P69 must own honestly: at the card moment the immersive canvas experience is
**left** — the app hands off to the provider's own verified domain (system browser on desktop,
kept pure `winit`+`wgpu` per X3; navigation on web; native sheet on mobile) — and then must
**detect the return and resume the canvas journey** without ever letting the client self-certify
payment. This is a genuine UX/state design problem (SYNTHESIS §4-A names it "a real design
problem"), not a redirect one-liner. P69's answer is a **suspend/resume journey state machine**
(§4.5): the journey suspends into a typed `Suspended` step, the draft + idempotency key persist
via P66 (query-before-replay authority, X6), return is detected by a **deep-link callback** *or*
by **polling P60's `query_status_by_key`** (the honest fallback when no deep-link arrives), and the
journey resumes into the **Inciting** beat only when the **webhook** (P60 §4.4, the sole truth
writer) has moved the kernel fold to `Captured`. A client redirect writes nothing.

### 1.3 Prices display because of the X7 leaf invariant, not a P69 price model (§16.17 ↔ X7)

P69 renders **no price of its own.** Every price on a menu card is a `PriceableLeaf.price`
(a `Money` carrying minor-units + `Currency` + `vendor_id`) resolved by P62's `resolve_line`, shown
as text via `present_money`/`format_money`. The X7 leaf invariant — "every purchasable leaf carries
a resolvable price (integer minor units), currency, and `vendor_id`" (SYNTHESIS X7; P62 owns it as
a type) — is *how prices actually display*: P69 cannot show an unpriced item because P62 cannot
construct one (`NodeBody::Leaf(PriceableLeaf)` is the only purchasable variant, P62 §1.2). The cart
total is `Cart::price` over the same leaves; the per-vendor split preview is `charge_legs` (P62 §3);
the bot-facing `Offer.price` is `menu_jsonld`'s `price_to_decimal_string` (P62 M5). One menu truth,
four consumers — P69 is a consumer of all four, an author of none.

### 1.4 The bot-facing pack is a projection of kernel state, so it cannot drift (§16.55 ↔ R1 §7)

R1 §7 is decisive and CLOSED as a sharpening of §16.55: **schema.org JSON-LD is the load-bearing
AEO substrate** (Restaurant/Menu/MenuItem/Offer — what Google/Bing rich results *and* the LLM
answer engines actually parse for food/menu facts), plus universally-consumed `robots.txt` +
`sitemap.xml` + Open Graph + `manifest.json`; **`llms.txt` is a forward-looking extra, JSON-LD
first, not a crawlability bet** (one 90-day study found only 408 AI-bot hits on `llms.txt`; Google
publicly declined it). The design verdict: the whole pack is **generated FROM kernel/catalog state
at publish time** (reusing P62's `menu_jsonld`/`price_to_decimal_string`), never hand-authored — so
it *cannot* drift from the real menu. It is a **separate output path** from the a11y mirror
(§16.55 insists on this; the mirror is SR-only DOM, the pack is static files), and it is
**build-time static** (like P51's MapPack, P62 §5.3 — no business in the mesh SyncFrame path).
The single most important design decision here: the pack is a pure function of catalog state, so
"the JSON-LD says €7.00 but the menu shows €8.00" is unrepresentable, not policed (§5.1).

### 1.5 Reuse-first: assemble, do NOT build new mechanism (standard item 19)

P69 writes almost no new *mechanism*. Every hard part is imported:

| Need | Owner (cited, not forked) | The exact contract P69 consumes |
|---|---|---|
| Typed field entry (name/address/search/notes) | **P57** | `TextField::{new,apply,value,set_value,caret_rect,glyph_runs}`, `EditCmd`, `Intent::Text(EditCmd)`, `ClipboardPort`, `in_wave0_scope` |
| a11y for every screen | **P58** | `SemanticScene`/`SemanticNode`/`Role`/`EditState`, `mirror(&SemanticScene)->A11yTree`, `a11yGate(page, manifest)`, the ARIA-textbox convention, `MIRROR_NODE_BUDGET_DEFAULT` |
| Card moment + idempotency + status query | **P60** | `PaymentProvider::{create_with_key,query_status_by_key}`, `ClientHandoff::{HostedRedirect,NativeSdkSession}`, `PaymentStatus`, `IdempotencyKey`, `NLegPlan`, `CLIENT_SESSION_TTL_S` |
| Menu data + prices + charge legs + JSON-LD | **P62** | `PriceableLeaf`, `CatalogNode`/`NodeBody`, `Cart`, `charge_legs`, `kitchen_tickets`, `menu_jsonld`, `price_to_decimal_string` |
| Intent navigation + friction gate + voice | **P64** | `Intent`, `InputRouter::tick`, `IntentClassifier`, `Composer::compose->ComposedResponse`, `FrictionFsm`/`CommitToken`, `VoiceSource: InputSource` |
| WebGL2/CPU floor parity | **P63** | SP-6 `floor-parity` gate; DoD one-liner "passes `floor-parity` at ΔE ≤ 0.02 on WebGPU, WebGL2, and CPU rungs" |
| Draft persistence + reconnect | **P66** | `Draft`/`PaymentInflight` states, query-before-replay, idempotency key minted at draft creation (X6) |
| Order submit + reachability | **P37** | the order route (place/advance/read) on `native-spa-server`; `place_order_priced` re-prices |
| Status notifications | **P61** | the `Notifier` fan-out (status updates are the M1 mandatory leg) |

What is genuinely **new** in P69 and nowhere else: (a) the **journey step state machine** and its
suspend/resume across the Path-C redirect (§4.5); (b) the **honest hub-offline status** as a typed
journey state (§4.6); (c) the **bot-pack file emission** wrapping P62's JSON-LD in a `Restaurant`
node + robots/sitemap/OG/manifest/llms (§4.7). Everything else is choreography of the above.

Rejected alternatives (DECART one-liners): **an in-canvas card field / iframe overlay** — rejected:
Path A is not the ruling; the no-card firewall means there is no card type to draw (P60 §4.1), and
an overlay is the struck `<input>` (P38 §12.1). **Trusting the provider "success" redirect to place
the order** — rejected: the webhook is the sole truth writer (P60 §4.4); a client redirect only
triggers a `query_status_by_key` re-check. **A central dowiz retry/queue when the hub is offline** —
rejected: §16.14 forbids any central order state; honest status + on-device draft only. **A
hand-authored `sitemap.xml`/JSON-LD** — rejected: it drifts from the menu the moment a price
changes; the pack is a projection of catalog state (§1.4). **A P69-local price/format** — rejected:
forks the money authority; `present_money`/`format_money`/`menu_jsonld` are the single sources
(§1.3). **A count-up / animated cart total** — rejected: `Money` implements no `FieldValue`, so it
does not compile (§5.1); the total SNAPS via `present_money`.

---

## 2. Scope — what P69 owns vs deliberately does NOT

### 2.1 P69 owns (build items §4)

| Item | Content |
|---|---|
| M1 | **The `/s/:slug` journey shell + step state machine** (`web/src/storefront/journey.mjs` + `pages/storefront.astro`): the full-wgpu multi-step wizard as the P38 Sea arc (Act 1 establishing → Act 2 menu → cart → fulfillment → payment → Inciting beat), intent-driven navigation (P64), no button/menu fallback (§16.40) |
| M2 | **Menu rendering from P62's catalog** (`storefront/menu.mjs`): render the `CatalogNode` tree + `PriceableLeaf` prices as `present_money`/`format_money` text (money never tweens); Act-2 diffusion; item-detail with modifier groups resolving through `resolve_line` |
| M3 | **Cart + delivery-or-pickup selection** (`storefront/cart.mjs`, `storefront/fulfillment.mjs`): unified cross-vendor cart via P62's `Cart`; `charge_legs` split preview; the `FulfillmentChoice` selector (Pickup is the wired M1 path; Delivery selectable, its dispatch is downstream) |
| M4 | **Typed field entry via P57** (`storefront/fields.mjs`): name, pickup contact, delivery address, search, notes — each a P57 `TextField` (Latin+Cyrillic Wave-0) with P58 ARIA-textbox a11y; a numeric tip parsed to `i64` minor units at the submit boundary, never a tweened money surface |
| M5 | **The Path-C card moment: handoff + resume** (`storefront/payment.mjs`): consume P60's `ClientHandoff::HostedRedirect`; suspend the journey; hand off (system browser/nav/native sheet); detect return (deep-link **or** poll `query_status_by_key`); webhook is sole truth; resume into the Inciting beat |
| M6 | **Honest hub-offline status** (`storefront/hub_status.mjs`): a reachability probe against P37's route; a typed `HubStatus::Offline` status node (a11y `Status`/`Alert`); no fake retry, no central fallback; the draft (P66) holds on device |
| M7 | **The bot-facing static pack** (`kernel/src/json_api.rs` projection + `storefront/bot_pack.mjs` emission): `robots.txt` + `sitemap.xml` + schema.org JSON-LD (`Restaurant`/`Menu`/`MenuItem`/`Offer`, wrapping P62's `menu_jsonld`) + Open Graph + `manifest.json` + `llms.txt` (secondary) — generated FROM catalog state, never hand-authored |
| M8 | **The per-screen a11y + floor-parity integration** (§M7 of P58 + SP-6 of P63): author a `SemanticScene` per journey step, import `a11yGate(page, manifest)` with a per-screen mirror-node budget, and carry the `floor-parity` DoD line on every screen |

### 2.2 P69 explicitly does NOT own

- **NOT the text-input engine.** Cursor/selection/clipboard/shaping/scope-gate are **P57**. P69
  *instantiates* `TextField`s and reads `value()` at submit; it re-implements no editing logic. A
  diff that hand-rolls a caret or a keydown handler outside P57's `KeyboardSource`/`Intent::Text`
  path is a scope violation (P38 §12.2 constraint 3).
- **NOT the a11y-mirror convention, its harness, or the ARIA-textbox spec** — **P58** owns them
  (SYNTHESIS single-owner). P69 authors a `SemanticScene` and *imports* `a11yGate`; it writes no
  bespoke a11y test (that is the X1 re-derivation P58 §M7 forbids).
- **NOT any card-data type / PAN / card rendering, on any platform** — hard PCI red-line (P60 §4.1).
  P69 consumes `ClientHandoff` (opaque handles/URLs only); the canvas never renders a card field
  because **there is no card type to bind**. A diff introducing a `card_*`/`pan`/`cvv` field
  anywhere is a scope violation regardless of test state.
- **NOT the idempotency contract** — **P60** owns `create_with_key`/`query_status_by_key`/
  `IdemLedger` (X6); P69 *calls* them. The key is minted by **P66** at draft creation; P69 passes
  it, never derives it.
- **NOT the draft / data-wallet persistence** — **P66** owns `Draft`/`PaymentInflight` +
  query-before-replay + autofill. P69 hands P66 a field snapshot to persist and asks P66 to restore
  on resume; P69 stores nothing across sessions (§5.3).
- **NOT the catalog data model, the leaf invariant, or the JSON-LD *string*** — **P62** owns them.
  P69 renders the tree and *writes the file* (`menu_jsonld` supplies the Menu/Offer string; P69
  wraps it in a `Restaurant` node and emits the pack, per P62 §2's explicit "P62 supplies the
  STRING, P69 writes the FILE").
- **NOT payment execution, N-leg atomicity, refunds, or the provider matrix** — **P60** (mechanism)
  and **P72** (food-court UX + cross-account matrix). M1 is single-vendor pickup; the N-leg saga is
  invoked through P60's port, never re-specified. Food-court multi-vendor checkout UX is **P72**.
- **NOT courier dispatch, the map, or delivery fulfillment** — **P65** (dispatch), **P51** (map),
  **P71** (courier surface). M1 is **pickup** (SYNTHESIS §3: pickup removes the courier surface from
  the first order's critical path without descoping). P69 offers the Delivery selector, but its
  courier/dispatch/live-map fulfillment is downstream; the M1 falsifiable test is a pickup order.
- **NOT the owner/menu-editor surface** — **P70** authors the `CatalogNode` tree P69 renders.
- **NOT the notification fabric** — **P61** owns the `Notifier`. P69 emits order events (via P37);
  P61 fans out the status updates (the M1 mandatory notification leg).
- **NOT the intent grammar, friction numeric mapping, or voice runtime** — **P64** owns
  `Intent`/`InputSource`/`FrictionFsm`/`VoiceSource`. P69 *drives* `InputRouter::tick` and gates its
  money action with a P64 `CommitToken`; it forks no enum and defines no friction constant.
- **NOT the shell decision or soft-keyboard mechanism** — **P63/P39-rev**. P69 consumes the shell
  verdict (system-browser handoff on desktop per X3) and the floor-parity gate; it decides neither.
- **NOT the `dowiz.org` landing page** — **P73** (its own storefront/bot-pack instance).

### 2.3 Build reality — Lane A (buildable now) vs Lane B (gated) — mirrors the W1 lane structure

- **Lane A (buildable TODAY, zero network unlock):** M1's journey step machine (pure JS/state), M3's
  cart wiring over P62's pure-`std` `Cart`, M6's hub-status typed states + reachability probe, M7's
  bot-pack projection (`kernel/src/json_api.rs` under `#[cfg(feature="json-api")]`, pure, testable)
  + the file emitter, M8's `SemanticScene` authoring + `a11yGate` import (P58's web path is
  network-free), and every RED test (Playwright headless against the CPU floor + Stripe **test
  mode**).
- **Lane B (gated — do NOT proceed without the upstream unlock/verdict; check the gate record):**
  M2's glyph render rides FE-06 MSDF (**O18a graphics unlock**, shared with P38/P57); M4's live
  editing rides P57's cosmic-text (**O18a**); M5's real Path-C redirect on desktop rides P63's
  **shell verdict** (system-browser handoff) and P60's Stripe adapter (out-of-kernel crate); native
  AccessKit rides P58's **AK-unlock**. Each Lane-B test carries an `#[ignore = "…"]`/headless marker
  that doubles as the gate marker (the P38/P57 convention).

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ════════════════════════════════════════════════════════════════════════════
//  P69 is a CONSUMER surface. The types below are the JOURNEY state machine and
//  the BOT-PACK projection — the two genuinely new things (§1.5). Everything a
//  price/edit/pay/menu touches is a CITED type from P57/P58/P60/P62/P64, NEVER
//  redefined here. Cited types are shown as `use` lines so the boundary is explicit.
// ════════════════════════════════════════════════════════════════════════════

// ── cited, never redefined (single-owner contracts) ─────────────────────────
use crate::money::Money;                                   // P62/kernel money.rs:58 (i64 minor + Currency)
use crate::catalog::{PriceableLeaf, CatalogNode, NodeBody}; // P62 (X7 leaf invariant)
use crate::cart::Cart;                                      // P62/kernel cart.rs (unified cross-vendor cart)
// P57:  TextField, EditCmd, EditEvent, ClipboardPort, in_wave0_scope, Intent::Text(EditCmd)
// P58:  SemanticScene, SemanticNode, Role, NodeState, EditState, mirror(), a11yGate(), MIRROR_NODE_BUDGET_DEFAULT
// P60:  PaymentProvider::{create_with_key, query_status_by_key}, ClientHandoff, PaymentStatus,
//       IdempotencyKey, NLegPlan, CLIENT_SESSION_TTL_S (= 900)
// P62:  charge_legs(order)->Vec<ChargeLeg>, menu_jsonld(vendor, leaves)->String, price_to_decimal_string
// P64:  Intent, InputRouter::tick, Composer::compose->ComposedResponse, FrictionFsm, CommitToken, VoiceSource
// P66:  Draft / PaymentInflight, query-before-replay, idempotency key minted at draft creation (X6)

// ── web/src/storefront/journey.mjs (mirrored spec — the JS state machine) ────
/// The storefront slug — a hub's public handle in `/s/:slug`. Free-form vendor-authored
/// text (no dowiz taxonomy), resolved to ONE `location_id` by P37's route.
pub struct StorefrontSlug(pub String);

/// A journey step. This is the multi-step wizard of §16.37, mapped onto the P38 Sea arc
/// (§1.1). Ordered; navigation is intent-driven (§16.40), NOT a fixed button flow — but
/// the ORDER of the arc is fixed (menu precedes cart precedes pay). `Suspended` is the
/// Path-C redirect state (§4.5); `Placed` is the C.2 **Inciting** beat.
pub enum JourneyStep {
    Storefront,                         // Act 1 establishing shot (C.4): full-bleed Море, hero well
    Menu,                               // Act 2 browse (P62 catalog render)
    Detail { leaf: LeafRef },           // item detail + modifier groups (resolve_line)
    Cart,                               // unified cross-vendor cart (P62 Cart) + charge_legs preview
    Fulfillment { choice: FulfillmentChoice }, // delivery-or-pickup selection
    Payment,                            // hand-off decision point (Path C / Path B)
    Suspended(SuspendState),            // left the canvas for the provider domain (§4.5)
    Placed { status: PaymentStatus },   // C.2 Inciting beat (amber burst 1.4 + held beat) once Captured
    OfflineHalt,                        // §16.14 honest hub-offline (§4.6) — a TERMINAL-until-online state
}

/// Wave-0 fulfillment axis. Pickup is the M1-wired path (no courier); Delivery is a real
/// selectable option whose dispatch/map is P65/P51/P71 (downstream, off the M1 critical path).
pub enum FulfillmentChoice { Pickup, Delivery }

/// The suspend/resume record that survives the Path-C round-trip out of the canvas (§4.5).
/// Persisted via P66 (query-before-replay) so an app-kill during redirect is recoverable.
pub struct SuspendState {
    pub key: IdempotencyKey,            // minted by P66 at draft creation (X6) — the reconnect anchor
    pub session_token: [u8; 32],        // P60 ClientHandoff single-use token
    pub ttl_deadline_unix_s: i64,       // now + CLIENT_SESSION_TTL_S; a return past this is refused (§4.5)
    pub await_since_unix_s: i64,        // when the handoff began (poll-timeout base)
    pub resume_step: Box<JourneyStep>,  // where to fall back to on Failed (Payment) or on OfflineHalt
}

/// How a return from the provider domain is detected (§4.5). BOTH may occur; neither writes
/// success — each only triggers a P60 `query_status_by_key(key)` re-check (webhook is truth, §4.4/P60).
pub enum ReturnSignal {
    DeepLink { session_token: [u8; 32] }, // dowiz://return?session=… (installed) or /s/:slug/return?session=… (web)
    Poll,                                 // the honest fallback: no deep-link arrived → poll query_status_by_key
}

/// The whole-journey machine. `advance` is intent-driven (P64); `suspend`/`resume` bracket the
/// Path-C redirect. Pure state — it OWNS no money, no card, no order truth (those live in the kernel).
pub struct Journey { /* step: JourneyStep + cart: Cart + slug: StorefrontSlug + hub: HubStatus */ }
impl Journey {
    pub fn advance(&mut self, intent: &Intent) -> JourneyStep;               // menu→cart→fulfillment→payment
    /// At Payment: call P60 create_with_key, persist via P66, transition to Suspended, and return
    /// the ClientHandoff the shell opens. NEVER captures — the handoff is opaque (§4.5).
    pub fn suspend(&mut self, handoff: &ClientHandoff, s: SuspendState) -> &ClientHandoff;
    /// On ReturnSignal: query P60 status by key; transition to Placed{Captured}, back to
    /// Payment on Failed, stay "confirming" on Authorized-not-yet-Captured, or OfflineHalt if
    /// the hub is unreachable. The client redirect NEVER writes Captured (§4.4).
    pub fn resume(&mut self, signal: ReturnSignal, status: PaymentStatus, hub: HubStatus) -> JourneyStep;
}

/// Honest hub reachability (§16.14). `Offline` renders an honest status node; there is NO
/// central-dowiz fallback variant (that state is unrepresentable — §5.1). Degrade-closed:
/// an ambiguous probe result is treated as Offline, never optimistically Online.
pub enum HubStatus { Online, Offline }
pub const HUB_PROBE_TIMEOUT_MS: u32 = 4000;        // reachability probe deadline (degrade-closed on timeout)
pub const RESUME_POLL_INTERVAL_MS: u32 = 2000;     // query_status_by_key poll cadence after a poll-return
pub const RESUME_POLL_DEADLINE_S: i64 = 900;       // ceiling == CLIENT_SESSION_TTL_S; past this → Payment retry

// ── kernel/src/json_api.rs (feature = "json-api") — the bot-pack PROJECTION ──
//  Generated FROM catalog state (reuses P62's menu_jsonld) so it CANNOT drift (§1.4).
//  A SEPARATE output path from the a11y mirror (§16.55). The web side WRITES the files.

/// schema.org `Restaurant` node wrapping P62's `Menu`/`MenuItem`/`Offer` string — the
/// load-bearing AEO substrate (R1 §7). `hasMenu` embeds `menu_jsonld(vendor, leaves)` verbatim;
/// name/address/openingHours are hub-authored (P62's free-form, opaque to dowiz).
pub fn restaurant_jsonld(slug: &str, hub_name: &str, addr: &PostalAddressFacts,
                         menu: &str /* == menu_jsonld output, P62 M5 */) -> String;
/// robots.txt / sitemap.xml — pure functions of the published slug set. Universally consumed.
pub fn robots_txt(sitemap_url: &str) -> String;
pub fn sitemap_xml(storefront_urls: &[String]) -> String;
/// Open Graph + manifest.json — link unfurl + installability facts, from catalog/hub state.
pub fn open_graph_tags(slug: &str, hub_name: &str, hero_desc: &str) -> String;
pub fn web_manifest(slug: &str, hub_name: &str) -> String;
/// llms.txt — the SECONDARY, forward-looking feed (R1 §7: JSON-LD first, NOT a crawlability bet).
pub fn llms_txt(slug: &str, hub_name: &str, menu_summary: &str) -> String;

/// PostalAddress facts — hub-authored, opaque strings (dowiz imposes no schema, §16.17).
pub struct PostalAddressFacts { pub street: String, pub locality: String,
                                pub region: String, pub postal_code: String, pub country: String }

/// The whole pack as one artifact — emitted by the web side to static files at publish time.
pub struct BotPack {
    pub robots_txt: String, pub sitemap_xml: String, pub restaurant_jsonld: String,
    pub open_graph: String, pub web_manifest: String, pub llms_txt: String,
}
pub fn build_bot_pack(slug: &str, hub_name: &str, addr: &PostalAddressFacts,
                      leaves_by_vendor: &[(/*VendorId*/ u64, Vec<PriceableLeaf>)]) -> BotPack;
```

Rejected alternatives (DECART one-liners): **a `JourneyStep::Confirmed`/`Delivered`/… mirroring the
whole order FSM** — rejected: those are `OrderStatus` (`order_machine.rs`), owned by the kernel; the
journey ends at `Placed` and *observes* the lifecycle arc, it does not fork the FSM. **A boolean
`paid: bool` on the journey** — rejected: payment truth is `PaymentStatus` from the webhook via P60;
a client-side bool would be the forged-success the ruling forbids. **A `HubStatus::CentralFallback`**
— rejected: §16.14 makes any central order state unrepresentable — there is no such variant to build.
**Hand-authored bot-pack strings** — rejected: they drift; the pack is a projection (§1.4).
**A P69 `format_price` helper** — rejected: `price_to_decimal_string`/`present_money` are the single
authorities (§1.3); a second is a money red-line fork.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

Dependency order: M1 → M2 → M3 → M4 → M6 → M7 → M8, with **M5 (the card handoff) last among the
journey items** because it depends on M4 (fields), M6 (offline status), and P60/P66/P63 verdicts;
the **end-to-end M1 test (§6)** is the capstone that exercises all of them.

### 4.1 M1 — the `/s/:slug` journey shell + step state machine (the Sea arc, §16.37)

`web/src/storefront/journey.mjs` + `web/src/pages/storefront.astro` (mirrors `pages/fieldsim.astro`),
extending `app.mjs` (never rewriting it — P38 §10 T4 charter). The `Journey` state machine (§3)
drives the wizard; **navigation is intent-driven** — `InputRouter::tick` (P64) yields `Intent`s, and
`Composer::compose(intent, state) -> ComposedResponse` (P64) selects the pre-built UI fragment for
the next step (§16.40: full replacement, no button/menu fallback). The **arc mapping is fixed**
(§1.1): `Storefront` = Act-1 establishing shot (full-bleed Море, hero = deepest well, C.4); the
wizard steps use the existing dive / sheet-rise montage grammar (C.4, no new "cut"); a paid order
transitions to `Placed` = the C.2 **Inciting** beat (amber burst 1.4 SNAP + a held beat of
~600-900 ms with no new field impulses). Every input routes through an `InputSource` (P38 §12.2
constraint 3); every scene position is a 3D `FieldPos` (constraint 2).

RED→GREEN (Lane A, pure state + Playwright headless on the CPU floor): `journey_advances_arc_order`
— `advance` walks Storefront→Menu→Cart→Fulfillment→Payment in order under a scripted intent
sequence; `placed_is_inciting_beat` — on a `Captured` status the step becomes `Placed` and the
composed response carries an `amber burst 1.4` field delta followed by a **held beat** (0 new source
impulses for N ms, asserted against the event-log the same way C.2's amplitude budget is checked).
**Adversarial (designed to break):** an intent that would skip Cart→Payment directly ⇒ refused (the
arc order is an invariant, not a suggestion — you cannot pay for an empty/unfulfilled cart); a
"place order" intent while the cart is empty ⇒ no transition, an honest `Alert` node, never a
zero-total order; the held beat must be **present and silent** — a test that injects a stray field
impulse during the held window fails the amplitude-budget assertion (the climax-earned-by-silence
invariant, C.2); an input bound to a raw `mousedown`/`touchstart` outside an `InputSource` adapter
⇒ the P38 §12.2 grep gate fires.

### 4.2 M2 — menu rendering from P62's catalog (prices as text, never a tween)

`web/src/storefront/menu.mjs`: render the vendor's `CatalogNode` tree (P62) — free-form categories
(no dowiz taxonomy, §16.17) as Act-2 Sheet groups over the Море, each `PriceableLeaf` as a menu card
whose **price is `present_money(leaf.price)` text** (money never interpolates — `money_guard.rs:22`).
The Detail step resolves modifier groups through P62's `resolve_line(base, components)` (absolute
overrides base, delta adds) and shows the live line total as `present_money` (SNAP, no count-up).
Item media follows P38's lazy/gated load discipline. Category select re-diffuses the Sheet (existing
SPREAD grammar); a closed hub renders the Море calm-dark (existing open-hours state, C.3).

RED→GREEN (Lane B glyph render is O18a; Lane A logic now): `menu_price_is_present_money_text` — a
leaf priced `Money::new(1250, Eur)` renders the exact string `"12.50 €"`-class label via
`present_money`, and a compile-proof that a `Spring<Money>`/`interpolate(leaf.price, …)` **does not
compile** (inherited from `money_guard`); `detail_resolves_modifiers` — base 500 + Delta(+150) shows
650 as text. **Adversarial:** a `SoldOut` leaf still renders its price (X7 holds through
availability, P62 §4.5) but its "add" intent is refused with an honest `Status` node; an attempt to
splice a modifier from another vendor onto a line ⇒ P62's `resolve_line` returns `CrossVendor`, the
card shows an honest error, the buffer is untouched (the client cannot forge a cross-vendor price);
a leaf priced `Money::new(i64::MAX, Eur)` renders its exact integer string, never `+Inf`/scientific
(the float failure mode a naive `as f64` would produce).

### 4.3 M3 — cart + delivery-or-pickup selection (unified cross-vendor, pickup-wired)

`web/src/storefront/cart.mjs` + `fulfillment.mjs`: the cart is P62's **`Cart`** (local-first,
cross-vendor, `Cart::price` overflow-safe subtotal, `Cart::reconcile` drops delisted + re-prices
survivors — P62 §0/§5.4). The cart total and the per-vendor split are shown as `present_money` /
`charge_legs` **preview** text (the split is derived, P62 §3; P60/P72 execute it — P69 only shows
it). `Fulfillment` offers `FulfillmentChoice::{Pickup, Delivery}`: **Pickup is the wired M1 path**
(no courier, no dispatch — SYNTHESIS §3); Delivery is selectable, but its address→dispatch→map
fulfillment is P65/P51/P71 (P69 collects the address via M4 and hands off; it does not route a
courier). The cart bar is a `<Money>` pill, **never** an animated number.

RED→GREEN (Lane A, over P62's pure `Cart`): `cart_total_snaps_present_money` — adding two items
shows the exact summed `Money` as text; `pickup_needs_no_courier` — the Pickup path reaches Payment
with zero courier/dispatch state (M1 has no P65 dependency); `reconcile_drops_delisted` — a leaf the
vendor delisted mid-session is dropped from the cart on reconcile and the survivors re-price (the
self-heal leg, §5.4). **Adversarial:** a cart mixing vendors in **different currencies** ⇒
`charge_legs` returns `Err` (cross-currency fail-closed, P62 §4.4 — the §4-D market flag, never a
silent conversion), the cart shows an honest "single-currency Wave-0" status; a cart total
overflowing i64 ⇒ `Err`, no wrap; selecting Delivery with no address entered ⇒ the journey cannot
advance to Payment (an honest required-field `Alert`, not a silent proceed).

### 4.4 M4 — typed field entry via P57 (name / address / search / notes)

`web/src/storefront/fields.mjs`: every typed field is a P57 **`TextField`** (Latin+Cyrillic Wave-0,
`in_wave0_scope` gate) — the customer name, pickup contact, delivery address, menu search, and order
notes. Each field's a11y is P58's **synthetic ARIA-textbox** (a `Role::TextInput` `SemanticNode`
carrying `EditState`, updated on every `EditEvent`); the field never creates a DOM `<input>` (P38
§12.1). A **tip** is a numeric field: its `TextField` value is parsed to `i64` minor units **at the
submit boundary** and presented via `present_money` — it is **never** a tweened money surface (§5.1,
inherited red-line). Focus routing is P57's rule (a focused `TextField` receives keyboard `Intent`s
as `EditCmd`s; nav intents suppressed for that field).

RED→GREEN (Lane B live-edit is O18a; Lane A wiring now): `field_is_p57_textfield` — a name field
round-trips `"Олена"`/`"Olena"` byte-for-byte through `TextField::apply` and `value()`;
`aria_textbox_caret_tracks` — via P58's `assertCaret`, the field's `data-caret`/`data-sel-anchor`
match after a keydown sequence (the live-edit a11y contract, cited from P58 §M3/§M5, not
re-derived). **Adversarial:** a hostile `keydown` whose key is `"中"` ⇒ P57 `Rejected(OutOfScope)`,
buffer unchanged (the v2 boundary holds at the storefront field edge); a paste of `"cafe中文"` ⇒
`"cafe"` inserts, `中文` refused per-grapheme; **no `<input>`/`contenteditable` exists in the
storefront DOM tree** (the P38 §12.1 tightened grep gate — `createElement` only inside
`a11y_mirror.mjs`); the tip field's value entering `present_money` as an `i64`, never as a `Spring`
channel (the compile-proof from `money_guard`).

### 4.5 M5 — the Path-C card moment: handoff + resume (the real design problem, §4-A CLOSED)

`web/src/storefront/payment.mjs`: the suspend/resume state machine that survives the redirect out of
the canvas and back. **This is the item P69 must not hand-wave** (SYNTHESIS §4-A). The full flow:

**Step 1 — mint + create.** The idempotency `key` was minted by **P66** at *draft creation* (X6), so
it already exists before Payment. P69 calls P60 `create_with_key(key, plan)` where `plan` is the
`NLegPlan` derived from the cart's `charge_legs` (N=1 for single-vendor M1). P60 returns a
`ClientHandoff::HostedRedirect { checkout_url, session_token, ttl_s }`.

**Step 2 — suspend.** `Journey::suspend(handoff, SuspendState { key, session_token, ttl_deadline =
now + CLIENT_SESSION_TTL_S, await_since, resume_step: Payment })` transitions to `Suspended` and asks
**P66** to persist the draft + `SuspendState` (query-before-replay authority). The journey snapshots
its canvas state. **Nothing is captured — the handoff is opaque** (no card data crosses).

**Step 3 — hand off (per platform, per X3).**
- **Web:** navigate the browser to `checkout_url` (the provider's **verified domain**), or open it
  in a new tab; the canvas journey is snapshotted for return.
- **Desktop (winit+wgpu, per X3):** open the **system browser** to `checkout_url` — Path C keeps
  desktop pure `winit`+`wgpu`, no webview.
- **Mobile (Tauri):** Path B `NativeSdkSession` native sheet (pending P63's bridge spike), with
  Path C as the fallback if the spike comes back empty.

**Step 4 — detect return (two mechanisms, neither self-certifying).**
- **Deep-link callback:** the provider redirect returns to `dowiz://return?session=<token>`
  (installed app) or `/s/:slug/return?session=<token>` (web) → `ReturnSignal::DeepLink`.
- **Polling (the honest fallback):** if no deep-link arrives (the app was backgrounded, the redirect
  URL was lost), on resume P69 polls P60 `query_status_by_key(key)` every `RESUME_POLL_INTERVAL_MS`
  until `Captured`/`Failed`/`RESUME_POLL_DEADLINE_S` → `ReturnSignal::Poll`. This is the X6
  reconnect-safe path — it survives an app-kill because the `key` was persisted by P66.

**Step 5 — resume (webhook is the sole truth).** `Journey::resume(signal, status, hub)`:
- `status == Captured` (webhook has moved the kernel fold, P60 §4.4) ⇒ transition to `Placed` = the
  C.2 **Inciting** beat (amber burst + held beat) and let P61 fire the status notification.
- `status ∈ {Authorized, IntentCreated}` (webhook not yet arrived) ⇒ stay in an honest
  "confirming payment…" state, keep polling — **never fake `Captured`**.
- `status == Failed(_)` ⇒ transition back to `Payment` (`resume_step`) with an honest failure node.
- `hub == Offline` at return ⇒ transition to `OfflineHalt` (§4.6), draft held, no fake retry.

**The load-bearing invariant (the falsifiable card-moment test):** the client redirect / deep-link
**never** writes `Captured` — it only triggers a `query_status_by_key` re-check; the *only* writer of
`Captured` is P60's webhook fold. RED→GREEN (Lane B real redirect is P63/P60-gated; Lane A state
machine + Stripe **test mode** now): `handoff_suspends_opaque` — `suspend` transitions to `Suspended`
carrying only opaque handles, no card field; `resume_captured_is_placed` — a `Captured` status
resumes into the Inciting beat; `resume_never_self_certifies` — a `ReturnSignal::DeepLink` with a
status that is still `Authorized` does **not** reach `Placed` (stays confirming). **Adversarial (the
teeth):** (i) a forged client "success" with **NO webhook** ⇒ the journey never reaches `Placed`,
the order is never `Captured` (the M1-critical forged-success test, P60 §4.4); (ii) a return with a
**stale `session_token`** (ttl_deadline elapsed) ⇒ refused, journey returns to Payment to re-mint
(P60 `session_token_single_use` / expired-TTL); (iii) **app killed during redirect** ⇒ on relaunch
P66 query-before-replay restores the draft + `key`, `query_status_by_key` resolves the true status,
and **no double charge** occurs (the idempotency key makes replay a no-op, X6); (iv) two rapid
Payment intents ⇒ P60's single-outstanding-intent cap refuses the second (X11); (v) a poll that
times out at `RESUME_POLL_DEADLINE_S` ⇒ honest "we couldn't confirm — check your bank / retry", never
a fabricated success.

### 4.6 M6 — honest hub-offline status (§16.14 — no fake retry, no central fallback)

`web/src/storefront/hub_status.mjs`: a reachability probe (a lightweight request to P37's order
route / a `/healthz`-class endpoint on the hub's own tunnel, `HUB_PROBE_TIMEOUT_MS`, **degrade-closed**
— an ambiguous/timed-out probe is treated as `Offline`, never optimistically `Online`). When
`HubStatus::Offline`, the journey renders an **honest** status node (a P58 `Role::Status`/`Alert`
with a truthful "this venue is offline" `value_text`) and, mid-checkout, holds the cart/fields as a
**local draft** (P66) — payment simply does not fire until the hub is reachable (§16.52 offline
checkout resilience). **There is no central dowiz queue and no disguised retry** — §16.14 forbids any
dowiz-operated order state, and the `HubStatus` enum has no `CentralFallback` variant to build (§5.1).

RED→GREEN (Lane A): `probe_timeout_is_offline` — a probe exceeding `HUB_PROBE_TIMEOUT_MS` yields
`Offline` (degrade-closed); `offline_shows_honest_status` — an `Offline` journey renders a truthful
status node and **no** spinner-that-fakes-progress; `offline_holds_draft_no_central` — the in-progress
cart/fields persist to the P66 on-device draft and **no request to any dowiz-central endpoint is
made** (asserted by a network-request inspection: zero calls to any non-hub origin). **Adversarial:**
a hub that recovers mid-`OfflineHalt` ⇒ the journey resumes from the P66 draft (the reconnect leg,
§5.4), the payment fires only then; a probe that returns a *disguised* success (a captive-portal-style
200 with no real hub) ⇒ still treated as needing a real order-route ack, not a fake Online; a client
that tries to "queue for later on dowiz.org" ⇒ unrepresentable (no such state/endpoint exists).

### 4.7 M7 — the bot-facing static pack (generated FROM catalog state, JSON-LD load-bearing)

Two halves, split exactly as P62 §2 assigns ("P62 supplies the STRING, P69 writes the FILE"):
**projection** in `kernel/src/json_api.rs` (extend, `#[cfg(feature = "json-api")]`, pure + testable)
and **emission** in `web/src/storefront/bot_pack.mjs` (writes the static files at publish time). The
pack (R1 §7 ranking — **JSON-LD first, `llms.txt` a secondary extra**):

1. **schema.org JSON-LD (load-bearing AEO substrate):** `restaurant_jsonld(...)` wraps P62's
   `menu_jsonld(vendor, leaves)` (the `Menu`/`MenuItem`/`Offer` string, P62 M5) in a `Restaurant`
   node with hub-authored `name`/`PostalAddress`/`OpeningHoursSpecification` (opaque, §16.17). Every
   `Offer.price` is `price_to_decimal_string` (integer divmod, no float — P62 §4.5).
2. **`robots.txt` + `sitemap.xml`:** pure functions of the published slug set — universally consumed.
3. **Open Graph + `manifest.json`:** link-unfurl + installability facts, from catalog/hub state.
4. **`llms.txt`:** the forward-looking extra (R1 §7 — cheap, dowiz's product is agent-facing, but
   **not a crawlability bet**; JSON-LD is the load-bearing one).

All are **generated from catalog state** — so a price change re-generates the pack and it *cannot*
drift (§1.4) — and live on a **separate output path** from the a11y mirror (§16.55; the mirror is
SR-only DOM, the pack is static files). It is a **build-time artifact** (P62 §5.3, like P51's
MapPack — not a mesh payload; §5.3).

RED→GREEN (Lane A, pure kernel projection): `restaurant_jsonld_embeds_menu` — `restaurant_jsonld`
output parses as JSON with `@type:"Restaurant"` → `hasMenu` containing every leaf's
`offers.price` from `menu_jsonld`; `bot_pack_prices_match_catalog` — every `Offer.price` string in
the pack equals `price_to_decimal_string(leaf.price)` for the same leaf (**the anti-drift test**: the
pack price is byte-equal to the catalog price, so drift is a RED test, not a hope);
`llms_txt_is_secondary` — the pack is valid with JSON-LD present even if `llms.txt` is empty (JSON-LD
is the required substrate, `llms.txt` the optional extra). **Adversarial:** a leaf priced
`Money::new(1, Eur)` renders `"0.01"` in the `Offer`, never `"0.00"` (the `€0.01`-never-`€0.00`
guarantee on the bot surface); a `SoldOut` leaf still emits an `Offer` with `availability:"SoldOut"`
(crawlers see it, priced, unavailable); hand-editing a price in the emitted `sitemap`/JSON-LD then
re-running the projection **overwrites** the hand-edit from catalog truth (the pack is a projection,
not a source — the drift test proves it); the pack contains **zero rendered menu markup** for the
a11y mirror to double-count (separate output path, §16.55).

### 4.8 M8 — per-screen a11y + WebGL2/CPU floor-parity (import P58 + P63, do not re-derive)

Every journey step does exactly three things (P58 §M7 contract, cited not re-derived): (1) **author
a `SemanticScene`** for the step (roles/names/states/tab-order/edit fields — the storefront's own
content; P58 owns only the shape) — menu items are `ListItem`s, prices ride `Status.value_text` from
`present_money`, fields are `TextInput` with `EditState`, the offline node is a `Status`/`Alert`;
(2) **import `a11yGate(page, manifest)`** with a **per-screen mirror-node budget** (a checkout step
with ~20 controls declares ~20, not the 256 default — so mirror bloat is caught, P58 §M7); (3)
**carry the FE-16 floor line verbatim** — the storefront must **render correctly** (not merely run)
on the WebGL2 and CPU rungs, proven by P63's **SP-6 `floor-parity`** gate: the one-line DoD other
blueprints paste is *"passes `floor-parity` at ΔE ≤ 0.02 on WebGPU, WebGL2, and CPU rungs"*
(P63 §3.6, `PARITY_PERCEPTUAL_DELTA_MAX = 0.02`).

RED→GREEN: `storefront_a11y_gate_green` — each step's `SemanticScene` passes `a11yGate` with its
declared manifest (roles/names/states + tab order + budget + no-visible-DOM); `storefront_floor_parity`
— the storefront corpus (menu, cart, payment, offline) passes `floor-parity` at ΔE ≤ 0.02 on all
three rungs (the ~18%-without-WebGPU protection). **Adversarial:** force `navigator.gpu = undefined`
(CPU floor) ⇒ the storefront renders AND every a11y assertion passes **unchanged** (a11y is
renderer-independent by construction, P58 §M6-3); a WebGPU-only visual effect with no WebGL2/CPU
equivalent ⇒ **fails `floor-parity`** by construction (P63 §3.6 adversarial); a checkout step whose
mirror exceeds its declared node budget ⇒ `assertMirrorNodeBudget` fires (mirror bloat caught);
money shown in a `Status` node as a **tweened** value ⇒ does not compile (`present_money` only,
§5.1).

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose:

- **A forged payment cannot place an order.** `Captured` has exactly one writer — P60's
  signature-verified **webhook fold** (P60 §4.4). The journey's `resume` reads `PaymentStatus` from
  `query_status_by_key`; a client redirect/deep-link only *triggers* that query. "Client
  self-certifies payment → order placed" is a **tested-unreachable** state (M5 adversarial (i)), not
  a policy.
- **A card cannot reach the canvas.** There is no card-data type (P60 §4.1 firewall); `ClientHandoff`
  carries only opaque handles/URLs. The wgpu canvas has **nothing card-shaped to bind** — "canvas
  renders a card field" is unrepresentable, not merely forbidden.
- **A displayed price cannot animate.** `Money` implements no `FieldValue` (`money_guard.rs:22`), so
  `Spring<Money>`/`interpolate(price, …)` does not compile; every price is `present_money`/
  `format_money` text. A count-up cart total is a compile error, not a lint.
- **The bot pack cannot drift from the menu.** It is a **pure function of catalog state**
  (`build_bot_pack` over `PriceableLeaf`s, §4.7); "JSON-LD price ≠ menu price" is a RED test
  (`bot_pack_prices_match_catalog`), and a hand-edit is overwritten by the next projection — drift is
  un-representable in a projection.
- **No central dowiz order state exists.** `HubStatus` has no `CentralFallback` variant; the offline
  path holds an **on-device** P66 draft and makes **zero** non-hub-origin requests (M6 adversarial).
  "Order queued on a dowiz server" is unrepresentable (§16.14).
- **A client cannot forge a line price or a vendor leg.** The submitted cart is re-priced by
  `place_order_priced` (`domain.rs:198`, ignores caller values) and `vendor_id` is
  catalog-authoritative (P62 §4.4); a tampered cart is a caught mismatch, not a mispriced order.
- **A double charge is unreachable on reconnect.** The idempotency `key` (P66-minted, X6) makes a
  replayed `create_with_key` return the same handoff, never a second charge (P60 §4.2); an app-kill
  during redirect resolves via `query_status_by_key`, not a fresh intent (M5 adversarial (iii)).

### 5.2 Schemas & scaling axes (item 8)

- **`Journey`:** axis = steps (bounded, ~6) — no scaling axis; it is a small FSM.
- **Menu render:** axis = leaves/vendor (P62: 10¹–10³ per menu). Break point: a menu > ~10⁴ leaves ⇒
  virtualize the list (mirror only the visible window + `aria-setsize`/`aria-posinset`, the standard
  large-list a11y pattern P58 §5.2 names) — not a Wave-0 concern (a menu is not a catalog-of-millions).
- **Cart:** axis = lines/cart. O(lines) `Cart::price`; a food-court 300-line cart is microseconds
  (P62 §7). `charge_legs` preview is O(lines) group-by (P62 §5.2).
- **`SuspendState`/drafts:** axis = open checkouts/device (P66-owned) — O(1) per device (one active
  checkout per wallet, the single-outstanding-intent cap, P60 X11).
- **Bot pack:** axis = leaves/hub → JSON-LD size; a menu-sized JSON blob (kilobytes), regenerated on
  publish. Break point: a hub with 10⁴+ leaves ⇒ paginate the sitemap (the standard >50k-URL split) —
  named, not built.
- **Mirror nodes/screen:** axis per P58 §5.2; each checkout step declares its budget (§4.8), tightened
  from the 256 default.

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation/bulkhead:** the journey is a **state consumer**, never a mutator of order truth — a
journey panic (web) cannot corrupt an order, because the kernel `place_order_priced`/`fold` validates
and re-prices the submitted cart (P62 §5.1), and the render inherits P38 §4.3's bulkhead. The card
moment is out-of-canvas on the provider's own domain; a provider outage reaches P69 only as a typed
`PayError`/`Failed` value (P60 §5.3), never a propagating failure. **Mesh awareness:** the storefront
is **node-local** — the order-intent submits over **P37's HTTP route** to the *venue's own hub*
(not gossiped; an additive order event on the already-carried wire, P62 §5.3), the card handoff goes
hub↔provider (out-of-canvas), and the **bot pack is a build-time static artifact** (P51 MapPack
posture — a menu-sized blob has no business in the SyncFrame path). **Zero mesh payload originates in
P69.** No cross-hub state, no money over the mesh. **Living memory:** draft persistence is **P66's**
(query-before-replay, LWW draft — X6); P69 exposes a field snapshot for P66 to persist and asks P66
to restore on resume, but **stores nothing across sessions itself** — the temporal/topological access
pattern is P66's to own (the reconnect recall = `query_status_by_key` over P60's `IdemLedger`, a
living-memory read). Historical prices are preserved as the `unit_price` snapshot on `order_item`
(P62 §5.3 demote-never-mutate) — a placed order keeps its priced-at numbers even after a re-price.

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

- **Self-Termination leg claimed:** the typed refusals (`HubStatus` degrade-closed to `Offline`;
  arc-order invariant; expired-TTL handoff; out-of-scope field codepoint; cross-currency cart; the
  un-compilable money-tween; the absent `CentralFallback`/card types) — hard invariant boundaries,
  not a supervisor's decision.
- **Self-Healing leg claimed narrowly:** (a) the **resume-from-draft** path — an app-kill during the
  Path-C redirect self-corrects to provider truth via P66 query-before-replay + P60
  `query_status_by_key` (reconnect regenerates journey state from the last valid draft); (b) the
  **cart reconcile** — a drifted cart heals to current catalog truth via P62's `Cart::reconcile`
  (drop delisted, re-price survivors). Claimed for the **journey/cart projection only**, not for
  order state.
- **Snapshot-Re-entry: claimed via P66** — the journey resumes from the P66 draft snapshot after an
  app-kill/reconnect; recovery is a cheap re-hydration from the last valid draft epoch, not a bespoke
  recovery path. Mechanical rollback: every P69 change is **additive** (new `web/src/storefront/`
  tree, new `pages/storefront.astro`, extended `json_api.rs`, extended `app.mjs`) — deletion + the
  `json-api` feature off restores today's exact tree.

### 5.5 Error-propagation gates (item 14) + Linux discipline (item 9) + tensor/spectral/eqc (item 16)

**Named gates that turn P69's bug classes into compile/CI failures:** the `bot_pack_prices_match_catalog`
drift test (menu↔JSON-LD divergence), the `resume_never_self_certifies` + forged-success tests
(client-certified payment), the money-tween compile-proof (animated price), the P38 §12.1 no-`<input>`
grep gate (DOM input on the storefront), `a11yGate` + `floor-parity` (a11y/render regressions), the
`offline_holds_draft_no_central` network-request assertion (central-fallback leak), the arc-order
invariant test (skip-to-pay). **Linux-discipline verdicts:** **ALREADY-EQUIVALENT** — one journey
composer (P64), one money-as-text authority (`present_money`/`format_money`/`price_to_decimal_string`),
one a11y mirror (P58), one parity gate (P63), one JSON-LD authority (P62), one order rail (P37);
**REINFORCES** — feature-gated bot-pack projection with an offline-clean default (the kernel-module
behind-a-flag discipline); **EXTENDS** — the **suspend/resume state machine across an out-of-canvas
redirect** is a new pattern this repo adds for provider-hosted flows (a new gate class: "a client
return never writes payment truth"); **GAP** honestly named — **no browser CI runner exists** for the
full canvas keyboard + real deep-link-return path; the web tests run headless (Playwright a11y-tree +
programmatic key dispatch + Stripe test mode), and real on-device deep-link return / soft keyboard /
system-browser handoff is unverified until P63/P45 provides a display runner — each headless/`#[ignore]`
marker doubles as that GAP marker. **Item 16 (tensor/spectral/eqc): NOT load-bearing, stated not
decoratively invoked** — the storefront is UI composition + integer money + static-file projection;
there is no closed-form organ, so `eqc-rs` does not apply and no spectral machinery is summoned. The
one honest reuse of field math is the **narrative pacing**, which *rides P38's existing* Laplacian /
settle / spectral-attending instruments (Додаток C) — cited, not re-derived, and explicitly not
ritual (the Anu/Ananke discipline forbids manufacturing a spectral form where none is load-bearing).

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no journey machine; arc-order tests absent | `journey_advances_arc_order`; `placed_is_inciting_beat` (amber burst + held beat); skip-to-pay refused | arc-order + held-beat tests (ledger row) |
| M2 | no menu render; price-as-text tests RED | `menu_price_is_present_money_text`; modifier resolve; money-tween does not compile; i64::MAX exact | **price-as-text (no-tween)** test (ledger row) |
| M3 | no cart/fulfillment | cart total snaps; pickup needs no courier; reconcile drops delisted; cross-currency ⇒ Err | pickup-no-courier + cross-currency tests |
| M4 | no P57 fields wired | field round-trips Latin+Cyrillic; ARIA-textbox caret tracks; hostile `中` rejected; no `<input>` in tree | **no-DOM-input** grep gate (ledger row) |
| M5 | no handoff/resume; forged-success RED by construction | suspend opaque; captured ⇒ Placed; **forged success (no webhook) ⇒ NOT placed**; stale TTL refused; app-kill no double-charge | **webhook-sole-truth / no-double-charge** tests (ledger row) |
| M6 | no hub-status | probe timeout ⇒ Offline (degrade-closed); honest status no fake retry; **zero non-hub-origin requests**; draft held | **no-central-fallback** test (ledger row) |
| M7 | no bot pack; drift test RED | Restaurant JSON-LD embeds Menu/Offer; **`bot_pack_prices_match_catalog`** (byte-equal to catalog); `0.01`-never-`0.00`; llms.txt secondary | **JSON-LD anti-drift** test (ledger row) |
| M8 | no per-screen a11y/parity gate | `a11yGate` green per step w/ budget; `floor-parity` ΔE ≤ 0.02 on WebGPU/WebGL2/CPU; gpu-undefined a11y unchanged | a11y-gate + floor-parity lines |
| **M1 milestone** | no end-to-end order path | **`m1_first_real_order_pickup`** (below) green | **the M1 e2e test** (ledger row — the program's capstone) |

**The end-to-end M1 falsifiable test (`m1_first_real_order_pickup`) — the task-mandated capstone:**
a Playwright drive against one hand-onboarded, dowiz-operated hub with the P60 Stripe **test-mode**
adapter live —
1. open `/s/:slug`; assert the **bot pack** is served (a `Restaurant` JSON-LD with a real
   `Menu`/`Offer`, plus `robots.txt`/`sitemap.xml`);
2. browse the **P62 menu**; assert a `MenuItem` price renders as `present_money` **text** (not a
   tween);
3. add one item; select **Pickup** (no courier path);
4. enter the customer name via a **P57 `TextField`**; assert P58's `assertCaret` tracks the ARIA-textbox;
5. reach Payment; assert the canvas renders **no card field** (there is no card type to bind);
6. trigger **Path C** → land on the Stripe test-mode **hosted page** (provider's own domain) →
   complete the test card → return via deep-link/return-URL;
7. assert P69 **polls `query_status_by_key`** and does **not** self-write success; the **webhook**
   (P60 M4) normalizes `payment_intent.succeeded` → `Captured`; the kernel fold advances the order to
   `Confirmed`;
8. assert the journey advances to the **Inciting beat** (amber burst + held beat), and **P61 fires the
   status notification** (the M1 mandatory leg);
9. the whole drive passes **`a11yGate`** on every step and **`floor-parity`** at ΔE ≤ 0.02.
This test **is** M1: a real customer paid real money for a real pickup order through this exact flow,
with status notification delivered.

**Not-done clauses:** any DOM `<input>`/`contenteditable`/editable element for text entry on any
storefront screen = **NOT done** (P38 §12.1 grep gate); a client redirect that lets the journey reach
`Placed`/`Captured` **without** the webhook = **NOT done** (§4.5, the forged-success red-line); a
price rendered through a tween/count-up = **NOT done** (🔴 money red-line); a bot-pack price that
diverges from catalog truth = **NOT done** (§4.7 drift); any central-dowiz order queue/fallback =
**NOT done** (§16.14); a hand-authored `sitemap.xml`/JSON-LD not generated from catalog state = **NOT
done**; a WebGPU-only effect that fails `floor-parity` = **NOT done** (excludes the ~18%); a
card-data type anywhere = **NOT done** (PCI red-line); a second money-format or a second a11y-mirror
implementation = **NOT done** (P38 §12.3 / P62 single-owner).

---

## 7. Benchmark plan (item 10) — journey latency + pack projection, measured against real budgets

Budgets (mid-tier device class, P38 §6's 16.6 ms/frame split; keystroke latency vs the GPUI 2 ms
class per P57 §7): **intent → composed step response ≤ 4 ms CPU** (fragment select + `SemanticScene`
author, P64 `Composer::compose` budget); **cart re-price on add ≤ 0.5 ms** (P62's `Cart::price` is
integer checked-adds); **`build_bot_pack` over a 500-leaf menu ≤ 5 ms** (reuses P62's
`menu_jsonld_500_leaves` < 5 ms bench, §4.7); **resume-poll overhead ≈ 0** (a `query_status_by_key`
is one out-of-kernel round-trip, not micro-benched in the kernel — the customer-facing latency is the
provider's). Criterion benches: `json_api/build_bot_pack_500` and `json_api/restaurant_jsonld` in the
kernel (pure projection), added **RED-commit-first** so the `bench_track` baseline auto-seeds
(P-A §6 / P51 §7 discipline, same `BENCH_HISTORY.md` append rule). The **held-beat** target
(0 field-integrator source impulses for N ms after the order-placed burst) is asserted by
`placed_is_inciting_beat` over a counted window — the benchmark IS a test (P38 §6 pattern), and the
C.2 amplitude budget (no intermediate beat louder than the climax) is the same event-log-checked
falsifier. Telemetry: step-transition latency + bot-pack-regen time + resume-poll count ride the
existing native-trackers hooks (P-H's lane), so a journey-latency or pack-bloat regression surfaces
without review.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §3 (M1 definition), §5 (W2 P69 row + build order), §4-A
(Path C, CLOSED), X1 (a11y-mirror-everywhere), X6 (idempotency — P60 owns, P66 mints), X7 (leaf
invariant — P62 owns), X11 (anti-abuse) · `docs/research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md`
§7 (schema.org JSON-LD load-bearing, `llms.txt` secondary), §6 (WebGL2 floor / 82% reach), §2
(a11y-mirror) · `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.37 (checkout = the arc),
§16.14 (honest hub-offline, no central state), §16.40 (intent-only, no fallback), §16.17 (free-form
catalog), §16.52 (offline checkout resilience + SMS/email fallback), §16.60 (pickup in scope from the
start) · `BLUEPRINTS-DOWIZ-INTERFACES.md` **Додаток C.2/C.4** (the pacing beats + camera language,
cited verbatim in §4.1) · `BLUEPRINT-P38-webgpu-render-engine.md` §3.3 (FE-06 glyph render), §3.5
(FE-14 settle = held-stillness instrument), §12.1 (struck `<input>` grep gate), §12.2 (AR/VR
`InputSource`/`FieldPos`-3D constraints), §12.3 (FE-15/FE-16 shared base P69 imports) · **the W1
contracts consumed:** `BLUEPRINT-P57-canvas-text-input.md` (`TextField`/`EditCmd`/`in_wave0_scope`/
`Intent::Text`), `BLUEPRINT-P58-a11y-mirror-everywhere.md` (`SemanticScene`/`mirror`/`a11yGate`/
ARIA-textbox convention/mirror-node budget), `BLUEPRINT-P60-payment-adapter-core.md`
(`ClientHandoff`/`create_with_key`/`query_status_by_key`/`PaymentStatus`/`IdempotencyKey`/`NLegPlan`/
webhook-sole-truth §4.4/no-card firewall §4.1), `BLUEPRINT-P62-catalog-multivendor-data-model.md`
(`PriceableLeaf`/`CatalogNode`/`Cart`/`charge_legs`/`menu_jsonld`/`price_to_decimal_string`),
`BLUEPRINT-P63-shell-platform-spike.md` (SP-6 `floor-parity`, ΔE ≤ 0.02, X3 desktop-shell verdict),
`BLUEPRINT-P64-intent-engine-friction-voice.md` (`Intent`/`InputRouter`/`Composer`/`FrictionFsm`/
`CommitToken`/`VoiceSource`) · `BLUEPRINT-P37-order-http-surface.md` (the order route + reachability
probe target) · `BLUEPRINT-P51-open-map-routing.md` (structural template; static-asset-not-gossip
posture) · `HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§9) · `docs/regressions/REGRESSION-LEDGER.md` (the
rows named in §6). **Consumed by / hands off to (downstream):** **P61** (fires the M1 status
notification), **P65/P51/P71** (delivery fulfillment beyond the M1 pickup path), **P72** (food-court
multi-vendor checkout UX, extends this journey with N-leg atomicity), **P70** (authors the catalog
P69 renders), **P66** (owns the draft/idempotency P69 calls). Memory:
`physics-ui-capture-quantum-math-arc-2026-07-14` (store S(t) not frames — the journey is
recall-by-recompute, §5.3) · `field-ui-engine-arc-2026-07-13` (Sea narrative arc provenance) ·
`dowiz-interfaces-design-arc-2026-07-13` (Додаток C pacing) ·
`dowiz-brand-voice-canon-2026-07-07` (Warm Cosmo-Noir — the grade rides existing tokens, no new
pacing token) · `test-integrity-rules-2026-06-27` (money-RLS-PII red-lines; no-f64/no-tween money) ·
`never-bypass-human-gates-2026-06-29` (money red-line: the card moment is provider-hosted, no PAN) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (§5.5's honest "spectral N/A", no ritual math).
Supersedes: nothing — additive; it is the customer surface the retired TS `MenuPage`/`checkout` once
occupied, rebuilt full-wgpu from the W1 contracts (the old OTP/localStorage/AnimatedNumber mechanics
are **not** carried — Path C, P66 drafts, and `present_money` replace them).

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source, code derived): kernel/catalog state is the source; the GPU frame
  (`compose`), the a11y tree (`mirror`), **and the bot pack** (`build_bot_pack`) are all *derived*
  from it — the bot pack is never hand-authored, so it cannot contradict the menu (§1.4/§4.7).
- **P2 CORRESPONDENCE** (one concept, one primitive): one menu truth (P62's leaf) → the cart price,
  the charge-leg preview, the `Offer.price`, and the a11y `value_text` — "as above the leaf, so below
  every consumer"; one money-as-text authority; one journey composer; one a11y mirror; one parity
  gate.
- **P6 CAUSE-AND-EFFECT** (determinism as law): the webhook is the deterministic gate for `Captured`;
  the client redirect writes nothing; every pacing beat is a determinate function of the order event
  (C.2); the bot-pack drift test and the floor-parity gate each carry a falsifier — no claim is
  un-checkable.
- **P7 GENDER** (paired verification, no self-certification): payment success is refereed by the
  *independent* provider webhook + the kernel fold — the client redirect **never** self-certifies
  (§4.5/§5.1); the render is refereed by the a11y mirror + the floor-parity oracle, not by its own
  pixels.
- **P4 POLARITY** (one axis, two poles): pickup and delivery are two poles of one fulfillment axis
  (M1 = the courier-less pole); single-vendor and food-court are the same journey at N=1 vs N>1 (P62
  §9) — one design, not two.

(P3/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the greenfield grep; the web tree; the P37 order rail; the W1 contracts on disk) |
| 2 DoD | §6 (incl. the end-to-end M1 capstone test) |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first per item; §4.1/§4.5 assert on intent/return-signal sequences |
| 4 predefined types/consts | §3 (journey machine + bot-pack projection; cited types shown as `use` boundaries) |
| 5 adversarial/breaking tests | §4.1–4.8 (skip-to-pay, i64::MAX price, hostile `中`, forged success, stale TTL, app-kill double-charge, offline central-leak, JSON-LD drift, gpu-undefined) |
| 6 hazard-safety as math | §5.1 (forged-payment / card-on-canvas / animated-price / bot-drift / central-state all unreachable by construction) |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (each with a named break point) |
| 9 Linux discipline | §5.5 (all four verdict classes incl. an honest GAP) |
| 10 benchmarks+telemetry | §7 |
| 11 isolation/bulkhead | §5.3 (state-consumer, kernel re-prices, out-of-canvas card) |
| 12 mesh awareness | §5.3 (node-local; order over P37, not gossiped; bot pack build-time static) |
| 13 rollback/self-heal vocabulary | §5.4 (three legs claimed precisely) |
| 14 error-propagation gates | §5.5 (named gates), §5.1 (typed refusals), §6 (ledger rows) |
| 15 living memory | §5.3 (drafts are P66's query-before-replay; price snapshot demote-never-mutate) |
| 16 tensor/spectral + eqc reuse | §5.5 (spectral honestly NOT invoked; pacing rides P38's existing instruments, not ritual math) |
| 17 regression ledger | §6 (rows named, incl. the M1 e2e capstone) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §1.5 (the consume-not-build table), §0 (P37/P57/P58/P60/P62/P63/P64 all reused); §3 (rejected alternatives) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Order below is the dependency order. **Lane A (buildable TODAY, no network unlock):** T1–T4, T6, T7
ride pure JS state + P62's pure-`std` `Cart` + the kernel `json-api` projection + the existing CPU
floor + Stripe **test mode**. **Lane B (gated — check the gate record first):** the glyph render
(O18a, T2/T4), the native AccessKit (AK-unlock, T4), the real Path-C desktop redirect (P63 shell
verdict, T5). Each Lane-B test carries an `#[ignore]`/headless marker doubling as the gate marker.

1. **T1 (M1 — the journey machine is the spine).** Create `web/src/storefront/journey.mjs` with the
   §3 `Journey`/`JourneyStep`/`FulfillmentChoice`/`SuspendState`/`ReturnSignal` state machine and
   `web/src/pages/storefront.astro` (mirror `pages/fieldsim.astro`). Extend `web/src/app.mjs`, do NOT
   rewrite it. Navigation via P64's `InputRouter::tick` + `Composer::compose`. Write RED first:
   `journey_advances_arc_order`, `placed_is_inciting_beat` (amber burst + held beat per Додаток C.2),
   skip-to-pay refused. Acceptance: Playwright headless green on the CPU floor.
2. **T2 (M2 — menu render).** Create `storefront/menu.mjs`: render P62's `CatalogNode` tree; prices
   via `present_money` (import from the engine — NEVER a local format). Detail step resolves modifiers
   via P62 `resolve_line`. RED: `menu_price_is_present_money_text` + the money-tween compile-proof.
   Acceptance: green (glyph render `#[ignore="O18a"]` until FE-06 lands).
3. **T3 (M3 — cart + fulfillment).** Create `storefront/cart.mjs` (over P62's `Cart`) +
   `fulfillment.mjs` (`FulfillmentChoice`; Pickup is the wired M1 path). RED: `cart_total_snaps_present_money`,
   `pickup_needs_no_courier`, cross-currency ⇒ Err. Acceptance: `cargo test`/Playwright green.
4. **T4 (M4 — P57 fields + P58 a11y).** Create `storefront/fields.mjs`: each typed field is a P57
   `TextField`; each is a P58 `Role::TextInput` `SemanticNode`. Import P58's `assertCaret`/`assertLiveEdit`
   — do NOT write a bespoke a11y test. RED: `field_is_p57_textfield`, `aria_textbox_caret_tracks`,
   hostile `中` rejected, **no `<input>` in the storefront tree** (the P38 §12.1 grep gate).
5. **T5 (M5 — the card handoff, the hardest item).** Create `storefront/payment.mjs`: `suspend`
   (P60 `create_with_key` → `ClientHandoff::HostedRedirect`, persist via P66), the platform handoff
   (system browser desktop / nav web / native sheet mobile), return detection (`ReturnSignal` deep-link
   **or** poll `query_status_by_key`), and `resume` (webhook-only `Captured`). Write the forged-success
   test FIRST (no webhook ⇒ never `Placed`), then the state machine. Adversarial per §4.5 (i)–(v).
   Acceptance: Lane-A state machine + Stripe **test-mode** green; real desktop redirect `#[ignore]`
   until P63's shell verdict. **This is the M1 red-line — freeze the "client never self-certifies"
   invariant here.**
6. **T6 (M6 — honest offline).** Create `storefront/hub_status.mjs`: the degrade-closed probe +
   `HubStatus::Offline` honest status + on-device draft hold (P66). RED: `probe_timeout_is_offline`,
   `offline_shows_honest_status`, `offline_holds_draft_no_central` (zero non-hub-origin requests).
7. **T7 (M7 — the bot pack).** Extend `kernel/src/json_api.rs` (`#[cfg(feature="json-api")]`) with
   `restaurant_jsonld`/`robots_txt`/`sitemap_xml`/`open_graph_tags`/`web_manifest`/`llms_txt`/
   `build_bot_pack` (reuse P62's `menu_jsonld`/`price_to_decimal_string` — NO float). Create
   `storefront/bot_pack.mjs` to write the static files at publish time. RED: `restaurant_jsonld_embeds_menu`,
   **`bot_pack_prices_match_catalog`** (the anti-drift test), `0.01`-never-`0.00`, `llms_txt_is_secondary`.
   Acceptance: `cargo test -p dowiz-kernel --features json-api json_api` green.
8. **T8 (M8 — a11y + floor-parity, per screen).** For each step author its `SemanticScene`, import
   P58's `a11yGate(page, manifest)` with a declared mirror-node budget, and paste P63's `floor-parity`
   DoD line. RED: `storefront_a11y_gate_green`, `storefront_floor_parity` (ΔE ≤ 0.02 on all three rungs).
9. **T9 (the M1 capstone).** Write `m1_first_real_order_pickup` (§6) end-to-end against a live hub +
   Stripe test mode: open `/s/:slug` → bot pack present → browse P62 menu → add item → Pickup → name
   via P57 → Path C → webhook `Captured` → Inciting beat → P61 notification, under `a11yGate` +
   `floor-parity`. Add the §6 ledger rows to `docs/regressions/REGRESSION-LEDGER.md` (naming the M1
   e2e test as the program's permanent capstone). Acceptance: the M1 test green = the first real order
   is reachable.
