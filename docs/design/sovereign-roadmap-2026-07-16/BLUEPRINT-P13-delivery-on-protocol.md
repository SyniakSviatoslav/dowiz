# BLUEPRINT — Phase 13: DELIVERY ON PROTOCOL (2026-07-16)

> dowiz rides the mesh. Integration surface + order spine. One phase of the 19-phase
> master roadmap (`R2-MERGED-PHASE-ROADMAP.md`). Grounded in `R1-D-product-on-protocol-gap-analysis.md`
> (read in full) and direct code reads of every primitive named below.
>
> **Anchors owned:** M1, M10, S4, E1, E39, F17, F41, F42, F43, F46, F50.
> **Depends on (hard):** Phase 4 (kernel graph-math library), Phase 7 (money-law closure), Phase 9
> (confidential self-healing wire), Phase 10 (hub runtime / kill-switch / boot).
> **Parallel-safe with:** Phase 11, 12, 18.
> **Position:** on the critical path `P3 → P9 → P10 → P13 → {P14, P16} → P17`. This phase gates
> Phase 14 (dispute/escrow reuses P13's ledger), Phase 16 (UI renders P13's real order flow), and
> Phase 18 (public-flip references a working first order, G11).
> **Precondition rulings (Phase 2 operator batch):** O7 (hub-ring semantics), O15 (E39 rewording),
> O19 (I-FINAL proof home). None are settled here; each is referenced where it bites.
> Canon is `ARCHITECTURE.md` (mesh = FOUNDATION; **dowiz = delivery service ON TOP of the protocol**).
> This blueprint plans; it writes no code and edits no canon.

---

## 0. The single biggest structural gap in the entire roadmap

**dowiz today has ZERO code-level dependency on the bebop protocol.** This is not "loosely coupled"
or "decoupled pending re-integration" — it is *absent*. A repo-wide read confirms the only reference
to the protocol from the live dowiz kernel is a **comment**, at `kernel/src/domain.rs:524`, which
mentions "bebop2 BP-21" as a *design analogy* for a Kalman variance-reduction gate — not an import,
not a call, not a wire. R1-D found exactly this one comment and nothing else. There is no
`SignedFrame` produced or consumed by dowiz, no envelope, no hub, no second-service proof-of-concept
anywhere in either repo.

Compounding this, R1-D's load-bearing ground-truth finding (its §0) is that **the centralized dowiz
product that F41–F50 assume is being "re-plumbed" no longer exists as live source.** Commits
`79ef316f6` / `db766de47` (2026-07-13, "kernel is now sole source of truth") deleted `apps/web`
(Storefront/Admin/Courier SPA), `packages/ui`, `packages/domain`, `packages/shared-types`; commit
`fce5738b0` quarantined `apps/api`, `apps/worker`, `packages/db`, `fly.toml` into `attic/`. At HEAD,
`git ls-files 'apps/*'` returns **0 files**. The order-intake path (Fastify `POST` + Postgres
`INSERT INTO orders`), the courier assignment (`SELECT … FOR UPDATE`), the settlement cycle — all of
it is now a *behavioral oracle in git history and design docs*, not editable code.

The honest consequence: this is **not** "wire dowiz's product code to the mesh." It is **"stand up the
delivery spine on the kernel + mesh primitives, proven across two hubs, importing the feature inventory
from the oracle."** Every "today the product does X centrally" statement below cites a *historical* path
— not a file you can edit. The falsifier for whether "dowiz rides the mesh" is *real* rather than
aspirational is one test (M1 + M10): **an order placed on hub A folds to an identical `DeliveryStatus`
on hub B, where hub B runs a genuinely different internal storage backend.** If that test is red, this
phase has not happened, regardless of what else is built.

The good news is that the *primitives* this phase needs already exist and are green — but they are
**stranded**: PoD lives unwired in a legacy crate, the double-entry ledger lives unwired in the same
crate, and the delivery-domain fold + DoD gate + HRW matcher live in bebop2 with no order flow on top.
This phase is overwhelmingly **integration and porting**, not green-field invention.

---

## 1. Current-state evidence (what exists, what is stranded, what is absent)

**Green and reusable (bebop2, canonical mesh side):**

