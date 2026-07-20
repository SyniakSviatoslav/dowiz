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

## D10. R-3 RootDelegationPolicy = Option A (OperatorSigned + per-anchor IssuanceBudget) — RULING RECORDED (operator, 2026-07-18)

- **Ruling:** adopt **Option A** — `RootDelegationPolicy::OperatorSigned` bound to a per-anchor
  monotonic `IssuanceBudget` predicate checked at delegation-sign time (`can_issue` / `charge_issuance`
  / `sign_delegation_budgeted`). Dated **2026-07-18**.
- **Authority / FLAG for override:** recorded under the **expanded autopilot mandate** (operator-
  authorized red-line/decision execution, dated 2026-07-18). **FLAGGED — the operator MAY OVERRIDE at
  any time.** This is a recorded ruling, not a lock; a future operator choice of B (`FirstContactQr` +
  hardware attestation), C (`WebOfTrust`), or a named hybrid supersedes it. Cross-ref:
  `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-D-consensus-capability.md` §11;
  `docs/design/CORE-ROADMAP-2026-07-17/P-D-audit-root-delegation-policy.md`.
- **Why A:** the P-D audit's recommended default — ships on today's `AnchorRoster`/`verify_chain`
  substrate, fully sovereign (operator is the only root; no Google/Apple attestation dependency),
  P06-independent, and it closes the Batch-7 Sybil residual (bounded per-epoch issuance). Options B and
  C remain unwired stubs and are **NOT adopted**.
- **Mechanism already built — no code change in this ruling:** `bebop-repo` commit `e08eb07`
  (`bebop2/proto-cap/src/node_id.rs:187-372`; `IssuanceBudget` / `IssuanceError` / `can_issue` /
  `charge_issuance` / `sign_delegation_budgeted`; 10 RED→GREEN tests; CI gate
  `scripts/ci-budgeted-issuance.sh`). This D10 entry is a **ruling RECORD**, not a code change. Per
  `BLUEPRINT-P-D-consensus-capability.md` §11 anti-scope, B's attestation overlay and C's flow-based
  construction are explicitly NOT adopted by this ruling.
- **Operator deployment actions still required (ops, not code):** (i) generate + Ed25519-sign the real
  anchor root cert(s) and populate a production `genesis.example.txt`-shaped anchor file
  (`load_genesis`, `node_id.rs:117-142`); (ii) set the production node's runtime `RootDelegationPolicy`
  to `OperatorSigned` **explicitly** — keep `Default = Unspecified` (`node_id.rs:169-174`, never flip the
  code default); (iii) confirm or override `DEFAULT_MAX_PER_EPOCH` (currently `1`, `:203`) against real
  onboarding throughput.
- **Standing anti-scope (unchanged):** do NOT silently change `DEFAULT_MAX_PER_EPOCH` or write a real
  production genesis/anchor file without this ruling on record; do NOT build B/C speculatively; do NOT
  re-couple R-3 to P06 `key_V` (independent per the audit §3).

## D11. Governed Self-Evolution (items 73-78) — apply-token, boundary & sequencing rulings (operator, 2026-07-20)

Rules the 7 open governance questions filed in
`docs/design/BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md` §9. Items 73-78 build the
channel by which the product's own AI proposes changes to its own kernel — the single most
red-line-adjacent capability in the space-grade roadmap. These are spec-plane rulings; no code
exists yet against them.

- **Q1 — apply-token holder:** each node's own operator, fully local. No delegated authority, no
  mesh-wide root — matches D0 (decentralized/local-first/mesh) most literally. A given node's
  self-change proposals are approved only by whoever runs that node.
- **Q2 — what the apply-token physically is:** a **2-factor** design —
  1. a `capability_cert` (ML-DSA-65) scoped to a new `apply:change-proposal` capability, reusing the
     existing `AnchorRoster`/`capability_cert.rs` enrollment substrate (the enrolled key already
     binds to a specific device — no separate device-id field needed);
  2. a live, time-boxed rotating code, TOTP-shaped (RFC 6238 semantics — a code that changes every
     ~30s off a shared secret + time) but HMAC'd with **SHA3-256** (the kernel's existing Keccak
     module) instead of classical HMAC-SHA1, so the second factor stays PQ-consistent and adds zero
     new hash primitives (one-escaper-implementation discipline, same rule as the kernel::json/
     fdr::json dedup ticket).
  Rationale: a stolen long-term signing key alone must not be sufficient; the rotating factor proves
  live human presence at the moment of approval. **Open sub-question, deliberately NOT ruled here:**
  the rotating-secret enrollment ceremony — defer to item 75/64's design.
- **Q3 — Pending-approval TTL:** dynamic and operator-configurable per node, **default 24 hours**.
  Not a fixed constant — item 75's `Pending` state must carry a configurable expiry, not a compiled-in
  number.
