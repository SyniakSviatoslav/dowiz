# BLUEPRINT P73 — dowiz.org landing + signup (full-wgpu marketing/demo surface) (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Wave **W3**,
> component **DELIVERY / public-infrastructure surface**. Scope authority:
> `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (W3 table, row **P73**) and its build-sequence
> line (§3.1: "P73 dowiz.org landing + signup — full wgpu §16.56; static-file SEO pack; interest
> form"). Binding operator rulings: `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
> **§16.21** (dowiz.org is PURE infrastructure — *no public vendor catalog at all, not even a
> directory of links*; it is a product/demo page for **prospective venue owners**), **§16.32**
> (claim mechanic; signup/interest form notifies the operator; dowiz.org may be minimal — landing +
> signup + GitHub link; the hub-software source is publicly visible on GitHub), **§16.56** (the
> landing page is **also full wgpu — NO static-page exception**, an explicit deliberate ruling),
> **§16.55** (bot-facing static files, separate from the a11y mirror), **§16.1** (hosting topology
> — Cloudflare Pages for edge-delivered client content), **§16.57** (abandoned claims stay with the
> vendor; the warm pool is net-consumed). Research grounding:
> `docs/research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` §7 (SEO/AEO: schema.org JSON-LD is
> load-bearing, `llms.txt` a forward-looking extra) and `docs/research/OPUS-R3-HUB-PROVISIONING-
> IDENTITY-2026-07-18.md` §6.1 (the claim-mechanic implementation this page's form hands off to).
> Structural template + rigor precedent: `BLUEPRINT-P51-open-map-routing.md`; the conversion-flow
> rigor precedent is `BLUEPRINT-P69-customer-storefront-checkout.md`'s checkout journey.
>
> **Binding scope is a closed operator decision — this document makes it buildable, it does not
> re-litigate it.** A full-wgpu landing/demo page hosted on Cloudflare Pages that (1) shows a
> **prospective venue owner** the actual product — the live field-engine UI itself, since that IS
> the differentiator — (2) collects an interest/signup via P57 canvas text fields and **hands off
> to P67's claim service** along two paths (instant warm-pool claim, or notify-operator interest),
> (3) links prominently to the **open-source hub-software** GitHub repo (§16.32), and (4) emits its
> own **bot-facing static pack** (robots/sitemap/JSON-LD/manifest/llms — a *marketing/product*
> schema, **not** `Restaurant`/`Menu`). It hosts **NO vendor catalog, NO discovery, NO "browse
> restaurants near you"** — see the forceful anti-scope §2.2.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**no landing / claim-client / signup code of any kind exists yet, and the claim service P73 hands
off to (P67) is not on disk** — P73 is greenfield, riding a landed field-render substrate + three
in-parallel W1 contracts (P57/P58, P59) + one in-parallel W3 contract it consumes (P67).

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| **Zero landing / claim / signup code anywhere.** grep `dowiz\.org\|landing\|claim_service\|claim_hub\|warm.?pool\|interest.?form\|signup` over `kernel/ engine/ wasm/ web/` (excl. tests/comments) → **0 hits** | repo-wide grep this pass | **VERIFIED — P73 is greenfield; only the mechanisms it composes are landed or in-parallel** |
| **P67 (hub provisioning + claim service) is NOT on disk** — written in parallel this same W3 wave | `ls docs/design/CORE-ROADMAP-2026-07-17/ \| grep -E 'P67\|P68'` → 0 | **VERIFIED — P73 cites the claim-service API from its authoritative research source (R3 §6.1) and carries a HARD `RECONCILE-P67` obligation (§3, §12), exactly as P57 does for P58's not-yet-final ARIA-textbox convention** |
| `kernel/src/json_api.rs` **exists** as the shared JSON string boundary (P37 extraction), gated `#[cfg(feature = "json-api")]`, serde-free default | `kernel/src/json_api.rs:1-16` (module doc), `:18` (`#![cfg(feature = "json-api")]`) | **VERIFIED — the bot-pack projection lives here; P73 ADDS a marketing-schema projection beside P69's catalog one** |
| **The bot-pack projection is greenfield** — `restaurant_jsonld`/`robots_txt`/`sitemap_xml`/`build_bot_pack` do NOT exist yet; `json_api.rs` today holds only `place_order_logic`/`apply_event_logic` | grep `restaurant_jsonld\|robots_txt\|build_bot_pack` over `kernel/` → 0; `kernel/src/json_api.rs:154,203` (only order fns) | **VERIFIED — P69 owns the catalog `BotPack`; P73 adds a `landing_*` sibling to the SAME module, reusing P69's slug-agnostic pure fns (`robots_txt`/`sitemap_xml`) verbatim (§4.4)** |
| Kernel `TokenBucket` exists for degrade-closed rate limiting: `new(capacity, refill_rate)`, `try_acquire(n) -> bool`, `available() -> f64` | `kernel/src/token_bucket.rs:26` (`TokenBucket`), `:34` (`new`), `:74` (`try_acquire`), `:94` (`available`) | **VERIFIED — the claim-intent limiter (X11) reuses this; P73 does not invent a limiter** |
| Money renders integer→integer via `TweenGuard`; `Money(i64)` implements no `FieldValue` (🔴 RED-LINE) | `engine/src/money_guard.rs:18` (`Money`), `:22-25` (`FieldValue` deliberately not for `Money`), `:60` (`present_money`) | VERIFIED — **there is no `Money` anywhere on dowiz.org** (§16.21 pure infra, no cart/price); the whole tween-money hazard class is structurally absent (§5.1), not merely guarded |
| `compose(scene, eq, w, h, steps) -> Vec<u8>` — the bit-deterministic physics-state→RGBA oracle; Scene + SDF primitives (CPU) | `engine/src/field_frame.rs:218`; `engine/src/scene.rs:29-44,71,88,168` | VERIFIED — the landing hero field-demo renders **through** this frame (P38 engine reused, not a second renderer) |
| P57 `TextField` (the signup form's fields) is a W1 blueprint, greenfield editor over cosmic-text/FE-06; exposes `value()`/`set_value()`/`apply(cmd,clip)` and `Intent::Text(EditCmd)` | `BLUEPRINT-P57-canvas-text-input.md:252-270` (`TextField`), `:427` (every typed field across P69/P70/P71/**P73** is a `TextField`) | **VERIFIED — P73's contact/venue/notes fields ARE `TextField`s; P57 §12 names P73 as a consumer** |
| P58 owns the a11y mirror + shared Playwright a11y-tree harness + synthetic ARIA-textbox convention; every surface (P69/P70/P71/**P73**) imports the harness as a DoD gate | `BLUEPRINT-P58-a11y-mirror-everywhere.md:14-19` (single-owner contract), `:44` (P51 one-surface harness generalized) | **VERIFIED — P73's a11y is P58's harness imported, not re-derived (§4.5)** |
| The claim mechanic is decided: warm-pool **assignment-only** hot path (zero infra on claim), background snapshot-refill, non-pool path = interest form notifies operator; each pooled hub pre-minted its own self-signed ML-DSA-65⊕Ed25519 root cert, online at `hub-<id>.hubs.dowiz.org` | `docs/research/OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md:358-373` (§6.1 steps 1-5), `:361` (`hub-<id>.hubs.dowiz.org`) | **VERIFIED — this is the exact API P73's form hands off to (§3, §4.3); P67 owns the impl, R3 §6.1 is the contract source until P67 lands** |
| SEO/AEO ruling: schema.org JSON-LD is the **load-bearing** AEO substrate; `llms.txt` a forward-looking extra (JSON-LD first, not a crawlability bet — 408 `llms.txt` hits in 500M bot visits) | `docs/research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md:349-363` (§7) | VERIFIED — P73's pack ranks JSON-LD first; but the schema **type** is marketing/product (`WebSite`/`Organization`/`SoftwareApplication`), NOT `Restaurant` — there is no menu on dowiz.org (§4.4) |
| §16.56: the landing page is **also full wgpu, no static-page exception**; §16.55's bot-facing files remain the SEO answer for it too | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:2442-2447` (§16.56) | **VERIFIED — a simpler static-HTML landing page is an explicitly forbidden design (§2.2, §6 not-done clause)** |
| §16.21: dowiz.org lists **no vendors at all, not even a directory of links** — a product/demo page for prospective venue owners + self-serve signup + installable-client hosting; supersedes §16.6's "directory" framing | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:2031-2042` (§16.21) | **VERIFIED — the single most-tempting feature-creep, ruled out categorically (§2.2)** |
| Hosting: Cloudflare Pages = edge delivery for static/brand content + `dowiz.org`-served client-app assets | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:1828-1834` (§16.1 mode 1) | VERIFIED — P73 deploys to Cloudflare Pages (§4.6); the wgpu WASM bundle + bot-pack files are the edge artifacts |
| The public GitHub source is the **hub software** (AGPLv3+TM+DCO); dowiz's **own** claim/CF-isolation/landing infra stays **closed** | `docs/research/OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md:32-34` (§0 licensing boundary); `MASTER-ROADMAP-…:2135-2138` (§16.32 "the GitHub link is to the *product's source*, not a vendor directory") | **VERIFIED — the landing page's GitHub link points to the OPEN hub-software repo; the landing page's OWN source is closed dowiz infra (§16.54) — an honest, load-bearing distinction (§3, §4.2)** |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. The binding decision, restated + the research it rests on (not re-opened)

### 1.1 The closed rulings, restated so every item below is checkable

1. **dowiz.org is pure infrastructure for prospective venue owners** (§16.21). It is a
   product/demo page + signup, **never** a customer-facing discovery surface. It lists **no
   vendors, no menus, no links to venues** — each venue markets its own `/s/:slug` through its own
   channels (§16.21). The "переглянути та спробувати" framing was about trying the **product**, not
   browsing food.
2. **The landing page is full wgpu — no static-page exception** (§16.56). The *ad fontes*
   render commitment (§16.30/§16.40) applies uniformly, *including the surface most tempted to cut
   as "just a landing page."* A simpler static-HTML page is a forbidden design, not a shortcut.
3. **Signup = a claim/interest form that hands off to the claim mechanic** (§16.32). Two paths,
   both real (R3 §6.1): the **fast path** claims a warm-pool hub (assignment-only, instant), the
   **slow path** registers interest for cases outside the pool (notify the operator, manual
   follow-up). "dowiz.org can be minimal — landing + signup + GitHub link" (§16.32).
4. **A prominent GitHub link** to the open-source **hub-software** repo (§16.32) — the product's
   source, publicly visible; not a vendor directory. (The landing page's own infra is closed,
   §16.54 — see §4.2.)
5. **A separate bot-facing static pack** for SEO/AEO (§16.55), JSON-LD load-bearing (R1 §7), but a
   **marketing/product schema**, not a catalog one — there is no menu here.

### 1.2 Why the demo IS the field engine, not a marketing mock (R1 §1, cited)

R1 §1 verified that a full GPU application UI is production-real at the hardest end (GPUI/Zed,
2 ms keystroke-to-pixel). dowiz's differentiator against every conventional-DOM food-delivery SaaS
is precisely that its UI is a live physics/field engine (Sea ambient field + Sheet brand SDF +
FE-06 MSDF text — the P38 render engine). So the honest, highest-converting hero for a prospective
venue owner is **the actual running product**, not a screenshot of it: the landing page's hero is a
live, interactive field-engine canvas. This is *why* §16.56 forbids a static-page exception — a
static marketing site would be showing a photograph of the thing while claiming the thing is
special. The landing page is the product demoing itself.

The keystroke/frame bar is inherited, not re-invented: the hero rides P38's 16.6 ms/frame budget
and P57's ≤2 ms keystroke→event budget for the form (§7); the WebGL2/CPU floor (FE-16) is a
standing DoD gate (SYNTHESIS §3.2 rationale 7), so the ~18% of web users without WebGPU still see
the same UI, not a fallback page.

### 1.3 What is genuinely custom in P73 (the honest residue)

- **The conversion-flow FSM** — a closed `ClaimJourney` state machine (§3) with event-sequence
  tests, mirroring P69's checkout-journey rigor but for one funnel: visitor → claimed hub / interest
  registered.
- **The claim-service client leg** — a thin `ClaimServicePort` (§3) that POSTs one of two requests
  and renders the outcome. P73 implements **no** claim logic, mints **no** certs, touches **no**
  warm pool, and talks to **no** Cloudflare/Hetzner API — all of that is P67. P73 is a client.
- **The landing bot-pack** — a marketing/product-schema sibling (`landing_jsonld` + a
  `LandingBotPack`) added to `kernel/src/json_api.rs` beside P69's catalog pack, reusing P69's
  slug-agnostic pure fns (`robots_txt`/`sitemap_xml`) verbatim (§4.4).
- **The landing scene composition** — a sequence of field/SDF/text sections rendered through the
  **existing** P38 engine; no bespoke renderer.

Rejected alternatives (DECART one-liners): **a static-HTML marketing site (Astro/plain HTML)** —
rejected: §16.56 forbids the static-page exception categorically; it is the one ruling this
blueprint exists to honor. **A vendor directory / "restaurants near you" browse** — rejected:
§16.21 rules it out *not even as a directory of links*; it is anti-scope §2.2, the sharpest
feature-creep. **A second bot-pack module for the landing** — rejected: the bot-pack authority is
`json_api.rs` (P69's module); a second one drifts and duplicates the `robots_txt`/`sitemap_xml`
pure fns. **P73 implementing the claim/pool/cert logic itself** — rejected: that is P67's owned
service (closed infra, §16.54); P73 duplicating it would fork the trust boundary R3 §1.5 names.
**Persisting the signup form as a P66 draft** — rejected: a signup form is not an order draft; a
lost signup is trivially re-enterable, so no query-before-replay machinery is warranted (§5.4).

---

## 2. Scope — what P73 owns vs deliberately does NOT

### 2.1 P73 owns (build items §4)

| Item | Content |
|---|---|
| M1 | **`ClaimJourney` FSM** (`landing/journey.rs`, pure, Lane A) — the closed visitor→claimed/interest conversion state machine; event-sequence tested; `PoolEmpty` routes to interest, never to `Failed` |
| M2 | **`SignupForm` over P57 `TextField`s** (`landing/form.rs`) — contact / venue_name / notes fields; validation at the submit boundary; the edge `ChallengeToken` seam (X11) |
| M3 | **`ClaimServicePort` client leg** (`landing/claim_client.rs`) — the two-request API (`claim_warm_pool_hub` / `register_interest`) P73 POSTs to P67's service; a mock adapter for tests; degrade-closed timeout → interest path |
| M4 | **The landing bot-pack** (`kernel/src/json_api.rs` extension + `landing/bot_pack.mjs` emission) — `robots.txt` + `sitemap.xml` (dowiz.org's own pages ONLY) + **marketing-schema JSON-LD** (`WebSite`/`Organization`/`SoftwareApplication`, NOT `Restaurant`) + Open Graph + `manifest.json` + `llms.txt` (secondary) |
| M5 | **The landing scene + a11y + floor-parity** — the full-wgpu hero field-demo + narrative + form + GitHub CTA rendered through P38; a `SemanticScene` per journey step imported into P58's `a11yGate`; FE-16 WebGL2/CPU floor-parity DoD line |
| M6 | **The claimed-hub handoff** — on a fast-path claim, render the success state (hub URL + deep-link into P70 owner surface) and hand the opaque `OwnerRootCert` off to the wallet / installed client (P66/P70); P73 receives it opaque and forwards it once, never parses/stores/mints it |

### 2.2 P73 explicitly does NOT own — the forceful anti-scope (the single most-tempting creep)

- **NOT any vendor catalog, vendor directory, vendor listing, or "browse/search restaurants near
  you" — in ANY form, not even a directory of links.** This is the categorical §16.21 ruling
  ("dowiz.org publicly lists no vendors at all… NOT even a directory of links"), and it is the
  single most tempting feature to bolt onto a "landing page." It is ruled out **absolutely**: there
  is **no** `VendorId`, `slug`, catalog, menu, price, search box, geolocation, or "near you"
  concept anywhere in P73's code. A diff that adds a vendor list / search / browse surface is a
  **scope violation regardless of test state**, falsified by the anti-scope grep gate and the
  `sitemap_has_no_vendor_slugs` test (§6). Each venue markets its own `/s/:slug` through its own
  channels (§16.21); dowiz never inserts itself as a discovery layer. **Cold-start help (§16.26)
  lives per-venue (P70 auto-posting / SEO tooling), NOT on dowiz.org** — citing §16.26 does not
  reopen §16.21, and P73 builds none of it.
- **NOT a static-HTML landing page.** §16.56 forbids the static-page exception explicitly and
  deliberately. The landing is full wgpu (P38 engine), with the FE-16 WebGL2/CPU floor as the only
  fallback — the *same* app degraded, never a different static page. A static-HTML page is **NOT
  done** regardless of how it looks (§6 not-done clause).
- **NOT the claim service, the warm pool, cert minting, or any Cloudflare/Hetzner provisioning
  call** — **P67 owns** all of it (closed dowiz infra, §16.54). P73 is a **client** of P67's API;
  it holds no signing key, no pool state, no CF/Hetzner API token. R3 §1.5's control-plane trust
  boundary is P67's, not P73's.
- **NOT the capability-cert format or custody** — the cert **format** is **P59's** (biscuit-style
  hybrid-signed chain); the cert **custody** (storing the owner root, autofill) is **P66's** wallet
  / **P70's** owner surface. P73 receives an `OwnerRootCert(Vec<u8>)` **opaque**, renders the
  handoff, and forwards it **once**; it never parses, long-term-stores, or mints it (§5.1).
- **NOT the a11y-mirror convention or its Playwright harness** — **P58 owns** them. P73 authors a
  `SemanticScene` per journey step and **imports** `a11yGate` (§4.5); it does not re-derive a
  mirror.
- **NOT the canvas text editor** — **P57 owns** `TextField`/`EditCmd`. P73's form fields are
  `TextField` instances; P73 wires them into the form but forks no editor.
- **NOT the shell / soft-keyboard / WebGL2-floor ladder mechanism** — **P63/P39-rev** own them.
  P73 consumes the FE-16 ladder and the shell verdict; the landing is web-first (Cloudflare Pages),
  so the mobile-web soft-keyboard spike outcome (§0.2-2) applies to its form exactly as to P69's.
- **NOT online payment / merchant onboarding** — dowiz.org takes no money (§16.21 pure infra;
  §16.16 dowiz takes no percentage). A food-court vendor's provider-account connection (§0.2-1) is
  a **P72/P70** step *inside a claimed hub*, not on the landing page. There is no `PaymentProvider`
  surface here.
- **NOT the installable-client build itself** — §16.21 lists installable-client hosting (§16.8) as
  part of dowiz.org's surface, so P73 renders a secondary "install the app" CTA/link, but the
  client bundle is **P39's** artifact; P73 links to it, it does not build it.

### 2.3 Two-lane build reality (mirrors P38/P57's O18a split)

The full-wgpu render leg rides P38's O18a `graphics-unlock` (wgpu) + P57's `text` feature (same one
network grant). So P73 is two-laned:

- **Lane A (buildable TODAY, zero network):** the `ClaimJourney` FSM (pure), the `SignupForm`
  wiring contract + submit-boundary validation, the `ClaimServicePort` trait + a mock adapter, the
  landing bot-pack **pure fns** (`landing_jsonld`/reuse of `robots_txt`/`sitemap_xml`) + every
  anti-scope test, the `SemanticScene` authoring for each step, and the Playwright e2e against the
  **mock** claim service + the bot-pack anti-scope assertions. Every RED test — render/wgpu-
  dependent ones marked `#[ignore = "O18a"]` (the P38/P57 convention).
- **Lane B (blocked on O18a + P67):** the wgpu render of the hero field-demo (rides P38 §3 + P57
  glyph render), and the **real** claim-service transport adapter (rides P67's landed wire format —
  the mock is swapped for the HTTP client). Do NOT `cargo add wgpu/cosmic-text` without the network
  grant; do NOT hardcode P67's wire shape before it lands — the mock is the contract stand-in.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── landing/journey.rs — NEW module, PURE, no wgpu (Lane A, lands NOW) ──
/// The visitor → claimed-hub / interest conversion journey — a CLOSED FSM (standard item 3:
/// tests assert on event SEQUENCES, not end-state). Mirrors P69's checkout-journey discipline
/// (BLUEPRINT-P69 §4.6) but for ONE funnel. `PoolEmpty` is NOT a failure state.
#[derive(Clone, PartialEq, Debug)]
pub enum ClaimJourney {
    Landing,                        // hero field-demo + narrative + GitHub CTA + "claim / try it"
    FormEntry(SignupForm),         // P57 TextFields; edge-challenge seam armed
    Submitting,                    // request in flight (challenge verified at the edge FIRST)
    Claimed(ClaimedHub),           // FAST PATH: warm-pool hub assigned, online, fixture-populated
    InterestRegistered(InterestAck),// SLOW PATH: pool-empty / out-of-pool → operator notified
    Failed(ClaimError),            // transport/challenge failure ONLY — recoverable, retryable
}
/// The events the journey folds (standard item 3). One source feeds render AND a11y mirror (X1).
#[derive(Clone, PartialEq, Debug)]
pub enum JourneyEvent {
    Started, FormOpened, FieldEdited /* delegates to P57 EditEvent */,
    SubmitRequested, ChallengePassed, ChallengeFailed,
    ClaimAssigned(ClaimedHub), PoolEmptyFellBackToInterest, InterestAcked(InterestAck),
    TransportFailed(ClaimError), Retried,
}

// ── landing/form.rs — the signup form over P57 (§4.2) ──
/// The interest/signup form. Every text field is a P57 `TextField` (BLUEPRINT-P57 §3) — NO DOM
/// <input> anywhere (§16.34 inherited). `challenge` is an EDGE token (X11), never canvas-drawn.
pub struct SignupForm {
    pub contact: TextField,        // P57 — email or Telegram handle (how the operator follows up)
    pub venue_name: TextField,     // P57 — the prospective venue's name
    pub notes: TextField,          // P57 — optional free text (out-of-pool needs, questions)
    pub challenge: Option<ChallengeToken>, // set by the edge (Turnstile/ALTCHA); None until passed
}
/// An edge-verified anti-abuse token (X11 / R2 §7). Produced at the Cloudflare edge/challenge
/// layer, NOT embedded in the wgpu canvas (same DOM tension as X3). Opaque to P73; the claim
/// service RE-verifies it (defense in depth) before touching a pool slot.
#[derive(Clone, PartialEq, Debug)] pub struct ChallengeToken(pub String);

// ── landing/claim_client.rs — the client leg of P67's claim service (§4.3) ──
/// The claim-service API — **OWNED by P67** (`BLUEPRINT-P67-hub-provisioning-claim.md`, greenfield
/// this pass; contract source = `OPUS-R3-…-2026-07-18.md` §6.1 steps 1-5). P73 is a CONSUMER: it
/// POSTs one of two requests and renders the outcome. P73 implements NO claim logic, mints NO
/// certs, touches NO warm pool, calls NO Cloudflare/Hetzner API. **HARD `RECONCILE-P67`: these
/// shapes are provisional until P67 lands its wire format; on divergence, P73 adopts P67's.**
pub trait ClaimServicePort {
    /// FAST PATH (R3 §6.1 steps 1-3: assignment-only, zero infra on the hot path). Returns a
    /// claimed, already-ONLINE, fixture-populated hub, OR `PoolEmpty` (NOT an error — the journey
    /// falls to `register_interest`).
    fn claim_warm_pool_hub(&self, req: ClaimRequest) -> Result<ClaimOutcome, ClaimError>;
    /// SLOW PATH (R3 §6.1 step 5 / §16.32 non-mandatory path): notify the operator for MANUAL
    /// follow-up. No automation, no infra — an ack id only.
    fn register_interest(&self, sub: InterestSubmission) -> Result<InterestAck, ClaimError>;
}
pub struct ClaimRequest {
    pub contact: String,           // SignupForm.contact.value() (P57)
    pub venue_name: String,        // SignupForm.venue_name.value()
    pub challenge: ChallengeToken, // edge-verified; the claim service re-verifies (§5.1)
}
pub struct InterestSubmission {
    pub contact: String, pub venue_name: String, pub notes: String, pub challenge: ChallengeToken,
}
#[derive(Clone, PartialEq, Debug)]
pub enum ClaimOutcome { Claimed(ClaimedHub), PoolEmpty }   // PoolEmpty ⇒ route to interest (§4.3)
/// What P67's service hands back on a successful claim (R3 §6.1 step 3 / §2.4).
#[derive(Clone, PartialEq, Debug)]
pub struct ClaimedHub {
    pub hub_id: HubId,             // opaque id
    pub hub_url: String,          // `hub-<id>.hubs.dowiz.org` (R3 §1.2) — where the vendor lands
    pub owner_root_cert: OwnerRootCert, // handed to the claimant (R3 §6.1-b) — OPAQUE to P73
    pub fixtures_ready: bool,     // §16.54 — hub is online & populated the instant it's claimed
}
#[derive(Clone, PartialEq, Debug)] pub struct HubId(pub String);
#[derive(Clone, PartialEq, Debug)] pub struct InterestAck { pub ack_id: String }
/// The owner root capability-cert — FORMAT is **P59's** (biscuit-style hybrid-signed chain),
/// CUSTODY is **P66's** wallet / **P70's** owner surface. P73 receives it OPAQUE, forwards it
/// ONCE (§4.6), never parses/stores/mints it (§5.1 — the unforgeable-cert argument).
#[derive(Clone, PartialEq, Debug)] pub struct OwnerRootCert(pub Vec<u8>);
#[derive(Clone, PartialEq, Debug)]
pub enum ClaimError { Timeout, ChallengeRejected, Transport(String), RateLimited }

// ── constants ──
/// P67-owned endpoints — illustrative until P67 lands them; `RECONCILE-P67`.
pub const CLAIM_ENDPOINT: &str    = "https://claim.dowiz.org/v1/claim";
pub const INTEREST_ENDPOINT: &str = "https://claim.dowiz.org/v1/interest";
/// The PUBLIC hub-software repo (§16.32 — the product's source; AGPLv3+TM+DCO). NOT the landing
/// page's own repo (closed dowiz infra, §16.54). Exact public slug is operator-set (ADR-020-gated).
pub const HUB_SOURCE_URL: &str = "https://github.com/dowiz/hub";     // RECONCILE with ADR-020 open-source flip
pub const CLAIM_REQUEST_TIMEOUT_MS: u32 = 8000;   // degrade-closed → offer the interest path on timeout
pub const CONTACT_MAX_BYTES: usize    = 256;      // P57 FIELD_MAX_BYTES scope
pub const VENUE_NAME_MAX_BYTES: usize = 256;
pub const NOTES_MAX_BYTES: usize      = 2048;
/// Client-side single-outstanding-claim cap (X11) — reuse `kernel/src/token_bucket.rs`.
pub const CLAIM_BUCKET_CAPACITY: f64  = 1.0;      // one in-flight claim at a time per visitor
pub const CLAIM_BUCKET_REFILL: f64    = 0.2;      // ≈ one claim / 5 s — degrade-closed on burst

// ── kernel/src/json_api.rs — NEW marketing-schema projection (§4.4), beside P69's catalog pack ──
/// schema.org marketing/product JSON-LD for dowiz.org — the load-bearing AEO substrate (R1 §7),
/// but the TYPE is `WebSite` + `Organization` + `SoftwareApplication` (the open hub software),
/// NEVER `Restaurant`/`Menu`/`Offer` (there is no menu on dowiz.org — §16.21 anti-scope §2.2).
pub fn landing_jsonld(canonical_url: &str, hub_source_url: &str) -> String;
/// The landing pack — a SEPARATE, thinner artifact than P69's `BotPack` (NO catalog fields).
/// `sitemap_xml` is fed dowiz.org's OWN pages ONLY (a fixed const set), never a vendor-slug set.
pub struct LandingBotPack {
    pub robots_txt: String, pub sitemap_xml: String, pub landing_jsonld: String,
    pub open_graph: String, pub web_manifest: String, pub llms_txt: String,
}
pub fn build_landing_bot_pack(canonical_url: &str, hub_source_url: &str) -> LandingBotPack;
// REUSES P69's slug-agnostic pure fns VERBATIM (do NOT re-implement):
//   json_api::robots_txt(sitemap_url), json_api::sitemap_xml(&[own_pages]),
//   json_api::open_graph_tags(...), json_api::web_manifest(...), json_api::llms_txt(...)
```

Rejected alternatives (DECART one-liners): **a `Restaurant`/`Menu` JSON-LD on the landing page** —
rejected: dowiz.org sells no food and lists no vendors (§16.21); the correct schema is
`SoftwareApplication`/`Organization`/`WebSite` for an open-source product page. **a `slug`/`VendorId`
field anywhere in P73** — rejected: it would make a vendor listing *representable*; the anti-scope
is enforced by the type system's silence (§5.1). **P73 owning a `ClaimService` impl** — rejected:
that is P67's closed infra (§16.54); P73 owns only the `ClaimServicePort` **client** trait + a mock.
**a bool `claimed: bool` instead of the `ClaimJourney` FSM** — rejected: a bool cannot distinguish
`PoolEmpty→interest` from `Failed→retry`, the exact branch the conversion flow turns on (§4.3).

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 4.1 M1 — the `ClaimJourney` conversion FSM (pure; Lane A, lands NOW)

`journey.rs` per §3 — the closed funnel. Transitions: `Landing → FormOpened → FormEntry →
SubmitRequested → (ChallengePassed → Submitting) → [ClaimAssigned → Claimed | PoolEmptyFellBackToInterest
→ InterestRegistered]`; `ChallengeFailed`/`TransportFailed → Failed`; `Failed → Retried → FormEntry`
(the form value survives a retry — it is not cleared on a recoverable failure). The **load-bearing
rule**: `PoolEmpty` is NOT `Failed` — it deterministically routes to `register_interest` and lands
in `InterestRegistered` (a pool-empty visitor is a *lead*, not an error). RED→GREEN (event-sequence
form): `journey_happy_fast_path` — the full `Started..ClaimAssigned` sequence yields `Claimed` with
the returned `hub_url`; `journey_pool_empty_becomes_interest` — a `ClaimOutcome::PoolEmpty` folds to
`PoolEmptyFellBackToInterest → InterestAcked → InterestRegistered`, **never** `Failed`;
`journey_retry_preserves_form` — a `TransportFailed` then `Retried` returns to `FormEntry` with the
same `SignupForm` values. **Adversarial (designed to break):** a `ChallengeFailed` **must not**
reach `Submitting` (assert no claim request is emitted when the challenge fails — the pool-drain
guard, §5.1); a duplicate `SubmitRequested` while `Submitting` is a no-op (single-outstanding-claim,
`CLAIM_BUCKET_CAPACITY = 1.0`) — the test that fails if a double-click fires two claims and consumes
two never-reclaimed pool slots (§16.57).

### 4.2 M2 — the signup form over P57 `TextField` (Lane A wiring, Lane B render)

`form.rs` per §3: three `TextField`s (contact/venue_name/notes), each a P57 editor (BLUEPRINT-P57
§3) — **no DOM `<input>`** (§16.34, inherited). Validation is at the **submit boundary** only:
`contact` non-empty and ≤`CONTACT_MAX_BYTES`; `venue_name` non-empty and ≤`VENUE_NAME_MAX_BYTES`;
`notes` optional ≤`NOTES_MAX_BYTES`. A `Submit` `EditEvent` from any field surfaces the journey's
`SubmitRequested`. The GitHub CTA (§4.5) and the "install the app" secondary CTA are non-editable
scene affordances, not fields. RED→GREEN: `form_requires_contact_and_venue` — submit with an empty
`contact` is refused at the boundary, journey stays in `FormEntry` with a typed
`FormError::MissingContact` (never a silent drop — self-termination leg, §5.4);
`form_carries_field_values_into_request` — a filled form maps to a `ClaimRequest`/`InterestSubmission`
byte-for-byte via `TextField::value()`. **Adversarial:** paste a mixed-script `"café中"` into
`venue_name` — P57's scope gate refuses the `中` per-grapheme (BLUEPRINT-P57 §4.2), the Latin
remainder survives, and the form still submits (the v2-script boundary holds at the landing form,
inherited not re-built); a `contact` of exactly `CONTACT_MAX_BYTES+1` is refused at the cap, caret
unchanged (P57 inherited).

### 4.3 M3 — the claim-service client leg (Lane A mock, Lane B real transport)

`claim_client.rs` per §3 — the `ClaimServicePort`. The flow, straight from R3 §6.1: **(fast path)**
`claim_warm_pool_hub(ClaimRequest)` → on `Claimed(ClaimedHub)` the journey advances to `Claimed`
(the hub is *already online* at `hub_url`, fixture-populated — R3 §6.1 step 3); on `PoolEmpty` the
client **immediately and deterministically** calls `register_interest(InterestSubmission)` (R3 §6.1
step 5 — the non-pool path) and advances to `InterestRegistered`. A `Timeout`/`Transport` error →
`Failed` (retryable; degrade-closed — after `CLAIM_REQUEST_TIMEOUT_MS` the UI offers the interest
path explicitly rather than hanging). The challenge is verified **at the edge before** the request
is accepted (X11); the claim service **re-verifies** it (defense in depth, §5.1). A Wave-0 **mock
adapter** (`MockClaimService`) returns scripted `Claimed`/`PoolEmpty`/`Timeout` outcomes so the whole
funnel is testable with **zero** network and **before P67 lands**. RED→GREEN: `client_claimed_advances`
(mock returns `Claimed` → journey `Claimed`, `hub_url` rendered); `client_pool_empty_calls_interest`
(mock `PoolEmpty` → exactly one `register_interest` call → `InterestRegistered`);
`client_timeout_offers_interest` (mock `Timeout` → `Failed`, and the recovery affordance is the
interest path, not an infinite spinner). **Adversarial:** a `Claimed` response whose `owner_root_cert`
is empty bytes → the handoff (§4.6) treats it as a **transport failure** (`Failed`), never a
"successful claim with no credential" (a claimed hub with no owner root is an unusable hub — the test
that fails if P73 renders success on a malformed response); two rapid `SubmitRequested` → the second
is rate-limited by the `TokenBucket` (`try_acquire(1.0)` false), **not** a second pool consumption.

### 4.4 M4 — the landing bot-pack (marketing schema; JSON-LD load-bearing; NO catalog, NO vendor slugs)

A `kernel/src/json_api.rs` extension emitting a **separate, thinner** pack than P69's, ranked per
R1 §7 (**JSON-LD first, `llms.txt` a secondary extra**), and **generated from fixed dowiz.org facts,
not from any catalog** (there is none):

1. **schema.org JSON-LD (load-bearing AEO substrate, R1 §7):** `landing_jsonld(canonical_url,
   hub_source_url)` emits a `WebSite` + `Organization` (dowiz) + `SoftwareApplication` (the open
   hub software: `applicationCategory: BusinessApplication`, `offers` = free/open-source AGPLv3,
   `codeRepository: hub_source_url`) graph. **NEVER** `Restaurant`/`Menu`/`MenuItem`/`Offer` — that
   is P69's catalog schema for a *venue's* `/s/:slug`, categorically wrong for dowiz's own product
   page (§2.2). No `SearchAction`, no `ItemList` of venues.
2. **`robots.txt` + `sitemap.xml`:** **reuse P69's slug-agnostic pure fns verbatim** —
   `json_api::robots_txt(sitemap_url)` and `json_api::sitemap_xml(&own_pages)`, where `own_pages` is
   a **fixed const set of dowiz.org's own URLs** (`/`, the install page, the GitHub redirect) — and
   **contains zero `/s/:slug` / vendor entries** (the structural anti-scope guarantee, §5.1).
3. **Open Graph + `manifest.json`:** link-unfurl + installability facts for the product page
   (reuse `json_api::open_graph_tags`/`web_manifest`).
4. **`llms.txt`:** the forward-looking extra (R1 §7 — cheap, and dowiz's product IS agent-facing,
   but **not a crawlability bet**; JSON-LD is the load-bearing one). Curates links to the GitHub
   source + install page — a routing file for agents, not an SEO bet.

The web side (`landing/bot_pack.mjs`) writes these to static files at deploy time (Cloudflare Pages
build output) — a **separate output path from the a11y mirror** (§16.55). RED→GREEN:
`landing_jsonld_is_software_not_restaurant` — the emitted JSON-LD `@type` set is
`{WebSite, Organization, SoftwareApplication}` and contains **no** `Restaurant`/`Menu`/`Offer` key;
`sitemap_has_no_vendor_slugs` — the emitted `sitemap.xml` matches only dowiz.org's own fixed pages
and contains **zero** `/s/` substrings (the falsifiable anti-scope test, §6 not-done clause);
`llms_txt_is_secondary` — the pack is valid with JSON-LD present even if `llms.txt` is empty.
**Adversarial (the anti-scope's teeth):** attempt to build the pack with a vendor-slug argument —
there is **no parameter to pass one** (the fn signature takes `canonical_url`/`hub_source_url` only),
so "a vendor leaked into dowiz.org's sitemap" is unrepresentable, asserted by the absent parameter +
the `sitemap_has_no_vendor_slugs` grep; hand-edit a `Restaurant` node into the emitted JSON-LD →
`landing_jsonld_is_software_not_restaurant` goes RED (the schema-type gate is the tripwire).

### 4.5 M5 — the landing scene + a11y + floor-parity (full wgpu; the demo IS the product)

The scene is composed through the **existing** P38 engine (§16.56 — no static-page exception), as a
sequence of sections rendered into the ONE frame (zero new render math): (a) **hero** — a live,
interactive field-engine canvas (Sea ambient field + "dowiz" brand SDF via FE-06 MSDF text, P38
§3.3) — the product demoing itself (§1.2); (b) **narrative** — a short value story (sovereign,
own-your-hub, no-percentage, open-source delivery infra for venue owners) as FE-06 text + SDF
affordances; (c) **the signup form** (§4.2); (d) **the GitHub CTA** — a prominent, non-editable
affordance linking `HUB_SOURCE_URL` (§16.32); (e) a secondary **"install the app"** CTA (§16.8 /
§16.21). Each journey step authors a `SemanticScene` (P58 §M1) so the a11y mirror reconciles from
the **same** state the renderer consumes (X1 draft-parity by construction). P73 **imports**
`a11yGate(page, manifest)` (P58 §4.5) with a per-screen mirror-node budget as a DoD gate, and
carries the **floor-parity** DoD line: the landing renders correctly on the FE-16 WebGL2 **and** CPU
floors (SYNTHESIS §3.2 rationale 7 — ~18% of web users have no WebGPU, and §16.56 forbids serving
them a different static page). RED→GREEN: `landing_a11y_tree_has_form_and_cta` (Playwright a11y-tree
snapshot — P58's harness — asserts a `role=textbox` per form field, a labeled GitHub link, and the
hero as a described `img`/region); `landing_renders_on_webgl2_floor` (`navigator.gpu = undefined` →
the scene + form + a11y assertions still pass — the FE-16 CPU/WebGL2 ladder, inherited P38 §3.6).
**Adversarial:** force `navigator.gpu = undefined` **and** assert the page is **not** replaced by a
static-HTML fallback (grep the served bundle: it is the same wgpu WASM app on its CPU floor, not a
`index.static.html`) — the test that fails if someone "optimizes" the no-WebGPU path into a
forbidden static page (§16.56).

### 4.6 M6 — the claimed-hub handoff (opaque cert forwarded once, never held)

On a fast-path `Claimed(ClaimedHub)`, the success state renders: the working `hub_url`, a **deep-link
into the P70 owner surface** at that hub (the vendor's first landing inside their claimed,
fixture-populated hub — R3 §6.1 step 3), and hands the **opaque** `OwnerRootCert(Vec<u8>)` off to the
**wallet / installed client (P66/P70)** — the custody boundary (§2.2). P73 forwards the cert bytes
**exactly once** to the handoff sink and retains **no** copy; it never parses, validates, or mints
it (the cert format is P59's, verified where the hub/wallet uses it — never self-certified here,
§5.1). On the slow path, `InterestRegistered(InterestAck)` renders an honest "we'll be in touch"
state with the `ack_id`. RED→GREEN: `claimed_handoff_forwards_cert_once` (a `Claimed` outcome invokes
the handoff sink exactly once with the exact `owner_root_cert` bytes, and P73 holds no residual copy
after); `interest_state_shows_ack` (`InterestRegistered` renders the `ack_id`, no hub URL).
**Adversarial:** a `Claimed` with `fixtures_ready = false` renders a "provisioning, check back"
state rather than deep-linking into a not-yet-ready hub (honest status, no forged readiness — the
same forged-success class P69 §5.1 forbids); the handoff sink being absent (no wallet/installed
client present, e.g. first-visit web with no P66) → the cert is offered as an explicit
save/download step, **never silently dropped** (a dropped owner root strands the vendor's fleet —
R3 risk #2 territory).

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose:

- **A vendor listing on dowiz.org is unrepresentable.** There is **no** `VendorId`, `slug`,
  catalog, menu, or search type anywhere in P73's modules; `build_landing_bot_pack` takes
  `canonical_url`/`hub_source_url` **only** — there is no parameter through which a vendor could
  enter the sitemap or JSON-LD. "dowiz.org listed/browsed a vendor" is a **type-level-unreachable**
  state, falsified by the anti-scope grep gate + `sitemap_has_no_vendor_slugs` (§6). §16.21 is
  enforced by the absence of the types, not by a policy check.
- **The warm pool cannot be drained by a bot storm through the landing page.** Every claim leaves
  the landing page only with a valid edge-verified `ChallengeToken` (Turnstile/ALTCHA at the CF
  edge — X11, never canvas-embedded, X3 DOM tension), passes the client-side `TokenBucket`
  (`kernel/src/token_bucket.rs`, `CLAIM_BUCKET_CAPACITY = 1.0` single-outstanding), and the claim
  service **re-verifies** the challenge before touching a slot (defense in depth). The
  `journey_challenge_fail_no_request` + `client_rate_limits_double_submit` tests make "a claim
  reached the pool without a challenge" a tested-unreachable state. (The pool-consumption authority
  is P67's; P73's contribution to the invariant is that **no claim leaves the funnel un-challenged**
  — the §16.57 no-reclaim rule makes each un-gated claim an irrecoverable cost.)
- **No money exists on dowiz.org, so the money-tween hazard class is structurally absent.** dowiz.org
  is pure infrastructure (§16.21) — no cart, no price, no `Money`. `money_guard.rs`'s `Money(i64)`
  never appears in the landing scene; there is nothing to tween, mis-round, or interpolate. The
  🔴 RED-LINE is honored by *absence of the surface*, not by a guard on it (stated precisely, not
  decoratively).
- **P73 cannot forge or leak a capability-cert.** P73 holds **no** signing key and treats
  `OwnerRootCert` as **opaque bytes** it forwards exactly once (§4.6). Cert issuance is P67/P59;
  verification happens where the hub/wallet consumes it (P7 GENDER — refereed elsewhere, never
  self-certified here). "The landing page minted/forged an owner root" is unreachable because P73
  has no minting primitive and no cert-parsing code.
- **No DOM input exists** (inherited P57/P38 §12.1). The form fields are P57 `TextField`s; the only
  DOM P73 touches is P58's a11y mirror (a projection, never an input). The anti-abuse **challenge
  lives at the CF edge layer**, not as a canvas-embedded widget (X11) — keeping the canvas
  DOM-free, exactly as X3 keeps the payment surface off the canvas.

### 5.2 Schemas & scaling axes (item 8)

`ClaimJourney`: axis = **concurrent visitors**; the landing WASM is **per-visitor, stateless** and
served from the Cloudflare Pages CDN edge (§16.1) — it scales as static assets do. The scaling wall
is **not P73's**: it is the warm-pool depth + the 1,000-tunnel-per-CF-account cliff (R3 §1.4), both
**P67's** axes. Break point for P73: none within the page itself. **`LandingBotPack`:** axis =
**pages on dowiz.org** → sitemap size; Wave-0 is a handful of fixed pages (landing, install, GitHub
redirect) → a few hundred bytes. The load-bearing guarantee is that the sitemap **does not scale
with vendor count** — there are no vendors on it (§2.2), which is the structural anti-scope
property, not a size optimization. `SignupForm`: O(1) — three fields, single-focus (P57 invariant).

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation:** the landing page is a **separate Cloudflare Pages deployment**, node-local and
stateless; it holds **no kernel state, no hub state, no order state**. A landing-page panic cannot
corrupt any hub or order — its only outbound effect is one claim/interest request to P67's service
(the trust boundary, R3 §1.5), which P73 is a thin client of. The render stack inherits P38 §4.3's
bulkhead; a hero-demo render failure degrades to the FE-16 floor, never to a different page (§4.5).
**Mesh:** the landing page originates **zero mesh payload**. The claim request is a client→claim-
service call over dowiz-run **closed** infra (§16.54), **not** a gossip/`SyncFrame` — it does not
touch `iroh_transport.rs`/`discovery.rs`. Once handed off, the *claimed hub* joins the mesh on its
own (P34/P37) — not P73's concern. **Living memory:** P73 stores **nothing** across sessions. The
signup form is transient (a reload restarts at `Landing`); the `OwnerRootCert` is forwarded to P66's
wallet, never held by the landing page. There is no temporal/topological access pattern — the
landing is a **stateless funnel**, deliberately *not* a living-memory participant (contrast P66,
which owns the wallet's persisted state, cited not duplicated).

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

- **Self-Termination leg claimed:** the typed `ClaimError`/`FormError` refusals; `PoolEmpty` as a
  deterministic non-error route; the challenge-gate-before-pool-touch; the unrepresentable vendor
  listing (a type-level boundary). These are hard invariant boundaries, not a supervisor's decision.
- **Self-Healing: NOT claimed.** A failed claim is retried by the visitor (or falls to the interest
  path); nothing error-corrects itself — claiming self-healing here would be loose use of the word.
- **Snapshot-Re-entry: NOT claimed.** The signup form is transient and deliberately **not**
  persisted (§1.3 rejected alt): a signup is trivially re-enterable, so no query-before-replay draft
  machinery (that is P66's, for order drafts) is warranted. A page reload restarts cleanly at
  `Landing`.
- **Mechanical rollback:** the landing is a Cloudflare Pages deployment — rollback = redeploy the
  prior build (CF Pages native atomic/immutable deploys, §16.1). The degrade fallback for a
  no-WebGPU visitor is the **FE-16 CPU/WebGL2 floor of the same wgpu app**, never a static-HTML page
  (§16.56 — the not-done clause, §6).

### 5.5 Linux discipline (item 9) + tensor/spectral/eqc (item 16)

Verdicts per the adoption framework: **ALREADY-EQUIVALENT** — one render pipeline (P38 engine
renders the hero demo; no bespoke landing renderer), one text editor (P57 `TextField` for every
form field), one a11y harness (P58's `a11yGate` imported), one bot-pack authority
(`kernel/src/json_api.rs` — P73 adds a projection beside P69's, reusing `robots_txt`/`sitemap_xml`
verbatim), one rate limiter (`kernel/src/token_bucket.rs`). **REINFORCES** — the bot-pack as a
**separate output path from the a11y mirror** (§16.55, the same discipline P69 applies); the
feature-gated `json-api` offline-clean default (the `landing_*` fns are `#[cfg(feature = "json-api")]`,
default build pulls no serde). **EXTENDS** — a **new schema projection** (`WebSite`/`Organization`/
`SoftwareApplication` marketing JSON-LD) added to the bot-pack authority alongside P69's `Restaurant`
catalog projection — the module gains a second, non-catalog output; and a new **stateless-funnel
FSM** discipline for a public marketing surface. **GAP** honestly named — **no live browser CI
runner** exists for the wgpu landing on real GPUs (same GAP as P57/P69); the Playwright specs run
headless (a11y-tree + mock-claim-service + bot-pack anti-scope), and real on-device WebGPU/soft-
keyboard behavior is unverified until P45/P63 provides a display runner — each web `#[ignore = "O18a"]`
marker doubles as that GAP marker. **DOES-NOT-TRANSFER** — n/a. **Item 16 (tensor/spectral/eqc):
NOT load-bearing, stated not decoratively invoked** — a landing page has no closed-form math organ;
the hero field-demo IS the engine's existing physics (P38's ONE Laplacian), reused not re-derived,
so `eqc-rs` does not apply and no spectral machinery is summoned (the Anu/Ananke discipline forbids
ritual math — P51/P57 §5.5 precedent).

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2), incl. the end-to-end claim test

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no `ClaimJourney`; sequence tests RED | `journey_happy_fast_path`; **`journey_pool_empty_becomes_interest`** (PoolEmpty ≠ Failed); `journey_retry_preserves_form`; `journey_challenge_fail_no_request`; double-submit no-op | **pool-empty-routes-to-interest test** (ledger row) |
| M2 | no `SignupForm` | `form_requires_contact_and_venue` (typed `FormError`, no silent drop); `form_carries_field_values_into_request`; mixed-script `venue_name` (P57 scope inherited) | form-boundary-validation test |
| M3 | no `ClaimServicePort`/mock | `client_claimed_advances`; **`client_pool_empty_calls_interest`**; `client_timeout_offers_interest`; empty-cert→Failed; **`client_rate_limits_double_submit`** | **single-outstanding-claim (pool-drain) test** (ledger row) |
| M4 | no landing bot-pack | **`landing_jsonld_is_software_not_restaurant`**; **`sitemap_has_no_vendor_slugs`**; `llms_txt_is_secondary`; hand-edited-`Restaurant`-node adversarial | **no-vendor-catalog / no-restaurant-schema test** (ledger row) |
| M5 | no landing scene / a11y | `landing_a11y_tree_has_form_and_cta`; **`landing_renders_on_webgl2_floor`**; no-static-fallback adversarial | **full-wgpu-no-static-page + floor-parity test** (ledger row) |
| M6 | no handoff | `claimed_handoff_forwards_cert_once` (opaque, once, no residual); `interest_state_shows_ack`; `fixtures_ready=false` honest state; absent-sink offers save | cert-forward-once test |
| **E2E** | no funnel harness | **`e2e_visitor_to_claimed_hub`** (§below) | **the end-to-end claim regression** (ledger row) |
| default build | — | `default_json_api_pulls_no_serde` (reuse P69/P37 gate: `cargo tree --no-default-features \| grep -c serde == 0`) | offline-clean guard test |