- **Delivery-domain fold** — `bebop2/delivery-domain/src/lib.rs`. `DeliveryStatus` with **pinned wire
  discriminants** (`Pending=0x10 … Delivered=0x15 … PickedUp=0x18`, :56-85), `OrderTransition{order_id,
  from, to}` with a canonical 10-byte `to_bytes`/`from_bytes` TLV (:101-121), and
  `assert_transition_local` (:145) mirroring the dowiz kernel order machine. Under the `kernel-rlib`
  feature (default OFF) it re-exports the **unmodified** kernel Law (`assert_transition`, `apply_event`,
  money fns — :23-32) so the canonical legality check is the kernel's, not a fork. Its tests already
  prove **two nodes fold the same status** (:199) and **a forged `Pending → Delivered` is rejected on
  every receiver** (:179). This is the state-fold half of "delivery on protocol," done.
- **DoD admission gate** — `bebop2/mesh-node/src/dod.rs`. `DodGate::admit(event, now, expires_at)`
  fail-closes on empty payload, zero id (placeholder), replay (per-node applied set), and expiry
  (:58-73). Every event crossing the mesh must pass it before it is applied.
- **HRW courier matcher** — `bebop2/proto-cap/src/matcher.rs`. `hrw_weight` (FNV-1a over
  `(order_id, courier_pubkey)`, :41) + `assign` (deterministic, coordination-free ranking, :63).
  **Structurally NO-COURIER-SCORING**: `Courier` carries only a 32-byte pubkey; the CI gate
  `scripts/ci-no-courier-scoring.sh` hard-locks it. This is the courier layer the hub-ring sits *on top
  of*, not a replacement for.
- **Wire + identity** — `SignedFrame` (`bebop2/proto-cap/src/signed_frame.rs:78`, with
  `verify_classical`/`verify_pq`/`verify`), carried in an opaque `Envelope`
  (`bebop2/proto-wire/src/envelope.rs:20`), over the live iroh-QUIC transport
  (`iroh_transport.rs` `send`/`recv` of `SignedFrame`). Hybrid identity is `NodeIdentity`
  (`crates/bebop/src/vault.rs`, ML-DSA-65 ⊕ Ed25519, `self_certify`). Phase 9 completes the
  confidential/self-healing properties; this phase consumes the frame format Phase 9 hardens.

**Green and reusable (dowiz kernel, product-math side):**

- Order Law `kernel/src/order_machine.rs` — `allowed_next`: `Pending→[Confirmed,Rejected,Cancelled]`,
  `Confirmed→[Preparing,InDelivery]`, `Ready→[InDelivery,PickedUp]`, `InDelivery→[Delivered]`,
  terminals empty. **Happy-path only — no compensation edges** (Phase 7 / P0-A4 adds them; consumed
  here, §5). `place_order_js`/`apply_event_js` wasm entry points exist.
- Integer money `kernel/src/money.rs` — `Money::checked_add` fail-closes cross-currency/overflow.
  **No reversal primitive today** (Phase 7 adds it).

**Stranded (exist, tested green, wired to nothing) — the two primitives this phase ports:**

- **PoD** — `crates/bebop/src/pod.rs` (legacy `bebop` crate). `DeliveryClaim{order_id, courier_id,
  timestamp, x, y}`, a **canonical locale-independent serialization** (`canonical()`, :44), a **SHA-512
  digest** (:52), and `sign_delivery`/`verify_delivery` over a **hybrid ML-DSA-65 ⊕ Ed25519** vault
  signature. Four falsifiable tests: roundtrip, **misattribution refused** (:127), **tamper fails
  verify** (:140), **replay-at-wrong-location fails** (:153). *It is single-signer and built on the
  legacy `crate::vault::NodeIdentity`, which links the `ml-dsa` + `ed25519-dalek` crates* — a fact that
  makes "port" mean "re-base onto bebop2's `SignedFrame`/identity," not "move the file" (§4, DECART).
  Not present anywhere under `bebop2/`.
