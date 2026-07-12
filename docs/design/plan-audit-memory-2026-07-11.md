# Plan Audit — Living-Memory Extraction for the Decentralized PQ-Secure Food-Vendor Delivery Protocol

> Read-only memory/context audit (2026-07-11). Purpose: extract the operator's STATED goal, decisions,
> constraints, and prior narrative about a decentralized, non-AI, post-quantum-secure delivery protocol
> for food vendors who want to stop relying on third-party couriers — to unify all plans into ONE
> blueprint. Sources quoted with `file:line`. No code was edited.

## Sources audited
- `/root/dowiz/HERMES.md` (mirror of Claude Code auto-memory index) — read in full.
- `/root/.claude/projects/-root-dowiz/memory/MEMORY.md` (live index) — read in full.
- `/root/.hermes/HERMES.md` and `/root/.hermes/MEMORY.md` — **do not exist** (only `~/.hermes/SOUL.md`). No Hermes-side project memory.
- `/root/dowiz/docs/design/sovereign-core-mvp/MANIFESTO.md` — operator's declared source-of-truth vision.
- `/root/dowiz/docs/design/sovereign-core-mvp/DECISIONS.md` — binding locked rulings (D1–D7).
- `/root/dowiz/platform-vs-protocol-logistics.md` — strategy/failure-mode brief for the protocol architect.
- Corroborating search hits in `rebuild/crates/domain/*` (PQC/mesh seams baked into codec + tests).

---

## 1. Narrative — what the operator wants

The operator's plan is **already captured under one umbrella: the "Sovereign Core" / dowiz-DeliveryOS
program**, which the operator himself declared "**the final state of truth for the desired product, MVP,
further features, main concerns and rules**" (MANIFESTO.md:3-5). The vendor/courier/decentralized
protocol the task describes is not a separate greenfield idea — it is the **Phase-2+ decentralization
roadmap of this existing program**, sitting on top of an MVP that ships first.

The through-line: **escape the aggregators.** Food-business owners lose their entire margin to
third-party platforms (25–30% commission — `platform-vs-protocol-logistics.md:25-31`). The product gives
the owner a **self-hosted, open-source hub they run and own**, that funnels all their own order channels
into one place at 0% commission, owns the customer + data, and (as the protocol matures) **dispatches and
manages their own/network couriers without a central extractor**.

Crucially, the operator has **already Red-Teamed the "space stack" (PQC, formal verification, mesh/P2P)
and hard-gated it behind the MVP** as roadmap, not MVP work — while insisting the cheap architectural
*seams* for it be baked in now. So "decentralized + PQ-secure + owner-managed couriers + multi-device
entrypoints" is the **declared destination**; the MVP is the Trojan horse that gets there.

There is a **strategic warning already on record** the blueprint must honor: the single most likely place
this protocol silently re-centralizes is the **matching/dispatch sequencer** — "*Decentralize the
matcher, not just the ledger. A logistics protocol that runs a single dispatch server is DoorDash with
extra steps*" (`platform-vs-protocol-logistics.md:95, 111`).

---

## 2. The operator's exact stated goal (quoted)

- **Escape-aggregators / owner-owned hub (the MVP):**
  > "A **modular hub that lets a food-business owner control their own data across their own channels from
  > one module** — escaping aggregators (0% commission, own-the-customer, own the data)." (MANIFESTO.md:28-30)
  > "The owner controls THEIR OWN data (menu, direct orders, customers) across their own channels
  > (web/QR/social/messaging), one direct 0%-commission checkout." (DECISIONS.md:7-8, D1)

- **Multi-device / multi-channel order entrypoints funnel into the owner hub:**
  > "own-channel distribution (web/QR/social/messaging) + ONE direct checkout + owned customer data"
  > (MANIFESTO.md:31-33). Channel attribution is the one hub primitive already built
  > (`routes/orders/channel.rs`, write-only `orders.metadata.channel`) (DECISIONS.md:57-58).