- **Q4 — the meta-governance boundary (item 74's registry, the AI's own safety perimeter):** the AI
  MAY reach and edit `agent scope` (`kernel/src/ports/agent/scope.rs`) and the governance module
  itself (items 73-76's own code) — self-evolution is allowed there, gated by the human apply-token
  per D11's Q1/Q2. The ONLY hard, non-negotiable line is **core kernel authority + the circuit
  breaker** (`kernel/src/breaker/`, item 9; the order/money/decide-fold core). Everything else is
  configurable/customizable by the human operator, who may narrow the boundary further at any time —
  the boundary itself is operator-drawn, never agent-drawn, per the blueprint's own Q4 framing.
- **Q5 — red-line registry row-removal ruling format:** a `DECISIONS.md` D-entry (this file) — matches
  existing precedent (D0-D10), keeps one canonical place for every red-line ruling, old and new. No
  separate ruling-doc format or commit-trailer convention.
- **Q6 — item 77 self-heal threshold:** **3 consecutive** adverse windows before a fix proposal may
  be opened. Still just a proposal — Q1/Q2's apply-token is required regardless of threshold.
- **Q7 — build sequencing:** item 77 (self-healing) before item 78 (self-upgrading), the roadmap's
  original numeric order — **operator override of the blueprint's own recommendation** (the architect
  suggested 78-before-77 to battle-test the human-gate on lower-urgency, operator-initiated changes
  first). Recorded per D8 (newest/operator ruling outranks the design doc's suggestion).
- **What this ruling does NOT do:** it does not authorize dispatching items 73-78 to code. Per the
  blueprint's §10, the transition from spec to code for this arc remains its own explicit decision;
  this D11 entry resolves the *content* of that future code's design, not the *go* to start writing
  it. Item 75's `apply` seam specifically stays gated on items 64/65 (unbuilt) regardless of this
  ruling.
- **FLAG for override:** recorded under the same operator-ruling authority as D10. The operator MAY
  override any clause here at any time; this is a recorded ruling, not a lock.

## D12. DeliveryOS launch blockers §4-A–D — the four operator decisions from SYNTHESIS-LAUNCH-BLOCKERS (2026-07-20)

Rules the four operator-only decisions flagged in
`docs/design/CORE-ROADMAP-2026-07-17/SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §4. All four block
blueprint-writing for their named surfaces; nothing here reopens any other closed ruling in that
document (§0.2's two rulings, or any §16/§17 decision it cites).

- **§4-A — Card-capture surface (web + desktop): Path A — scoped provider-iframe overlay.** One
  narrow, documented DOM exception at the card moment only, keeping the user inside the immersive
  wgpu canvas experience for the rest of the flow. Mobile stays Path B (native provider SDK sheet,
  zero DOM) as the source doc already recommended — unaffected by this ruling.
  **Real architectural consequence, not a side detail:** per the source doc's X3 dependency table,
  choosing Path A **forces the desktop shell to host a live webview** (Tauri) for the card moment —
  it rules out the alternative the doc flagged as "cleaner" (pure `winit`+`wgpu`+AccessKit with no
  webview at all). P39-rev (app shell/installability) must record Tauri-with-webview as the
  desktop shell decision, not present it as still-open pending the P63 spike; P63's shell spike
  narrows to *validating* the Tauri-webview approach, not choosing between it and a webview-free
  alternative.
- **§4-B — Self-custody severity: absolute, both surfaces, no exceptions.**
  (i) Backup break-glass: **no `dowiz_break_glass_pubkey` in any backup recipient set, ever.**
  dowiz cannot read backup plaintext by construction — a vendor who loses their key loses every
  backup for that hub. (ii) Owner-root loss: **no recovery path.** A lost owner root eventually
  strands its hub fleet as certs expire under the short-TTL revocation mechanism; this is accepted
  as the cost of (i)'s guarantee applied consistently, not a separate looser policy for keys that
  happen to gate more blast radius. One philosophy, both surfaces — exactly the consistency the
  source doc asked for. P59's revocation section and P68's backup-recipient section must both cite
  this ruling; neither may introduce a recovery mechanism independently.
- **§4-C — Abandoned-claimed-hub power-down: no suspension, every claimed hub stays hot
  indefinitely.** A long-inactive claimed hub is NOT suspended/compute-released — it remains fully
  provisioned and running for as long as it is claimed, consistent with §16.57's "stays the
  vendor's forever" read the most literal way. **Real cost consequence, not a side detail:** the
  warm-pool is net-consumed with zero recycling from inactive hubs; P67's pool-economics section
  must size the pool (and its refill cadence) assuming claimed capacity never returns to the pool,
  not assuming a suspend-and-reclaim offset.
- **§4-D — Food-court Wave-0 market scope: Albania / EU**, matching `PRODUCT.md`'s existing primary
  market (Albania + EN/UK speakers). P72's per-vendor provider-account onboarding and provider
  matrix are scoped to providers that support per-vendor split/connect charges in EUR/ALL — not a
  global-from-day-one matrix. §16.20's requirement that the *architecture* stay market-agnostic is
  unaffected; this only scopes where the *feature* is proven first.
- **Unblocked by this ruling:** blueprint-writing may now proceed for P39-rev, P59, P60 (client
  leg), P63, P67, P68, P69, P72 — all four were named blockers in the source doc's swarm-dispatch
  summary (§5 "Before W1 writing starts, raise §4-A–D with the operator").
- **FLAG for override:** recorded under the same operator-ruling authority as D8/D10/D11. The
  operator MAY override any clause here at any time; this is a recorded ruling, not a lock.