- **Double-entry ledger** — `crates/bebop/src/ledger.rs` (238 lines). `Account{id, balance: i128}`,
  `Transfer{from, to, amount>0, nonce}`, deterministic `transfer_id = SHA256(from|to|amount|nonce)`
  (:38), the **conservation invariant `conserved()` — Σ balances == 0** (:79), and a `transfer` that is
  fail-closed (rejects `amount<=0`, unknown account, insufficient funds) **and idempotent** (an
  `applied` id set → replay is a clean no-op, :89-113). Five tests including idempotent-replay and
  insufficient-funds. Uses `sha2` + `serde` (a DECART note on the canonical digest, §5). Not in bebop2,
  not in the kernel.

**Absent everywhere (must be designed this phase):**

- Any dowiz→protocol port. No hub-ring module (grep zero across both repos). No PoD in the product
  (the historical delivery flow was a courier UI **slide → `POST /assignments/:id/delivered`**,
  `DeliveryPage.tsx:202` historical — **no cryptographic proof of any kind**). Payout was a
  **legacy central-DB history table** (`courier_payouts`, owner-approved settlement cycle,
  `owner/settlements.ts` historical). **No gRPC/protobuf anywhere** — no tonic, no prost, no `.proto`
  file in either repo.

---

## 2. Hub-ring / consistent-hash ownership overlay (E1 / F41, per O7)

**The term "hub-ring" is, in the entire codebase and canon, literally two words with no formal spec.**
E1 records it as "hub-ring + sparse-P2P(C)"; nothing else defines it. The adopted reading — flagged by
R1-D and carried into the roadmap as **operator decision O7** — is: *a consistent-hash ring for
order/region **ownership**, layered on top of the existing HRW courier hashing.* **This is not settled
canon.** A literal star-topology "hub" would directly contradict **M7** (no single point of failure)
and the SOVEREIGN doc's "any node is producer/consumer; no central hub." **This phase designs to the
ownership-overlay reading and does not presuppose O7 is ratified; if the operator rules otherwise, §2
is the only section that changes.**

**Design (ownership overlay, not physical topology):**

- Each hub hashes its self-certifying id onto a 64-bit ring. Each `order_id` (and each region key, for
  region-scoped ownership) hashes onto the same ring. The **owner** of an order is the first hub
  clockwise (the successor). The **replica set** is the owner plus the next `R−1` successors.
- **No SPOF (M7-consistent):** ownership is *authority to author the canonical fold*, not a routing
  chokepoint. Frames still gossip peer-to-peer over Phase 9's wire; the ring only decides *whose fold is
  authoritative* and *who replicates it*. If the owner drops, its first successor already holds the
  replica and assumes authority — the mesh heals via Phase 9's HRW island-merge + spanning-tree
  re-route. Nothing routes *through* a single hub.
- **Coordination-free & deterministic:** ownership is a pure function of `(order_id, live hub set)`, so
  every hub computes the same owner with zero coordination — the exact property `matcher.rs::assign`
  already gives for couriers. Consistent hashing's minimal-disruption property means a hub join/leave
  remaps only ≈`1/N` of keys, not the whole space.
- **Relationship to HRW:** HRW (`matcher.rs`) stays untouched for **courier** assignment. The ring is a
  *new, higher* overlay for **order/region** ownership. The two compose: the owner hub runs
  `assign(order, couriers)` to pick the courier, both layers coordination-free.

**Minimal-reuse alternative to flag for O7:** order ownership could instead reuse HRW directly —
`HRW(order_id, hub_pubkey)`, primary = top-ranked hub, replicas = next `R−1`. This adds *no* new
primitive (it is `matcher.rs` re-parameterized over hubs) and yields the identical done-test outcome.
The consistent-hash *ring* buys cheaper incremental remap under churn; HRW-over-hubs buys zero new code.
**O7 should choose between "ring-successor" and "HRW-over-hubs"; both satisfy the falsifier.** This
phase builds to the ownership *contract* (a total, deterministic `owner(order) → hub` +
`replicas(order) → set`) so the implementation choice is swappable behind it.

---

## 3. Order intake → signed envelope → DoD gate → state fold, proven across two hubs (M1 / M10)

This is the spine and the phase's core falsifier. The pipeline, end to end:

1. **Intake** (REST edge, §6): a customer/owner places an order. Intake is a *thin* edge that produces
   an **intent**, never authoritative state. It calls the kernel's `place_order` to construct the
   `Order`, then emits an `OrderTransition{order_id, from, to}` for each state change.
