# DECISIONS — dowiz/bebop

> Operator-confirmed red-line decisions. Source of truth alongside `MANIFESTO.md`.
> Date: 2026-07-12. Status: AUTHORITATIVE.

## D0. Hard invariants (operator, 2026-07-12)
These six words are the lens for EVERY change, in priority order:
**decentralized · local-first · post-quantum · crypto · mesh · reliability-over-latency.**

If a change breaks any of these, it is rejected. They outrank roadmap sequencing,
feature requests, and "MVP-first" pragmatism (C8/YAGNI still applies to *scope*, not to
these invariants — the invariants are non-negotiable; only their *machinery depth* is phased).

## D1. Drop the centralized server (CONFIRMED)
- `server/` (axum + rusqlite centralized deploy) is **DROPPED**. Not refactored, not kept
  as "single-node mode" — removed from the build.
- Rationale: a centralized dispatch/deploy server is the anti-pattern the protocol exists to
  kill (MANIFESTO C13, "DoorDash with extra steps").
- Replacement: peer nodes, each running the Rust/WASM kernel + a local SQLite DB. No
  server process, no central DB, no Supabase, no Fly.
- Action: delete `server/` crate; remove from workspace `Cargo.toml`; keep any genuinely
  reusable pure logic (e.g. `reliability.rs` retransmit queue) by porting it into the node
  crate as a transport-agnostic module.

## D2. Manifest location (CONFIRMED)
- `MANIFESTO.md` lives at repo **root** (`/root/dowiz/MANIFESTO.md`), NOT `docs/design/`.
- The existing `docs/design/MANIFESTO.md` is copied to root and the stale copy removed, so
  blueprint citations (`MANIFESTO.md:28-30`) resolve.
- `DECISIONS.md` (this file) also lives at root.

## D3. Transport selection — spacecraft/lab-grade, reliability > latency (LOCKED, 2026-07-12)
- Requirement: must tolerate long delays, intermittent links, and partial outages
  (couriers drop offline constantly; use case extends to labs/satellites).
- **LOCKED STACK (primary research, PRIMARY sources only):**
  - **DTN / Bundle Protocol v7 (RFC 9171)** = the custody/store-and-forward substrate.
    Store-carry-forward overlay; explicitly targets "intermittent connectivity, sender and
    receiver not concurrently present" (RFC 9171 §1) — EXACTLY the courier use case.
    Rust core: **dtn7-rs** (real, active, Apache-2.0/MIT).
  - **QUIC (RFC 9000) or TCPCLv4 (RFC 9174)** = reliable convergence layer on connected
    segments (carries bundles). quinn = Rust impl. QUIC alone does NOT meet reliability>latency
    (needs a live path) — underlay only.
  - **Link layer** may be SpaceWire/SpaceFibre (sat/lab, ESA/CCSDS), LoRa, or Ethernet (ground).
  - **libp2p-gossipsub REJECTED** as primary: latency-optimized, conflicts with C11.
  - **Zenoh / Reticulum REJECTED** as substrate: Zenoh optimizes latency (not custody);
    Reticulum is Python (no Rust core) + classical crypto (not PQ) + no custody primitive.
- **Custody transfer MUST be real:** BPv7 core only *requests* custody-acceptance reporting
  (flag bit 15). Implement **BIBE (draft-ietf-dtn-bibect)** for actual hop custody handoff.
  Verify dtn7-rs supports BIBE; if not, contribution required. Without BIBE, "custody" is poetry.
- **Replay protection:** enforce bundle lifetime expiry + dedupe on (source EID, Creation
  Timestamp) per RFC 9171; BPSec (RFC 9172) integrity blocks prevent tamper-at-rest.
- **PQ envelope at PROTOCOL layer REGARDLESS of transport:** QUIC/Zenoh/Reticulum are ALL
  classical (X25519/Ed25519/AES/TLS1.3). Our from-scratch ML-KEM-768 + ML-DSA-65 MUST wrap the
  bundle payload (custom BPSec security context, RFC 9172/9173 pattern, or app-layer envelope)
  so PQ holds whether underlay is QUIC, TCPCLv4, Zenoh, or SpaceWire. Do NOT rely on transport
  native crypto for PQ.
- **Auditability:** prefer RFC-spec'd protocols over code-as-spec (Reticulum) for space/lab grade.
- Source: `/root/dowiz/RESEARCH-transport-dtn-mesh.md` (PRIMARY: RFC 4838/9171/9172/9174/9000,
  eclipse.dev/zenoh, reticulum.network, ccsds.org, dtn7-rs, quinn).

