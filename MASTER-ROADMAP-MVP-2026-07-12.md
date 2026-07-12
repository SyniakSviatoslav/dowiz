# MASTER ROADMAP & MVP — dowiz / bebop decentralized PQ delivery protocol

> **Source of truth: this doc (dated 2026-07-12) + MANIFESTO.md + DECISIONS.md.**
> Supersedes every 2026-07-11 artifact: `ROADMAP-GROUND-TRUTH`, `MASTER-BUILD-SEQUENCE`,
> `PARALLEL-EXECUTION-PLAN`, `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3`.
>
> **Precedence (D8 / bebop RULES.md setting):** newest approved decision outranks older.
> The operator's latest in-session message > newest D-series > 2026-07-11 docs.
> **Mesh + post-quantum are NOT deferred** (D6 override of C8). Anu QRNG is wired in but
> native OS entropy is the DEFAULT + FALLBACK (D9). Do not reverse D0–D7 without operator approval.

---

## 0. Verified ground truth (what is ACTUALLY done, from `cargo test`)

Recorded 2026-07-12, `feat/pq-crypto-tier1` (dowiz-pq) + `feat/decentralized-pq-protocol` (dowiz).

| Layer | Component | Status | Proof |
|---|---|---|---|
| L0 | Pure core policy (no clock/RNG/env/floats in kernel) | DONE | `kernel/src/` has no `SystemTime`/`rand` in hot path |
| L1 | PQ envelope (ML-DSA-65 sign/verify, tamper RED gate) | DONE | `envelope.rs` tests `red_wrong_key_rejected`/`red_tampered` GREEN |
| L1 | X25519 transit | DONE (KAT-verified) | `x25519.rs` uses `curve25519-dalek::mul_clamped`; RFC 7748 §6.1 vectors **corrected** vs OpenSSL `cryptography`+dalek (RFC published values are typos) |
| L1 | ML-KEM-768 (FIPS 203) | DONE (ACVP byte-exact) | `kem.rs` KAT-gated; `red_tamper`/`kem_soak_random_seeds` GREEN |
| L1 | ML-DSA-65 (FIPS 204) | DONE | `envelope.rs` roundtrip + determinism-KAT |
| L2 | Hybrid KEM `X25519 + ML-KEM-768`, both required, no classical-only fallback | DONE | `hybrid.rs` `hybrid_encaps`/`hybrid_decaps`; 4 hybrid tests GREEN; RED gate drops on either-leg failure |
| L2 | Node custody / store-and-forward / replay / lifetime (BPv7-shaped) | DONE | `node/src/lib.rs` 7 tests GREEN (accept/expired/replay/wrong-dest/tampered/forward/secret) |
| L2 | **Confidential transit (D4):** bundle payload encrypted under recipient hybrid key; non-recipient cannot read | DONE | `make_secret_bundle`/`deliver_secret`; `green_secret_bundle_roundtrip_only_recipient_decrypts` GREEN |
| L1 | Entropy seam: `SHAKE256(a‖b)`, Anu QRNG provider, **`master_seed()` native DEFAULT+FALLBACK** | DONE | `entropy.rs` `master_seed()` (qrng off → OS; qrng on+reachable → quantum⊕os; qrng fail → OS, never errors) |
| L1 | Fractal fingerprint + chaotic routing tag (deterministic artifacts, NOT key/entropy) | DONE | `fractal.rs` avalanche test (byte-flip) GREEN |
| — | money.rs clippy errors (self-compare / modulo-1) | FIXED | `cargo clippy --lib` → 0 errors (kernel + node) |
| **Totals** | **kernel**: 132 lib tests GREEN, 0 clippy errors. **node**: 7 tests GREEN, 0 clippy errors. qrng feature builds. | | verified this session |

**NOT yet done (open workstreams below):**
- dtn7-rs real BPv7 daemon integration (L3 transport).
- BIBE actual custody handoff (D3).
- QUIC/TCPCLv4 convergence layer (D3 underlay).
- Per-node local SQLite (D5 / C4).
- 3 roles wired to real flows (owner/merchant, courier, customer).
- Adapters: NOSTR / ActivityPub / MCP (D5, each wrapped in PQ envelope).
- At-rest: AES-256-GCM volume key wrapped via ML-KEM (D4.3).
- Code-signing: node update blobs ML-DSA-signed vs pinned root (D4.4).
- Web/WASM frontend (Astro/Svelte) talking to the kernel.