2. **Envelope**: each transition is serialized via the delivery-domain's canonical `to_bytes` (10-byte
   TLV) into a `SignedFrame` payload, signed with the emitting hub/edge's hybrid identity, and wrapped
   in an `Envelope` for the wire. The **owner hub** (§2) authors the canonical frame; other interested
   hubs receive it.
3. **DoD gate**: every receiving hub runs `DodGate::admit` before applying — rejecting void payloads,
   placeholder ids, replays, and expired bundles (fail-closed, `dod.rs:58-73`).
4. **Fold**: the admitted frame decodes back to an `OrderTransition`; the hub folds it through the
   **canonical Law** — `assert_transition` then `apply_event` (the kernel Law, re-exported under
   `kernel-rlib`). A forged skip (`Pending → Delivered`) is rejected by the Law on **every** receiver,
   exactly as `delivery-domain` test `:179` already proves.
5. **Local log** (E39, §7): the fold appends to the hub's **hash-chained event log** — the honest
   restatement of E39's "signed event_log" (per O15): *a hash-chained log fed by hybrid-verified
   frames.* The frames are signed (hybrid ML-DSA ⊕ Ed25519, verified at the gate); the log links them by
   `H(prev || frame)`. Phase 7's `commit_after_decide` dedup fix (P0-A2) is a hard precondition — a
   double-append bug here would double-finalize orders.

**The two-hub proof (M1 + M10 — the falsifier that this is real).** Stand up **two hubs with
genuinely divergent internal storage**: hub A on the kernel's in-memory / file-JSONL event store, hub B
on a different backend (e.g. pgrust, or any store — the point is *different bytes on disk*). Place an
order on A; let its frames propagate to B over the Phase 9 wire. **Assert B's folded `DeliveryStatus`
== A's**, byte-for-byte via the pinned discriminants. This is precisely M10 ("inter-hub protocol
defined; intra-hub anarchy allowed"): B's storage is B's own business; agreement comes from the
*protocol and the shared Law*, not shared storage. If A and B agree only because they share a database,
the test is a fraud — the storage divergence is the whole point. This done-test **is** the merged
Phase-A-C done-test #1 ("dowiz order frame verified + folded by a second hub").

---

## 4. Proof-of-Delivery: port `pod.rs`, k-of-n threshold, unified geo + photo capture (F42)

**Today PoD is completely absent from the product.** The historical completion flow was a courier UI
slide stamping `delivered_at` — no photo, no signature, no handover proof (repo-wide historical grep:
zero PoD hits). A real primitive exists, stranded, in `crates/bebop/src/pod.rs` (§1).

**Port (re-base, do not move the file):** `pod.rs` is built on the legacy `crate::vault::NodeIdentity`,
which links `ml-dsa` + `ed25519-dalek` at the crate boundary. Dragging that into `bebop2` would violate
**M6** (zero external crate at the wire/trust boundary — proto-cap is std-only). So "port" =
re-implement `DeliveryClaim`'s three assets — the **canonical serialization**, the **SHA-512 (or
canon-sha3) digest**, and the **sign/verify with misattribution + tamper + wrong-location refusal** —
on top of bebop2's `SignedFrame` and its hybrid identity. The claim's design is preserved verbatim:
`courier_id` is the courier's **self-certifying vault id** (a hash of the public key, **not PII**),
binding the proof to `(order, courier, ts, location)` so a later replay at a different spot fails
(`pod.rs:153` behavior). The edge (courier device) signs with its own **self-certifying ML-DSA key
(M4)** — no central CA.

**Multi-signal evidence.** The single geo point in the legacy claim is upgraded to multi-signal per
blueprint v3 L1: a **geo fix** validated by the kernel (`haversine_meters` / `is_arriving` against the
drop polygon — proving the courier is *at* the drop, not merely claiming it) **plus an optional photo**.

