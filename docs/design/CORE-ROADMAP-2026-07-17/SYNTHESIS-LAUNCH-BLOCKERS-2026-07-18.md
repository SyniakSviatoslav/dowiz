# SYNTHESIS — Launch Blockers: reconciled build plan from OPUS-R1..R5 (2026-07-18)

> **Planning document — writes no product code.** Synthesizes the five 2026-07-18 Opus research
> passes (`docs/research/OPUS-R1..R5-*-2026-07-18.md`) against the binding operator decisions in
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16–§17, plus **two operator rulings made
> after the research completed** (§0.2 below). Consumed by the blueprint-writing swarm: §5 is the
> blueprint breakdown, each unit to be written against the 20-point contract in
> `CORE-ROADMAP-STANDARD-2026-07-17.md` (format precedent: `BLUEPRINT-P51-open-map-routing.md`).
> Nothing here re-litigates a closed decision — full-wgpu UI (§16.30), intent-only navigation
> (§16.35/§16.40), *ad fontes* (§16.42–§16.43), AR/VR readiness now (§17.5), voice Wave-0
> (§16.31), online payment Wave-0 (§16.13) all stand exactly as ruled. This document makes them
> buildable.

---

## 0. Inputs and the two fresh rulings

### 0.1 Source reports (all read in full this pass)

| Report | Surface | Verdict in one line |
|---|---|---|
| **R1** `OPUS-R1-INTERFACE-RENDERING` | wgpu UI, a11y, text input, typography, voice, deployment, SEO, AR/VR | Full GPU application UI is production-real (GPUI/Zed); the hard text algorithms are solved crates (cosmic-text/parley); AccessKit is native-ready but **web/canvas backend is planning-only**; two problems needed operator rulings (both now made, §0.2 / §4). |
| **R2** `OPUS-R2-PAYMENT-MONEYFLOW` | Payment adapter, PCI vs no-DOM, split payment | PCI SAQ-A **requires** provider-rendered card capture — canvas card entry is ruled out absolutely; three compliant paths (iframe / native SDK / hosted redirect); split is post-tokenization and compatible with §16.49. |
| **R3** `OPUS-R3-HUB-PROVISIONING-IDENTITY` | CF Tunnel automation, capability-certs, crypto-agility, Hetzner | Remotely-managed tunnels + warm-pool claim are fully automatable; build a biscuit-style capability chain over the existing HybridSigner (not SPIRE, not X.509); **dowiz's exact hybrid is already IETF OID `1.3.6.1.5.5.7.6.48`**. |
| **R4** `OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS` | Notifications, offline drafts, order machine, wallet transfer | HRW matcher and the unified order machine **already exist in code**; the genuinely missing piece is the dispatch orchestrator above HRW; drafts need no CRDT; notification fabric is three thin ports. |
| **R5** `OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS` | Backup, auto-update, food-court model, brand preview, moderation, licensing | age-envelope backup + A/B-slot update supervisor; row-scoped single-DB food-court model; brand preview is a uniform-buffer swap; AGPL packaging already exists in-repo — split-repo boundary is the open piece. |

### 0.2 Two operator rulings made after the research — applied consistently below, CLOSED

1. **Food-court merchant-of-record: each vendor is their own merchant-of-record** (Stripe-Connect
   separate-charges-and-transfers style mechanics) — **dowiz never becomes a party to the money.**
   This closes R2 §4.3's "sharpest open policy question" and supersedes both of R2's proposed
   models as written: there is no platform-MoR entity, not dowiz and not a designated lead vendor.
   Consequence (engineering, not a reopening): the food-court "one payment" is one **checkout
   UX** over **N vendor-scoped money legs** — each vendor's own provider account is MoR for its
   portion of the cart. R5 §3.4's `settlement_split` becomes the per-vendor charge-leg computation
   (still derived from `order_item.vendor_id`), and the hard new engineering item is
   **N-leg atomicity**: authorize all vendor legs, capture only if all authorized, void on any
   failure (auth-then-capture two-phase). This lands in the payment blueprint (§5, P60/P72).
   Onboarding consequence: a food-court vendor is payable only after connecting their own
   provider account — a named step in the claim/vendor-setup flow.
2. **Text input scripts: Wave-0 ships Latin + Cyrillic only, via fully custom canvas text input —
   no DOM anywhere.** Non-Latin scripts requiring IME composition (Arabic, CJK, Thai, Indic) are
   **deferred to v2**, consistent with §16.58's existing RTL-deferred-to-v2 ruling — a scope
   boundary, not a new exception. This closes R1 Risk #1 (option (c)) and resolves R1 §0's named
   contradiction: **P38 G6/FE-16's planned "transparent `<input>` overlay for IME/autofill/mobile
   keyboard" is struck** — §16.34 wins, P38 needs a canon-diff (§5, P38-rev). Residual engineering
   consequences (named in §4-E, not reopened): (a) the web a11y mirror for live text editing must
   be R1's **synthetic ARIA-textbox** variant, since the hidden-editable-element variant is now
   ruled out; (b) raising the **mobile-web soft keyboard** without any editable DOM element has no
   standard mechanism — needs a spike; the installed Tauri client (the daily-use path per §16.8)
   has native soft-keyboard control and is unaffected.

---

## 1. Executive summary — the reconciled architecture

