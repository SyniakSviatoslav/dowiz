# Bebop Core — Crypto / Network / PQC / Mesh / Torrent-like (expanded plan)

> Guidance source: dowiz `docs/design/sovereign-core-mvp/GRAND-PLAN.md` + `MANIFESTO.md`.
> Operating constraints (user, this session): OSS-only, reliable, autonomous, fundamental — must not
> rely on other systems. Core = cryptography + network + post-quantum + mesh + torrent-like behavior.
> Better Auth stays the DEFAULT local auth (already wired), but it is a leaf, not the core.

## 0. What "torrent-like" means for an agent CLI (the key reframe)

A torrent does NOT trust peers and does NOT have a server. It splits a payload into **content-addressed
chunks** (hash = identity), lets any peer seed any chunk, repairs missing chunks by **erasure coding**,
and verifies every byte by hash. For Bebop the "payload" is NOT files — it is the **agent's event log,
knowledge, and code/memory**:

- The `Envelope { seq, at, cause }` event log (already in `loop.ts`) is ALREADY content-addressable:
  `cause` is a canonical `request_hash` (FNV-1a today, SHA-256/ML-DSA-hash later). An envelope's
  identity is its hash, not its location. → that is the "chunk."
- **Swarm sync**: any two Bebop nodes exchange missing envelopes by hash, like piece availability in
  BitTorrent (`have`/`want` bitfield ≡ `known_seq` set). No central server.
- **Erasure coding**: split the log into N data + M parity chunks so the log survives loss of any M
  peers/devices (Reed-Solomon over content-addressed blocks). This is the "resilient distribution."
- **Verification by hash, not trust**: a node accepts an envelope only if its `cause`/signature
  validates against the kernel's fold rule. A malicious peer cannot forge state — the pure core rejects
  it (MANIFESTO: "Invariants OUTSIDE the agent").
- **Gossip**: new envelopes flood the mesh via epidemic gossip (gossipsub-style), bounded by the
  known-seq bitfield so bandwidth is O(new).

This reuses the GRAND-PLAN's own seam: *"Deterministic `fold` over a totally-ordered log IS the
replication primitive; ordering is the transport's problem."* Bebop's torrent layer IS that transport.

## 1. Architecture (core-first, seams from GRAND-PLAN)

```
┌──────────────────────────────────────────────────────────────┐
│ Bebop node (autonomous, offline-first)                        │
│                                                                │
│  ┌──────────── KERNEL (pure, deterministic, no IO) ────────┐  │
│  │ decide / fold / replay  + Envelope{seq,at,cause} log    │  │  ← GRAND-PLAN kernel law
│  │ guard gates (red-line/scope)  + token ledger            │  │
│  └────────────────────────────────────────────────────────┘  │
│         ▲ emits signed, content-addressed envelopes           │
│  ┌──────────── COG (orchestration, local) ────────────────┐  │
│  │ routing/rotation · profile(5-axis) · living-memory ·   │  │
│  │ conductor (dispatch to BYOK backends)                   │  │
│  └────────────────────────────────────────────────────────┘  │
│         ▲ wraps each envelope in a crypto envelope            │
│  ┌──────────── CRYPTO CORE (the new core) ────────────────┐  │
│  │ @noble/post-quantum: ML-KEM (FIPS203) + ML-DSA (FIPS204)│  │  ← PQC, hybrid-by-default
│  │ @noble/ciphers: AES-256-GCM / XChaCha20-Poly1305        │  │  ← at-rest + in-transit
│  │ Argon2id device key (already in dowiz; reuse pattern)   │  │
│  │ content-addressing: SHA-256 (cause) → ML-DSA signature  │  │
│  └────────────────────────────────────────────────────────┘  │
│         ▲ chunks + erasure-codes + gossips                     │
│  ┌──────────── NETWORK / MESH (the new core) ─────────────┐  │
│  │ hypercore/hyperswarm (content-addressed P2P swarm, MIT) │  │  ← torrent-like transport
│  │ OR libp2p (GRAND-PLAN D6 seam) — impl #3 of contract    │  │
│  │ gossipsub + known-seq bitfield (have/want)              │  │
│  └────────────────────────────────────────────────────────┘  │
│         ▲ optional, user-run                                  │
│  Better Auth (self-hosted sync node, DEFAULT local auth)      │
└──────────────────────────────────────────────────────────────┘
```

## 2. Verified building blocks (all MIT, installable, offline-capable)