**The unified capture flow (one capture, two consumers — designed as ONE flow, not two).** The courier
device performs a **single** capture at handover that yields: (a) a **geo fix**, folded into the signed
`DeliveryClaim`; and (b) an **optional photo**, whose **hash** is included in the signed claim
(tamper-evident PoD evidence) while its **raw bytes** are queued for **Phase 17's
`SplatReconstructionJob`** (the address-splat bootstrap). This is the synergy R1-D names (P0-B3): the
photo is captured **once**, hashed into the claim, and routed to the splat queue — never two separate
captures. **PII handling (F50-consistent):** the photo may contain PII, so raw bytes stay local /
route to a per-job rented GPU (per the splat arc); **only the hash crosses the wire** inside the claim.

**k-of-n threshold settlement (F42, blueprint v3 L3).** Single-signer PoD (as in `pod.rs`) is
insufficient for trustless settlement; v3 L3 requires a **threshold verifier — ≥k-of-n signatures on
the PoD before settlement.** Design: the canonical `DeliveryClaim` digest is signed independently by up
to `n` designated signers (mandatory courier edge signature, plus a subset of {customer handover
confirmation, owner, witnessing replica hubs from §2}). Settlement fires only when **≥k valid, distinct
signatures over the same digest** are collected. **DECART honesty:** "threshold signature scheme" here
means **k-of-n multi-signature quorum counting** (k independent hybrid signatures over one digest,
verified against a designated signer set) — **not** a cryptographic `(t,n)` threshold scheme (FROST /
threshold-BLS), which would introduce a new dependency at the trust boundary (M6 violation) and has no
place in the current PQ stack. If a true threshold scheme is ever wanted, that is a separate DECART, out
of scope here. **Falsifier:** a **tampered** PoD claim is rejected (extends `pod.rs:140`); a valid
**k-of-n** claim settles; a claim with `<k` signatures does **not** settle.

---

## 5. Payout saga: port `ledger.rs`, compensation via Phase 7 (F43)

**Today payout is a legacy centralized-database history table** — `courier_payouts` (deliveries_count,
total_earned), an owner-approved settlement cycle (`routes/owner/settlements.ts`, `settlement-period.ts`
historical), Postgres only. It is *not* protocol-native and does not survive the re-plumb as-is. A real
double-entry primitive exists, stranded, in `crates/bebop/src/ledger.rs` (§1).

**Port:** bring `ledger.rs`'s **double-entry law** — the `conserved()` Σ==0 invariant, the idempotent
content-addressed `transfer_id`, the fail-closed `transfer` — into the canonical path. **DECART:**
`ledger.rs` uses `sha2` + `serde`; the canonical dowiz path content-addresses with **sha3** and the
kernel money law is already integer + fail-closed. The port swaps the digest to the in-repo sha3 (or
keeps the primitive kernel-side where `money.rs` already lives) so it satisfies zero-dep; the conservation
math is unchanged. **S9 LOCK** (integer + event-sourcing + saga-compensation) is the frame.

**The payout saga.** A settled PoD (§4) drives an event-sourced saga of integer ledger transfers:
`escrow/order-account → courier-earnings` on `Delivered` (the *earn* leg), each transfer a double entry
so Σ stays zero at every step. Cash reconciliation (the As-Built 77%-cash market) is modeled as a
paired entry against a `cash` account — **cash-reconciliation feature parity with the legacy settlement
cycle is a hard requirement** (R1-D: "cash reconciliation must survive the re-plumb"), reconciled from
the same double-entry ledger rather than a separate history table.

**Compensation (consumes Phase 7).** A cancel-after-confirm or dispute must **reverse** cleanly.
Phase 7 (P0-A4) adds the **money reversal primitive** and the **FSM compensation edges** (today
`order_machine.rs` is happy-path only — no compensation edges, no reversal in `money.rs`). This phase
**consumes** those; it does not duplicate the red-line FSM/money change (single owner, no duplicate
edit — Phase 7 owns the golden-signature drift-gate re-key). A compensation is a reversing double-entry
transfer (`courier-earnings → escrow`, a distinct nonce) that exactly undoes the earn leg. **Falsifier:
the ledger sums to EXACTLY zero across (a) one full delivered-order lifecycle AND (b) one
cancel-after-confirm compensation case** — the second is the one that fails if reversal is wrong.

---

## 6. gRPC-internal / REST-edge split — the first `.proto` contract (S4 / F17)