One sentence: **a Rust/wgpu field-engine client (canvas-only, intent-driven, Latin+Cyrillic
Wave-0) talking to an isolated per-venue Rust hub (kernel order machine + HRW dispatch +
capability-cert identity), provisioned by claim from a warm pool behind one dowiz Cloudflare
account, with every external dependency — tunnel, VPS, payment, push/SMS/email, storage, AI
model — behind a trait port with a Wave-0 default adapter.**

What the five reports collectively established:

1. **Nothing forces a walk-back of any §16/§17 decision.** R1 §10: full GPU application UI is
   proven at the hardest end (Zed/GPUI, 2 ms keystroke-to-pixel); WebGPU has ~82% browser reach
   with the FE-16 WebGL2/CPU ladder covering the rest; the field model is already 3D-shaped for
   §17.5's AR/VR insurance. R4 §0: the two scariest-sounding order-flow requirements (unified
   pickup/delivery machine, no-scoring courier matching) are **already implemented and tested in
   the kernel** — `kernel/src/order_machine.rs`, `bebop2/proto-cap/src/matcher.rs`.
2. **The one absolute external constraint is PCI.** R2 §2: card data must be captured by
   provider-rendered surfaces (iframe / native SDK / hosted page) or dowiz falls into SAQ-D full
   scope and violates §16.49. Drawing a card field on the wgpu canvas is ruled out permanently —
   every blueprint must carry this as a red-line, type-enforced (no card-data type exists in
   hub-core, R2 §5.2). The remaining fork (which compliant surface, per platform) is the single
   highest-priority operator decision still open (§4-A).
3. **Ambition is preserved by two-tracking, not by dilution.** Typography: MSDF from real fonts
   is Tier-1 on the critical path; field/wave-generated glyphs (§16.39) is Track-R, gated by an
   objective legibility bar, never blocking an order screen (R1 §4). Same shape for AR/VR (four
   cheap architecture constraints now, OpenXR/WebXR backends post-Wave-0 — R1 §8) and for the
   intent-UI research program (§16.35/§16.40 is the one place with *no* fallback, by explicit
   operator acceptance — the mitigation is milestone visibility, not a hidden plan B).
4. **Identity is one mechanism, used everywhere.** The biscuit-style signed-block capability
   chain over the existing ML-DSA-65⊕Ed25519 `HybridSigner` (R3 §2.3–2.4) serves hub roots
   (§17.7), owner multi-hub delegation (§16.48), courier onboarding (§16.3/P52), and the claim
   handoff (§16.32) — one crypto surface, one adversarial review, four consumers. Crypto-agility
   is not an invention: the current hybrid **is** IETF composite suite `id-MLDSA65-Ed25519-SHA512`
   (OID `1.3.6.1.5.5.7.6.48`); versioning = a suite-ID field mapping to that registry, with
   SSH-style overlap rotation (R3 §3).
5. **"No central dowiz state" holds everywhere it was tested.** Push tokens hub-local (R4 §1),
   drafts on-device with no CRDT (R4 §3), wallet transfer device-to-device with no relay
   (R4 §6), backups encrypted to vendor-held keys before upload (R5 §1), brand preview owner-only
   via a uniform swap (R5 §4), moderation as per-hub reports + optional subscribable abuse
   blocklists (R5 §5). The only deliberate exceptions remain the ones §16.53/§17.5 already
   named: liveness heartbeat and opt-in anonymous aggregate telemetry.

---

## 2. Cross-cutting dependency map

These are the places where one report's finding changes what another report's surface can build.
Each is resolved here, explicitly, so no blueprint discovers it late.

### X1. AccessKit-web is planning-only → every web screen hand-rolls its mirror (R1 §2 → R2, R4, R5)

R1 verified the AccessKit web/canvas adapter is "planning-only, probably most difficult of all,
no timeline." Consequences fan out:
- **Native (winit desktop, Android):** AccessKit crate directly — production-ready, covers
  caret/selection via `SetTextSelection`. Zero hand-rolling.
- **Web (WASM):** the hand-rolled hidden-DOM semantic mirror (P38 FE-15) is the *only* option and
  must extend to **every** screen — including R2's checkout wizard, R4's notification/status
  surfaces, and R5's brand-preview owner screen. Mirror-node budgets are a per-blueprint DoD
  item, and the Playwright accessibility-tree harness (P51 §4.7 precedent) becomes a shared gate
  every surface blueprint imports (P58, §5).
- **Live text editing on web** uses the synthetic ARIA-textbox mirror (forced by ruling §0.2-2).
  Weaker screen-reader results than a native editable element — recorded honestly; the native
  clients are the strong-a11y path.
- **The one surface where web a11y is free: card entry.** If Path A or C is chosen (§4-A), the
  provider's iframe/hosted page is real DOM with real native a11y — the payment moment needs no
  mirror nodes of its own.
