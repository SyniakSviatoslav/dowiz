# MANIFESTO — dowiz/bebop decentralized NON-AI post-quantum delivery protocol

> Source of truth for red-line constraints. Supersedes any stale roadmap/blueprint claim.
> Date: 2026-07-12. Status: AUTHORITATIVE (operator-confirmed).

## 0. The one sentence
A self-hosted, **decentralized network of autonomous nodes** (owner/merchant, courier, customer),
each with a **local database**, running a **NON-AI, post-quantum-secure** delivery protocol where
**reliability > latency** and **decentralization is a hard invariant, not a feature**.

## 1. Non-negotiable constraints (gate EVERY change)
| # | Constraint | Rationale |
|---|---|---|
| C1 | **No AI in protocol/runtime logic** — deterministic Rust/WASM only; AI only for R&D/back-office | prevent probabilistic business logic |
| C2 | **Pure core** — no clock/RNG/env/floats/network/battery vocabulary reaches the kernel | determinism + verifiability |
| C3 | **Immutable event-sourced state machine is the law**: `Intent → decide → Event`, `state = fold(events)`; forbidden transitions are errors | auditability, no lost state |
| C4 | **Local-first + no central server** — each node owns its data + DB; sync is transport-agnostic | anti-re-centralization |
| C5 | **Integer-only money** (`i64` minor units, no `From<f64>`) — single money surface | no float drift on money |
| C6 | **Open-source destination** — AGPLv3 + trademark + DCO, gated on secrets scrub | commons, not captive |
| C7 | **Verified-by-Math / falsifiable proof** — every change needs a RED+GREEN assertion | no false-green |
| C8 | **Over-engineering is the #1 enemy** — PQ/mesh is roadmap, hard-gated behind MVP seams | YAGNI |
| C9 | **Ethics charter** — no AI for warfare; AI is a commons, never captured | operator red-line |
| C10 | **Crypto from-scratch, zero-dep, NON-AI, RNG-free hot path** — caller-supplied entropy only | trust-minimized |
| C11 | **Reliability > latency** — transport must be store-and-forward / delay-tolerant, retransmit-until-ack, custody-transfer. NOT low-latency gossip. Satellite/ lab-grade durability. | operator mandate 2026-07-12 |
| C12 | **Post-quantum is a PROTOCOL, not primitives** — PQ KEM + PQ signature + PQ at-rest + PQ code-sign + PQ in-transit packet integrity, all composed. Hybrid (classical+PQ) during transition. | operator mandate 2026-07-12 |
| C13 | **No centralized server** — `server/` (axum/rusqlite centralized deploy) is DROPPED. Replaced by peer nodes with local SQLite. | operator mandate 2026-07-12 |

## 2. Decentralization invariant (anti-re-centralization)
- **Matcher/sequencer** = the economic control point → MUST be open + replicable (any node runs it), not a single dispatcher.
- **Settlement** = threshold-signature verifier (≥k of n), NOT a single oracle.
- **Identity root** = self-certifying (`id = H(pq_pub ‖ classical_pub)`), NO directory/phone-home.
- **Access layer** = thin client + reference alt-client (escape "open protocol, closed access").
- Rule: *a logistics protocol that runs a single dispatch server is DoorDash with extra steps.*

## 3. Post-quantum protocol scope (C12 — what "PQ" means here)
1. **Transit channel**: hybrid KEM `X25519 + ML-KEM-768` (FIPS 203), both must verify (no classical-only fallback).
2. **Signatures**: ML-DSA-65 (FIPS 204) for packets, node identity, code-signing.
   - ⚠️ Reference: ML-DSA = **FIPS 204**, NOT SP 800-208 (that is LMS/XMSS). Correct this everywhere.
3. **At-rest local data**: AES-256-GCM volume key, **wrapped via ML-KEM encaps** (or HPKE RFC 9180 ML-KEM) to node pubkey, envelope signed with ML-DSA. No "PQ disk encryption" snake-oil.
4. **Code / supply chain**: node update blobs ML-DSA-signed, verified against pinned root BEFORE apply.
5. **In-transit packets**: AEAD inside PQ channel + ML-DSA signature over `(state, seq)`. Unsigned ⇒ drop.

## 4. Transport requirement (C11 — satellite/lab-grade)
- Store-and-forward, delay-tolerant (DTN, RFC 4838 class). Offline-first.
- Retransmit-until-ack, persistent queue, custody transfer between relays.
- Works over intermittent links (couriers drop offline constantly).
- libp2p is a framework (not only gossipsub) — usable as the reliable overlay; Zenoh is purpose-built
  for cyber-physical/space reliability. Both evaluated; choice recorded in DECISIONS.

## 5. Roles + adapters
- **3 roles**: owner/merchant, courier, customer. Each = autonomous node, local SQLite.
- **Adapters/bridges** (NOT core transport): NOSTR (messenger/social), ActivityPub (fediverse),
  MCP (tool entrypoint). Each message wrapped in ML-DSA/ML-KEM envelope before any bridge.

## 6. What this is NOT
- Not a business plan. Not formal verification (Coq is Phase-3). Not "0% fee = moat" (poetry).
- Not abandoning MVP for the space-stack — seams now, machinery per schedule.