**There is no gRPC / protobuf anywhere in the codebase**, and that is correct until now. S4/F17
(gRPC-internal + REST-edge) were **premature** in every earlier phase because there was never a point
where **two independent in-host services genuinely needed to talk.** Phase 13 is that first point: the
**delivery-service** process (order intake, PoD, payout saga) and the **mesh-hub / protocol** process
(envelope produce/consume, delivery-domain fold, DoD gate) are now two real services in one host.
**Do not introduce gRPC before this phase** — it is dead weight until §3 exists.

**tonic/prost DECART (required).** Adopting `tonic` (gRPC) + `prost` (protobuf codegen) is a new
dependency, gated by a **DECART report** per the rust-native rule. The report weighs mature Rust-native
schema tooling and a contract-pinned internal boundary (the point of S4) against a non-trivial dep tree
+ codegen step; the honest alternative is the in-repo std-only framing already on the wire
(`SignedFrame` + canonical TLV) extended to the internal boundary — zero new deps, no schema tooling.
**The DECART, not this blueprint, chooses.** What this phase fixes regardless: the **internal vs edge
split becomes real** — service-to-service is a pinned contract; the customer/courier-facing surface is
REST (GraphQL client-edge only, never internal, per S4).

**First `.proto` contract sketch** (illustrative design artifact, not implementation — the pinned
contract is an `OrderProjection` read surface so both boundaries serve the *same* projection):

```proto
// order_projection.proto — the ONE pinned contract for the P13 golden-encoding test.
syntax = "proto3";
package dowiz.delivery.v1;

enum DeliveryStatus {            // discriminants MIRROR delivery-domain (0x10..0x18)
  DELIVERY_STATUS_UNSPECIFIED = 0;
  PENDING = 16; CONFIRMED = 17; PREPARING = 18; READY = 19;
  IN_DELIVERY = 20; DELIVERED = 21; REJECTED = 22; CANCELLED = 23; PICKED_UP = 24;
}

message OrderProjection {
  uint64 order_id = 1;
  DeliveryStatus status = 2;
  string owner_hub_id = 3;       // from the hub-ring (§2)
  bytes  event_log_head = 4;     // hash-chain head (E39, §7)
  bool   pod_settled = 5;        // k-of-n threshold met (§4)
}

service OrderProjectionService {                 // gRPC-INTERNAL only
  rpc GetOrderProjection(GetOrderProjectionRequest) returns (OrderProjection);
}
```

**Golden-encoding equivalence test (S4/F17 falsifier).** A **gRPC-internal** call and a **REST-edge**
call must serve the **same** `OrderProjection` for the same order, derived from the one pinned `.proto`.
The test asserts semantic/byte equivalence of the two encodings of one projection — the edge is a
*view* of the internal contract, never a second source of truth.

---

## 7. F46 partition-tolerant finalization, built to the I-FINAL proof's content (pending O19)

**F46 ("partition-tolerant delivery") is absent everywhere** (grep for union-find/DSU/MST across both
repos: zero). Its *topology-healing* half (DSU/MST, HRW island-merge, shortest-path re-route) is
Phase 4 (math) + Phase 9 (wire heal) — consumed here, not built here. F46's **order-state** half — *who
may finalize `Delivered`/a terminal state while the mesh is partitioned* — is this phase's design, and
it needs the **I-FINAL quorum-intersection proof** as its formal backing.

**I-FINAL's mathematical content (design to this now):** *two mesh nodes never finalize conflicting
delivery state for the same order.* Provable by quorum intersection — two signed quorums `Q_A`, `Q_B`
at `n > 3f` must overlap in **≥1 honest node**, which cannot have signed two different finalizations for
the same `(order, epoch)` without contradiction (SYNTHESIZED P0-A5, lines 168-184). The **runtime rule**
this phase can build *without* the proof file:

- A terminal fold (`… → Delivered`, or a compensated terminal) that is **contested** across a partition
  must carry a **finalization certificate** = a signed quorum over `(order_id, epoch, terminal_status)`.
- The **hub-ring owner** (§2) may finalize an **uncontested** order within its authority solo (island
  mode, F12). A **genuine contest** (conflicting terminal frames for the same order) requires a quorum
  cert; at small `n` a single-hub partition is `< quorum`, so **neither side may unilaterally finalize a
  contested order during a split** — they reconcile on merge.