- **Brand-preview parity (R5 risk #5) falls out structurally, don't solve it twice:** the mirror
  reconciles from the same widget/scene state the renderer consumes (P38 §3.6), and the preview
  is a uniform-buffer swap on that same state (R5 §4.3) — one pipeline ⇒ one mirror ⇒ draft
  parity by construction. The P58 blueprint must state this as an invariant and test it.

### X2. The IME ruling reshapes P38 and pins the text stack (§0.2-2 → R1 §3, P38, P39)

- P38 G6's transparent-`<input>` overlay is **struck** (canon-diff in P38-rev, §5). No DOM input
  exists on any platform.
- Text model = **cosmic-text** (first choice; editing surface is more mature) with parley as the
  named alternative if the `WidgetStore` layout prototype favors it (R1 §3). Shaping crates are
  *ad fontes*-compatible primitives (same category §16.43 keeps for crypto) — re-implementing
  Unicode shaping would be a correctness regression, not simplification.
- Latin+Cyrillic scope means **no composition events are needed anywhere** — the entire
  keydown→buffer→GPU-glyph path is self-contained. v2's IME work becomes a bounded future
  project, not a Wave-0 unknown.
- Mobile-web soft keyboard without an editable element: **named spike** (§4-E). The Tauri
  installed clients call native `show_soft_input`; web-mobile falls back to the spike's outcome,
  with voice (§16.31, an equal intent channel per §16.50) as the honest interim input on that
  one platform combination if the spike comes back empty.

### X3. Payment path ↔ shell decision are one coupled choice (R2 §6 ↔ R1 §6)

R1 recommends `winit`+`wgpu`+AccessKit on desktop (no webview at all — cleaner than Tauri's
surface-conflict-prone webview for a no-DOM UI). R2's Path A (scoped provider iframe) **requires
a live webview/DOM host**. These compose only in certain combinations:

| Platform | Available card-capture paths | Shell consequence |
|---|---|---|
| Web browser | A (iframe overlay) or C (redirect) | none — DOM host is the browser itself |
| Desktop | A (needs Tauri/webview shell) or C (system browser; winit-only shell stays viable) | **choosing C frees desktop to be winit+wgpu+AccessKit; choosing A forces a webview host** |
| Mobile (Tauri) | B (native SDK sheet — zero DOM, cleanest §16.30 fit) or C | B needs the unproven Tauri-plugin-over-GPU-surface bridge (R2 risk #3) — same spike as R1's mobile-shell question; **one combined spike** (P63, §5) |

The operator decision in §4-A should be taken **with this table in view** — it is simultaneously
the payment-UX ruling and the desktop-shell ruling.

### X4. The composite-sigs OID reframes crypto-agility from invention to adoption (R3 §3.1 → R3's own §17.2 answer, P59)

Because `id-MLDSA65-Ed25519-SHA512` = OID `1.3.6.1.5.5.7.6.48` already exists in
`draft-ietf-lamps-pq-composite-sigs` (v19), the crypto-agility blueprint's stance is: **dowiz's
current hybrid is suite v1 of a standard registry, not a bespoke scheme needing its own
versioning design.** Concretely: an internal `alg_suite: u16` enum that *maps* to the OID (the
draft is pre-RFC; an OID shift is a one-line remap, R3 risk #6), AND-verification of both halves
(matches the B4 batch-verify precedent), TLS-style suite negotiation with downgrade binding, and
SSH-style overlap rotation for fleet migration. The blueprint's job shrinks to wire format +
negotiation + rotation procedure — the algorithm-identification problem is already solved
upstream.

### X5. GPUI/Zed precedent grounds the uniform-swap preview and the whole render posture (R1 §1 → R5 §4, P38)

R5's brand-preview design (two 5-token records, one pipeline, bind-time uniform swap) is exactly
the state-swap pattern production GPU UIs use — GPUI re-renders its entire editor from changed
state at 120 FPS with no parallel pipeline. R1's verdict "mine GPUI for technique, do not adopt"
extends to R5: no preview subdomain, no staging deploy, no second renderer — the preview is a
buffer bind plus the FE-14 settle gate re-waking the field. The P70 owner-surface blueprint
should cite GPUI's glyph-atlas and damage-tracking techniques for the live slider-drag preview
(R5 §4.3 step 5) rather than inventing a refresh strategy.

### X6. One idempotency design spans payment, drafts, and reconnect (R2 §5.2 ↔ R4 §3.3)

R4's draft state machine (`Draft` → `PaymentInflight`, idempotency key minted at draft creation,
query-before-replay on reconnect) and R2's adapter requirement (idempotency keys on every
mutating call) are **one contract**, not two. The `PaymentProvider` port must standardize
"create-with-key" and "query-status-by-key" across providers (R4 risk #5: not all providers
expose the latter identically — the adapter normalizes or the reconnect-safety guarantee is
provider-dependent). This contract is owned by P60 and consumed by P66; neither blueprint may
define it independently.

### X7. The catalog leaf invariant serves checkout, split legs, AND JSON-LD (R5 risk #4 ↔ R2 ↔ R1 §7)

§16.17's free-form vendor catalog needs exactly one non-negotiable floor: **every purchasable
leaf carries a resolvable price (integer minor units), currency, and `vendor_id`.** Three
independent consumers force the same invariant: the unified cart/checkout (R5 §3.4), the
per-vendor charge-leg computation under the new MoR ruling (§0.2-1), and the bot-facing
schema.org `Menu`/`MenuItem`/`Offer` JSON-LD generation (R1 §7 — the load-bearing AEO substrate;
llms.txt is a forward-looking extra, not a crawlability bet). P62 owns the invariant; P60, P69,
and the static-file pack consume it.

### X8. Identity chain is upstream of claim, couriers, owners, and wallets (R3 §2 → R3 §6.1, P52, P48, R4 §6)

The capability-cert chain (P59) must exist before: the claim mechanic can hand an owner root or
append a child block (R3 §6.1 step 3); courier onboarding can mint courier certs (§16.3, P52);
the owner multi-hub client can fan out N hub connections under one root (§16.18/§16.48); and the
pooled hubs can be pre-minted self-signed roots at snapshot time (R3 §6.1 step 2). R4's wallet
transfer shares the primitive *family* (X25519/HKDF/AEAD from RustCrypto) but is deliberately a
separate, simpler mechanism — do not merge wallet-transfer crypto into the cert chain; do reuse
the self-custody framing and the §6.4 anti-phishing confirmation lesson.

### X9. The golden snapshot is the integration point for provisioning, update, and backup (R3 §4 ↔ R5 §1–§2)

One Packer-built image must bake in: the hub binary in an A/B slot layout under the
`dowiz-hub-supervisor` (R5 §2.3), `cloudflared` with a pre-created remotely-managed tunnel token
(R3 §1.2), the pre-minted self-signed root cert (R3 §6.1), demo fixtures (§16.54), the age
backup scheduler (R5 §1.3), and the shadow local ingress config that makes the §17.3 tunnel
escape hatch a real switch rather than a rebuild (R3 risk #5). P67 (provisioning) and P68
(supervisor/backup) share this image as their contract — the image spec is written once, in P67,
with P68 as co-owner of the slot/backup layout. The supervisor's promote step **must** take an
age state snapshot first (R5 risk #1: code rollback must never outrun forward-only schema
migrations — restoring the pre-promote snapshot is the rollback story for state).

### X10. Notification coverage matrix is forced by platform gaps (R4 §1, §8.3 ↔ §16.8, §16.52)

iOS Safari web push works only for home-screen-installed PWAs; web-first customers on iOS
(§16.8's zero-friction path) therefore *cannot* be reached by push. §16.52's mandatory SMS/email
fallback is what makes the product honest here — but the P61 blueprint must enumerate the
channel-per-platform matrix and prove coverage (every customer who placed an order has ≥1
working status channel), not assume it. Email from fresh per-hub Hetzner IPs will spam-folder
(R4 §2.2) — managed-API default, `lettre` SMTP as flagged opt-in; sender-domain default proposed
in §4-F.

### X11. Anti-abuse reuses the kernel TokenBucket; challenges live at the edge (R2 §7 ↔ §16.53)

Checkout-intent rate limiting = the existing kernel `TokenBucket` (agentic-mesh B3), keyed by
wallet client-id + coarse IP, degrade-closed, plus a single-outstanding-intent cap per wallet.
Turnstile (already-in-stack CF) or self-hosted ALTCHA runs at the edge/redirect layer — never
embedded in the canvas (same DOM tension as X3). No reputation, no scoring, ever (§16.26,
§16.59, mesh red-line).

### X12. Voice, translation, and the assistant share one local-inference substrate (R1 §5 ↔ §16.38 ↔ §16.4/§16.52)

Three §16 requirements all resolve to local model inference: streaming ASR for commands
(Moonshine-class + keyword-spot wake; whisper.cpp small as the multilingual fallback — R1 §5),
localization via a local open-source translation model (§16.38), and the single in-hub assistant
(§16.4, three-mode AiMode per P40/P41). These must share one `LocalInference`/model-runtime port
(BYO-model per §16.52) rather than three runtimes — native (Tauri-side), never WASM, for
NEON/battery (R1 §5). Owned by P64 for the input channels, referencing P41's AiMode for the
assistant.

---

## 3. Build sequence — dependency order to "first real order"

Definitions: **M1 (first real order)** = a real customer pays real money for a real pickup order
on one hand-onboarded, dowiz-operated hub, through the full-wgpu intent UI, with status
notifications delivered. Pickup-first is a sequencing choice only (§16.60: pickup is the same
flow minus dispatch) — it removes the courier surface from the very first order's critical path
without descoping anything. **M2** adds delivery (courier + dispatch + map). **M3** = first
*claimed* hub (automation). **M4** = first food-court order. §16.40's "full set at once" governs
the UI paradigm (no fallback UI is ever built); it does not forbid milestone ordering.

### 3.1 What must exist before what (the critical chain)

```
[exists] kernel: order_machine, HRW matcher, router, geo/kalman, TokenBucket, HybridSigner, event log
[exists] engine: compose(), Scene/SdfShape, WidgetStore, FE-14 settle, FE-15 mirror base, FE-16 ladder

W1 (foundations, all parallel):
  P63 shell+payment-bridge spike  ──┐  (verdict feeds P39-rev, P60 client leg, P52 battery baseline)
  P57 canvas text input (Lat+Cyr) ──┤  needs P58 mirror conventions for its a11y half
  P58 a11y-mirror-everywhere      ──┤
  P64 intent engine + friction map + voice ─┤
  P59 capability-cert chain       ──┤  (independent of UI entirely)
  P62 catalog + food-court data model ─┤
  P60 payment adapter (port + server side; client leg gated on §4-A ruling) ─┤
  P61 notification fabric         ──┘

W2 (assembly, needs W1):
  P66 data wallet + offline drafts     (needs P60's idempotency contract [X6], P57 for entry UI)
  P65 dispatch orchestrator            (needs P59 courier certs conceptually; buildable vs order_machine now)
  P69 customer storefront + checkout   (needs P57/P58/P64 + P62 + P60 + P66; emits bot-facing static pack [X7])
  P70 owner surface                    (needs P57/P58/P64 + P59 + P62; brand preview per X5)
        ──► M1: FIRST REAL ORDER (pickup) = P69 + P70(minimal confirm) + P61 + P60 + one manual hub

W3 (delivery + automation, parallel after M1 path is de-risked):
  P71 courier surface (P52-rev)        (needs P65 + P51 map + P64 voice + P63 battery gates)
        ──► M2: first delivery order
  P67 hub provisioning + claim         (needs P59; golden image per X9)
  P68 supervisor: update + backup      (co-owns image with P67)
        ──► M3: first claimed hub
  P73 dowiz.org landing + signup       (full wgpu §16.56; static-file SEO pack; interest form)

W4 (multi-vendor + hardening):
  P72 food-court checkout (N-leg MoR)  (needs P62 + P60 + P69; §0.2-1 atomicity design)
        ──► M4: first food-court order
  P74 moderation reports + blocklist   (thin; R5 §5)
  Track-R (continuous, off critical path): procedural glyphs (R1 §4), AR/VR backends (R1 §8),
  intent-UI research program deepening (§16.35/§16.39/§16.44 beyond the P64 v1 spec)
```

### 3.2 Sequencing rationale (the load-bearing orderings)

1. **P63 (spike) is first among equals.** The shell verdict (winit vs Tauri per platform), the
   mobile wgpu-surface question, the native-payment-SDK bridge feasibility (R2 risk #3), the
   mobile-web soft-keyboard mechanism (§0.2-2), and the full-shift battery baseline (R1 risks
   #3/#7) are all cheap-to-measure, expensive-to-guess. Every W2 surface inherits its verdicts.
   It also produces the evidence the §4-A operator decision deserves.
2. **Text input before any checkout UI.** The wgpu substrate + P57 (Latin/Cyrillic editing with
   mirror parity) must exist before P69's address/name fields can be built — there is no interim
   DOM form to borrow (§16.34). P57's Latin-only-Wave-0 scope makes this a bounded, fully
   specified build (R1 §3: "Latin-script Wave-0 text input is fully in-canvas and achievable").
3. **P59 before claim, before couriers, before multi-hub owner** (X8) — but *not* before M1:
   the first hand-onboarded hub can run on directly-issued certs from the same chain code, so
   P59 sits in W1 while P67's automation sits in W3.
4. **P60's server-side half is ruling-independent.** The port trait, Stripe server adapter,
   webhook verification, idempotency contract, and TokenBucket wiring need no card-UI decision.
   The client card leg starts the moment §4-A is ruled — in parallel with rendering work, as
   the payment surface is provider-rendered under every compliant path (R2 §6).
5. **Dispatch orchestrator (P65) is the largest un-designed order-flow piece** (R4 risk #1):
   accept-timeout → advance-rank, decline handling, offer-to-primary policy, queued-order
   re-poll — stateful hub-side coordination that must not reintroduce scoring or a central
   queue. It gates M2, not M1 (pickup needs no dispatch).
6. **Provisioning automation (P67/P68) deliberately trails M1.** One real venue on one manually
   provisioned hub proves the product; the warm pool, tunnel automation, and supervisor prove
   the *business* (§16.12/§16.32). Building them in W3 keeps the first-order path short without
   descoping Wave-0 — they remain launch blockers for M3, and the port traits they need
   (`TunnelProvider`, `VpsProvider`) are named in W1 blueprints so nothing hardcodes against
   them (R3 §5's Synapse lesson: a port retrofitted is not a port).
7. **WebGL2-floor parity is a standing gate, not a phase** (R1 risk #9): every surface blueprint
   (P69/P70/P71/P73) carries a DoD line "renders correctly on the FE-16 WebGL2 and CPU floors" —
   ~18% of web users have no WebGPU.

---

## 4. Flagged operator decisions

Collected from all five reports' "riskiest unknowns," deduplicated, with the two §0.2 rulings
removed (closed). Split honestly: **A–D need the operator** (product/business/red-line forks);
**E–F are engineering** with proposed defaults, listed so nobody mistakes them for silent gaps.

> **A–D all CLOSED 2026-07-20 — see `DECISIONS.md` D12 for the authoritative ruling text.**
> One-line summary, D12 is the source of truth: **A** = Path A (scoped provider-iframe overlay;
> desktop shell is therefore Tauri-with-webview, not pure winit+wgpu). **B** = self-custody
> absolute on both surfaces, no recovery path either for backups or a lost owner root. **C** = no
> hub suspension — every claimed hub stays hot indefinitely (pool-economics must size for
> zero-recycling). **D** = Albania/EU, matching `PRODUCT.md`'s existing primary market. W1
> blueprint-writing (P39-rev, P59, P60, P63, P67, P68, P69, P72) is unblocked.

### Operator decisions (raise before blueprint-writing begins) — historical record below, CLOSED

**A. Card-capture surface per platform** (R2 §6.4 / risk #1 — the top item; coupled to the
desktop shell per X3). Recommendation to present: **B (native provider SDK sheet) on Tauri
mobile** — zero DOM, cleanest §16.30 fit, pending the P63 bridge spike; the real fork is
**web + desktop: Path A** (scoped provider-iframe overlay at the card moment only — one
documented, narrowly-scoped DOM exception; keeps the user inside the canvas experience) **vs
Path C** (hosted redirect — zero DOM exception anywhere, lightest PCI residual (6.4.3/11.6.1
shift to the provider), frees desktop to be pure winit+wgpu — at the cost of momentarily leaving
the immersive canvas). Both are PCI-compliant; this is an aesthetics/immersion-vs-purity call
that also fixes the desktop shell. **Blocks: P60 client leg, P69 checkout step, P39-rev.**

**B. Self-custody severity — one consistent ruling across two surfaces.**
(i) Backup break-glass: is a `dowiz_break_glass_pubkey` ever in the backup recipient set
(R5 §1.3/risk #2)? Including it means dowiz *could* read backups — weakening "dowiz never sees
plaintext" from construction to policy; excluding it means a vendor who loses their key loses
every backup. Recommended default: **no dowiz recipient** (matches §16.47's "loss is the user's
responsibility"). (ii) Owner-root loss (R3 risk #2): with short-TTL re-mint as the revocation
mechanism, a lost owner root eventually strands the fleet (certs expire); long TTLs widen the
compromise window; any recovery mechanism is a trust-model exception. These are the same
philosophical fork as §16.47 — asking for **one ruling** ("self-custody is absolute" vs "a named,
narrow recovery path exists") applied to both, so the wallet, backup, and cert designs stay
consistent. **Blocks: P59 revocation section, P68 backup recipients.**

**C. Abandoned-claimed-hub power-down** (R3 risk #4). §16.57 rules a claimed hub stays the
vendor's forever. May a long-inactive claimed hub be **suspended-but-preserved** (state retained,
compute released — still theirs, re-wakeable) rather than kept hot indefinitely? This qualifies
an existing operator ruling, so it is not an engineering call. It materially changes warm-pool
economics (the pool is net-consumed with no recycling either way; this only affects the *cost*
of consumed slots). **Blocks: P67 pool-economics section.**

**D. Food-court Wave-0 market scope** (R2 risk #4, sharpened by §0.2-1). Vendor-as-own-MoR
N-leg checkout depends on each vendor's provider supporting per-vendor charges in that market
and currency (Mollie splits are EUR/GBP-only; cross-currency handling varies). Which market(s)
must food-court checkout work in for Wave-0? §16.20 requires the *architecture* to be
market-agnostic — this decision only scopes where the food-court *feature* is proven first.
**Blocks: P72 provider matrix.**

### Engineering decisions (named defaults; the blueprint decides, the operator need not)

**E. Named engineering unknowns with owners:** mobile-web soft-keyboard mechanism with no
editable element (P63 spike; interim = voice channel + installed app on that platform combo,
§0.2-2/X2) · dispatch timeout/decline/offer policy values (P65) · §16.44 friction numeric
mapping — stake → amplitude/color/rhythm, gesture complete/cancel grammar, audio-channel
equivalence, objective a11y gate (P64; the *mechanism* is closed, the mapping is design work) ·
wallet-transfer return transport: animated-QR vs BLE vs same-LAN, plus the mandatory
anti-phishing confirmation UX (P66; R4 §6.4) · CF API-token custody + append-only tunnel-config
mutation log + account-pool-ready config (P67; R3 risk #1 — §16.45 already scoped Wave-0 to one
account; the compensating controls are engineering) · cert TTL values + revocation-blob gossip
cadence (P59, downstream of ruling B) · `async-stripe` 1.0-alpha pin vs hand-rolled ~5-endpoint
client (P60; R2 risk #5) · idempotency-contract normalization across providers (P60, X6) ·
catalog leaf invariant exact shape (P62, X7) · warm-pool depth/refill cadence (P67, downstream
of ruling C) · Turnstile (default, already in stack) vs ALTCHA at the edge (P60/P67; R2 §7) ·
public/closed repo split with a CI dependency-graph fence (P67; R5 §6.4 Option A — split repos,
modeled on the existing kernel-fence guards) · suite-ID internal enum → OID mapping (P59; the
draft is pre-RFC, R3 risk #6).

**F. Confirm-by-default (operator may veto, default proceeds):** dowiz provides a **managed
default email sending domain** for hub notifications (vendor's own domain as the opt-out) — the
established §16.52 managed-default-with-opt-out pattern applied to email; noted because a
dowiz-run sending domain is a soft ongoing dowiz dependency for notification email
(R4 risk #4), mitigated by the opt-out being real from day one.

---

## 5. Proposed blueprint breakdown (feeds the blueprint-writing swarm)

Every unit below is one coherent, independently buildable blueprint against the 20-point
standard (`CORE-ROADMAP-STANDARD-2026-07-17.md`; rigor precedent: `BLUEPRINT-P51`). Numbering
continues from the existing P56. **Blueprint WRITING can be fully parallel across a wave**
(different files, no shared hot file); the wave structure below is the **build** dependency
order. Shared contracts that two blueprints touch are assigned a single owner (noted inline) —
the other blueprint cites, never redefines.

### Canon-diffs to existing blueprints (small revisions, write first — they unblock the rest)

| Rev | Change | Why |
|---|---|---|
| **P38-rev** `BLUEPRINT-P38-webgpu-render-engine.md` | Strike G6's transparent-`<input>` overlay (superseded by §0.2-2); add the four AR/VR insurance constraints as hard requirements (view/projection-matrix-driven pipeline, 3D `FieldPos` end-to-end, intent-abstracted `InputSource`, one XR seam — R1 §8); reaffirm FE-15 mirror + FE-16 ladder as the base every new blueprint imports | Resolves the R1 §0 contradiction in canon, not just in prose |
| **P39-rev** `BLUEPRINT-P39-app-shell-installability.md` | Shell decision intake: winit+wgpu+AccessKit desktop candidate vs Tauri webview, per P63 verdicts + §4-A ruling (X3) | The shell choice is now coupled to payment path |
| **P48/P52** | Not revised now — consumed by P70/P71 below, which supersede-or-extend them explicitly in their §2 scope sections | Avoid double-owning the owner/courier surfaces |

### Wave W1 — foundations (8 blueprints, build-parallel, zero file collisions)

| # | Blueprint | Scope (one coherent unit) | Depends on | Feeds |
|---|---|---|---|---|
| **P57** | **Canvas text input & editing (Latin+Cyrillic)** | cosmic-text buffer/cursor/selection/clipboard over MSDF glyph render (FE-06); keydown wiring native+web; explicit v2 boundary for IME scripts; live-edit a11y via AccessKit native + P58's ARIA-textbox web convention | P38-rev; P58 conventions (a11y half only) | P69, P70, P71, P73 — every typed field |
| **P58** | **a11y-mirror-everywhere + harness** | FE-15 generalized to all screens; synthetic ARIA-textbox spec; AccessKit native adapter wiring; the shared Playwright a11y-tree gate (role/name/state + keyboard order + live-edit caret assertions); draft-parity invariant (X1) | P38-rev | every surface blueprint's DoD |
| **P59** | **Capability-cert chain & crypto-agility** | biscuit-style signed-block chain over `HybridSigner`; root/child delegation with Datalog-style scope facts; `alg_suite` field mapped to OID `1.3.6.1.5.5.7.6.48`; AND-verify; suite negotiation + downgrade binding; overlap rotation; TTL+revocation-blob design (pending §4-B); **mandatory independent adversarial review gate** (block canonicalization, reordering/truncation, cross-suite confusion — R3 risk #3, B4 lesson) | HybridSigner (exists) | P67, P70 (owner root), P71 (courier certs), M1 hub certs |
| **P60** | **Payment adapter core** | `PaymentProvider` port (R2 §5.2 shape); Stripe server adapter; webhook verify/normalize; **owns the idempotency contract** (X6); vendor-as-MoR leg model incl. N-leg auth-then-capture design (§0.2-1); refund routing (§16.29); TokenBucket checkout-intent limiting + single-outstanding-intent cap + edge challenge (X11); type-level no-card-data enforcement; client card leg spec'd per §4-A ruling | order machine (exists); §4-A for client leg only | P66, P69, P72 |
| **P61** | **Notification fabric** | `Notifier` fan-out + `PushPort`/`SmsPort`/`EmailPort`; adapters: web-push/a2/fcm_v1, hand-rolled SMS REST (Twilio-shaped, provider-per-market), sesv2/resend default + lettre opt-in; dead-token eviction; **the coverage matrix proof** (X10); per-hub-local tokens/contacts (§16.22) | order machine events (exist) | M1 (status updates are mandatory) |
| **P62** | **Catalog & multi-vendor data model** | vendor_id-row-scoped free-form catalog tree (R5 §3.4); **owns the leaf invariant** (X7: price minor-units + currency + vendor_id); order_item.vendor_id fan-out; per-vendor charge-leg derivation (§0.2-1); shared courier_pool; RLS-FORCE reuse; single-vendor as degenerate N=1 | kernel event log (exists) | P69, P70, P72, static-file pack |
| **P63** | **Shell & platform spike (measured verdicts)** | winit+wgpu+AccessKit desktop prototype vs Tauri; mobile raw-wgpu-surface vs WebGPU-in-webview; native-payment-SDK-over-GPU bridge feasibility (R2 risk #3); mobile-web soft-keyboard mechanism; **full-shift battery benchmark harness on real budget hardware** (FE-14 savings measured, not asserted — R1 risks #3/#7); WebGL2-floor parity check method | P38-rev | P39-rev, P60 client leg, P71 battery gates, §4-A evidence |
| **P64** | **Intent engine, friction spec & voice** | intent-driven navigation runtime v1 (intent → composed UI functions); §16.44 friction numeric mapping + gesture complete/cancel grammar + audio channel + objective a11y gate; implicit onboarding mechanism v1 (§16.50); voice: keyword-spot wake + Moonshine-class streaming + whisper.cpp multilingual fallback behind `VoiceSource: InputSource`; shared `LocalInference` port with P41's AiMode (X12) | P38-rev InputSource | P69, P70, P71 (all interaction) |

### Wave W2 — assembly (4 blueprints; parallel except as noted)

| # | Blueprint | Scope | Depends on |
|---|---|---|---|
| **P65** | **Dispatch orchestrator** | accept-timeout → advance-rank driver over the HRW ranked list; decline handling; offer-to-primary policy; queued no-courier orders (wait-at-Ready, requeue-never-drop); `fulfillment_type` discriminator + per-type side-structs (R4 §5.3); no scoring, no central queue — red-lined in the blueprint itself | matcher + order machine (exist); P59 (courier certs, interface only) |
| **P66** | **Data wallet & offline drafts** | on-device wallet store + checkout autofill (§16.23); versioned-JSON LWW draft (no CRDT — R4 §3.1); `Draft`/`PaymentInflight` states + query-before-replay (cites P60's contract, X6); outbox loops (tauri-plugin-store explicit-save / IndexedDB + online-event); Signal-style QR transfer (X25519+HKDF+AES-GCM, transport per §4-E, confirmation UX per R4 §6.4) | P60 (contract), P57 (entry UI) |
| **P69** | **Customer storefront & checkout journey** | `/s/:slug` full-wgpu menu → cart → delivery → payment wizard as the P38 narrative arc (§16.37); wallet autofill; honest hub-offline status (§16.14); card moment per §4-A; **bot-facing static pack**: robots/sitemap/JSON-LD(Restaurant/Menu/Offer)/OG/manifest + llms.txt extra, generated from kernel state (R1 §7, X7); WebGL2-floor DoD | P57/P58/P64, P62, P60, P66, P61 |
| **P70** | **Owner surface** | orders/menu/courier management (extends P48); brand draft/live preview via uniform swap + mirror parity (X5, X1); basic marketing auto-posting Wave-0 (§16.36, via P43 ports); GDPR delete-customer tool (§16.58); multi-hub client mode under one owner root (§16.18/§16.48); analytics explicitly deferred (§16.36) | P57/P58/P64, P59, P62 |

**→ M1 gate:** P69 + P70 (minimal confirm path) + P61 + P60 live on one manually-provisioned hub.

### Wave W3 — delivery + automation (4 blueprints, parallel)

| # | Blueprint | Scope | Depends on |
|---|---|---|---|
| **P71** | **Courier surface** | extends P52: full-wgpu courier app; voice-primary in motion (§16.53); P51 map/route integration; accept/decline against P65; **battery gates from P63's measured baseline as DoD** | P65, P51, P64, P63 |
| **P67** | **Hub provisioning & claim** | warm pool (assignment-only hot path) + Packer golden-snapshot refill (R3 §4.1-B/A); CF tunnel automation (R3 §1.2 flow) behind `TunnelProvider`; `VpsProvider` trait; **owns the golden-image spec** (X9) incl. shadow local ingress; pre-minted roots; claim service (closed repo) + public/closed split + CI fence; 1,000-tunnel-cliff alerting; heartbeat (§16.53); pool economics per §4-C | P59; §4-C ruling |
| **P68** | **Hub supervisor: update + backup** | A/B slots + atomic symlink flip + real-code-path health gate + auto-rollback (R5 §2.3); `self_update` fetch/verify only; version pinning; **age-snapshot-before-promote** (state rollback story, R5 risk #1); age X25519 backup envelope + rclone transport to `hetzner:dowiz`/vendor S3; recipients per §4-B | P67 (image co-owner) |
| **P73** | **dowiz.org landing + signup** | full-wgpu landing (§16.56, no static-page exception); interest form (P57 input); GitHub link (§16.32); its own bot-facing static pack; claim entry UX handing off to P67's service | P57/P58, P67 (service API) |

**→ M2 gate** (P71 + P65 live) · **→ M3 gate** (P67 + P68 live).

### Wave W4 — multi-vendor + thin remainder

| # | Blueprint | Scope | Depends on |
|---|---|---|---|
| **P72** | **Food-court checkout (N-leg, vendor-as-MoR)** | multi-vendor unified cart UX; N per-vendor charge legs with auth-all-then-capture atomicity + partial-failure/void semantics + refunds-against-multi-vendor-order (§0.2-1; R5 risk #3 re-scoped to the new ruling); per-vendor KDS routing by `order_item.vendor_id`; per-vendor provider-account onboarding step; market scope per §4-D | P62, P60, P69; §4-D ruling |
| **P74** | **Moderation: reports + shareable abuse blocklist** | per-hub `report` event; optional signed subscribable **abuse** blocklist (never quality/reputation — §16.59 red-line); legal-takedown endpoint only on the dowiz side (R5 §5.3) | event log (exists) |

**→ M4 gate** (P72 live).

### Track-R (continuous, never on the critical path, own cadence)

Procedural field/wave typography behind the MSDF Tier-1 + objective legibility gate (R1 §4);
OpenXR/WebXR backends behind the one XR seam once upstream wgpu support matures (R1 §8);
intent-UI research deepening beyond P64's v1 (the §16.35/§16.39/§16.44 program). The four AR/VR
insurance constraints are NOT Track-R — they are P38-rev hard requirements, in W1.

### Swarm dispatch summary

- **Write order:** P38-rev/P39-rev first (they change canon others cite), then all of W1's eight
  in one parallel fan-out, then W2–W4 in wave order (W2+ blueprints cite W1 contracts by
  blueprint number — the contracts must exist on paper first).
- **Single-owner contracts (collision guards):** idempotency contract → P60; catalog leaf
  invariant → P62; golden-image spec → P67 (P68 co-signs the slot/backup layout); a11y harness
  + ARIA-textbox convention → P58; `InputSource`/intent grammar → P64; suite-ID/cert wire
  format → P59.
- **Before W1 writing starts, raise §4-A–D with the operator** — A blocks P60's client leg and
  P39-rev's verdict; B blocks P59's revocation section and P68's recipients; C blocks P67's
  pool economics; D blocks P72's provider matrix. Everything else in every blueprint is
  writable now.