- **Decentralization as a design INVARIANT (the protocol goal):**
  > "Formula: **Local-first data × Local execution (WASM/edge) × P2P protocol = decentralized
  > reliability;** each node (venue, courier, client) an autonomous decision center. Reachable for FREE
  > from an immutable, deterministic, replayable, WASM-pure signed event log." (MANIFESTO.md:74-77)

- **Owner-managed couriers / dispatch without a central extractor** — honest dispatch already exists in
  the legacy app (`attemptHonestDispatch`, memory `flow-simplification-patch-2026-06-28.md:18-20`); the
  protocol target is an **open competitive matcher market, not a single dispatcher**
  (`platform-vs-protocol-logistics.md:104-107`).

- **Post-quantum security** — an explicit roadmap item, seams pre-baked:
  > "a content hash or signature over a command/event (Phase Three PQC — stable bytes to sign)"
  > (`rebuild/crates/domain/src/codec.rs:5-6`); PQC signing + mesh replication proven to stand on the
  > canonical-bytes layer (`rebuild/crates/domain/tests/kernel_hard_truth.rs:113-115, 184-186, 326-328`).

---

## 3. Hard constraints (non-negotiable, quoted)

1. **Determinism > AI — NO AI in the runtime/protocol logic.**
   > "Determinism > AI. AI is a tool for writing code/tests in R&D/back-office ONLY. System runtime logic
   > is hard-deterministic (Rust/WASM). No probabilistic decisions in business logic." (MANIFESTO.md:17-19)

2. **Pure, zero-platform-vocabulary core.**
   > "Platform vocabulary (clocks, RNG, env, floats, battery, network) never enters the pure core."
   > (MANIFESTO.md:21-22); "`dowiz-core` is pure (no side-effects, no I/O, no clock/RNG)." (MANIFESTO.md:40)
   > Enforced by `clippy.toml` wasm32 disallowed-methods gate (DECISIONS.md:22-23).

3. **Immutable event-sourced state machine as the law** — `Intent → decide → Event`, `state =
   fold(events)`, forbidden transitions are compile/runtime errors (MANIFESTO.md:38-39). This is the
   single truth the whole protocol (PQC signatures, mesh replication) stands on.

4. **Local-first + no central server** as a design invariant reachable "for FREE" from the signed event
   log (MANIFESTO.md:74-77; DECISIONS.md:13-19, D2).

5. **Money is integer-only** (`Lek(i64)`, no `From<f64>`) — single-money-surface invariant; aggregator
   order-intake is explicitly banned from the MVP because it breaks it (MANIFESTO.md:41; DECISIONS.md:9-11).

6. **Open-source is the declared final goal** — AGPLv3 + trademark + DCO, gated on secrets scrub + EU
   trademark (HERMES.md:118; memory `open-source-goal-adr020-2026-07-03`).

7. **Over-engineering is the #1 enemy — space-stack is roadmap, hard-gated behind MVP.**
   > "The space-stack — PQC (Kyber/Dilithium), formal verification (Coq/Aeneas), Canvas/vello UI,
   > mesh/P2P — is **roadmap, hard-gated behind the MVP** (D6)." (MANIFESTO.md:90-92; DECISIONS.md:41-43)

8. **Verified-by-Math / falsifiable proof** governs every change (HERMES.md:53-59; DECISIONS.md:32-39 D5):
   proofs must drive the real deployed/bound surface and be able to go RED.

9. **Ethics charter (non-negotiable):** no AI for military/warfare; AI is a commons, never captured for a
   narrow group (HERMES.md:26-31). Directly reinforces the anti-extractor protocol thesis.

---

## 4. Decided vs open

### Already DECIDED (locked)
- **MVP scope = own-channel owner hub + read-only aggregator view**, one 0%-commission direct checkout;
  NOT marketplace order-ingestion (DECISIONS.md D1).
- **Decentralization = invariant, baked as cheap seams NOW, machinery deferred to Phase 2+** — event
  sourcing, WASM-pure core, per-event content-hash + signature slot, transport-agnostic sync port are IN;
  libp2p/mesh, CRDT merge, per-actor Ed25519/PQC auth root, full offline-first are DEFERRED (DECISIONS.md D2).