- On **merge**, conflicting certs are impossible-by-construction (intersection); the DoD replay-dedup
  (`dod.rs`) + the hash-chained log (§3, E39) **detect** any conflicting terminal and reject the second.

**Falsifier (F46 fully closed):** a **partition-then-merge** scenario that attempts
double-finalization of the same order (both partitions receive conflicting terminal frames) is **RED by
design before** the I-FINAL-backed rule (both finalize) and **GREEN after** (the quorum-cert +
intersection + hash-chain conflict-detection rejects the second finalization).

**O19 — the proof's file home (precondition for FULL closure, not silently resolved).** SYNTHESIZED
P0-A5 cites `eqc-proofs/lambda_max_of_d.rs` as the *pattern* to follow for a machine-checked I-FINAL
proof. **Correction to the roadmap's O19 premise (verified this session):** that file **does exist** —
but at `/root/bebop-repo/rust-core/eqc-proofs/lambda_max_of_d.rs`, a **legacy `rust-core/` crate that is
a *third* location** — neither of O19's two candidate homes (bebop2's consensus path vs. dowiz's
`tools/eqc` emitted-proof directory). It is a genuine `// GENERATED by eqc` self-asserting proof (exit 0
⟺ emitted f64/fixed-point code matches the SymPy reference), and dowiz's `tools/eqc` (the Python
equation compiler) is exactly its generator. So the pattern is real and works; it is simply **homed
where neither candidate lives**, which *sharpens* rather than weakens O19: the operator must decide
whether I-FINAL is a hand/eqc-authored Rust proof under **bebop2's consensus path**
(new `eqc-proofs/i_final.rs` alongside `mesh_consensus`) or an artifact **emitted by dowiz `tools/eqc`**
into a dowiz-side proof directory. **This phase designs and builds the runtime rule + the RED/GREEN
partition test to the proof's mathematical content now; the machine-checked proof *artifact* and its
directory wait on O19.** F46 is "closed except its proof-file home" until O19 rules.

---

## 8. F50 as a standing invariant threaded through every sub-design (F50)

