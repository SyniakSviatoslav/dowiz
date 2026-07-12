# DECISIONS — dowiz/bebop decentralized PQ delivery protocol

> Operator-confirmed decisions. Each gates implementation. Date: 2026-07-12.

## D1 — Drop centralized `server/`
The axum+rusqlite centralized deploy (`server/`, WAVE-4) is RETIRED. It was a local-first
single-node prototype; it re-centralizes (one dispatcher). Replaced by peer nodes, each with
local SQLite, syncing over the PQ mesh. Source: operator mandate 2026-07-12.

## D2 — Local-first nodes, local DB
Every node (owner/merchant, courier, customer) runs the Rust/WASM kernel with its OWN SQLite
(local memory, no server). Sync is a transport-agnostic port; the signed event log makes
decentralization reachable for free (C4).

## D3 — Transport: reliability > latency (satellite/lab-grade)
- **Chosen class**: Delay-Tolerant / store-and-forward (DTN, RFC 4838). NOT low-latency pub/sub.
- **Candidates evaluated**:
  - libp2p (gossipsub): production pub/sub, but gossipsub is low-latency-first → does NOT meet C11.
    Usable only as a reliable request/response + persistent-store overlay, not gossip.
  - **Zenoh (Eclipse)**: purpose-built for cyber-physical / space; pub/sub + query + storage;
    reliability over latency; lightweight. **Closer to C11 than gossipsub.**
  - Yggdrasil/cjdns: mesh VPN underlay (E2E-encrypted routing real); optional, not core.
- **Decision**: adopt Zenoh as the primary reliable mesh transport; libp2p considered as fallback
  overlay. Both wrap every message in the PQ Envelope (D5). Final lock after a prototype spike
  (reliability test: node offline 24h, replays full state on reconnect, 0 lost events).

## D4 — Post-quantum protocol composition (C12)
| Layer | Primitive | Status in repo |
|---|---|---|
| KEM (transit + at-rest wrap) | ML-KEM-768 (FIPS 203) | `kernel/src/pq/kem.rs` — from-scratch, KAT-verified (r3) |
| Signature (packets, identity, code) | ML-DSA-65 (FIPS 204) | `kernel/src/pq/dsa.rs` — from-scratch, **NIST ACVP byte-exact** (r4/r5) |
| Hybrid transit | X25519 + ML-KEM-768 | NOT yet built — roadmap (needs rustls/aws-lc-rs or oqs for prod; kernel stays reference) |
| At-rest | AES-256-GCM + ML-KEM-wrapped key + ML-DSA envelope | NOT yet built — roadmap |
| Code-sign | ML-DSA over update blob, pinned root | NOT yet built — roadmap |

## D5 — PQ Envelope (L1 identity seam) — MUST be added to kernel
Every protocol message = `Envelope { payload, content_hash=SHAKE256(payload), sig=ML-DSA-65(node_sk, content_hash) }`.
The kernel's `order_machine.rs` / `domain.rs` events are NOT yet signed. Add the Envelope + a
`verify_signed_transition` gate before any transition enters `decide/fold` (C7 red-line: unsigned ⇒ drop).

## D6 — Hybrid transition safety (C12)
Never allow silent classical-only fallback. Handshake requires both X25519 and ML-KEM to verify;
reject any peer offering only classical. (Matches rustls/OpenSSH/AWS shipping X25519MLKEM768.)

## D7 — Red-line guardrails (where enforced)
1. **Supply chain**: node updates ML-DSA-signed, verified vs pinned root BEFORE apply. Unsigned ⇒ hard reject. (enforced in boot/verify path)
2. **Key mgmt**: PQ private keys NEVER plaintext at rest; ML-KEM-encapsulated keyfile or OS keystore; rotation required.
3. **Downgrade/replay**: hybrid groups reject classical-only; HPKE (RFC 9180) replay protection; seq in signed packets.
4. **RNG**: kernel hot path RNG-free (caller-supplied entropy). Caller MUST supply FIPS 140-3 / SP 800-90A DRBG; never reuse `rnd`.
5. **State integrity**: only signed transitions enter `order_machine.rs`; money = integer `i64`, never float, never cleartext.

## D8 — Manifesto/Decisions are authoritative
Where this file or MANIFESTO conflicts with any roadmap/blueprint/agent-narrative, MANIFESTO+DECISIONS WIN.
Ground truth = code + `cargo test`, not docs. Push plans to remote before execution (AGENTS.md).