**The falsifiable end-to-end claim test (`e2e_visitor_to_claimed_hub`, Playwright, the DoD keystone):**
drive the deployed landing page (or its CPU-floor build) → assert the hero + GitHub CTA render and
appear in the a11y tree (P58 harness) → fill the P57 form (contact + venue_name) → pass a **stubbed
edge challenge** → with a **`MockClaimService` returning `Claimed`**, assert the journey reaches
`Claimed`, the `hub_url` is rendered, and the handoff sink receives the exact `owner_root_cert` bytes
once; then, with the **mock returning `PoolEmpty`**, re-drive and assert the journey reaches
`InterestRegistered` with an `ack_id` (**not** `Failed`); and assert the emitted `sitemap.xml`
contains **zero** `/s/` vendor entries and the JSON-LD `@type` set excludes `Restaurant` (the
anti-scope leg baked into the same e2e). This one test proves the whole conversion contract + the
anti-scope in one run; it is the CI tripwire, not a report.

**Not-done clauses:** any vendor catalog / directory / listing / "browse restaurants" / search /
"near you" surface = **NOT done** regardless of green totals (🔴 §16.21 categorical); a static-HTML
landing page, or a static fallback served to no-WebGPU visitors instead of the wgpu CPU floor = NOT
done (§16.56); a `/s/:slug` or any vendor entry in `sitemap.xml`, or a `Restaurant`/`Menu`/`Offer`
node in the landing JSON-LD = NOT done; the claim endpoint reached without a verified
`ChallengeToken` = NOT done (pool-drain); a DOM `<input>` on the form = NOT done (P57/P38 inherited);
P73 parsing/minting/long-term-storing the `OwnerRootCert` = NOT done (P59/P66/P70 boundary); an
`#[ignore = "O18a"]` test silently deleted instead of un-ignored at unlock = NOT done.