F50 (living-organism unbounded) is **not a one-time build** in this phase; it is a **standing design
constraint** on every sub-design above, plus a literal acceptance test. The obligation is narrow and
falsifiable (R1-D's product reading of F50): **no flow rebuilt in this phase may reintroduce a mandatory
central service.** The legacy product violated this everywhere (single Postgres, N=1 WebSocket rooms,
central JWT). The re-plumb must not smuggle a new one in.

Concretely, threaded through §2–§7:

- **§2 hub-ring:** ownership authority + replicas, never a routing chokepoint (M7). No central owner-of-owners.
- **§3 spine:** the fold Law, DoD gate, and hash-chained log are **in-process pure primitives**; the
  mesh is a **sync peer, not a dependency**. Every network interaction degrades-closed to solo operation.
- **§4 PoD:** edge self-certifying identity (M4), no central CA. k-of-n over designated signers, not a
  central verifier.
- **§5 payout:** the ledger is an in-process double-entry primitive; no settlement server.
- **§6 gRPC/REST:** both are *in-host* services; neither is a remote mandatory dependency.

**The literal acceptance test (F50's standing-invariant test; also F12's regression check from
Phase 9): a solo-hub island run — the FULL order-to-delivery flow (place → confirm → prepare → ready →
in-delivery → PoD sign → k-of-n verify → settle) completes successfully with ZERO other services or
hubs running.** Because every step above is an in-process primitive, this must pass. If any sub-design
introduces a hard external call, this test is the one that catches it.

---

## 9. Acceptance criteria — consolidated numbered checklist

Each item is falsifiable. Items **AC-1 … AC-7** are the phase's done-test verbatim; **AC-8 … AC-12**
are the design-integrity and precondition checks.

1. **AC-1 (M1 + M10 — core cross-hub consistency).** An order placed on **hub A** folds to an
   **identical `DeliveryStatus`** on **hub B**, where **hub B runs a genuinely different internal
   storage backend**. Agreement comes from the protocol + shared Law, not shared storage.
2. **AC-2 (forgery rejection).** A forged `Pending → Delivered` transition is **rejected** on every
   receiver (extends `delivery-domain` test `:179`).
3. **AC-3 (PoD integrity + threshold).** A **tampered** PoD claim is **rejected**; a valid
   **k-of-n-threshold** claim **settles correctly**; a claim with `<k` signatures does **not** settle.
4. **AC-4 (ledger conservation + compensation).** The ledger sums to **EXACTLY zero** across (a) one
   full delivered-order lifecycle **and** (b) one **cancel-after-confirm compensation** case (the
   compensation consumes Phase 7's reversal primitive + FSM compensation edges).
5. **AC-5 (S4 / F17 golden encoding).** A **gRPC-internal** call and a **REST-edge** call serve the
   **same** order projection from the **one pinned `.proto`** contract (golden-encoding equivalence).
6. **AC-6 (F50 / F12 solo-island).** A **solo-hub island run** completes the **full**
   order-to-delivery flow with **ZERO** other services/hubs running.
7. **AC-7 (F46 partition-then-merge).** A partition-then-merge scenario attempting **double-finalization**
   of the same order is **RED before** the I-FINAL-backed rule and **GREEN after** (F46 fully closed,
   contingent on O19 for the proof's file home).
8. **AC-8 (dowiz→protocol dependency is real).** dowiz produces and consumes real `SignedFrame`s over
   Phase 9's wire — the "one comment at `domain.rs:524`" state is gone; a grep proves a live code
   dependency, not a comment.
9. **AC-9 (E39 unification, per O15).** The delivery event log is a **hash-chained log fed by
   hybrid-verified frames**, unified with this phase's envelope format; the "signed event_log" wording
   is retired in favor of the accurate description.
10. **AC-10 (unified capture, one flow / two consumers).** The courier handover is a **single** capture
    producing a geo fix + optional photo; the photo's **hash** enters the signed claim while its raw
    bytes route to the Phase 17 splat queue — greppably one capture, not two.
11. **AC-11 (no-SPOF / M7 for the ring).** The hub-ring is an ownership overlay with replica successors;
    killing the owner hub leaves an authoritative replica — no flow routes through a single hub.
12. **AC-12 (DECART discipline).** Written DECART reports exist for: (a) porting `pod.rs` onto bebop2's
    `SignedFrame` rather than dragging legacy `ml-dsa`/`ed25519-dalek` across the M6 boundary;
    (b) porting `ledger.rs`'s double-entry law with a sha3 digest; (c) adopting (or declining) tonic/prost
    for the internal boundary.

**Precondition rulings that gate specific ACs (Phase 2 operator batch — not resolved here):**
**O7** (hub-ring semantics: ring-successor vs HRW-over-hubs) gates §2 / AC-11; **O15** (E39 rewording)
gates AC-9; **O19** (I-FINAL proof home) gates the *full* closure of AC-7. **Consumed hard
dependencies:** Phase 4 (kernel graph-math), Phase 7 (P0-A4 money reversal + FSM compensation edges,
P0-A2 event-log dedup fix), Phase 9 (live wire on ≥2 nodes), Phase 10 (hub runtime / boot / kill-switch).

**Explicit non-goals (defer to later phases, do not build here):** dispute/arbitration + escrow +
per-hub graph-wiki (Phase 14 / F44 / F48, gated on O3/O4); the product UI rebuild — 26 pages, i18n,
WCAG (Phase 16); the scripted delivery demo, splat tiers, GPU-unlock video (Phase 17). This phase builds
the *spine and the two-hub proof*, nothing above it.

---

*P13 blueprint complete. Primitives verified this session by direct read: `crates/bebop/src/pod.rs`,
`crates/bebop/src/ledger.rs`, `bebop2/delivery-domain/src/lib.rs`, `bebop2/mesh-node/src/dod.rs`,
`bebop2/proto-cap/src/matcher.rs`, `bebop2/proto-cap/src/signed_frame.rs`, `crates/bebop/src/vault.rs`,
`kernel/src/order_machine.rs`, `kernel/src/domain.rs:524`, `rust-core/eqc-proofs/lambda_max_of_d.rs`,
`tools/eqc/`. Grounded in R1-D (read in full), R2 master roadmap, SYNTHESIZED P0-A5, ARCHITECTURE.md
canon. This document plans; it changes no code and no canon.*