## D4. Post-quantum is a PROTOCOL (CONFIRMED, MANIFESTO C12)
- Not isolated primitives. The composed scope:
  1. Transit: hybrid KEM `X25519 + ML-KEM-768` (FIPS 203), both-verify, no classical-only fallback.
  2. Signatures: ML-DSA-65 (FIPS 204) — packets, node identity, code-signing.
     CORRECTION: ML-DSA is **FIPS 204**, NOT SP 800-208 (that is LMS/XMSS). Fix all citations.
  3. At-rest: AES-256-GCM volume key, wrapped via ML-KEM encaps (or HPKE RFC 9180) to node
     pubkey, envelope ML-DSA-signed. No "PQ disk encryption" snake-oil.
  4. Code/supply-chain: node update blobs ML-DSA-signed, verified against pinned root BEFORE apply.
  5. In-transit: AEAD inside PQ channel + ML-DSA sig over `(state, seq)`; unsigned ⇒ drop.
- Our from-scratch kernel (ML-KEM-768 + ML-DSA-65, ACVP byte-exact) is the **local reference
  core** (verified, not yet independently audited for production). For production node-to-node
  channels, prefer rustls+aws-lc-rs (X25519MLKEM768) + liboqs ML-DSA until our core is
  independently validated.

## D5. Roles + adapters (CONFIRMED, MANIFESTO §5)
- 3 autonomous node roles: owner/merchant, courier, customer. Each = local SQLite + kernel.
- Adapters/bridges (NOT core transport): NOSTR (messenger/social), ActivityPub (fediverse),
  MCP (tool entrypoint). Every bridged message wrapped in ML-DSA/ML-KEM envelope first.

## D6. Sequence (operator override of C8/D6)
- MANIFESTO C8 gates *over-engineering*; it does NOT block the invariants. Mesh machinery is
  NOW required (operator mandate), not deferred. Seams already exist in L0; L1/L2 machinery is
  in-scope. YAGNI still applies to anything outside the 6 invariants + MVP food-vendor gaps.

## D7. Verification discipline
- Every change ships a RED+GREEN falsifiable assertion (MANIFESTO C7).
- Terminal must stay unblocked: no hanging background builds; verify with fresh `cargo test`
  output, never agent narrative.

## D8. Plan precedence — newest outranks older (operator, 2026-07-12)
- **Conflict rule:** when an older roadmap/blueprint and a newer approved decision conflict, the
  **NEWEST wins**. MANIFESTO.md + DECISIONS.md (dated 2026-07-12) are the live source of truth and
  **SUPERSEDE** the 2026-07-11 `ROADMAP-GROUND-TRUTH` and any `MASTER-BUILD-SEQUENCE` / stale
  blueprint citation.
- Concretely: mesh machinery is NOW in-scope (D6 overrides C8's deferral); transport = DTN/BPv7
  (RFC 9171) + QUIC/TCPCLv4 + BIBE (D3), NOT libp2p; PQ is a protocol not primitives (C12/D4).
- Before acting on a roadmap tier, check the date + D-series. If conflict → follow the newer artifact.
- Mirrored in `bebop-repo/docs/RULES.md` (precedence setting, Anu line).

## D9. Anu QRNG wiring — native entropy is DEFAULT + FALLBACK (operator, 2026-07-12)
- **Remote quantum entropy IS wired in**: `kernel/src/pq/entropy.rs` `provider` module pulls REAL
  vacuum-fluctuation noise from **ANU QRNG** (`qrng.anu.edu.au`) behind the `qrng` feature, mixed via
  `SHAKE256(quantum ‖ os)` (NIST SP 800-90B: never raw quantum alone).
- **Native (OS) entropy is the DEFAULT and the FALLBACK.** The sanctioned entry point
  `entropy::master_seed()` returns OS `/dev/urandom` seed by default; when `qrng` is on AND ANU is
  reachable it upgrades to quantum⊕os; on ANY failure (unreachable / parse error / offline) it
  **transparently falls back to pure OS entropy** — no error, no broken boot.
- **Rule:** a node MUST boot and produce entropy identically offline (OS-only) and online
  (quantum-boosted). The application must NEVER hard-depend on the remote QRNG being available.
- Production note: the bundled `mini_get` reaches ANU over plain TCP (ponytail stub). Real deployments
  MUST gate a TLS client (reqwest/rustls) behind a `qrng-tls` feature — tracked, not silently shipped.
- Cross-repo: bebop's `bebop2` entropy seam follows the same DEFAULT+FALLBACK contract.