- **Energy-aware consent-compute "grail" = Phase 3, never in the pure core** (DECISIONS.md D3; MANIFESTO.md §7).
- **PQC (Kyber/Dilithium) = Phase 2+**, seams only where free (DECISIONS.md D6).
- **Universal-Rust stack** (backend + WASM frontend + shared `dowiz-core`), hybrid Astro + Svelte UI with a
  fallback for every enhancement (MANIFESTO.md §3).
- **Strategic verdict recorded:** decentralize the matcher/dispatch, not just the ledger; open competitive
  matcher market + force-inclusion fallback + multi-signal delivery attestation (no single oracle)
  (`platform-vs-protocol-logistics.md:102-111`).
- Current build state: `dowiz-core` pure crate with 10-status order machine, integer money, idempotency,
  codec, wasm32+clippy gate; `decide` composes machine→actor-gate→cc1→pricing (0b-3 done); 0b-5 red-proof
  complete (HERMES.md:123-124; DECISIONS.md:50-58).

### Still OPEN
- **Actual PQC scheme + per-actor key identity** (Ed25519 vs ML-DSA/ML-KEM) — deferred, not chosen (DECISIONS.md D2).
- **Mesh/P2P transport** (libp2p vs alternative) and **CRDT merge** engine — deferred (DECISIONS.md D2).
- **Open matcher-market protocol spec** (permissionless matchers, force-inclusion timeout, attestation
  aggregation) — described as the target design but **not yet specced into the roadmap docs** (only in the
  strategy brief).
- **Aggregator order-intake** — parked behind a money council (single-money-surface invariant) (DECISIONS.md:9-11).
- **Open-source cutover** — gated on secrets remote-scrub force-push (HARD blocker) + EU trademark
  (HERMES.md:117-118).
- The task's framing ("owner-managed couriers, multi-device entrypoints") maps cleanly onto D1 hub +
  honest-dispatch, but a **unified protocol-level courier-management spec** does not yet exist as a doc.

---

## 5. Conflicting / tension points to resolve in the unified blueprint

1. **"Decentralized / no central server" vs the MVP's single hosted owner hub.** The MVP intentionally
   ships a hub on a server DB (one sync-port impl); decentralization is a deferred invariant. The blueprint
   must state clearly that the owner-run hub is the *thin, replaceable access layer*, not the chokepoint —
   or it re-centralizes at the access layer (`platform-vs-protocol-logistics.md:85, 109`).
2. **"Owner-managed couriers" (single owner dispatches) vs "decentralize the matcher" (open matcher
   market).** These are two different topologies: a single food-vendor owner running their own couriers
   (MVP-shaped) vs a permissionless multi-matcher protocol (protocol-shaped). The blueprint must reconcile
   the local single-owner hub with the network-level open matcher, and where the boundary sits.
3. **Terminology drift:** "hub" is used both for (a) the owner's local control module (MANIFESTO §2) and
   (b) the task's "owner HUB that dispatches couriers." Same concept, but the unified doc should pin one
   definition.
4. **No Hermes-side memory exists** (`~/.hermes/HERMES.md`/`MEMORY.md` absent) — all authoritative context
   lives in the dowiz repo + Claude Code memory. No conflicting Hermes-side plan; nothing lost by that gap.

---

## 6. One-line synthesis for the blueprint author

> Build ONE open-source, self-hosted **owner hub** (local-first, Rust/WASM, pure deterministic
> event-sourced core, integer money, **no AI in runtime**) that funnels a vendor's multi-channel /
> multi-device order entrypoints into a single 0%-commission checkout and dispatches their couriers — with
> the **event log's canonical-bytes + signature-slot seams baked now** so that **Phase-2+ post-quantum
> signatures, mesh/P2P, and an open competitive matcher market (not a single dispatch server)** can be
> switched on without a rewrite. Everything the MVP defers (PQC scheme, libp2p, CRDT, per-actor keys) is
> the protocol's stated destination, already Red-Teamed and hard-gated behind a shippable Trojan-horse MVP.
