# BLUEPRINT P52 — Courier working surface: shift, claims, run, proof-of-delivery, earnings (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). Component:
> **DELIVERY**. **This phase is minted by operator direction from the same-day MVP audit**
> (`docs/design/DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md` §6 M1): "**Courier
> working surface** … protocol fully built/planned, screen owned by nobody — **the largest
> single omission this audit found**", MVP-blocking (§7 item 1: "the courier cannot see,
> accept, or attest a delivery without SOME surface"). The old stack treated the courier as a
> first-class user with a complete 7-page app; the new roadmap covers the courier PROTOCOL
> exhaustively (P34's claim_machine/matcher/PoD/settlement — the MOST built part of the whole
> stack) and, until this phase, gave the courier zero pixels. P52 is the **third leg of the
> one physics-render pattern**: customer = P38b (Sea & Sheet), owner = P48 (hub, WebGPU per
> the resolved §11.2-2 ruling), courier = **P52** — same P38a substrate, same zero-visible-DOM
> canon, NOT a fourth UI technology. Structural template: `BLUEPRINT-P-A-kernel-primitives.md`
> (numbering mirrored); sibling precedents: `BLUEPRINT-P38-webgpu-render-engine.md`,
> `BLUEPRINT-P51-open-map-routing.md`.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working trees on `main` (`/root/dowiz`) and `/root/bebop-repo`, 2026-07-18. All fresh reads.
The two load-bearing findings: **the courier protocol is built and tested end to end**, and
**no availability/shift concept exists anywhere in it** — the surface consumes the first and
must supply the second.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| Old stack: a complete courier app — 7 pages (`LoginPage, CourierInvitePage, TasksPage, DeliveryPage, ShiftPage, EarningsPage, HistoryPage`) + 5 route files (`courier/{auth,me,shifts,assignments,settlements}.ts`) | `git log --all --diff-filter=A --name-only -- 'apps/web/src/pages/courier/*' 'apps/api/src/routes/courier/*'` (14 files listed this pass; deleted in the `79ef316f6`/`f9ab28ff1` purge) | **VERIFIED — the feature bar is historical fact, not invention** |
| Claim lifecycle Law BUILT: `ClaimStatus { Offered 0x20, Claimed 0x21, Released 0x22, PickedUp 0x23 }` (pinned wire bytes), `assert_transition`, `fold_transitions`, table `Offered→[Claimed,Released]`, `Claimed→[Released,PickedUp]`, terminals closed | `/root/bebop-repo/bebop2/proto-cap/src/claim_machine.rs:21-30` (enum), `:34-41` (discriminants), `:72-81` (table), `:85` (`assert_transition`), `:98` (`fold_transitions`) | VERIFIED — P52 relays intents into this Law; it owns ZERO transition logic |
| Matcher BUILT, and **the caller supplies the candidate set**: `assign(order, candidates, max)` — HRW hash of `(order_id, courier_pubkey)`, deterministic tie-break, `primary_for` requeue "never dropped" | `matcher.rs:63` (`assign`), `:41` (`hrw_weight`), `:70` (tie-break), `:80-84` (`primary_for` + never-drop doc) | VERIFIED — WHO feeds `candidates` is the unowned half (next row) |
| **NO availability/shift concept exists**: grep `shift\|on_duty\|availab` over `delivery-domain/src` + `proto-cap/src` → zero real hits (only "available/Unavailable" in unrelated entropy/facade prose) | grep this pass (re-confirming the MVP audit §4's identical grep) | **VERIFIED GAP — the audit's M4; P52 §3.1 owns the answer** |
| NO-COURIER-SCORING is structural + CI-locked: `Courier { pubkey }` "the type literally cannot carry a score"; claim state carries no score field; gate script exists | `matcher.rs:30-36`, `claim_machine.rs:13-17`; `/root/bebop-repo/scripts/ci-no-courier-scoring.sh` (ls this pass) | VERIFIED — binding on every P52 type (§4.1) |
| Wire vocabulary BUILT (exact variant names): `Action::{ClaimOffered, ClaimAccepted, ClaimReleased}` (`Resource::Claim`) decode to `DeliveryEvent::Claim(ClaimPayload { claim_id, order_id, courier: CourierKey })`; `Action::SettlementRecorded` (`Resource::Ledger`) → `DeliveryEvent::Settlement(LedgerPayload { order_id, amount_i64 })`; `OrderStatusChanged` receiver-side legality incl. `Ready→[InDelivery, PickedUp]`, `InDelivery→[Delivered]`; `DeliveryStatus::PickedUp` pinned 0x18 | `/root/bebop-repo/bebop2/proto-cap/src/scope.rs:90-104` (Action variants); `event_dict.rs:116-120` (ClaimPayload), `:124-128` (LedgerPayload), `:278-283` (DeliveryEvent enum), `:294-301` (claim/settlement decode arms), `:75-85` (legality table), `:53` (PickedUp 0x18) | **VERIFIED — P52 consumes exactly these; minting event variants is out of scope (§1)** |
| PoD BUILT — and its exact shape: `DeliveryClaim { order_id, location: Vec<u8>, timestamp, signers: Vec<HubSigner>, threshold, sigs }`; settles on **k-of-n DISTINCT hub hybrid signatures** (Ed25519 ⊕ ML-DSA-65) over `sha3_256(order_id ‖ len ‖ location ‖ timestamp)`; tamper/misattribution/duplicate-signer/wrong-digest all adversarially tested; **`location` is an opaque byte string; there is NO photo field, NO customer-signature field, NO GPS-fence logic** | `/root/bebop-repo/bebop2/delivery-domain/src/pod.rs:62-74` (struct), `:38-45` (canonical bytes), `:98` (digest), `:132-144` (`valid_signers`/`is_settled`), `:165` (`sign_claim`), tests `:229-357` | **VERIFIED — the crypto core is done; what fills `location`, what optional evidence attaches, and the capture UX are ALL unbuilt — §3.4 owns them** |
| Hub single-writer overlay: order ownership = HRW over hub keys, deterministic failover, "no SPOF"; reuses `matcher::hrw_weight` verbatim | `delivery-domain/src/hub_ring.rs:1-20` | VERIFIED — a courier surface cannot locally fabricate claim state; the hub owns the order record (§4.1) |
| Mesh fold entry: `DeliveryReceiver::admit_and_fold` | `delivery-domain/src/intake.rs:223` (struct), `:234` (fn) | VERIFIED — P34's seam, cited not re-wired |
| P51 BUILT-ON-PAPER same day, and P52 consumes it: in-kernel router landed (`kernel/src/router.rs:1-14` Dijkstra/A*/CH), `CourierTrack`/`TrackEvent` estimator (M5), `TrackFrame` "two consumers, one implementation — the **courier surface** (route + own marker + ETA — Sea grammar, P38b DZ-08) and the customer live-track view", `map_scene` render layers (M4), `CourierPositionUpdated` privacy invariant (emit only between assignment-accept and delivery-complete) | `BLUEPRINT-P51-open-map-routing.md` §0 (router row), §3 (`CourierTrack`/`TrackFrame` types), §4.4-§4.6 (M4-M6), §2 (P49 overlap note) | **VERIFIED — P52's navigation/tracking is 100% consumption of P51; re-designing any of it is a scope violation** |
| DZ-08 COURIER flows exist as an arc design unit: shift timer + start/end, tasks accept/decline with countdown, delivery act with live map + mark-picked-up + cash-collected + SwipeToComplete ("never-fake-success"), earnings `<Money>` no-count-up, history | `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:225-244` (DZ-08 unit incl. its GATE line) | VERIFIED — the interaction-design substance P52 reuses; its old-stack mechanics (email/pw login, WS, 12s GPS heartbeat) are superseded by cert auth, the P34 wire, and P51's estimator — §3 maps each |
| P38b absorbs DZ-01..12 but is customer-facing by its own §10.5.3 text ("Sea … the customer storefront"; P38 §11.1 lists DZ-08 with no owner text) — nobody executes DZ-08's courier flows | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:961-971`; `BLUEPRINT-P38-webgpu-render-engine.md` §11.1 | VERIFIED — the ownership vacuum this phase closes; reconciliation in §1 |
| P48 rendering ruling RESOLVED and inherited: "WebGPU, NO DOM exemption … §10.3 invariant 4 holds uniformly; FE-15's a11y mirror remains the only DOM survivor" | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:1243-1247` | VERIFIED — P52 renders through P38a, full stop |
| P48 DoD-4 (the invite seam's owner half): "an owner grants and revokes a courier capability cert through the surface, exercising the existing proto-cap issuance + `RevocationSet`" | roadmap `:1269-1272`; `bebop2/proto-cap/src/revocation.rs` (cited there, verified present) | VERIFIED |
| P23-P2 (the invite seam's device half): "device generates hybrid keypair … TOTP-verified enrollment mints a capability cert … revocation = existing RevocationSet"; QR/otpauth enrollment UX named | `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md:212-219` (§5.2); P39 blueprint §3.4 (enroll_device decide-path, being authored same day) | VERIFIED |
| **The seam between them is named-and-unowned** (MVP audit, exact wording): "The crypto mechanism is fully specified. The gap is exactly the seam the task asked about: the **invite handoff UX** — how the owner's grant reaches the courier's not-yet-enrolled device (out-of-band link/QR that bootstraps P23-P2 enrollment against P48's roster) is implied by both DoDs and named by neither." (M10: "a manual ceremony acceptable" for courier #1) | `DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md` §5 courier-invite row, §6 M10, §7 item 2 | **VERIFIED — §3.6 designs the concrete handoff, not a named gap** |
| P47 Wave-0 cash rail is operator-CONFIRMED and its input is the courier: "the courier's signed cash-collected attestation as the `SettlementRecorded` source"; DoD-2 drives place→deliver→attestation→`SettlementRecorded` over P37's wire | roadmap `:1162-1165` (role), `:1198-1199` (DoD-2), RESOLVED note `:1169-1191` | VERIFIED — P52 §3.7 is that attestation's input surface; the money logic stays P47's |
| Kernel-side capability machinery for the surface's auth: `verify_chain` / `RevocationSet` / `DOMAIN_DELEGATION` / `RefSigner` | `kernel/src/ports/agent/cap.rs:480/:406/:33/:102` | VERIFIED — same substrate as P37 W37-3/P39 W39-4; nothing new |
| ML-DSA-65 signature size 3309 B, pubkey 1952 B (drives the PoD payload budget §4.4) | `pod.rs:187-196` (the byte-length constants in the helpers) | VERIFIED |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P52 owns vs deliberately does NOT

**Ownership reconciliation, stated once:** DZ-08's *interaction design* (shift timer, task
cards, delivery act, earnings) is REUSED as P52's UX spec — P38b keeps its customer scope and
its DZ-01..07/09..12 units untouched; P52 becomes DZ-08's executing owner, binding it to the
new-stack vocabulary (cert auth replaces email/pw; `DeliveryEvent::Claim` replaces WS
`task_assigned`; P51 `CourierTrack` replaces the 12s GPS heartbeat + `CourierLiveMap`). The
P38b §11.1 table's DZ-08 row should gain a one-line pointer here at merge time (a cross-ref
edit, not a scope change).

**P52 owns (build items §3):**

| Item | Content |
|---|---|
| K1 | **Availability**: the Wave-0 candidate-set RULE (stopgap, stated as law) + a node-local `on_duty` fold + the cap-gated toggle route — closes audit M4 without touching P34's wire vocabulary |
| K2 | **Claim inbox**: render `ClaimOffered` for this courier; accept/decline actions relayed as `ClaimAccepted`/`ClaimReleased` intents; claim-Law legality stays kernel/receiver-side |
| K3 | **Delivery run screen**: active claim + P51's `map_scene`/`TrackFrame` (route, own marker, ETA) + the status actions the legality table grants the courier (`PickedUp`, `Delivered` via `InDelivery`) |
| K4 | **PoD capture flow**: fill `DeliveryClaim`'s inputs (canonical `location` bytes + timestamp), attach optional evidence (photo hash), drive k-of-n signature collection, surface `is_settled` — the UI for the built crypto |
| K5 | **Earnings view**: a courier-scoped read-only fold over `SettlementRecorded` events (the D5 statement pattern's second reader — derive-only, zero new money logic) |
| K6 | **Onboarding/invite handoff**: the concrete P48-DoD-4 ↔ P23-P2 bridge — owner mints a short-lived single-use enrollment invite; the courier's device redeems it on first launch and comes out cert-enrolled (audit M10 closed as design, with the manual-ceremony fallback documented for courier #1) |
| K7 | **Cash-collected attestation input**: the one-tap signed attestation whose event is P47's `SettlementRecorded` source (P47 owns the money semantics; P52 owns the button and the signing call) |

**P52 explicitly does NOT own (each with its owner):**

- **NOT a fourth rendering technology** — every pixel goes through P38a's pipelines (G2
  particles, G3 SDF/text, G5 settle, G6 a11y mirror); the P48 ruling (WebGPU, no DOM
  exemption) applies verbatim. A visible DOM widget here fails P38's existing
  zero-visible-DOM gates — no new gate needed.
- **NOT P34's event vocabulary or matcher logic** — P52 consumes `DeliveryEvent::Claim` /
  `Settlement` / `StatusChanged` exactly as pinned (§0); it mints NO new `Action`/`Resource`
  variants (P34's anti-scope forbids it, and the audit's M4 note agrees: the availability
  answer must not live "inside P34 itself"). K1 is deliberately node-local for this reason.
- **NOT routing/maps/tracking design** — P51 owns router consumption, MapPack, pin, snap,
  ETA, re-route, `TrackFrame`, and the position-event privacy invariant. K3 consumes P51 §4.4
  and §4.6 outputs; any map/estimator code appearing in P52's diff is a scope violation.
- **NOT payment/settlement semantics** — P47's lane (waves, rails, reconciliation). K7 emits
  the attestation P47 DoD-2 already specifies; K5 reads folds P47/P13 produce.
- **NOT the owner/admin hub** — P48's lane (menu, live orders, roster grant/revoke UI,
  omnichannel intake). K6 consumes P48's grant as its input, it does not build the granting
  surface.
- **NOT customer identity/notifications** — P49's lane entirely.
- **NOT courier scoring, ratings, rankings, reputation — EVER** — the standing structural
  rejection (`matcher.rs:15-18`, CI-locked; memory: SOVEREIGN-EVENT-EXCHANGE stance "trust =
  signed capability, NEVER reputation"). No P52 type may carry a score field; §4.1 extends
  the CI gate over this phase's types.
- **NOT multi-order batching** — the audit's own ground rule: neither product history nor
  roadmap establishes it ("TasksPage was a list"). One active claim at a time, Wave-0.
- **NOT voice/turn-by-turn** — DZ-10 Phase-9b + P51 anti-scope, unchanged.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/src/courier_surface.rs — NEW module (pure fold/decide helpers; NO I/O) ──
/// K1 Wave-0 candidate-set LAW (the audit-M4 stopgap, promoted from implicit to law):
///   candidates(roster, revocations, duty) =
///     { c ∈ roster.certified | !revocations.contains(c) ∧ duty.on(c) }
/// with duty.on(c) = true for ALL certified couriers when the duty fold is empty
/// (bootstrap semantics: before anyone toggles, everyone certified is a candidate —
/// honest at first-client scale, 1-3 couriers, because claims are PULL-based: an
/// off-duty courier simply never accepts, and matcher::primary_for's requeue
/// already handles refusal without dropping the order).
pub struct DutyFold { /* courier_key -> on_duty, last_ts; node-local, fold-derived */ }
pub enum DutyEvent { On { courier: [u8; 32], ts: i64 }, Off { courier: [u8; 32], ts: i64 } }
pub fn fold_duty(events: &[DutyEvent]) -> DutyFold;                  // pure, deterministic
pub fn candidates(/* roster, revocations, &DutyFold */) -> Vec<[u8; 32]>;
// NOTE: DutyEvent is a NODE-LOCAL event family (hub-side event_log spine), NOT a
// proto-cap wire variant — deliberately, so P34's pinned vocabulary is untouched.
// Mesh-wide availability gossip = named future unit at the P34B boundary, not built.

/// K2/K3 — the courier's own surface state: one fold over the events this courier
/// can see. NO score field (CI gate extended over this file, §4.1).
pub struct CourierSurfaceState {
    pub duty: bool,
    pub inbox: Vec<ClaimCard>,          // ClaimOffered for me, not yet terminal
    pub active: Option<ActiveRun>,      // at most ONE (Wave-0 no-batching law)
    pub earnings: Statement,            // K5 fold output
}
pub struct ClaimCard { pub claim_id: u64, pub order_id: u64, pub offered_ts: i64 }
pub struct ActiveRun {
    pub claim_id: u64, pub order_id: u64,
    pub status: u8,                     // DeliveryStatus discriminant, relayed opaque
    pub track: Option<TrackFrameRef>,   // P51 M6's TrackFrame — consumed, never derived here
}
pub const OFFER_DECISION_WINDOW_SECS: u64 = 60;  // DZ-08's 60s auto-decline, kept: expiry
                                                  // emits ClaimReleased (requeue via primary_for)

/// K4 — PoD capture: what the surface assembles for the BUILT DeliveryClaim.
/// pod.rs's `location` is opaque bytes (§0) — P52 pins its canonical encoding ONCE:
///   location = "geo:" ‖ i32 lat_micro7 LE ‖ i32 lng_micro7 LE   (12 bytes)
/// (same 1e-7-degree integer form as P51's MapPack — one coordinate convention,
/// P51 §3 cited as the authority; a free-text location is refused, not encoded).
pub struct PodCapture {
    pub order_id: u64,
    pub geo: (i32, i32),                // lat/lng micro7 from P51 track estimate
    pub ts: u64,
    pub photo_hash: Option<[u8; 32]>,   // content-address of optional evidence photo
}
pub fn pod_location_bytes(geo: (i32, i32)) -> [u8; 12];   // the ONE encoder
// The photo itself is EVIDENCE-ADJACENT, not signature-load-bearing: it rides the
// blob path the MVP audit's M3 names (one storage decision shared with menu media),
// keyed by photo_hash. The signed claim commits to order_id‖location‖timestamp
// exactly as pod.rs:38-45 already law-pins — P52 does NOT extend the signed tuple
// (changing canonical_claim is a protocol change, P34/bebop2's gate, not a UI's).

/// K5 — earnings statement: derive-only second reader (D5 pattern) over
/// DeliveryEvent::Settlement folds scoped to this courier's completed claims.
pub struct Statement { pub today_i64: i64, pub week_i64: i64, pub rows: Vec<StatementRow> }
pub struct StatementRow { pub order_id: u64, pub amount_i64: i64, pub ts: i64 }
pub fn fold_statement(/* settlement events, my claim history */) -> Statement;
// Money law: i64 minor units end to end; rendered via TweenGuard::present_money/jump
// ONLY (engine/src/money_guard.rs:50 — landed and binding; DZ-08's own "«Money»
// NOT count-up" line agrees).

// ── K6 — enrollment invite (the P48-DoD-4 ↔ P23-P2 bridge), over EXISTING cap machinery ──
/// An invite IS a capability: a short-lived, single-use, delegation-scoped cert
/// signed by the owner's key under DOMAIN_DELEGATION (cap.rs:33), whose sole
/// grant is "redeem-for-enrollment". No new crypto, no new trust concept.
pub struct EnrollInvite {
    pub invite_id: [u8; 16],            // random, single-use (spent set on the hub)
    pub issued_by: [u8; 32],            // owner key (chain-verifiable to the roster root)
    pub expires_at: u64,
    pub scope: &'static str,            // "courier-enroll" — redeemable for exactly that
}
pub const INVITE_TTL_SECS: u64 = 24 * 3600;      // short-lived: one working day
pub enum InviteReject { Expired, AlreadySpent, BadChain, RevokedIssuer, WrongScope }
/// redeem: device presents (invite, its fresh hybrid pubkey) → hub verifies chain
/// + freshness + unspent → P39's enroll_device path mints the device capability
/// cert → invite marked spent. Err ⇒ NO cert, invite state unchanged unless spent.
pub fn redeem_invite(/* invite, device_pubkey, roster, spent_set, now */)
    -> Result<Vec<u8>, InviteReject>;
// Transport of the invite to the device: QR / deep-link rendered by P48's surface
// (otpauth-style URL carrying the invite bytes) — scanned on the courier app's
// first launch (the "not-yet-enrolled device" boot path, §3.6).
// MANUAL CEREMONY (courier #1, audit M10's MVP fallback): operator runs both
// sides on one bench — documented as a runbook step in §10 T6, not software.

// ── K7 — cash attestation (input only; semantics = P47) ─────────────────────
/// A SignedFrame whose payload is P47's cash-collected attestation shape for
/// (order_id, amount_i64), signed by the courier's device cert. P52 owns the
/// tap-and-sign; P47 owns what it settles. Emitted ONLY from an ActiveRun in
/// Delivered-pending state (emit-site guard, §4.1).
pub const CASH_ATTEST_SCOPE: &str = "cash-collected";
```

Rejected alternatives (DECART one-liners): **availability as a new proto-cap wire variant** —
rejected: P34's vocabulary is pinned and its anti-scope forbids additions; node-local duty fold
covers a single-hub deployment, the only deployment Wave-0 has. **Offer push via a new
transport** — rejected: the inbox is a fold over frames the P34/P37 wire already delivers;
polling the read route suffices at ≪1 event/sec (P37 §4.2's stated budget), wake-push is P43's
lane. **Free-form `location` strings in PoD** — rejected: one pinned 12-byte geo encoding, or
refusal — an unparseable location makes the claim un-auditable. **Extending the PoD signed
tuple with photo_hash** — rejected Wave-0: changing `canonical_claim` is a cross-node protocol
migration (every verifier re-pins); evidence rides content-addressed alongside, and promoting
it into the signature is a named P34-boundary decision if ever needed. **Batching UI** —
rejected: audit ground rule (no feature history nor roadmap establishes).

---

## 3. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 3.1 K1 — availability: the stopgap LAW + the duty fold + the toggle

Three pieces, smallest-first: (1) **the rule as law** — `candidates()` per §2, with the
bootstrap semantics written into the fn doc and asserted by test (empty `DutyFold` ⇒ all
certified-unrevoked couriers; this is the audit-M4 stopgap made explicit, "stated, not
implicit"); (2) **the fold** — `fold_duty` pure + deterministic (same event list ⇒ same fold,
order-independent within a courier by ts); (3) **the toggle** — one cap-gated route on P37's
surface (`POST /api/courier/duty`, admitted by the SAME `CapVerifier` middleware, P37 §3.3 —
no new auth machinery) appending a `DutyEvent` to the node-local event log.

- **RED:** `k1_candidates_bootstrap_all_certified` (empty duty ⇒ full roster);
  `k1_toggle_off_excludes` (Off event ⇒ excluded from `candidates`, matcher `assign` over the
  reduced set still deterministic). Both fail today (module absent).
- **Adversarial:** a REVOKED courier toggling On ⇒ the toggle route 403s at the cap layer AND
  `candidates` excludes them regardless (two independent walls — defense stated); duplicate
  On events idempotent (fold, not counter); an Off arriving with an ACTIVE claim does NOT
  release the claim (duty gates future offers only — the claim Law owns claim state; asserted
  so nobody "helpfully" couples them).

### 3.2 K2 — claim inbox: offer → accept/decline, Law-relaying only

The inbox folds `DeliveryEvent::Claim` frames where `payload.courier == my key` and the claim
is non-terminal, rendered as `ClaimCard`s through P38a (G3 cards + G2 feedback per DZ-08's
"один ripple + ping + dedupe"). Actions: **accept** relays a `ClaimAccepted` intent frame;
**decline** (or `OFFER_DECISION_WINDOW_SECS` expiry) relays `ClaimReleased`. The surface never
evaluates legality — `claim_machine::assert_transition` runs receiver-side on every node
(§0); a rejected transition renders as the typed error, state unchanged.

- **RED:** `k2_offer_folds_to_inbox` (Offered frame for me ⇒ card; for another key ⇒ no
  card); `k2_accept_emits_claim_accepted` (event-sequence assertion: `[Offered, Accepted]`
  fold ⇒ `active = Some`, inbox empty); `k2_expiry_releases` (60s window ⇒ Released intent,
  requeue-visible via `primary_for` — the never-drop invariant re-asserted from the consumer
  side).
- **Adversarial:** accept on an already-Released claim ⇒ receiver-side `IllegalClaimTransition`
  surfaces as typed refusal, `active` stays `None`, no shadow state (the fold is the ONLY
  state authority); TWO offers accepted in one window ⇒ the second accept is refused
  surface-side by the `active: Option` (at-most-one law, §2) BEFORE any frame is sent —
  asserted; a forged offer frame for my key but bad signature never reaches the fold
  (HybridGate/cap admission upstream — cited as P34/P37's tested wall, not re-proven here).
- **Offline honesty (F12 without fabrication):** claim state is hub-owned (hub_ring single
  writer, §0). On an island, the inbox renders last-known folds marked stale, and an accept
  becomes a QUEUED INTENT rendered as pending-unconfirmed — never a locally-fabricated
  `Claimed`. Test: island accept ⇒ `active = None`, pending badge; rejoin ⇒ intent submits,
  fold confirms or the requeue took it elsewhere (both outcomes rendered truthfully).

### 3.3 K3 — the delivery run screen (P51 consumption + the courier's legal actions)

One screen = `ActiveRun`: P51's `map_scene` layers (roads/route/marker) + `TrackFrame`
(ETA/remaining) composed with the order card — literally the same `compose()` frame family
(P51 §1.3: "map, route, marker, and product field are layers of a single compose() pass").
Actions surfaced = exactly the edges the receiver legality table grants the courier's leg:
`Ready→PickedUp` (mark-picked-up), `InDelivery→Delivered` gated through K4's PoD flow, DZ-08's
SwipeToComplete gesture kept for the terminal action with its "never-fake-success" gate —
the swipe completes ONLY on a receiver-confirmed fold, resets on refusal.

- **RED:** `k3_run_renders_trackframe` (a fixture `TrackFrame` ⇒ ETA/remaining glyphs present
  in the composed frame — pixel-region assertion per P38's oracle discipline);
  `k3_pickup_relays_status` (event-sequence: `[Claimed, PickedUp]` claim fold + a
  `StatusChanged{Ready→PickedUp}` order intent — both Law-checked upstream).
- **Adversarial:** `k3_swipe_never_fakes` — receiver refuses the status intent ⇒ the swipe
  visually resets and the fold state is byte-unchanged (DZ-08's own GATE line, now a test);
  `TrackFrame` absent (P51 island/no-GPS) ⇒ the run screen renders order + honest no-track
  state, never a stale marker presented as live (staleness labeled — P51's `route_version`
  field consumed); a `Delivered` attempt while status is `Ready` ⇒ receiver-side illegal
  (table §0), typed refusal rendered.

### 3.4 K4 — PoD capture: the UI for the built crypto

Flow: arrival (`is_arriving` via P51/geo.rs) unlocks the PoD step → the surface assembles
`PodCapture` (geo from the track estimate, ts caller-supplied per the counter discipline,
optional photo captured → hashed → stored content-addressed) → `pod_location_bytes` (the ONE
encoder, §2) → `DeliveryClaim::new(order_id, location, ts, signers, k)` with the hub roster as
`signers` → the claim digest goes to the wire for hub signatures (`sign_claim` runs hub-side)
→ the surface renders collection progress (`valid_signers()/threshold`) → `is_settled()` flips
the run to Delivered-pending and unlocks K7's cash step (cash orders) / completion.

- **RED:** `k4_capture_to_settled_roundtrip` — fixture: 3 enrolled hub signers, k=2; drive
  capture → two signatures → `is_settled()` true → status intent `InDelivery→Delivered`
  relayed (event-sequence asserted end to end). Fails today (no surface module).
- **Adversarial (riding pod.rs's own tested arms, asserted AT THE SURFACE):** below-threshold
  (1 of k=2) ⇒ surface shows unsettled, Delivered intent NOT emitted (the emit-site is gated
  on `is_settled` — asserted unreachable otherwise); tampered location after signing ⇒
  `any_tampered()` renders as a hard error state, never silently re-requested; duplicate hub
  signature counts once (surface progress must not double-count — reuses `valid_signers`, no
  local counter); photo-hash mismatch on later fetch ⇒ evidence marked corrupt, claim validity
  UNAFFECTED (photo is not signature-load-bearing, §2 — this test proves the separation);
  non-finite/zero geo ⇒ capture refused before any claim is built.
- **Named dependency, honest:** the photo blob path is the audit-M3 storage decision (shared
  with menu media). K4's photo leg goes `#[ignore = "M3-blob-path"]` until that lands —
  ignored-not-deleted, the P38 O18a convention; geo+timestamp PoD (the signature-load-bearing
  part) has no such gate and lands Wave-0.

### 3.4b NFC-tag PoD (optional evidence leg, added 2026-07-18 — real build attempt, not a proposal)

A real, tested attempt (`docs/design/CORE-ROADMAP-2026-07-17/RESEARCH-NFC-FLIPPER-ZERO-DEV-TOOLING-2026-07-18.md`)
answered the operator's NFC/hardware-key question with working code, not speculation. Verdict,
evidence-based: passive **NTAG213/215/216** tags (~$0.10–0.50 each) carrying a dowiz NDEF
External-Type record (`dowiz.io:pod`) are an **optional supplementary evidence source for K4**,
never the sole PoD mechanism — the k-of-n hub-signed `DeliveryClaim` (§3.4) stays the
signature-load-bearing artifact regardless of whether a tag exists at a given address.

- **Codec, real and tested:** `tools/nfc-pod-codec` — `order_id: u64` (bound to `event_dict.rs`'s
  `OrderPlacedPayload`) + `issued_at` + 16-byte MAC = 33-byte payload, 48 bytes on the wire incl.
  NDEF framing, fits NTAG213's 144 B with room to spare. MAC = `SHAKE256(key ‖ context ‖ fields)`,
  **reusing** `kernel::pq::keccak::shake256` — no new crypto. 17/17 tests green (round-trip,
  tampered-order-id, tampered-timestamp, wrong-key, malformed-header, all `BadMac`/reject).
  Verification is **server-side only** — the tag and the reading phone never hold the
  provisioning key.
- **Reader:** the courier's own **phone** (Web NFC / native NDEF) — zero special hardware,
  consistent with §0's phone-only stance. A tap is optional corroborating evidence folded into
  K4's capture step alongside geo+photo, never a gate on `is_settled()` (that stays pod.rs's
  signature threshold, unchanged).
- **What was explicitly rejected, with the reason:** Flipper Zero (any firmware, incl. Momentum)
  as *production* courier hardware. Not a cost objection alone — a build-surfaced fact: an
  ML-DSA-65 signature (3309 B) does not fit a passive tag (144 B), so the tag layer was always
  going to be a symmetric MAC, which a $170 device reads no better than the phone every courier
  already carries. Dev-tooling use (`tools/nfc-pod-flipper`, a flipperzero-rs FAP that builds
  clean on stable) is a legitimate optional bench aid for validating this exact format
  pre-production — named as such, not smuggled in as a courier-facing requirement.
- **RED:** `k4b_tag_tap_adds_corroborating_evidence_never_gates_settlement` — fixture: capture
  with a valid tag read but only 1-of-k hub signatures ⇒ `is_settled()` still false, Delivered
  intent still withheld (proves the tag cannot substitute for the threshold). Fails today (no
  surface module, matches K4's own RED).
- **Anti-scope:** do not make tag presence required to complete a delivery (many addresses will
  never have one); do not store the provisioning MAC key on any courier device; do not revisit
  the Flipper-as-production question without new evidence — this pass's rejection is grounded in
  a physical size constraint, not a preference.

### 3.5 K5 — earnings: the second reader over settlements

`fold_statement` per §2: filter `DeliveryEvent::Settlement` to orders whose claim history
contains my `ClaimAccepted→PickedUp` chain, sum by day/week windows (integer, no float
windows — day boundaries from caller-supplied ts). Rendered via `TweenGuard` money discipline
(no count-up, DZ-08 + FE-09 law).

- **RED:** `k5_statement_folds_my_settlements_only` (mixed-courier fixture ⇒ only mine);
  `k5_sums_integer_exact` (property: statement total == Σ rows exactly, i64).
- **Adversarial:** a settlement for an order I never claimed ⇒ excluded (no
  "close-enough" attribution); a NEGATIVE settlement (refund leg) folds and renders signed
  (the statement must not clamp reality); reconciliation cross-check: my statement + all other
  couriers' statements + venue legs == the ledger fold total exactly (reuses P47 DoD-3's
  property shape, cited not forked).

### 3.6 K6 — the invite handoff (audit M10, closed as a concrete flow)

The full bridge, each half already owned, the seam now designed: **(owner side, P48)** roster
surface mints `EnrollInvite` (§2 — a DOMAIN_DELEGATION-scoped, TTL'd, single-use capability
signed by the owner key) and renders it as QR/deep-link. **(device side, P52)** the courier
app's first-launch boot path detects no device cert → shows the scan/paste entry → device
generates its hybrid keypair → `redeem_invite` presents (invite, pubkey) to the hub →
P39's `enroll_device` mints the device cert (TOTP leg per that flow where the owner requires
it) → invite marked spent → the surface proceeds to K1's duty screen, enrolled.

- **RED:** `k6_invite_roundtrip_mints_cert` — mint → redeem → the minted cert passes
  `verify_chain` (`cap.rs:480`) and admits a `POST /api/courier/duty` through P37's
  middleware (the end-to-end proof that a courier can go from nothing to working).
- **Adversarial:** expired invite ⇒ `Expired`, no cert; second redemption of a spent invite ⇒
  `AlreadySpent`, no cert, first cert unaffected; invite signed by a non-roster key ⇒
  `BadChain`; invite whose ISSUER was revoked between mint and redeem ⇒ `RevokedIssuer`
  (chain checked at redeem time, not mint time — the test that catches "cached trust");
  a redeemed cert later revoked via P48 DoD-4 ⇒ the courier's next mutating request 403s
  (P48's own test, re-run from this side — one revocation authority, two consumers).
- **Courier #1 fallback (M10's MVP answer, recorded):** the same flow run as an
  operator-executed ceremony on one machine (mint CLI-side, redeem device-side) — a runbook
  step in §10 T6, explicitly NOT new software.

### 3.7 K7 — cash-collected attestation (P47's input surface)

One action on the Delivered-pending run (cash orders only): tap → the device cert signs the
P47-shaped attestation for `(order_id, amount_i64)` → frame to the hub → P47's path folds
`SettlementRecorded` → K5's statement gains the row. P52 asserts the emit-site guard and the
signature; the settlement semantics test is P47 DoD-2's (shared fixture, one test authority).

- **RED:** `k7_attest_emit_site_gated` — attestation constructible ONLY from a
  Delivered-pending `ActiveRun` (type-level: the builder takes the run state as witness);
  attempts from any other state do not compile / return typed refusal.
- **Adversarial:** amount tampered between display and sign ⇒ the signed amount is the
  fold-derived order total, never a UI-supplied number (the surface passes the order_id; the
  hub derives the amount — a courier device cannot attest an arbitrary figure); double-tap ⇒
  one attestation (idempotent by order_id at the hub, duplicate refused, statement has one
  row).

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

Reachability arguments, not prose: **a courier cannot self-assign** — assignment order is
`hrw_weight`'s pure function of `(order_id, pubkey)` computed identically on every node (§0);
the surface can only accept an offer that exists in the fold, and the hub (single-writer,
hub_ring) rejects the rest — the "grabbed someone else's order" state has no wire
representation. **A courier cannot fake delivery** — `Delivered` is emit-gated on
`is_settled()`, which requires k distinct enrolled hub signatures over the canonical digest
(pod.rs's tested arms: tamper, misattribution, duplicate, wrong-digest all refuse); the
surface holds no signing authority over the claim. **A courier cannot inflate earnings** —
the attestation amount is hub-derived from the fold (§3.7), and statements are pure readers.
**No scoring can leak in** — `ci-no-courier-scoring.sh` extends over `courier_surface.rs`
(the grep set gains this file — a score/rating/rank field is a CI failure, not a review
catch). **Privacy** — position rendering consumes P51's `TrackFrame`; the emit-side invariant
(position events only between assignment-accept and delivery-complete) is P51 M6's, inherited
and re-asserted at K3's consumer (a track render outside an ActiveRun is unrepresentable —
`track` lives INSIDE `ActiveRun`, §2). **Duty ≠ claim coupling refused** — §3.1's adversarial
arm makes "going off-duty drops my active delivery" structurally false.

### 4.2 Schemas for scaling (item 8)

Stated axes: **couriers/venue** (Wave-0 rule is honest at 1-3, the audit's stated scale; break
point: when couriers × offers makes see-every-offer consent unreasonable (~10²), the duty fold
gains zone scoping — a filter on `candidates`, not a matcher change — named, not taken);
**claims/day** (10²/day ⇒ folds are trivial; no break point in sight); **PoD payload** (one
hybrid sig = 64 + 3309 B ⇒ a settled 2-of-3 claim ≈ 7 KiB + 12 B location — well under P34's
1 MiB SyncFrame ceiling; axis = k, break point none at sane k); **statement window** (rows/
month ≤ 10³ ⇒ linear fold fine; demote-to-archive at years, living-memory pattern).

### 4.3 Isolation / bulkhead (item 11) + error-propagation gates (item 14)

The surface is a consumer of folds and an emitter of intents — a surface crash corrupts
nothing (no shared mutable state with the kernel/hub; same bulkhead argument as P38 §4.3,
inherited). The duty toggle and courier routes ride P37's API sub-router bulkhead
(`MAX_INFLIGHT_API`, body limits — P37 §4.3, unchanged). Named CI gates per bug class:
no-courier-scoring grep (score leak), emit-site witness types (fake-delivered,
arbitrary-amount attestation), the never-fake-success swipe test (UI lying), the
island-accept honesty test (fabricated claim state), the invite spent-set test (credential
replay).

### 4.4 Mesh awareness (item 12)

Frames consumed/emitted, with budgets: `ClaimPayload` = 48 B, `StatusChangedPayload` = 10 B,
`LedgerPayload` = 16 B (fixed layouts, event_dict.rs §0) at ≪ 1 event/sec per courier; PoD
signature collection ≈ 7 KiB per delivery (once); duty events are NODE-LOCAL (not gossiped —
§2's deliberate choice; mesh-wide availability is the named P34B-boundary future). Position
events: P51's budget (≤ 32 B at ≤ 0.5 Hz), not re-counted here. Nothing in this phase needs
the transport layer beyond what P34/P37 already carry.

### 4.5 Rollback / self-healing vocabulary (item 13, used precisely)

**Self-Termination leg claimed:** typed refusals everywhere (`InviteReject`, claim-Law errors,
capture refusals); witness-typed emit sites (attestation, Delivered); at-most-one ActiveRun.
**Self-Healing leg claimed narrowly:** offer expiry → `ClaimReleased` → `primary_for` requeue
is genuine error-correction (an unresponsive courier never strands an order — the matcher's
never-drop invariant, consumed); claimed for offer flow only. **Snapshot-Re-entry: NOT
claimed** — surface state is always fold-derived; recovery = re-fold (re-derivation, the F12
island re-entry P37 §4.5 already names). Mechanical rollback: every module is additive
(`courier_surface.rs`, the toggle route, the web surface units); deletion restores today's
tree — with the honest note that deleting P52 re-opens the audit's M1 MVP-blocker.

### 4.6 Living memory (item 15) + tensor/spectral (item 16) + Linux discipline (item 9)

Item 15: the inbox/statement are temporal folds (append-only events, windowed reads);
completed runs demote to history, never delete (DZ-08's History tab = the demoted tier).
Item 16: honestly N/A for new math — the surface computes nothing (HRW, claim Law, PoD
crypto, Kalman track, route math are ALL upstream authorities); stated, not decorated.
Item 9 verdicts: **ALREADY-EQUIVALENT** — mechanism/policy split (the duty RULE is data over
the roster; the matcher mechanism is untouched); **REINFORCES** — one coordinate encoding
(P51's micro7) reused for PoD location (one concept, one form); **EXTENDS** — emit-site
witness gating as the discipline for every money/terminal action on this surface; **GAP**
honestly named — no real camera-capture hardware path exists in the render stack yet (photo
leg gated on M3, §3.4); a second honest GAP — DZ-08's tip/messenger-save lines are dropped
Wave-0 (tipping is the audit's recorded non-feature needing its own operator ruling; recorded
here so silence isn't drift).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| K1 | no duty module; bootstrap rule implicit | `k1_candidates_bootstrap_all_certified`, `k1_toggle_off_excludes`, revoked-toggle 403, duty≠claim decoupling | decoupling test (ledger row) |
| K2 | no inbox; accept path absent | offer→card fold; accept/decline event sequences; expiry-requeue; illegal-accept refusal; island-accept honesty | island-accept honesty (ledger row) |
| K3 | no run screen | `TrackFrame` render assertion; pickup relay; `k3_swipe_never_fakes`; stale-track labeling | never-fake-success (ledger row) |
| K4 | no capture flow | `k4_capture_to_settled_roundtrip` (k-of-n end to end); below-threshold no-Delivered; tamper surfaces; photo-separation proof | emit-gate-on-settled test (ledger row) |
| K5 | no statement fold | my-settlements-only; integer-exact sums; negative-leg honesty; reconciliation cross-check | reconciliation property |
| K6 | no invite flow (the M10 gap) | `k6_invite_roundtrip_mints_cert` end to end; expired/spent/bad-chain/revoked-issuer refusals; revocation round-trip with P48 | spent-set replay test (ledger row) |
| K7 | no attestation input | emit-site witness; hub-derived amount; double-tap idempotency; P47 DoD-2's shared fixture passes from this side | witness-type compile gate |

**Phase-level falsifier (the audit's M1, made one test):** `p52_courier_end_to_end` — from a
fresh un-enrolled device: redeem invite → duty On → receive offer → accept → run (track
fixture) → PickedUp → arrive → PoD 2-of-3 settled → Delivered → cash attestation →
statement row present. RED today at the FIRST step (no surface exists); GREEN = the MVP
transaction's courier leg is real. **Not-done clauses:** any P52 type carrying a
score/rating/rank field = NOT done regardless of green totals; a locally-fabricated `Claimed`
or `Delivered` state = NOT done; a UI-supplied attestation amount = NOT done; map/estimator
code inside P52 modules = NOT done (P51's lane); a new proto-cap `Action` variant = NOT done
(P34's lane).

---

## 6. Benchmark plan (item 10) — small and honest

The surface computes nothing hot; its budgets are inherited and cross-checked rather than
invented: **frame budget** = P38 §6's 16.6 ms split (the run screen's map layers are P51's
`map_layer/scene_2k_segments` bench — cited, not duplicated); **claim-accept round trip** =
P37 §6's p50 ≤ 5 ms / p99 ≤ 25 ms budgets apply to the duty/claim routes (the r15 pattern
extends to them — one new series in the same bench, RED-commit-first); **PoD settle path** =
one new measured number, `pod_verify_2of3` (two hybrid verifies ≈ dominated by ML-DSA-65 —
measured, appended to `BENCH_HISTORY.md`, budget ≤ 50 ms on the reference machine so the
capture flow never blocks the frame loop; if exceeded, verification moves off the render
thread — the named step). Fold benches (inbox/statement) deliberately unmeasured at 10²-row
scale — decorative benching refused, stated.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3 (P37/P38 substrate), §11
(P47/P48/P49 seams + the P48 rendering ruling), §12 (P51), §13 (this phase's index entry) ·
`DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md` (M1/M3/M4/M10 — the minting
authority; its §7 priority argument) · `BLUEPRINT-P51-open-map-routing.md` (M4/M5/M6
consumed; coordinate encoding authority) · `BLUEPRINT-P38-webgpu-render-engine.md` (render
substrate + zero-visible-DOM gates) · `BLUEPRINT-P37-order-http-surface.md` (route/cap/bulkhead
substrate) · `BLUEPRINT-P39-app-shell-installability.md` (enroll_device consumed by K6; the
installed shell K-screens live in) · `BLUEPRINT-P34-mesh-kernel-wiring.md` (wire vocabulary
owner — consumed, never extended) · `BLUEPRINT-P47-P50-gap-closing-phases.md` (P47 attestation
semantics; P48 roster; P49 boundary) ·
`docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:225-244` (DZ-08 — the reused
interaction spec) · `docs/regressions/REGRESSION-LEDGER.md` (six rows named §5). Memory:
`mesh-real-arc-2026-07-13` (MESH-03/04/05 provenance) · SOVEREIGN-EVENT-EXCHANGE stance
("trust = signed capability, NEVER reputation" — §4.1's law) ·
`test-integrity-rules-2026-06-27` (money red-lines) · `verified-by-math-2026-07-07` ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (style; the honest GAP notes §4.6).
Supersedes: nothing — fills the ownership vacuum §0 proves; DZ-08 gains an executing owner.

---

## 8. Hermetic principles honored (item 20 — load-bearing only)

- **P2 CORRESPONDENCE** (one concept, one authority): one claim Law (relayed, never
  re-implemented); one coordinate encoding (P51's micro7, reused for PoD); one revocation
  authority serving P48's grant UI and P52's redemption; one `TrackFrame` shared with the
  customer view (P51's own two-consumers rule, honored from this side).
- **P6 CAUSE-AND-EFFECT** (determinism as law): HRW assignment, duty folds, statements, and
  the end-to-end sequence are all pure functions of event history — every claim in §3 carries
  an event-sequence falsifier, not an end-state check.
- **P7 GENDER** (paired verification, no self-certification): the surface's every terminal
  claim is refereed by an independent authority — Delivered by k-of-n hub signatures,
  earnings by the ledger reconciliation, claim state by the receiver-side Law on every node;
  the surface certifies nothing about itself.

(Other principles not load-bearing here; not claimed decoratively.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites both repos; the availability-gap grep re-run; the PoD-shape finding) |
| 2 DoD | §5 (per-item + the phase-level end-to-end falsifier) |
| 3 spec/event-driven TDD | §2 spec-first; §3 RED-first; event-sequence assertions throughout (K2/K3/K4) |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.1-3.7 (revoked toggle, illegal accept, island fabrication, swipe-fake, tamper/misattribution/duplicate at the surface, spent invite, double-tap, amount tamper) |
| 6 hazard-safety as math | §4.1 (six unreachable-state arguments) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 (each with named break point or honest none) |
| 9 Linux discipline | §4.6 (verdicts incl. TWO honest GAPs: camera path, tipping silence) |
| 10 benchmarks+telemetry | §6 (inherited budgets cross-checked; one new measured number; decorative benches refused) |
| 11 isolation/bulkhead | §4.3 |
| 12 mesh awareness | §4.4 (per-frame byte budgets; node-local duty stated) |
| 13 rollback/self-heal vocabulary | §4.5 (requeue = the one Self-Healing claim; Snapshot refused) |
| 14 error-propagation gates | §4.3 (five named gates incl. the extended no-scoring grep) |
| 15 living memory | §4.6 (history = demoted tier) |
| 16 tensor/spectral | §4.6 (honestly N/A — all math is upstream) |
| 17 regression ledger | §5 (six rows named) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §0/§1 (claim/matcher/PoD/router/track/cap ALL consumed; DZ-08 reused; five rejected alternatives §2) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Dependency order below. T1-T3 are buildable today (kernel-side, zero network, zero GPU);
T4-T5 need P37's routes merged; T6 needs P39's `enroll_device`; T7's render legs need P38a
pipelines (CPU compose path works today per P51's precedent); nothing waits on O18a except
final GPU rendering.

1. **T1 (K1).** Create `kernel/src/courier_surface.rs` per §2 (`DutyFold`/`DutyEvent`/
   `fold_duty`/`candidates` with the bootstrap-rule doc comment verbatim). RED-first:
   `k1_candidates_bootstrap_all_certified`, `k1_toggle_off_excludes`, duty≠claim decoupling.
   Extend `ci-no-courier-scoring.sh`'s file set to cover this module (prove teeth: add a
   `score: f32` field, confirm CI RED, remove). Acceptance:
   `cargo test -p dowiz-kernel courier_surface` green; gate teeth shown in the commit message.
2. **T2 (K2 fold half).** Add `CourierSurfaceState`/`ClaimCard`/`ActiveRun` + the inbox fold
   per §3.2 with the event-sequence tests + illegal-accept + at-most-one-active arms. Claim
   legality via the receiver Law ONLY (`event_dict::assert_status_transition`,
   `claim_machine::assert_transition` on the bebop side — consume, never copy the tables).
   Acceptance: fold tests green.
3. **T3 (K5).** `fold_statement` per §2/§3.5 incl. the reconciliation property (reuse P47
   DoD-3's shape — coordinate with that phase's fixture, do not fork it). Acceptance:
   statement tests green, integer-exact.
4. **T4 (K1 route + K2 wire).** Add `POST /api/courier/duty` + the claim accept/decline
   relays to `tools/native-spa-server` under P37's existing `CapVerifier` middleware and
   bulkhead layers (extend `api.rs`'s route table per its §2 conventions — any FSM/money
   vocabulary in handlers trips P37's r11 grep gate). RED-first: revoked-courier 403; island
   queued-intent honesty test. Acceptance: new routes green; P37's r1-r15 suite re-runs green.
5. **T5 (K4).** PoD capture per §3.4: `pod_location_bytes` (12-byte micro7 encoding — cite
   P51 §3 in the doc comment), capture→claim assembly→signature-collection surface flow with
   the pod.rs adversarial arms asserted at the surface. Photo leg `#[ignore = "M3-blob-path"]`
   with the marker string exactly as written. Acceptance: `k4_capture_to_settled_roundtrip`
   green with a 3-hub/k=2 fixture.
6. **T6 (K6).** `EnrollInvite`/`redeem_invite` per §2 over `DOMAIN_DELEGATION`
   (`kernel/src/ports/agent/cap.rs:33`) + P39's `enroll_device`; QR/deep-link payload format
   documented in the module header; the five refusal arms RED-first; the courier-#1 manual
   ceremony documented as a runbook section in the module README (NOT code). Acceptance:
   `k6_invite_roundtrip_mints_cert` green end to end through P37's middleware.
7. **T7 (K3 + K7 + surface render).** The run screen composing P51's `map_scene`/`TrackFrame`
   through P38a (Sea grammar per DZ-08; SwipeToComplete with the never-fake gate as a test);
   K7's witness-typed attestation builder; the phase-level `p52_courier_end_to_end`. GPU legs
   `#[ignore = "O18a"]` per the standing convention; CPU-compose assertions land now.
   Acceptance: end-to-end test green on the CPU path; six REGRESSION-LEDGER rows added.

**Forbidden in this phase (for the zero-context reader):** no score/rating/rank/reputation
field ANYWHERE (CI-locked); no new proto-cap `Action`/`Resource` variants; no map/routing/
Kalman code (consume P51); no money arithmetic in surface code (hub-derived amounts only); no
DOM-visible widgets (P38 gates apply); no batching; no tipping (needs its own operator
ruling); no photo work before the M3 blob decision beyond the ignored marker.

---

## 11. K8 — run-scoped conversation pane (2026-07-18 addendum; consumes P48 §10, owns the courier half)

> Appended after §10; §0–§10 untouched (the P43-§11 append convention). Origin: the
> operator's same-day follow-up to the conversations directive — "так, і це фіча як для
> власника, так і для кур'єра" — the unified-conversation feature serves the courier too.
> The SHARED spine (the `Conversation` type, the channel-routing law, channel-continuity,
> the autonomy machinery, and the structural order-authority exclusion) is designed once,
> in `BLUEPRINT-P48-owner-hub-surface.md` §10 — read §10.1/§10.2/§10.6/§10.7 there before
> building this. This section owns ONLY what is courier-specific: the run-screen pane, the
> access-window enforcement, and the courier-side gates.

### 11.1 The pane (spec)

The `ActiveRun` screen (§3.3) gains a conversation region: the thread bound to the active
claim's order (the order's intake binding names its `ConversationKey` — P48 §10.1), rendered
through P38a in log order with P48 §10.3's provenance badges (customer/owner/courier/agent).
The courier can: read the thread, send a manual reply ("залишу під дверима — добре"), and —
where the VENUE's `AutonomyPolicy` permits — receive agent-drafted reply suggestions
(`DraftFamily::{StatusAnswer, ClarificationReply}` via the same `draft_conversation_reply`
tool, `Surface::Courier`). Every outbound reply obeys the channel-routing law: it egresses
on the conversation's OWN channel via P43's `ChannelSend` — the customer who ordered on
Telegram gets the courier's "not home? — leaving at the door" on Telegram, never elsewhere.

**One thread, no handoff (the shared-vs-separate answer, restated from P48 §10.7 as this
side's law):** the courier does NOT get a separate thread with the customer and there is no
handoff ceremony at pickup — owner and courier are stage-scoped participants in the ONE
conversation per (venue, channel, peer). The customer's channel-side view is continuous;
routing decides who acts. The owner's inbox keeps full visibility throughout (the venue owns
its log — M5), so an in-transit exchange the courier handles is never invisible to the owner.

### 11.2 The access window — a law, and deliberately the SAME window as P51's position privacy

Courier access to the conversation (read AND draft AND send) exists exactly from
claim-accept to delivery-complete — the identical window P51 M6 already imposes on
`CourierPositionUpdated` emission, consumed here as a second application of the same
invariant: **a courier's visibility into a customer, in every form (location, messages),
exists only while carrying that customer's order.** Before accept and after `Delivered`,
access is a typed refusal; the window is fold-derived from the claim Law (like duty —
never a stored flag). This is not a venue setting — no configuration widens it.

### 11.3 Courier-side gates (cross-referenced, NOT redesigned — each already lives where it lives)

- **Order authority:** the agent on this surface can never confirm/cancel an order — P48
  §10.6's three walls (no `ToolAction` variant, closed config, `no-agent-order-authority`
  CI gate) apply verbatim; the CI grep set covers the courier lane.
- **Delivery-complete / PoD / cash attestation:** remain witness-typed human acts exactly
  as §3.4/§3.4b/§3.7 design them — none is a `ToolAction`, none is a `DraftFamily`, none
  is autonomy-eligible under any configuration. An agent may draft the MESSAGE "I've
  arrived"; it can never emit the `Delivered` intent, sign a PoD leg, or attest cash. The
  §10.6 grep pattern's `Complete|Settle|Deliver` alternates are exactly these.
- **Autonomy config:** the venue's `AutonomyPolicy` governs; the courier's surface adds a
  personal kill switch and may only tighten, never widen (P48 §10.7).
- **No scoring:** conversation events carry no courier metric of any kind;
  `ci-no-courier-scoring.sh`'s file set extends over the K8 modules (same teeth-proof
  choreography as §10-T1).

### 11.4 RED tests, DoD row, adversarials

RED today: no conversation concept exists anywhere (P48 §10.8's grep preamble). GREEN:

- `k8_courier_access_windowed` — read/draft attempts before claim-accept and after
  `Delivered` ⇒ typed refusal, zero `ConversationEvent`s appended (the window law).
- `k8_one_thread_per_peer` — customer messages before order, in transit, and after
  delivery land in ONE `conversation_id`; the courier sees only the in-window slice
  surface-side; the customer-side channel never changes.
- `k8_reply_same_channel` — courier reply on a Telegram-originated conversation egresses
  via the Telegram adapter (spy `ChannelSend`; `receipt.channel == key.channel`).
- `k8_agent_cannot_complete_delivery` — grep/compile check: no agent-lane symbol reaches
  the `Delivered` emit-site, the PoD signing call, or the attestation builder (the
  witness types make this a compile fact; the test pins it against regression).
- **Adversarial:** (i) two concurrent orders, one peer, two couriers ⇒ both run panes
  surface the thread badge-tagged per order; an order-ambiguous inbound routes through
  H1's `Ambiguous` family (clarification), never a guess — P48 §10.7's named case,
  asserted from this side; (ii) a courier attempting to read a conversation for an order
  they DECLINED ⇒ refusal (window never opened); (iii) hostile message text renders inert
  on the run screen (P48 §10.3-adv-ii's fixture class, re-run in this pane).

DoD table (extends §5): **K8** | RED: no conversation pane | GREEN: the four tests above +
adversarials | permanent: `k8_courier_access_windowed` (ledger row, joining P48 §10.8's
row h). The §5 phase-level falsifier `p52_courier_end_to_end` is NOT extended — K8 is
additive and must not gate the MVP courier leg (a venue with zero conversation traffic
still delivers); stated so nobody couples them.

**Sequencing:** after P48's T9 (the spine) and P52's T2/T4 (claim fold + routes) — the
window is derived from the claim fold, so K8 cannot land first. Forbidden here: any
courier-side conversation store (the venue's log is the one store); any access-window
configuration surface; any second thread per order or per courier.
