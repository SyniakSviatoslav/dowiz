# BLUEPRINT P34 — Wire mesh-real's proven delivery-domain into the dowiz kernel (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 below — every point
> addressed, none skipped). This phase IS `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
> §10.5.2's **P34** — the phase the roadmap itself names "THE highest-leverage single phase in
> this entire document" (§10.1), because it converts ~70% of already-built, already-tested
> protocol code from stranded to load-bearing. This blueprint formalizes §10.5.2's DoD-1..6 to
> the standard; it does not contradict or re-derive them. Source blueprints reused, not
> re-derived: `docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` + `MESH-REAL-PLAN.md`
> (MESH-12 resolution: `mesh-real/MESH-12-RESOLVED-2026-07-14.md`).
>
> **Headline ground-truth finding of this pass (leads everything below):** the "proven" spine is
> **RED right now** — `cargo test -p bebop-delivery-domain --features kernel-rlib` fails to
> compile (E0004, §0 row 12) because dowiz-kernel's P07 compensation states drifted past the
> bebop-side wire mapping and **no CI in either repo gates the cross-repo feature build**. P34 is
> therefore not only registration-and-wiring: it is the anti-rot ratchet that makes this exact
> failure class impossible to reintroduce silently.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `dowiz` `main` @ `03ac0fefe9457d1fa717151bbf3455aa9cff7450` and
`bebop-repo` `main` @ `e56ba6a35258ced76752510625511f37a6367a77` (both working trees clean,
`git status --porcelain` empty). Inherited cites come from §10.5.2's done-inventory and the
dispatch brief; matches and drifts stated exactly. All bebop paths below are relative to
`/root/bebop-repo/bebop2/`; dowiz paths relative to `/root/dowiz/`.

| # | Claim | Fresh `file:line` (this pass) | Inherited cite | Status |
|---|---|---|---|---|
| 1 | MESH-01 delivery-domain crate, 1844 lines | `delivery-domain/src/`: `lib.rs` 558 + `intake.rs` 485 + `pod.rs` 358 + `finalization.rs` 252 + `hub_ring.rs` 191 = **1844** | §10.5.2: "1844 lines" | **MATCH exact** |
| 2 | Solo-island offline proof | `delivery-domain/src/intake.rs:408` `ac6_solo_island_full_flow_no_peers` (full lifecycle, zero peers, asserts `OrderStatus::Delivered` — the **kernel's** enum) | §10.5.2: same name | **MATCH** — but see row 12: it does not compile today |
| 3 | MESH-02 KernelFacade, generic seam | `proto-cap/src/facade.rs`: `trait EventSink` `:56` (`fn apply(&self, frame: &SignedFrame) -> Vec<Event>`, **`&self`, no `Result`**), `struct KernelFacade` `:65`, `submit_intent` **`:123`** (`(&self, frame: &SignedFrame) -> Result<Vec<Event>, Reject>`), `Event` `:28`, `Reject{reason: CapError}` `:39-43` | §10.5.2 + brief: "`facade.rs:64` submit_intent" | **DRIFT** — `:64` is the doc comment naming the method; the fn is at `:123`. |
| 4 | **SECOND KernelFacade (new finding — dual-facade)** | `delivery-domain/src/lib.rs:271` `pub mod facade` (kernel-rlib-gated): its own `KernelFacade` with `submit_intent` `:329` — `(&self, frame, chain: &[Delegation], order: &Order, next: OrderStatus, now: u64) -> Result<AppliedEvent, Reject>`; `AppliedEvent` `:283`; `Reject::{Wire,Law,Money}` `:291`. Explicit WIRE→LAW→MONEY gates in the body (`:337-352`) | not in any inherited cite | **NEW** — two structs named `KernelFacade`. The delivery-domain one is the facade the proven spine actually uses (`intake.rs:46`); proto-cap's is the transport-generic sink seam. §1 resolves which one P34 consumes. |
| 5 | MESH-03 event vocabulary "five `DeliveryEvent` variants" | `proto-cap/src/event_dict.rs:278` `pub enum DeliveryEvent` has **FOUR** variants: `OrderPlaced(OrderPlacedPayload)`, `StatusChanged(StatusChangedPayload)`, `Claim(ClaimPayload)`, `Settlement(LedgerPayload)`; fail-closed `decode` `:286`. The **six** delivery *actions* live in `proto-cap/src/scope.rs:90-104`: `OrderPlaced`, `OrderStatusChanged`, `ClaimOffered`, `ClaimAccepted`, `ClaimReleased`, `SettlementRecorded` | §10.5.2: "`event_dict.rs:278-299`, `DeliveryEvent::{OrderPlaced,ClaimOffered,ClaimAccepted,ClaimReleased,SettlementRecorded}`" | **DRIFT (imprecise inherited claim)** — those five names are `Action` variants (and omit `OrderStatusChanged`), not `DeliveryEvent` variants. Vocabulary = 4 payload shapes × 6 actions. §3.2 and the DoD use the live shape. |
| 6 | Payload shapes | `event_dict.rs`: `OrderPlacedPayload{order_id:u64, amount_i64:i64, src, dst}` `:106`, `ClaimPayload{claim_id, order_id, courier: CourierKey}` `:116`, `LedgerPayload{order_id, amount_i64}` `:124`, `StatusChangedPayload{order_id, from, to}` `:132`; `CourierKey = [u8;32]` `:25`; fixed-layout `encode`/`decode` pairs `:192-262` | — | verified |
| 7 | MESH-04 claim_machine | `proto-cap/src/claim_machine.rs:85` `assert_transition(from: ClaimStatus, to: ClaimStatus) -> Result<(), ClaimError>`; `fold_transitions` `:98`; `ClaimStatus` `:21` = `{Offered, Claimed, Released, PickedUp}` — zero scoring fields, module doc `:13-17` names the NO-COURIER-SCORING structural constraint | §10.5.2: `claim_machine.rs:85` | **MATCH exact** |
| 8 | MESH-05 matcher, deterministic HRW | `proto-cap/src/matcher.rs:63` `assign(order: &Order, candidates: &[Courier], max: usize) -> Vec<CourierKey>`; `hrw_weight` `:41` (pure FNV-1a over `order_id‖pubkey`); tie-break weight-DESC-then-pubkey-ASC in `assign`; `Courier` `:34` carries **only** `pubkey` (score is unrepresentable at type level) | §10.5.2: `matcher.rs:63` | **MATCH exact** |
| 9 | hub_ring assignment | `delivery-domain/src/hub_ring.rs:68` `assign(order_id: u64, hubs: &[Hub], replica_count: usize) -> Ownership`; replica clamp `hubs.len().saturating_sub(1)`; R=0 solo-island degenerate case documented in the doc comment | §10.5.2 + brief: `hub_ring.rs:62` | **DRIFT +6 lines** — `:62` is inside `ranked()`'s comparator; the fn is at `:68` |
| 10 | MESH-07 Merkle log | `proto-wire/src/sync_pull.rs:422` `pub struct MerkleLog`; file is 1181 lines | §10.5.2: same | **MATCH exact** |
| 11 | **The reverse dependency edge already exists (opt-in)** | `delivery-domain/Cargo.toml:16` `kernel-rlib = ["dep:dowiz-kernel", "dep:bebop-proto-cap", "dep:bebop-mesh-node", "dep:bebop2-core"]`; `:25` `dowiz-kernel = { path = "../../../dowiz/kernel", default-features = false, features = ["std"], optional = true }`; `intake.rs:43-44` imports `dowiz_kernel::domain::{apply_event, place_order, Order, OrderItem}` + `order_machine::{assert_transition, OrderStatus}`; `lib.rs:23-31` re-exports kernel Law/money fns under the feature | P13 blueprint claims zero dependency (row 14) | **NEW vs the P13 story** — bebop→dowiz edge is real, feature-gated, and carries the whole P13 spine (`lib.rs:363-380`: `hub_ring`/`intake`/`pod`/`finalization` all `#[cfg(feature = "kernel-rlib")]`) |
| 12 | 🔴 **LIVE RED — the spine does not compile today** | `cd /root/bebop-repo && cargo test -p bebop-delivery-domain --features kernel-rlib` → `error[E0004]: non-exhaustive patterns: OrderStatus::Refunding and OrderStatus::CompensatedRefund not covered` in `from_order_status` (`intake.rs:69-82` — covers 10 of the kernel's 12 states; `Scheduled => None` handled, the two P07 compensation states are not). Run this pass on a clean tree at the HEADs above | §10.5.2 status: "all absorbed units DONE"; §10.2: "PROVEN" | **REGRESSION FOUND THIS PASS** — dowiz-kernel `order_machine.rs:8-25` gained `Refunding`/`CompensatedRefund` (P07) after the spine was written. "Proven" was true when written, is false live. Fix is 1 match arm (§3.1); the *cause* (zero CI gating) is W-1's whole point |
| 13 | kernel-rlib is CI-gated nowhere | `grep -rn "kernel-rlib" /root/bebop-repo/.github/workflows/` → 0 hits; `grep -rn "kernel-rlib\|bebop" /root/dowiz/.github/workflows/ci.yml` → 0 hits. Bebop's default `cargo test --workspace` never compiles the feature (`delivery-domain/Cargo.toml` `default = []`) | — | verified — explains how row 12 rotted undetected |
| 14 | The false premise to correct | `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md:23`: "**dowiz today has ZERO code-level dependency on the bebop protocol.** This is not \"loosely coupled\" or \"decoupled pending re-integration\" — it is *absent*." Still present unqualified this pass | §10.5.2 connective-tissue finding | **CONFIRMED still live.** True as a *dowiz→bebop wiring* statement (no dowiz crate imports bebop — verified: `grep -rn "bebop" dowiz/*/Cargo.toml` → one comment in `kernel/Cargo.toml:18`). **False as a build-readiness statement**: rows 1-11 are ready-to-consume prerequisites, and the bebop→dowiz reverse edge (row 11) already carries P13's own spine — built in bebop-repo, under `kernel-rlib`, after that sentence was written. DoD-6 amends it |
| 15 | dowiz kernel wiring surface | `kernel/src/order_machine.rs`: `OrderStatus` `:8` (12 variants incl. `Scheduled` scaffold + P07 `Refunding`/`CompensatedRefund`), `assert_transition` `:139`, `fold_transitions` `:156`. `kernel/src/domain.rs`: `OrderItem{product_id, modifier_ids, quantity, unit_price}` `:30`, `Order` `:42` (`id: String`, `ledger`, `price_trusted`, …), `post_earn` `:86`, `ledger_balance` `:104` (conservation probe: 0 ⟺ fully compensated), `compute_order_total` `:129`, `place_order` `:156`, `apply_event(order: &Order, next: OrderStatus) -> Result<Order, TransitionError>` `:256` (pure — no mutation on `Err`). `kernel/src/money.rs`: `reversed_leg` `:164`, `ledger_append` `:185`, `apply_tax` `:270` | brief: "domain.rs (order_machine decide/fold), money.rs" | verified, sharpened to exact symbols |
| 16 | dowiz repo layout + CI | **No root workspace** — 6 standalone crates (`kernel/`, `engine/`, `wasm/`, `llm-adapters/`, `agent-adapters/`, `agent-governance-wasm/`); CI `cargo-test` job at `.github/workflows/ci.yml:106-120` runs kernel+engine `--offline` per-manifest (P01 CI-Truth Floor) | — | verified — the adapter crate joins as a 7th standalone crate + its own CI job (§2, §3.1) |
| 17 | Status maps, both directions | wire→kernel `to_order_status` `intake.rs:54` (total over the 9 wire states); kernel→wire `from_order_status` `intake.rs:69` (partial, `Option`, fail-closed `None` for unmappable states — used at `intake.rs:260` → `FoldError::BadPayload`) | — | verified — the fail-closed pattern §3.2 extends to the P07 states |
| 18 | **Dual legality tables (drift hazard)** | wire-side mirror: `event_dict.rs:75` `allowed_next(DeliveryStatus)` + `:90` `assert_status_transition`, self-described "1:1 with `dowiz_kernel::order_machine`"; kernel-side: `order_machine.rs:139`. Two tables, one Law, **no parity test exists** between them | — | **NEW finding** — the same drift class as row 12, currently ungated. §3.2's parity sweep closes it |
| 19 | Replay/DOD hygiene already proven | `intake.rs:87` `event_id_of` = low 64 bits of SHA3-256(payload); tests `ac1_dod_replay_rejected_on_second_apply` `:459`, `ac1_dod_rejects_empty_payload_before_wire` `:428`, expired-frame `:477`; forged-skip rejection `lib.rs:182` `r_mesh01_forge_pending_to_delivered_rejected_on_every_receiver` | brief/mesh-real AC-1/AC-2 | verified (subject to row 12 — none of these currently compile) |
| 20 | mesh-real's own status table is stale-RED | `docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md:202` still lists MESH-02 as 🔴 | §10.5.2 supersedes with live DONE | confirmed stale — §10 is authoritative; this blueprint cites the mesh-real doc for *design*, never for *status* |

Ground truth is non-discussible; everything below builds on the fresh column only.

---

## 1. Scope — what P34 owns and what it deliberately does NOT own

**P34's single sentence:** make a dowiz workspace member the consumer of the already-built bebop2
delivery protocol — dependency edge, CI gate, event-vocabulary proof, claim/matcher consumption,
and the solo-island flow re-anchored on dowiz's own decider — ending at "the dowiz kernel can
drive delivery-domain," NOT at "a customer can place an order over HTTP."

**P34 owns (build items §3):**

| Item | §10.5.2 DoD | Content |
|---|---|---|
| W-1 | (new) DoD-0 + DoD-1 | Fix the live E0004 regression (§0 row 12, one match arm, fail-closed); create the `mesh-adapter` crate + cargo dependency edge; CI job gating BOTH directions of the cross-repo build |
| W-2 | DoD-3 | Event-vocabulary proof: encode/decode round-trip on all 4 payload shapes × 6 actions; total/partial status-map proofs; the dual-legality-table parity sweep (§0 row 18) |
| W-3 | DoD-3 | claim_machine integration: full claim lifecycle folded dowiz-side, illegal-skip adversarial |
| W-4 | DoD-4 | matcher + hub_ring consumption: determinism proven (repetition, permutation, teeth), replica clamp, zero scoring inputs |
| W-5 | DoD-5 | Solo-island re-anchor: `ac6`'s scenario driven from a REAL dowiz `place_order` order with money invariants asserted, plus forged-skip/replay adversarials, dowiz-side |
| W-6 | DoD-2 + DoD-6 | Compilation-firewall red-proof (committed, CI-executed) + blueprint reconciliation (P13/P09/P10 amended, quote date-scoped) |

**P34 does NOT own (anti-scope, binding — each with its owner):**

- **No rewrite or fork of delivery-domain.** It is built and (modulo §0 row 12's one-arm fix)
  proven; P34 consumes the sibling crate via path dependency. Any "modernize the spine" impulse
  is out (standard item 19 cuts both ways: reuse-first).
- **No new event variants.** The vocabulary is the live one in §0 rows 5-6 (4 payload shapes,
  6 actions) — nothing added, nothing renamed. Kernel states with no wire mapping stay
  unmappable-fail-closed (§3.2), not newly mapped.
- **No courier scoring / reputation / ranking of any kind** — standing rejection (trust = signed
  capability, never reputation; the `Courier` type structurally cannot carry a score, §0 row 8,
  double-locked by `bebop-repo/scripts/ci-no-courier-scoring.sh`).
- **P34B** owns per-node pgrust storage, the CRDT compile-fence, the iroh transport half, and
  ML-KEM KAT (§10.5.2 P34B). P34 is node-local and transport-free.
- **P36** owns the insecure-TLS default and the `no_std` wasm32 regression. P34 must not
  serialize behind it (§10.5.2: "never serialize P34 behind it") — and note W-1's E0004 fix is a
  *third*, previously-uncounted live regression, in delivery-domain not bebop2-core, so it
  belongs to P34 (it is the wiring seam itself), not to P36.
- **P35** owns wasm-host/zero-OCI runtime homes. Independent lane.
- **P37 (DELIVERY)** owns the HTTP/API surface. P34's exit is a green in-process test suite,
  deliberately one layer below any server.
- **P20 DM-1 / P39** own discount math and app-shell wiring — untouched.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

### 2.1 The adapter crate — decision and justification

**Decision: new standalone crate `mesh-adapter/` (package `dowiz-mesh-adapter`)**, a 7th sibling
of `kernel/`/`engine/` (§0 row 16 — there is no root workspace to join, so "workspace member" in
§10.5.2 DoD-1 means "repo crate with its own CI-gated manifest", same standing as `engine`).

**Why a new crate is structurally forced, not stylistic (hazard-safety-as-math, item 6):** the
`kernel-rlib` feature closure of `bebop-delivery-domain` **contains `dowiz-kernel`** (§0 row 11).
Therefore any dependency edge `dowiz-kernel → bebop-delivery-domain(kernel-rlib)` creates the
package cycle `dowiz-kernel → bebop-delivery-domain → dowiz-kernel`, which cargo rejects at
resolution time — the consumer crate provably cannot be `dowiz-kernel` itself. Wiring inside
`engine` is rejected because engine is the spectral/field-math organ (wrong concern, and it would
drag bebop2-core/mesh-node into every engine build). Rejected alternative names (DECART, one line
each): `bebop-adapter` — vendor-named, but the seam is the mesh *protocol* (bebop is the current
supplier behind it); existing dirs name the domain (`llm-adapters`, `agent-adapters`), so:
`mesh-adapter`, package `dowiz-mesh-adapter` (prefix convention of `dowiz-kernel`/`dowiz-engine`).

### 2.2 The manifest — the dependency edge, verbatim

```toml
# mesh-adapter/Cargo.toml  (NEW — the P34 dependency edge, DoD-1)
[package]
name = "dowiz-mesh-adapter"
version = "0.1.0"
edition = "2021"
license = "AGPL-3.0-or-later"
description = "P34 wiring: dowiz-kernel as the consumer/driver of the bebop2 delivery protocol (delivery-domain kernel-rlib spine + proto-cap vocabulary/matcher/claims). Test-first adapter; no HTTP, no transport, no storage."

[dependencies]
dowiz-kernel = { path = "../kernel", default-features = false, features = ["std"] }
bebop-delivery-domain = { path = "../../bebop-repo/bebop2/delivery-domain", features = ["kernel-rlib"] }
bebop-proto-cap = { path = "../../bebop-repo/bebop2/proto-cap" }

[dev-dependencies]
criterion = "0.5"

[[bench]]
name = "criterion"
harness = false
```

**Type-identity argument (load-bearing):** the adapter's `dowiz-kernel` resolves to
`/root/dowiz/kernel`; delivery-domain's optional `dowiz-kernel` resolves via
`../../../dowiz/kernel` to the same canonical path (§0 row 11). One path ⇒ cargo unifies to ONE
package instance ⇒ `dowiz_kernel::domain::Order` seen by the adapter **is the same type** folded
inside `intake.rs` — no type-identity split is representable. Feature union is exact:
both request `default-features = false, features = ["std"]`.

### 2.3 New named types/constants (everything new, up front — no magic numbers)

```rust
// ── mesh-adapter/src/lib.rs — the ONLY new production types P34 introduces ──

/// CI supply-chain pin: the OpenBebop commit the cross-repo CI job checks out.
/// Bumping it is a reviewed, deliberate commit (never a floating branch ref).
pub const OPENBEBOP_CI_PIN: &str = "e56ba6a35258ced76752510625511f37a6367a77";

/// Dowiz-side host state for driving the spine in tests and for the
/// proto-cap generic-sink proof. Keyed by the WIRE order id (u64); the kernel
/// `Order.id` (String) is derived as `format!("ord-{wire_id}")` — one direction,
/// one format, stated here so no second convention can appear.
pub struct MeshHost {
    orders: std::collections::BTreeMap<u64, dowiz_kernel::domain::Order>,
    claims: std::collections::BTreeMap<u64, bebop_proto_cap::claim_machine::ClaimStatus>,
    /// Monotonic ledger-entry id source for `Order::post_earn` (never reused).
    next_entry_id: u64,
}

/// Why the host refused to apply a decoded DeliveryEvent. Mirrors the spine's
/// FoldError tagging (intake.rs:98) at the host level; every arm is fail-closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostFault {
    UnknownOrder(u64),
    Vocabulary(&'static str),                                  // decode / unmappable state
    Law(dowiz_kernel::order_machine::TransitionError),          // assert_transition refused
    Money(String),                                              // ledger_append/post_earn refused
}

impl MeshHost {
    /// The event-application law of the host: decode → dispatch → kernel fold.
    /// Pure per call w.r.t. the kernel (apply_event is &Order -> Order); the
    /// host map is updated ONLY on Ok (no partial application representable).
    pub fn apply_delivery_event(
        &mut self,
        scope: bebop_proto_cap::scope::Scope,
        payload: &[u8],
    ) -> Result<(), HostFault>;
}

// ── the bebop-side fix (W-1, delivery-domain/src/intake.rs:69) — spec of the diff ──
// from_order_status gains exactly one arm; fail-closed like Scheduled, because the
// P07 compensation states are node-local money-repair states with no wire vocabulary
// (anti-scope: no new event variants). A compensation that must cross the wire is
// future P34B/P37 scope, by a NEW dated decision, never a silent nearest-status map:
//   OrderStatus::Refunding | OrderStatus::CompensatedRefund => None,
```

**Concrete facade consumption call sites (the two seams, exactly as typed today):**

```rust
// (i) THE SPINE SEAM — what W-5 drives (delivery-domain facade, kernel-rlib):
//     delivery-domain/src/lib.rs:329
let applied: Result<AppliedEvent, Reject> = facade.submit_intent(
    &frame,               // &SignedFrame  — hybrid-signed capability frame
    edge.chain(),         // &[Delegation] — anchor-rooted delegation chain
    &order,               // &Order        — LOCAL state (receiver never trusts sender)
    next,                 // OrderStatus   — the intended kernel transition
    now,                  // u64           — monotonic tick for expiry
);
// Reject::{Wire,Law,Money} tags which gate refused — WIRE→LAW→MONEY ordering is
// carried by the function body (lib.rs:337-352), not by caller convention.

// (ii) THE GENERIC SINK SEAM — what W-2's sink proof exercises (proto-cap facade.rs:123):
//     EventSink::apply is `&self` and returns Vec<Event> (facade.rs:56-59) — it cannot
//     carry a Law rejection. The MeshHost impl therefore wraps interior mutability and
//     maps HostFault to an EMPTY event vec (fail-closed: refusal produces no event);
//     Law-tagged rejection reporting lives on seam (i). This asymmetry is a live seam
//     wart, named here, not papered over — P37's server must consume seam (i).
let events: Result<Vec<Event>, Reject> = kernel_facade.submit_intent(&frame);
```

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

Spec-driven + event-driven TDD: §2's types are the spec, every item's RED test precedes its
code, and the asserted objects are event sequences (frames applied / refused in order), matching
the kernel's own decide/fold law.

### 3.1 W-1 — the regression fix, the dependency edge, and the CI gate (DoD-0 + DoD-1)

**(a) Regression fix, RED-first by construction.** The RED already exists and is not written by
us: `cargo test -p bebop-delivery-domain --features kernel-rlib` fails E0004 today (§0 row 12).
GREEN = add the one fail-closed arm (§2.3 spec) in `intake.rs:69` `from_order_status` + a named
unit test in `intake.rs`'s test mod:

```rust
#[test]
fn p07_compensation_states_have_no_wire_mapping_fail_closed() {
    // The two P07 states are node-local money-repair; the wire must refuse them,
    // never nearest-map them (a silent Refunding→Cancelled map would lie to peers).
    assert_eq!(from_order_status(OrderStatus::Refunding), None);
    assert_eq!(from_order_status(OrderStatus::CompensatedRefund), None);
    assert_eq!(from_order_status(OrderStatus::Scheduled), None); // existing precedent pinned
}
```

This lands in **bebop-repo** (pushed to the `openbebop` remote per the standing repo-routing
rule — bebop files → `/root/bebop-repo`, never `/root/dowiz`). The exhaustive `match` (no
wildcard!) is the smart-index gate (item 14): any FUTURE kernel `OrderStatus` variant is a
compile error at this exact site, forcing a deliberate mapping decision — provided something
compiles the feature, which is (c).

**(b) The dependency edge.** Create `mesh-adapter/` with §2.2's manifest verbatim + a smoke test
(`tests/edge.rs`) that constructs an `IntakeEdge`, emits one frame, folds it through
`DeliveryReceiver::admit_and_fold`, and maps back via `to_order_status` — proving the whole
kernel-rlib closure links from the dowiz side. RED: the test file committed before the crate
compiles (missing manifest) fails `cargo test`; GREEN: edge resolves, smoke passes.

**(c) CI job — the anti-rot ratchet.** New job `mesh-adapter` in `.github/workflows/ci.yml`
(additive; the existing offline `cargo-test` job at `:106-120` is untouched, its P01 offline
floor intact). Sibling checkout layout mirrors `/root` exactly so BOTH relative path deps
resolve (adapter's `../../bebop-repo/…` and delivery-domain's `../../../dowiz/kernel`):

```yaml
  mesh-adapter:
    name: mesh-adapter (P34 dowiz<->bebop wiring, kernel-rlib gate)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { path: dowiz }
      - uses: actions/checkout@v4
        with:
          repository: SyniakSviatoslav/OpenBebop
          ref: e56ba6a35258ced76752510625511f37a6367a77   # = OPENBEBOP_CI_PIN; bump = reviewed commit
          path: bebop-repo
      # Gate the REVERSE edge (the one that rotted, §0 row 12):
      - run: cargo test --manifest-path bebop-repo/Cargo.toml -p bebop-delivery-domain --features kernel-rlib
      # Gate the FORWARD edge (DoD-1) + the whole P34 suite:
      - run: cargo test --manifest-path dowiz/mesh-adapter/Cargo.toml
      # Firewall red-proof (W-6): a build that MUST fail, § 3.6:
      - run: bash dowiz/mesh-adapter/proofs/firewall-red.sh
```

Honest divergence note: this job fetches the network (checkout + `cargo fetch`), unlike the
P01 offline floor. Rationale: first cross-repo job; the pin gives reproducibility. The named
hardening step (not P34-blocking) is vendoring both lockfiles into the offline cache.

**Adversarial case (designed to break W-1):** a CI dry-run with `OPENBEBOP_CI_PIN` pointing at
the CURRENT (pre-fix) `e56ba6a…` must go RED on the reverse-edge step — proving the gate detects
exactly the live regression class, not merely compiling something. This run is performed once
and its failure recorded in the PR description before the fix lands (the ratchet's own
red-proof).

### 3.2 W-2 — event-vocabulary mapping proof (DoD-3, first half)

Spec: the live vocabulary (§0 rows 5-6, 17, 18). Tests in `mesh-adapter/tests/vocabulary.rs`:

1. **Round-trip, all shapes:** for each payload struct (`OrderPlacedPayload`,
   `StatusChangedPayload`, `ClaimPayload`, `LedgerPayload`): `decode(encode(x)) == x`; and
   `DeliveryEvent::decode(scope, bytes)` (`event_dict.rs:286`) dispatches each of the six
   actions (`scope.rs:90-104`) to the right variant.
2. **Status-map retraction (total on the wire side):** ∀ `d` of the 9 `DeliveryStatus` values:
   `from_order_status(to_order_status(d)) == Some(d)` — the wire→kernel→wire loop is the
   identity, so no wire state aliases another through the kernel.
3. **Fail-closed partiality (the §3.1 test's dowiz-side twin):** the three kernel-only states
   map to `None`, and `DeliveryReceiver` surfaces that as `FoldError::BadPayload`
   (`intake.rs:260`) — refusal, never a fallback.
4. **Dual-legality-table parity sweep (§0 row 18 — the test that closes the ungated drift
   hazard):** ∀ (from, to) ∈ DeliveryStatus² (81 pairs):
   `assert_status_transition(from, to).is_ok()` (`event_dict.rs:90`, the wire mirror)
   `== dowiz_kernel::order_machine::assert_transition(to_order_status(from), to_order_status(to)).is_ok()`
   (`order_machine.rs:139`, the Law). One table drifting from the other — the exact class that
   produced §0 row 12 — becomes a named RED, not a silent fork of the Law.
5. **MeshHost dispatch:** `apply_delivery_event` (§2.3) applied to a scripted event sequence
   (OrderPlaced → StatusChanged* → Settlement) produces the same terminal `Order` as calling
   the kernel directly — the mapping adds routing, never semantics.

RED→GREEN: tests 1-4 are RED against a stub `MeshHost` and before the §3.1(a) fix compiles
(item 4 cannot even build today — its first green IS the wiring proof). **Adversarial:**
(i) truncated payload bytes (`len-1`) into every `decode` → `Err`, never a partial value;
(ii) a scope whose `(Resource, Action)` pair is legal-but-mismatched for the payload bytes
(e.g. `Claim` scope over `LedgerPayload` bytes) → decode `Err` (fail-closed dispatch,
`event_dict.rs:286`); (iii) *the table-sabotage mutation*: a test-local copy of the wire table
with one extra edge (`Pending → Delivered`) must FAIL parity test 4 — proving the sweep has
teeth (run once against the mutated copy, assert inequality, restore).

### 3.3 W-3 — claim_machine integration (DoD-3, second half)

Spec: `ClaimStatus` lifecycle (§0 row 7), driven dowiz-side, tied to a kernel order. Test
`mesh-adapter/tests/claims.rs`:

- **Happy path (event-sequence assertion):** for a placed kernel order, fold
  `Offered → Claimed → PickedUp` through `claim_machine::fold_transitions`
  (`claim_machine.rs:98`) while the order folds `Confirmed → … → InDelivery`; assert the joint
  final state `(ClaimStatus::PickedUp, OrderStatus::InDelivery)` and that every intermediate
  pair was legal (zero illegal transitions — DoD-3's phrasing, made checkable).
- **Release/requeue path:** `Offered → Claimed → Released` then a fresh claim id re-offers —
  the released claim is terminal-legal, the ORDER state is untouched by claim churn (claims are
  pure coordination records, §0 row 7).

RED→GREEN: written against a stub claims map in `MeshHost` (RED on first fold), GREEN once
`apply_delivery_event` routes `DeliveryEvent::Claim` by action. **Adversarial:** (i) the skip
`Offered → PickedUp` (bypassing `Claimed`) → `Err` at exactly index 1 with the fold reporting
the reached state (`fold_transitions` contract), mirroring claim_machine's own `:119` case from
the consumer side; (ii) double-accept `Claimed → Claimed` → `Err` (same-state rejection);
(iii) a `ClaimPayload` naming an unknown `order_id` → `HostFault::UnknownOrder`, and the claims
map is unchanged (no orphan claims representable).

### 3.4 W-4 — matcher + hub_ring consumption with determinism adversarials (DoD-4)

Spec: `matcher::assign` (§0 row 8) + `hub_ring::assign` (§0 row 9). Test
`mesh-adapter/tests/matcher.rs`:

- **Determinism (DoD-4's literal falsifier):** identical `(order, candidates)` → byte-identical
  `Vec<CourierKey>` across (i) two calls, (ii) two independently-constructed candidate vectors,
  (iii) a fresh process (the test binary reruns the pure fn — FNV-1a has no ambient state).
- **Permutation invariance:** shuffle the candidate slice (deterministic Fisher-Yates with a
  fixed seed, no rand dep) → identical assignment (HRW ranks by weight, not input order).
- **hub_ring:** `assign(order_id, hubs, replica_count)` with `replica_count = 0` (solo-island
  degenerate case, documented at `hub_ring.rs:62-67`) and with `replica_count > hubs.len()-1`
  (clamp asserted — never claims a replica that does not exist).

RED→GREEN: trivially GREEN once the edge (W-1b) exists — so the RED here is supplied by the
adversarials, which are written first against deliberately-wrong expectations to prove they can
fail, then corrected (teeth-first discipline). **Adversarial:** (i) *input-sensitivity teeth*:
`order_id+1` must change the ranking for at least one fixture set (proving the weight actually
binds the order id — a constant-output stub would pass pure-determinism tests); (ii) *the
scoring red-line*: a compile-level assertion that `matcher::Courier` has exactly one public
field (`pubkey`) — encoded as a struct-literal construction `Courier { pubkey }` that would
fail to compile if any new (scoring) field were added; the standing gate
`bebop-repo/scripts/ci-no-courier-scoring.sh` double-locks repo-side, this pins it
consumer-side. No ranking/reputation input exists to test because none is representable.

### 3.5 W-5 — solo-island re-anchored on dowiz's own decider (DoD-5)

The existing `ac6` (§0 row 2) drives the spine from a synthetic `fresh_order`. DoD-5 requires
the scenario "driven from the dowiz-kernel decider side, using dowiz's Law as the fold" — so the
re-anchor test starts from a REAL kernel order with money, not a status shell. Test
`mesh-adapter/tests/solo_island.rs`:

```rust
#[test]
fn solo_island_full_flow_from_dowiz_decider_with_money() {
    // 1. A REAL dowiz order: place_order (domain.rs:156) with real items — money law engaged.
    let items = vec![OrderItem { product_id: "espresso".into(), modifier_ids: vec![],
                                 quantity: 2, unit_price: 250 }];
    let mut order = place_order("ord-7".into(), None, items, 1_000, None, None).unwrap();
    let subtotal = order.subtotal;               // kernel-computed, not test-invented
    // 2. The spine, zero peers: single-hub ring (R=0), one intake edge, one receiver.
    let edge = IntakeEdge::new(0x42);
    let mut recv = edge.receiver();
    // 3. Drive the FULL lifecycle as signed frames; fold each through the receiver
    //    (DOD → WIRE → LAW → MONEY) and mirror the result back into the kernel order.
    for (from, to) in lifecycle(7) {             // Pending→Confirmed→…→Delivered
        let frame = edge.emit(7, from, to);
        let status = recv.admit_and_fold(&frame, &order, NOW).expect("solo fold");
        order = apply_event(&order, to_order_status(status)).expect("kernel fold");
    }
    assert_eq!(order.status, OrderStatus::Delivered);
    // 4. Settlement leg: LedgerPayload{order_id:7, amount_i64: subtotal} → post_earn;
    //    conservation probe holds (ledger_balance == subtotal, domain.rs:104).
    order.post_earn(1, subtotal, eur()).unwrap();
    assert_eq!(order.ledger_balance(), subtotal);
    assert_eq!(order.total, subtotal);           // money law and wire agree, integer-exact
}
```

RED→GREEN: RED while §3.1(a) is unfixed (does not compile) and again RED against a stub
`lifecycle`; GREEN completes DoD-5. **Adversarial (the mesh-real AC-2 forge, re-anchored):**
(i) a validly-signed frame carrying `Pending → Delivered` (legal signature, illegal jump) is
rejected by the LAW gate — `Err` tagged `Reject::Law` / `FoldError::Gate` — and the kernel
order is untouched (`apply_event` purity, §0 row 15); (ii) the SAME frame folded twice →
second refused by the DOD replay set (`intake.rs:87` event ids); (iii) an expired frame
(`now` past capability expiry) refused before any kernel touch. All three are re-anchors of
proven bebop-side tests (§0 row 19) onto the dowiz driver — same invariants, new authority.

### 3.6 W-6 — compilation-firewall red-proof + blueprint reconciliation (DoD-2 + DoD-6)

**(a) Firewall red-proof (DoD-2).** The invariant, stated precisely from §0 rows 3-4: *ports and
transport crates never link the kernel; the kernel is reached only through a facade
`submit_intent`.* The only packages whose closure may contain `dowiz-kernel`:
`bebop-delivery-domain` (feature-gated) and `dowiz-mesh-adapter` (the host side — the host
legitimately links both; the firewall constrains PORTS, not the host). Two mechanisms, both
committed:

1. `mesh-adapter/tests/firewall.rs`: runs `cargo metadata` on the bebop workspace and asserts
   `dowiz-kernel` is absent from the dependency closure of `bebop-proto-wire`,
   `bebop-mesh-node`, and `bebop-proto-cap` (the same dependency-graph-lint pattern as the
   existing `bebop-repo/ci/crdt-fence` crate — reuse of a proven gate shape, item 19).
2. `mesh-adapter/proofs/firewall-red.sh` + `firewall-red.patch` (committed): the patch adds
   `use dowiz_kernel::order_machine::assert_transition;` to `proto-wire/src/lib.rs`; the script
   applies it to a scratch copy, runs `cargo check -p bebop-proto-wire`, and **exits 0 only if
   the build FAILS** (E0432 unresolved import — the crate is not in proto-wire's graph, so the
   import cannot resolve), then cleans up. This is §10.3 invariant 5's "committed red-proof
   demonstrat[ing] that adding a direct dowiz-kernel import to any port fails the build",
   executable, in CI (§3.1c last step).

**Adversarial:** the red-proof script run against `bebop-delivery-domain` WITH `kernel-rlib`
enabled must report "import resolves" (i.e. the script's fail-expectation would not hold there)
— proving the proof distinguishes the sanctioned seam from the forbidden ones rather than
rubber-stamping "everything fails".

**(b) Blueprint reconciliation (DoD-6).** Amend, with dated correction notes (append-only, the
aebbbe199 correction-commit style):
- `BLUEPRINT-P13-delivery-on-protocol.md:23` — date-scope the quoted claim: true pre-P34 as a
  dowiz→bebop wiring statement, false as a build-readiness statement since the prerequisites
  (§0 rows 1-11, by MESH unit ID + file path) exist and P13's own spine already landed
  bebop-side under `kernel-rlib` (§0 row 11) — the amendment cites this blueprint's §0.
- `BLUEPRINT-P09-confidential-self-healing-wire.md` + `BLUEPRINT-P10-hub-runtime-kill-switch-boot.md`
  — add the MESH-01/02/04/05 unit-ID + file-path cites (§10.5.2 DoD-6's wording).
- Falsifier: `grep -rn "ZERO code-level dependency" docs/design/sovereign-roadmap-2026-07-16/`
  finds only the date-scoped, amended form.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11-16)

### 4.1 Hazard-safety as math (item 6) — the unsafe states bad wiring risks, and why each is unreachable

- **Double-fold** (same frame applied twice → double money/status effect): the DOD replay set
  keys on `event_id_of` = low 64 bits of SHA3-256(payload) (`intake.rs:87`); an identical frame
  re-presented is refused before the wire gate (proven: `ac1_dod_replay_rejected_on_second_apply`,
  re-anchored in §3.5-ii). Truncation honesty: 64-bit ids collide (birthday) at ~2³² *distinct*
  frames; a collision REJECTS a distinct frame — the failure direction is fail-closed
  (false refusal), never fail-open (double application). At §4.2's frame rates that horizon is
  decades; the axis is named, not hidden.
- **Split-brain event application** (two partitions finalize one order differently, merge
  accepts both): F46's quorum-cert + hash-chain conflict rule (`finalization.rs:1-22`) DETECTS
  the conflicting terminal pair at merge and quarantines the order — rejection + quarantine,
  never last-write-wins. P34 consumes this (it is in the kernel-rlib module set); it does not
  rebuild it.
- **Vocabulary fork** (wire legality table silently diverges from the kernel Law — the class
  that ALREADY happened, §0 rows 12/18): two structural gates after P34: the exhaustive
  wildcard-free `match` in `from_order_status` makes any new kernel state a compile error at
  the seam (item 14's compile-time gate), and §3.2's 81-pair parity sweep makes any edge-level
  drift a named CI RED. Reachability of an undetected fork requires defeating both a compiler
  error and a CI-gated exhaustive sweep simultaneously.
- **Forged-skip acceptance** (valid signature, illegal jump): the LAW gate runs the kernel's
  own `assert_transition` on the RECEIVER's local state (`lib.rs:337-352` gate 2) — acceptance
  of an illegal transition is unrepresentable on every honest node, not policed on the sender
  (§0 row 19's `r_mesh01…` test, re-anchored §3.5-i).
- **Money non-conservation via settlement wiring**: `apply_event` is pure (`&Order → Order`,
  no mutation on `Err`); the ledger is append-only with reversal legs (`money.rs:164,185`);
  `ledger_balance()` (`domain.rs:104`) is the conservation probe asserted in §3.5 step 4. A
  half-applied event is not representable: the host map updates only on `Ok` (§2.3).
- **The cycle argument** (§2.1): a dowiz-kernel→delivery-domain edge is rejected by cargo's
  acyclicity requirement — the unsafe topology cannot build.

Per the authority-boundary doctrine: every P34 check is in the internal-invariant class (parity,
purity, exhaustiveness — authority dissolves into the type system and CI); the tamper leg
(P06/key_V) is untouched and NOT a P34 dependency.

### 4.2 Schemas designed for scaling (item 8) — growth axes, stated with break points

- **Frames per order:** 1 OrderPlaced + ≤6 StatusChanged (longest legal path
  Pending→Confirmed→Preparing→Ready→InDelivery→Delivered) + ~3 Claim + 1 Settlement ≈ **~10-11
  frames/order**.
- **Bytes per frame:** payload is tiny (`OrderTransition::to_bytes` = 10 bytes, `lib.rs:100`);
  the envelope dominates — hybrid leg ≈ Ed25519 sig 64 B + ML-DSA-65 sig 3309 B (+ chain/pk
  material) ⇒ **~5-6 KB/frame upper bound**, so ~60 KB of signed event material per order.
- **Event-log/MerkleLog growth:** leaves = frames, linear; at 1 000 orders/day ≈ 10⁴
  leaves/day. Stated break point: ~10⁶ leaves (≈3 months at that rate) is where per-node
  compaction/epoch-snapshotting is needed — that is **P34B's per-node storage scope by
  design** (MESH-06), named here, not built here.
- **Host maps (`MeshHost`):** `BTreeMap` keyed by u64, test-scale (10²-10⁴ orders); the
  production store is P34B/pgrust — the adapter's maps are deliberately not a storage design.

### 4.3 Isolation/bulkhead (11), mesh awareness (12), rollback/self-healing (13), living memory (15)

- **Isolation:** the dependency arrow is strictly `mesh-adapter → {dowiz-kernel, bebop-*}`;
  `dowiz-kernel` gains ZERO new dependencies (its manifest is untouched by P34 — the only bebop
  string in it stays a comment, §0 row 14). A delivery-domain defect therefore cannot enter any
  kernel build, test, or wasm artifact; blast radius of an adapter/bebop failure is the
  `mesh-adapter` CI job. **Panic domain:** in P34 the two sides share a process only inside the
  adapter's own test binaries — a delivery-domain panic fails a test, never a product path (no
  production process couples them until P37, which must then decide its own supervision
  boundary; named handoff, not silently deferred).
- **Mesh awareness:** P34 is deliberately **node-local** — no transport, no gossip, no iroh/WSS
  (P34B/P36 own those). Everything is proven in-process on one node; the payload/frequency
  budget the future propagation layer inherits is §4.2's (~5-6 KB/frame, ≪ the 1 MiB SyncFrame
  ceiling). The solo-island case is not a degraded mode but the F50 base case (R=0,
  `hub_ring.rs:62-67`).
- **Rollback (item-13 vocabulary, used precisely):** P34 claims **Self-Termination /
  unrepresentable-state** (fail-closed vocabulary maps, gate-chain refusals, cargo acyclicity,
  exhaustive matches) and **Snapshot-Re-entry** (the fold is deterministic and pure — replaying
  the frame log from any prior valid state reproduces the exact state; un-folding a bad money
  event = appending its reversal leg (`money.rs:164`), never deleting — the ledger is the
  epoch log). **Self-Healing (redundancy math) is NOT claimed by P34 itself** — the k-of-n
  PoD threshold (`pod.rs:1-16`) is a real redundancy property but it is consumed as built, and
  its k-of-n settlement story belongs to P37's payout saga. Mechanical rollback of P34 whole:
  delete `mesh-adapter/` + the CI job + revert the 2-line intake.rs arm — zero dowiz-kernel
  files are touched by this phase.
- **Living memory (item 15):** the frame log is a time-ordered, content-addressed event stream
  (SHA3 event ids, Merkle leaves) — recall of order history is keyed by invariant content, not
  storage location, the same content-not-location principle as
  `internal-retrieval-living-memory-arc-2026-07-14`; P34B's per-node store is where its
  temporal tiering (attic/demote-never-delete) will apply.

### 4.4 Linux-discipline verdict framework (item 9)

Applying `BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md`'s categories:
consuming the sibling crate instead of forking it = **ALREADY-EQUIVALENT** ("one implementation
of one concept"); the exhaustive-match vocabulary seam = **ALREADY-EQUIVALENT** (the compiler as
gatekeeper — E0004 is doing its job; W-1 merely makes something compile it); the pinned
dual-checkout CI job = **EXTENDS** (cross-repo CI is new machinery for this repo, justified by
the live rot it would have prevented); the committed firewall red-proof = **REINFORCES** (the
repo's existing red-proof/gate culture, e.g. `ci/crdt-fence`, extended to the kernel boundary).

### 4.5 Non-contradiction constraints (sequencing, hard)

P34 must not contradict: (i) §10.5.2 P34's own anti-scope (checked: §1 restates it item-for-item);
(ii) P36's independence — W-1's E0004 fix touches `delivery-domain`, NOT `bebop2-core`/
`proto-wire`, so no P36 file is co-modified; (iii) the P13 spine's design — W-5 drives
`IntakeEdge`/`DeliveryReceiver` as-is (no signature changes); the ONLY bebop-side source diff in
all of P34 is the one match arm of §3.1(a). If implementation discovers a second required
bebop-side diff, that is a stop-and-reassess signal (scope alarm), not a silent expansion.

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Sharpens §10.5.2's DoD-1..6 (kept 1:1, renumbered to build items, extended with test names and
commands — no item weakened, one added). P34 is DONE iff every row is demonstrably true:

| Item | §10.5.2 | RED (fails before) | GREEN (passes after) | Command / falsifier |
|---|---|---|---|---|
| W-1a | DoD-0 (new) | `cargo test -p bebop-delivery-domain --features kernel-rlib` fails E0004 **today** (§0 row 12) | same command green; `p07_compensation_states_have_no_wire_mapping_fail_closed` passes | run the command at bebop HEAD; falsified by any wildcard arm appearing in `from_order_status` (grep) |
| W-1b | DoD-1 | no dowiz crate depends on bebop (§0 row 14) | `cargo tree --manifest-path mesh-adapter/Cargo.toml -e normal \| grep -c "bebop-delivery-domain\|bebop-proto-cap"` ≥ 2; smoke test green | falsified by `cargo tree` showing no such edge (DoD-1's own falsifier, kept verbatim) |
| W-1c | DoD-1 "builds in CI" | no CI in either repo compiles `kernel-rlib` (§0 row 13) | `mesh-adapter` job green on push, gating BOTH directions; the §3.1 adversarial dry-run recorded RED at the pre-fix pin | falsified by the job absent from `ci.yml` or not running the reverse-edge step |
| W-2 | DoD-3 (vocabulary half) | parity sweep + retraction tests don't exist; the drift class is ungated (§0 row 18) | `cargo test --manifest-path mesh-adapter/Cargo.toml --test vocabulary` green incl. the 81-pair sweep + table-sabotage teeth | falsified by the mutated-table run NOT failing (teeth check) |
| W-3 | DoD-3 (claim half) | no dowiz-side claim fold exists | `--test claims` green: full lifecycle zero-illegal + skip/double-accept/unknown-order adversarials | falsified by `Offered→PickedUp` not erring at index 1 |
| W-4 | DoD-4 | no dowiz-side matcher call exists | `--test matcher` green: determinism ×3 forms + permutation + teeth + clamp; scoring unrepresentable pinned | falsified by any scoring/ranking input appearing (standing rejection; `ci-no-courier-scoring.sh` double-lock) |
| W-5 | DoD-5 | `ac6` exists only bebop-side, synthetic order, currently uncompilable | `--test solo_island` green: real `place_order` order → Delivered with `ledger_balance` conservation + 3 adversarial refusals, zero peers | falsified by the test not calling `place_order`/`apply_event` (drive-from-decider requirement) |
| W-6a | DoD-2 | no firewall proof exists | `--test firewall` green + `proofs/firewall-red.sh` exits 0 (build-must-fail proof) in CI | falsified by the script passing when run against the sanctioned `kernel-rlib` seam (§3.6 adversarial) |
| W-6b | DoD-6 | P13:23's claim unqualified (§0 row 14) | P13/P09/P10 amended with MESH unit IDs + file paths; claim date-scoped | `grep -rn "ZERO code-level dependency" docs/design/sovereign-roadmap-2026-07-16/` finds only the amended form |

Permanent regression rows (item 17) added to `docs/regressions/REGRESSION-LEDGER.md`:
(1) "P34 cross-repo kernel-rlib gate — guardrail: CI job `mesh-adapter`, reverse-edge step"
(the §0-row-12 class); (2) "P34 wire↔Law legality parity — guardrail: `vocabulary.rs` 81-pair
sweep"; (3) "P34 solo-island decider-driven flow — guardrail: `solo_island.rs`". Ledger ratchet
rule applies verbatim: red→green proof before any "done".

---

## 6. Benchmark plan (item 10) — measure the wiring tax, build no new harness

**The actual perf risk, named:** the spine adds, per frame, (i) hybrid signature verification —
ML-DSA-65 + Ed25519, the expected dominant cost by orders of magnitude; (ii) SHA3-256 of the
payload for the DOD replay id; (iii) DOD set ops; (iv) a 10-byte decode; (v) the facade hop
itself (a function call — expected noise). The kernel fold it wraps is already benched
(`kernel/benches/criterion.rs:60` `fold_transitions/5_hops`).

Plan (criterion, same `bench_track`-compatible discipline as the P-A §6 precedent):

1. `mesh_spine/admit_and_fold_one_frame` — full DOD→WIRE→LAW→MONEY per-frame cost, and
   `mesh_spine/emit_one_frame` — signing-side cost. Baselines seeded in the first GREEN commit;
   results recorded in `mesh-adapter/benches/BENCH_HISTORY.md` (measured numbers, never prose
   estimates).
2. **The reported number that matters:** `admit_and_fold_one_frame − fold_transitions/5_hops` ≈
   the protocol tax per event. Expectation to falsify, not assert: tax is dominated by
   ML-DSA-65 verify; at §4.2's traffic shape (~10 frames/order; 1 000 orders/day ≈ 0.12
   frames/sec sustained) even a multi-millisecond tax leaves ≥4 orders of magnitude headroom —
   the bench exists to pin the trend and catch regressions (e.g. an accidental re-verify loop),
   not to hit a tight budget.
3. **Kernel-untouched proof:** the existing kernel benches (`place_order/5_items`,
   `fold_transitions/5_hops`) run before/after P34 must be flat within threshold — P34 adds no
   code to the kernel, so any movement is environmental noise or a process error; a real
   regression here is a scope alarm (§4.5).

Telemetry hook: the bench-regression CI gating is P-H's deliverable; these benches are written
`native-trackers bench`-compatible so they are gated the day that job lands — dependency named,
nothing new built.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.1/§10.2/§10.3 (invariants 1/2/5)/
§10.5.2 (the P34 charter this blueprint formalizes; P34B/P35/P36 anti-scope boundaries) ·
`CORE-ROADMAP-INDEX.md` §0 + `:152` (mesh-real absorption row) ·
`docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` + `MESH-REAL-PLAN.md` (MESH-01..05/07/09-12
design source — reused for design, NOT for status, §0 row 20) ·
`mesh-real/MESH-12-RESOLVED-2026-07-14.md` (genesis resolution) ·
`sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md` (the corrected premise,
§0 row 14 + W-6b) · `BLUEPRINT-P09-…` + `BLUEPRINT-P10-…` (DoD-6 co-amendments) ·
`CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md` (structural template; bench/ledger
discipline precedent) · `docs/regressions/REGRESSION-LEDGER.md` (item-17 mechanism) ·
`HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§8). Memory files:
`mesh-real-arc-2026-07-13` (MESH-01..14 arc; PHASE-1 DONE; MESH-12 operator-gated) ·
`sovereign-core-mvp-handoff-2026-07-06` (shell→kernel::decide keystone lineage) ·
`cross-branch-todo-map-2026-07-10` (**the repo-routing rule W-1a obeys: bebop files →
`/root/bebop-repo`, never `/root/dowiz`**; push to `openbebop` remote) ·
`worktree-remote-push-collision-avoidance-2026-07-18` (push after every milestone; fetch before
push) · `test-integrity-rules-2026-06-27` (money red-lines — §3.5's conservation assertions) ·
`never-bypass-human-gates-2026-06-29` (nothing here flips a gated authority) ·
`verified-by-math-2026-07-07` (§4.1's reachability arguments) ·
`internal-retrieval-living-memory-arc-2026-07-14` (§4.3 living memory) ·
`rust-native-bare-metal-decision-2026-07-14` (adapters-not-purges; DECART one-liners in §2.1).
Supersedes: nothing — this is §10.5.2 P34 formalized to the standard; the roadmap section
remains the charter, this file the execution spec.

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P2 CORRESPONDENCE** (as above, so below — one concept, one primitive): one Law for order
  legality, `dowiz_kernel::order_machine::assert_transition`, enforced identically on every
  receiver; the wire mirror table is bound to it by the 81-pair parity sweep (§3.2-4), so the
  mirror can never become a second authority. One scale of money (integer minor units) end to
  end — `amount_i64` on the wire, i64 ledger in the kernel, no float crosses the seam.
- **P6 CAUSE-AND-EFFECT** (determinism as law): HRW assignment is a pure function of
  `(order_id, candidate set)` — proven by repetition/permutation/teeth (§3.4); the fold is a
  deterministic reducer — replay reproduces state exactly (§4.3 Snapshot-Re-entry); every
  determinism claim has a falsifier, none is asserted.
- **P7 GENDER** (paired creation, no self-certification): no node certifies its own frames —
  every receiver re-derives legality from ITS OWN state through the Law (§3.5-i re-anchors the
  forged-skip proof); settlement's k-of-n PoD requires k *distinct* signers over one digest
  (consumed as built); and P34's own DoD is refereed by tests that were proven able to fail
  (teeth checks in §3.2/§3.4, the pre-fix RED pin in §3.1).

(P1/P3/P4/P5 are not load-bearing for this wiring and are not claimed decoratively, per the
Anu/Ananke discipline.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 20 rows, every cite re-verified live this pass, 5 drifts + 3 new findings + 1 live regression surfaced |
| 2 DoD | §5 — RED→GREEN per item, commands + falsifiers, §10.5.2 DoD-1..6 kept 1:1 and sharpened |
| 3 spec/event-driven TDD | §2 (spec first), §3 per-item RED tests, event-sequence assertions (§3.3 joint fold, §3.5 frame sequence) |
| 4 predefined types/consts | §2.3 — `MeshHost`, `HostFault`, `OPENBEBOP_CI_PIN`, the one-arm diff spec, both facade call-site signatures verbatim |
| 5 adversarial/breaking tests | §3.1-3.6 — one+ per item, incl. two teeth checks (mutated table §3.2, pre-fix CI pin §3.1) and a must-fail build proof §3.6 |
| 6 hazard-safety as math | §4.1 — double-fold birthday bound, split-brain F46, vocabulary-fork double gate, purity/conservation, cargo-acyclicity proof; §2.1 cycle argument |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 — frames/order, bytes/frame, log-growth break point handed to P34B by name |
| 9 Linux discipline | §4.4 — four verdicts in the adopted framework |
| 10 benchmarks+telemetry | §6 — protocol-tax bench, kernel-flatness proof, bench_track-compatible, measured-not-estimated |
| 11 isolation/bulkhead | §4.3 — kernel manifest untouched, one-way arrow, panic domain named, P37 handoff explicit |
| 12 mesh awareness | §4.3 — node-local by design, payload budget stated, transport explicitly P34B/P36 |
| 13 rollback/self-heal vocabulary | §4.3 — Self-Termination + Snapshot-Re-entry claimed with mechanisms; Self-Healing explicitly NOT claimed |
| 14 error-propagation gates | §4.1 vocabulary-fork paragraph — compile-time exhaustive match + CI parity sweep; §3.1c the anti-rot CI ratchet |
| 15 living memory | §4.3 — content-addressed event stream; tiering deferred to P34B by name |
| 16 tensor/spectral + eqc reuse | **Explicit N/A, not decorative**: no closed-form arithmetic organ exists on this path (crypto + state machines; nothing eqc-able); the hash/crypto primitives are already single-authority. Claimed as a reasoned exemption per "where applicable" |
| 17 regression ledger | §5 — three named permanent rows, ratchet rule applied |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §1 anti-scope (no rewrite), §2.1 DECART rejections, §3.6 fence-pattern reuse, §6 (no new harness), design reused from mesh-real not re-derived |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

You are wiring two sibling repos: `/root/dowiz` (this repo) and `/root/bebop-repo` (push remote
`openbebop` → `github.com/SyniakSviatoslav/OpenBebop`; the `origin` remote is ARCHIVED read-only
— never push there). Rule: bebop files are edited in `/root/bebop-repo`, dowiz files in
`/root/dowiz` — never copies across. Execute in order; T1 is the critical path's first unit.

1. **T1 (W-1a — the live regression; bebop-repo).** Reproduce the RED first:
   `cd /root/bebop-repo && cargo test -p bebop-delivery-domain --features kernel-rlib` — must
   FAIL with E0004 naming `Refunding`/`CompensatedRefund` (if it passes, someone fixed it —
   re-verify §0 row 12 before proceeding; ground truth is non-discussible). Then in
   `bebop2/delivery-domain/src/intake.rs` `from_order_status` (`:69`) add exactly:
   `OrderStatus::Refunding | OrderStatus::CompensatedRefund => None,` (NO wildcard arm — the
   exhaustive match is a load-bearing gate). Add the
   `p07_compensation_states_have_no_wire_mapping_fail_closed` test (§3.1 verbatim) to the
   intake tests mod. Acceptance: the command above green (all AC tests incl.
   `ac6_solo_island_full_flow_no_peers` now compile and pass). Commit to a feature branch,
   push to `openbebop`.
2. **T2 (W-1b — the adapter crate; dowiz).** Create `mesh-adapter/` with §2.2's `Cargo.toml`
   verbatim, `src/lib.rs` with §2.3's `OPENBEBOP_CI_PIN` (update the SHA to T1's pushed commit)
   + `MeshHost`/`HostFault` skeletons, and `tests/edge.rs` (smoke: `IntakeEdge::new` → `emit` →
   `admit_and_fold` → `to_order_status`). Acceptance:
   `cargo test --manifest-path mesh-adapter/Cargo.toml` green;
   `cargo tree --manifest-path mesh-adapter/Cargo.toml -e normal | grep bebop-delivery-domain`
   non-empty. Do NOT touch `kernel/Cargo.toml` or any `kernel/src` file — P34 has zero kernel
   diffs by design (§4.5); if you think you need one, stop and flag it.
3. **T3 (W-6a — firewall proofs; dowiz).** Add `mesh-adapter/tests/firewall.rs` (cargo-metadata
   closure assertion: `dowiz-kernel` absent from `bebop-proto-wire`/`bebop-mesh-node`/
   `bebop-proto-cap` closures) and `mesh-adapter/proofs/firewall-red.sh` + `firewall-red.patch`
   (§3.6 — script exits 0 iff the injected import FAILS to build; also run the §3.6 adversarial
   once and record it in the script's header comment). Acceptance: test green, script exits 0.
4. **T4 (W-1c — CI; dowiz).** Add the `mesh-adapter` job to `.github/workflows/ci.yml`
   (§3.1c YAML — sibling checkout layout `dowiz/` + `bebop-repo/` is mandatory; both relative
   path deps break otherwise). Perform the adversarial dry-run: point `ref:` at the PRE-T1 pin
   `e56ba6a35258ced76752510625511f37a6367a77`, confirm the reverse-edge step goes RED, record
   the run link/output in the PR description, then set `ref:` = T1's commit. Acceptance: job
   green on push.
5. **T5 (W-2; dowiz).** Write `mesh-adapter/tests/vocabulary.rs` — §3.2 items 1-5 + the three
   adversarials (truncated bytes, mismatched scope, table-sabotage teeth: mutate a LOCAL copy,
   assert the sweep fails, restore). The 81-pair parity sweep iterates both `DeliveryStatus`
   enums' full cross product; use the live tables (`event_dict.rs:90`, `order_machine.rs:139`),
   never a hand-copied one. Implement `MeshHost::apply_delivery_event` (§2.3) to make item 5
   green. Acceptance: `--test vocabulary` green; teeth check demonstrably able to fail.
6. **T6 (W-3 + W-4; dowiz).** Write `tests/claims.rs` (§3.3: joint order+claim fold, skip
   `Offered→PickedUp` errs at index 1, double-accept errs, unknown-order leaves state
   untouched) and `tests/matcher.rs` (§3.4: determinism ×3, permutation invariance,
   `order_id+1` teeth, hub_ring clamp + R=0). NO scoring input may appear anywhere — trust =
   signed capability, never reputation (standing rejection; the `Courier` type enforces it).
   Acceptance: both suites green.
7. **T7 (W-5; dowiz).** Write `tests/solo_island.rs` (§3.5 verbatim shape): REAL
   `place_order` with items → full lifecycle via signed frames → `Delivered` → settlement leg →
   `ledger_balance` conservation; plus the three adversarial refusals (forged skip / replay /
   expired). The test MUST call `dowiz_kernel::domain::{place_order, apply_event}` directly —
   that is the "driven from the dowiz-kernel decider side" requirement (DoD-5); a test that
   only mirrors statuses without the kernel fold does not satisfy it. Acceptance:
   `--test solo_island` green.
8. **T8 (close-out: benches, docs, ledger).** Add `mesh-adapter/benches/criterion.rs` (§6
   benches), run, record real numbers in `mesh-adapter/benches/BENCH_HISTORY.md`; run the
   kernel benches before/after and confirm flatness (§6.3). Amend
   `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md:23` (+P09,
   +P10) per §3.6b with dated correction notes; verify with the DoD grep. Append the three
   REGRESSION-LEDGER rows (§5). Final acceptance sweep: every §5 row's command run and green;
   do not mark P34 done if any adversarial was weakened, `#[ignore]`d, or had its expectation
   inverted — the ledger's ratchet rule applies verbatim. Push dowiz work; push bebop work to
   `openbebop`; fetch before every push, never force.

**Stop-and-flag conditions (do not improvise past these):** (i) any second bebop-side source
diff beyond T1's one arm (§4.5 scope alarm); (ii) any impulse to add an event variant, a
scoring field, transport code, storage, or an HTTP endpoint (all owned elsewhere — §1);
(iii) the pre-fix RED not reproducing (stale ground truth — re-verify §0 first);
(iv) `kernel/` diffs of any kind.