---

## 7. Benchmark plan (item 10) — the hero-demo frame budget + the funnel latency

Budgets (mid-tier device class, P38 §6's 16.6 ms/frame split): the **hero field-demo holds
16.6 ms/frame** (it is the P38 engine — its budget, cited not re-derived), and the form keystroke
rides P57's **≤2 ms keystroke→`EditEvent`** (§4.2); **time-to-first-frame on the Cloudflare Pages
CDN edge** is the marketing-critical number (measured against the edge cache, not asserted); the
**claim round-trip** is dominated by P67's service — P73 measures only its **own client leg + the
`CLAIM_REQUEST_TIMEOUT_MS = 8000` degrade-closed ceiling** (the interest-path fallback must appear
within that bound). Criterion benches `landing/benches/journey.rs`: `journey_fold_fast_path`,
`bot_pack_build` (the pure-fn pack emission) — added **RED-commit-first** so the `bench_track`
baseline auto-seeds (P-A §6 / P51 §7 / P69 discipline, same `BENCH_HISTORY.md` append rule).
Telemetry: claim-attempt / pool-empty-fallback / challenge-fail counts + funnel-stage transitions
ride the existing native-trackers hooks (P-H's lane) so a conversion-drop or a challenge-fail spike
surfaces without review. The `e2e_visitor_to_claimed_hub` test (§6) doubles as the funnel tripwire —
the benchmark IS a test (P38/P57/P69 pattern).

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (W3 table, P73 scope + deps), §3.1/§3.2-7 (build order +
floor-parity standing gate), X1 (a11y draft-parity), X7 (bot-pack invariant — P73's is the
catalog-free sibling), X11 (edge anti-abuse / TokenBucket) ·
`docs/research/OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` §7 (SEO/AEO: JSON-LD load-bearing,
`llms.txt` secondary), §1 (GPUI/Zed — the demo IS the product) ·
`docs/research/OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md` §6.1 (**the claim-mechanic
implementation P73's form hands off to — the contract source until P67 lands**), §1.2 (`hub-<id>.hubs.dowiz.org`),
§1.5 (the provisioning-service trust boundary P73 is a client of), §0 + risk #2 (owner-root loss —
why the cert handoff never silently drops) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.21 (no vendor catalog — anti-scope), §16.32
(claim/interest form + GitHub link + minimal homepage), §16.55 (bot-facing files, separate from
a11y), §16.56 (full-wgpu landing, no static-page exception), §16.57 (no-reclaim, net-consumed pool),
§16.1 (Cloudflare Pages hosting), §16.54 (closed dowiz infra vs open hub software) ·
`BLUEPRINT-P57-canvas-text-input.md` §3/§12 (the `TextField` form fields; P57 names P73 a consumer) ·
`BLUEPRINT-P58-a11y-mirror-everywhere.md` §M1/§4.5 (`SemanticScene` + the imported `a11yGate` harness) ·
`BLUEPRINT-P59-capability-cert-chain.md` (the `OwnerRootCert` format P73 forwards opaque) ·
`BLUEPRINT-P69-customer-storefront-checkout.md` §4.6 (checkout-journey rigor precedent), §4.7 +
`kernel/src/json_api.rs` (the bot-pack pattern + the reused `robots_txt`/`sitemap_xml` pure fns) ·
`BLUEPRINT-P38-webgpu-render-engine.md` §3.3/§3.6/§12 (the render engine + FE-16 floor + no-DOM canon) ·
`BLUEPRINT-P51-open-map-routing.md` (rigor/format precedent) ·
`docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md` (O18a gate, shared) ·
`docs/regressions/REGRESSION-LEDGER.md` (five+ rows named in §6). **In-parallel W3 sibling P73
consumes:** **P67** (`BLUEPRINT-P67-hub-provisioning-claim.md` — the claim service; greenfield this
pass, so P73 cites R3 §6.1 as the contract and carries a HARD `RECONCILE-P67` obligation on the wire
shapes, §3/§12) and its co-owner **P68** (supervisor/backup — not P73's concern). **Consumers P73
feeds:** the operator's inbox (interest submissions) and P67's warm pool (claim assignments) — plus
the **P70 owner surface** as the claimed-hub deep-link target (§4.6). Memory:
`physics-ui-capture-quantum-math-arc-2026-07-14` (the hero demo IS the engine's ONE Laplacian) ·
`field-ui-engine-arc-2026-07-13` (FE-06/FE-16 substrate) · `ecosystem-strategy-arc-2026-07-13`
(dowiz.org = INFRA kit, not a discovery surface) · `open-source-goal-adr020-2026-07-03` (the GitHub
link's public-repo gate) · `anu-ananke-strict-discipline-feedback-2026-07-17` (style; §5.5's refusal
to invoke spectral math decoratively) · `test-integrity-rules-2026-06-27` (money 🔴 RED-LINE — here,
absent-by-construction, §5.1). Supersedes: nothing.

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the `ClaimJourney`/`JourneyEvent` FSM + the `ClaimServicePort`
  + the marketing-schema bot-pack types (§3) precede every line of implementation; the claim wire
  shape is bound to a spec surface (the mock), never free-handed into a network call.
- **P2 CORRESPONDENCE** (one concept, one primitive): one render engine (P38 hero demo), one text
  editor (P57), one a11y harness (P58), one bot-pack authority (`json_api.rs`, reusing P69's pure
  fns), one rate limiter (`token_bucket.rs`) — the landing composes existing primitives, forks none.
- **P6 CAUSE-AND-EFFECT** (determinism as law): event-sequence tests (not end-state); `PoolEmpty`
  deterministically routes to `InterestRegistered`, never `Failed`; a challenge-fail deterministically
  emits **no** claim request — every branch carries a falsifier (§4, §6).
- **P7 GENDER** (paired verification, no self-certification): the claim outcome is refereed by the
  claim service (P67) + the Playwright e2e, never by P73's own optimism; the `OwnerRootCert` is
  verified where the hub/wallet consumes it (P59/P66), never self-certified on the landing page; the
  a11y projection is refereed by P58's accessibility-tree harness, never by its own reconcile code.

(P3/P4/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the greenfield finding; the P67-not-on-disk + `json_api.rs`-exists nuances) |
| 2 DoD | §6 (incl. the end-to-end `e2e_visitor_to_claimed_hub` keystone) |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; §4.1/§4.3 event-sequence assertions |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4.1–§4.6 (double-submit pool-drain, mixed-script paste, empty-cert→Failed, hand-edited `Restaurant` node, no-static-fallback, absent-handoff-sink) |
| 6 hazard-safety as math | §5.1 (unrepresentable vendor listing, un-drainable pool via edge challenge, money absent-by-construction, unforgeable opaque cert, no-DOM-input) |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (each with a named break point; the pool/tunnel cliff is P67's, not P73's) |
| 9 Linux discipline | §5.5 (all verdict classes incl. an honest GAP) |
| 10 benchmarks+telemetry | §7 (P38 frame budget, CDN TTFF, degrade-closed ceiling, bench_track seeding, funnel tripwire) |
| 11 isolation/bulkhead | §5.3 (separate CF Pages deploy; node-local stateless; thin client of P67's trust boundary) |
| 12 mesh awareness | §5.3 (zero mesh payload; claim is closed-infra client call, not gossip) |
| 13 rollback/self-heal vocabulary | §5.4 (self-termination claimed; self-healing + snapshot refused precisely; CF Pages atomic rollback) |
| 14 error-propagation gates | §6 (named ledger rows), §5.1 (typed refusal classes; type-level anti-scope) |
| 15 living memory | §5.3 (stateless funnel; cert forwarded to P66's wallet, stored nowhere by P73) |
| 16 tensor/spectral + eqc reuse | §5.5 (honestly NOT invoked — no closed-form organ; the hero demo reuses P38's Laplacian) |
| 17 regression ledger | §6 (five+ rows incl. the end-to-end claim regression) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§1.3 (P38/P57/P58/P69-json_api/token_bucket all reused; five rejected alternatives §3; the client-vs-own-service comparison §2.2) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Two lanes (§2.3). **Lane A (buildable TODAY, no network):** T1–T4, T5-scaffold. **Lane B (blocked on
O18a for wgpu + on P67 for the real claim transport — do NOT `cargo add wgpu/cosmic-text` without the
network grant; do NOT hardcode P67's wire shape before it lands, use the mock):** T5-render, T6.

1. **T1 (M1 — the conversion FSM first; pure, Lane A).** Create `landing/journey.rs` per §3
   (`ClaimJourney`, `JourneyEvent`). Write RED first: `journey_happy_fast_path`,
   `journey_pool_empty_becomes_interest`, `journey_retry_preserves_form`,
   `journey_challenge_fail_no_request`, double-submit no-op (§4.1). Acceptance: the crate's
   `cargo test journey` green; ledger row added.
2. **T2 (M3 client + mock — pure, Lane A).** Create `landing/claim_client.rs` per §3 —
   `ClaimServicePort`, `ClaimRequest`/`InterestSubmission`/`ClaimOutcome`/`ClaimedHub`/`ClaimError`,
   and a `MockClaimService` returning scripted outcomes. Wire the `TokenBucket` claim limiter
   (`kernel/src/token_bucket.rs`, `CLAIM_BUCKET_CAPACITY`). RED: `client_claimed_advances`,
   `client_pool_empty_calls_interest`, `client_timeout_offers_interest`, empty-cert→Failed,
   `client_rate_limits_double_submit` (§4.3). Acceptance: green (no network).
3. **T3 (M2 form contract — Lane A wiring).** Create `landing/form.rs` — `SignupForm` over three
   P57 `TextField`s (contract only where P57 is not yet landed: stub `TextField::value()`), the
   submit-boundary validation, the `ChallengeToken` seam. RED: `form_requires_contact_and_venue`,
   `form_carries_field_values_into_request` (§4.2). Acceptance: green; `RECONCILE-P57` TODO where
   the `TextField` API is stubbed.
4. **T4 (M4 landing bot-pack — pure, Lane A).** Extend `kernel/src/json_api.rs` (feature `json-api`)
   with `landing_jsonld`, `LandingBotPack`, `build_landing_bot_pack` per §3/§4.4 — **reuse**
   `json_api::robots_txt`/`sitemap_xml`/`open_graph_tags`/`web_manifest`/`llms_txt` (do NOT
   re-implement; if P69 has not landed them yet, coordinate — P69 owns those pure fns). Feed
   `sitemap_xml` a **fixed const set of dowiz.org's own pages only**. RED:
   `landing_jsonld_is_software_not_restaurant`, `sitemap_has_no_vendor_slugs`, `llms_txt_is_secondary`,
   the hand-edited-`Restaurant`-node adversarial (§4.4). Keep `default_json_api_pulls_no_serde`
   green. Acceptance: `cargo test -p dowiz-kernel --features json-api` green; ledger row added.
5. **T5 (M5 scene + a11y — scaffold Lane A, render Lane B).** Author a `SemanticScene` per journey
   step and the `landing/bot_pack.mjs` emission (Lane A); wire the hero field-demo through P38's
   engine + P57 glyph render (Lane B, O18a). Create `web/tests/landing.spec.mjs` with the §4.5
   assertions (`landing_a11y_tree_has_form_and_cta`, `landing_renders_on_webgl2_floor`, the
   no-static-fallback adversarial). **Import P58's `a11yGate` harness — cite it, leave `RECONCILE-P58`
   TODOs** where the ARIA convention is provisional. Acceptance: Playwright specs green headless;
   render specs `#[ignore = "O18a"]` until the engine unlock.
6. **T6 (M6 handoff + the real transport + E2E — Lane B).** Implement the claimed-hub success/interest
   states and the opaque-cert handoff (§4.6). Swap `MockClaimService` for the **real** HTTP transport
   **only once P67 lands its wire format** (`RECONCILE-P67`). Write the DoD keystone
   `e2e_visitor_to_claimed_hub` (§6) against the mock (works pre-P67). Acceptance: the e2e green
   headless; ledger rows present in `docs/regressions/REGRESSION-LEDGER.md`; deploy target =
   Cloudflare Pages (§16.1, §4.6).

---

## 12. Dependencies & blocks (the wiring, stated once)

**Inputs P73 depends on (cited, never redefined):**

| Input | What P73 takes from it | Reconciliation obligation |
|---|---|---|
| **P67** (`BLUEPRINT-P67-hub-provisioning-claim.md` — the claim service; **greenfield this pass**) | The claim-service API: `claim_warm_pool_hub`/`register_interest`, the `ClaimedHub` shape (`hub_url`, `owner_root_cert`, `fixtures_ready`), `PoolEmpty` semantics, the interest→operator-notify path. Contract source until P67 lands = **R3 §6.1 steps 1-5** | **HARD `RECONCILE-P67`:** P73's `ClaimServicePort` shapes are provisional; the `MockClaimService` is the stand-in; on divergence, P73 adopts P67's wire format (one contract, one client). P73 implements NO claim/pool/cert/CF/Hetzner logic. |
| **P57** (canvas text input) | `TextField`/`EditCmd`/`value()`/`Intent::Text` for the three form fields | P73 cites, does not fork; `RECONCILE-P57` where the API is stubbed pre-landing |
| **P58** (a11y-mirror-everywhere) | The `SemanticScene`/`a11yGate` harness + ARIA-textbox convention (draft-parity X1) | P73 imports the harness as a DoD gate; `RECONCILE-P58` on the provisional convention |
| **P59** (capability-cert chain) | The `OwnerRootCert` **format** (biscuit-style hybrid-signed chain) — P73 treats it OPAQUE | P73 never parses/mints it; forwards once (§4.6) |
| **P66 / P70** (wallet / owner surface) | The cert **custody** sink + the claimed-hub deep-link target | P73 hands off; storage/autofill is P66's, the owner surface is P70's |
| **P69** (storefront + bot-pack) | The `kernel/src/json_api.rs` bot-pack module + the reused `robots_txt`/`sitemap_xml` pure fns | P73 adds a `landing_*` projection beside P69's catalog one; reuses the slug-agnostic fns verbatim |
| **P38 / P63 / P39-rev** (engine + shell + floor) | The render engine (hero demo), the FE-16 WebGL2/CPU floor, the web-first shell verdict | P73 consumes; the demo IS the engine (§1.2), not a second renderer |

**Consumers P73 feeds:** the **operator's inbox** (interest submissions, manual follow-up — §16.32),
**P67's warm pool** (claim assignments — the net-consumed pool, §16.57), and the **P70 owner surface**
(the claimed-hub deep-link, §4.6). P73 is a **terminal/leaf public surface** — no blueprint builds a
typed field or a screen *on* P73.

**Build ordering (SYNTHESIS §3):** P73 is a **W3** blueprint — written in parallel with P67/P68/P71
(no file collision), **built after the M1 first-order path is de-risked** (§3.1). Its Lane-A code
(T1–T4, T5-scaffold) is buildable now against mocks; its Lane-B code (T5-render, T6 real transport)
unlocks with O18a (wgpu) and with **P67's landed claim service**. The blocking edge is real but
one-directional: **P73 needs P67's claim service to complete the fast path**, and needs P57/P58 for
its form + a11y — but nothing downstream needs P73. It gates the **M3 "first claimed hub"** business
milestone's public front door (SYNTHESIS §3.1: P67+P68 → M3), not the M1 first-order path.