---

## 1. Tier spine (shared dowiz ↔ bebop)

```
T1  stabilize core math (PQ crypto)        ██████████ DONE (verified)
T2  ship prod truth (PQ envelope + node)    ████████░░ DONE L1+L2; L3 transport open
T3  quality bars (RED+GREEN per change)     ████████░░ enforced; clippy clean
T4  FIRST REAL ORDER (G11 GREEN)            ░░░░░░░░░░ target — needs roles+local DB+transport
T5  rewrite substrate (dtn7-rs mesh)        ░░░░░░░░░░ in-scope NOW (D6)
```

---

## 2. PARALLEL-SAFE workstreams (independent files, zero pivot-risk → own autopilot branch)

Each can be delegated to a parallel sub-agent / worktree. None touches red-line crypto
constants, money surface, or D0–D7. All ship a RED+GREEN assertion (C7).

- **P1 · Local DB per node** — `node` crate gains a `store` module over `rusqlite` (or `sqlx`);
  custody store + replay set persist to a local SQLite file. No server. (C4)
- **P2 · At-rest crypto** — `volume.rs`: AES-256-GCM key wrapped via `hybrid_encaps` to node pubkey;
  envelope ML-DSA-signed. RED gate: wrong key ⇒ decrypt fails. (D4.3)
- **P3 · Code-signing** — `codesign.rs`: update-blob ML-DSA verify vs pinned root before apply.
  RED gate: unsigned/tampered blob ⇒ refuse. (D4.4)
- **P4 · Roles + flows** — `roles.rs`: owner/merchant, courier, customer intent→decide→event
  state machines; dispatch/accept/deliver transitions. (C3, D5)
- **P5 · Adapters** — `adapters/{nostr,activitypub,mcp}.rs`: each bridges a message into the PQ
  envelope before any external send. (D5)
- **P6 · Web/WASM frontend** — Astro/Svelte thin client calling the kernel via wasm-bindgen;
  reads local node state. (C4 thin-client mandate)
- **P7 · Transport research depth** — spacecraft/lab-grade transport bake-off:
  DTN/dtn7-rs vs QUIC/TCPCLv4 vs Zenoh vs Reticulum vs SpaceWire/SpaceFibre, with BIBE custody
  verification. Feeds D3 implementation. PRIMARY sources only (RFC 9171/9172/9174/9000,
  eclipse.dev/zenoh, ccsds.org, dtn7-rs, reticulum.network). (D3)

## 3. SEQUENTIAL GATES (red-line / external / tier-dependent — NOT parallel)

- **S1 · dtn7-rs integration (L3)** — depends on P1 (store) + L2 node. Replaces the in-crate
  `Bundle` simulation with the real `dtn7-rs` daemon; maps `Bundle` → BPv7 structures 1:1.
  Operator-gated (new dependency). Verifies BIBE custody (D3).
- **S2 · QUIC/TCPCLv4 convergence** — underlay for connected segments; depends on S1.
- **S3 · Production PQ swap** — for node-to-node channels, offer `rustls+aws-lc-rs
  (X25519MLKEM768)` + `liboqs ML-DSA` until our from-scratch core is independently audited.
  Our core stays the local reference. (D4)
- **S4 · FIRST REAL ORDER** — end-to-end: owner posts → courier accepts (custody) → delivers →
  customer confirms, all over real transport + local DB. The G11 GREEN trigger.

---

## 4. Autopilot execution plan (max parallelism)

1. Launch **P1–P7** as parallel sub-agents (independent branches/worktrees) — each with its own
   RED+GREEN test, no shared mutable pivot.
2. Sequence **S1 → S2 → S3 → S4** after P1 + L2 land; S-gates are red-line, reported, not silent.
3. Every landed change: `cargo test` GREEN + `cargo clippy` clean before commit. Terminal stays
   unblocked (no hanging builds) — verify with fresh output, never agent narrative (D7).
4. Push plans/docs to remote FIRST; commit code after green.

---

## 5. Hard invariants (D0 — never reversed without operator approval)

**decentralized · local-first · post-quantum · crypto · mesh · reliability-over-latency.**
C8 (YAGNI) gates *over-engineering*, never these six. D6 confirms mesh + PQ machinery is NOW
in-scope. D9 confirms Anu QRNG is wired but native entropy is the DEFAULT+FALLBACK.