- `@noble/post-quantum@0.6.1` — ML-KEM (encapsulation) + ML-DSA (sign/verify). Auditable, ~0 deps.
- `@noble/ciphers@2.2.0` — AES-256-GCM, XChaCha20-Poly1305.
- `hyperswarm@4.17.0` + `hypercore@11.33.5` — DHT-less P2P swarm + content-addressed append-only log.
- `argon2` (already a dowiz dep) — device-key derivation for the local vault.
- Better Auth (wired) — self-hosted identity, optional sync server only.

## 3. Decisions locked (research → conclusion)

1. **Pure core stays pure.** PQC + mesh wrap the OUTSIDE of the kernel, exactly as GRAND-PLAN says
   ("signing is a shell envelope; kernel keeps consuming plain `Command`"). No RNG/clock/network enters
   `decide`/`fold`. This preserves replay-determinism and the wasm gate.
2. **Hybrid-by-default crypto.** Classical (Ed25519 + X25519/AES-GCM) AND PQC (ML-DSA + ML-KEM) together
   — matches 2026 production practice (AWS turned hybrid PQC TLS on by default). Not PQC-only (not yet
   universally trusted); not classical-only (harvest-now-decrypt-later risk).
3. **Content-addressing is the identity.** `cause` hash (upgrade FNV-1a → SHA-256) IS the chunk id. The
   torrent layer needs no new ID system — it reuses the event log's existing causality hash.
4. **Erasure coding, not full replication.** Reed-Solomon over content-addressed blocks → survive loss
   of M of N devices. This is what makes a node "autonomous and reliable" without a server.
5. **Mesh is a transport seam, not a rewrite.** GRAND-PLAN D6 already placed it as "impl #3 of the same
   contract suite." Start with hypercore/hyperswarm (TS, ships today); libp2p is a later Rust impl of
   the SAME contract. Swap-not-rewrite.
6. **Offline-first.** Every operation works with zero peers. A single node is fully functional; the
   swarm is an accelerator, not a dependency. This satisfies "must not rely on other systems."

## 4. Build phases (each RED+GREEN, per MANIFESTO "Testing as Religion")

- **P0 — Content-addressed log.** Upgrade `cause` to SHA-256; add `chunk(envelopes)` → blocks with
  content-hash; `verify(block)` by hash. Pure, testable offline. RED: tampered block fails verify.
- **P1 — Crypto envelope.** ML-DSA sign each block; ML-KEM for peer key exchange; AES-GCM at rest in
  the local vault. RED: wrong key / bad signature rejected. (Reuses Better Auth's dev-secret gating.)
- **P2 — Erasure coding.** Reed-Solomon split → N+M shards; `repair()` from any N. RED: delete M shards
  → repair still reconstructs; delete M+1 → fails loud.
- **P3 — Swarm transport.** hypercore feed + hyperswarm gossip; known-seq bitfield (have/want). RED:
  node A with seq 1..10, node B with 5..20 → both converge to 1..20 by hash, no server.
- **P4 — Conductor + mesh.** routing/rotation already selects backends; now a "peer" is also a backend
  — dispatch a task to a peer node, results gossip back as envelopes. Uniform with CLI backends.
- **P5 (later) — libp2p impl** of the same contract (GRAND-PLAN D6), swap-not-rewrite.

## 5. What I will NOT do without your go (red-line respect)

- I will NOT rip out dowiz's production auth or migrate off Fly yet — that's a separate gated track
  (Phase 4 of the earlier plan). The user authorized Better Auth as default for Bebop; dowiz migration
  stays its own approved phases.
- I will NOT add RNG/clock/network into the pure kernel — ever (MANIFESTO doctrine 2).
- I will NOT phone-home, add telemetry, or default keys (autonomy principle).

## 6. Open question for you (one decision)

Language for the CORE: the GRAND-PLAN mandates **Universal Rust + WASM** for the sovereign core, but
Bebop is currently **TS-first** (ships today, OS-agnostic, 14/14 tests pass). Two viable paths:
- **(A) TS core** using `@noble/*` + `hypercore` (ships now, aligns with "reliable/autonomous today").
- **(B) Rust/WASM core** (`rebuild/crates/bebop`) matching GRAND-PLAN, with a TS shim — purer, but the
  Rust crate is currently BROKEN at HEAD and needs sealing first (GRAND-PLAN 0b arc).

Recommendation: **(A) now** for the torrent/PQC/mesh core (it's where the MIT libs are mature and
shippable), keeping the kernel law 1:1 with `rebuild/crates/domain` so a Rust port later is a
swap-not-rewrite (the GRAND-PLAN seam). I will wait for your pick before writing P0 code.
